use kalandar::{
    align_first_pieces, build_token_classification_request, decode_argmax, decode_bio_viterbi,
    decode_first_piece_labels, decode_first_piece_spans,
    load_capture_parser_artifact_manifest_from_root, load_ort_session, rebuild_word_spans,
    validate_sequence_decoder_sidecar, whitespace_words, CaptureParserError,
    CaptureTokenizerConfig, CaptureWordPieceTokenizer, DecodedSpan, RuntimeTensorData,
    SequenceDecoderError, SequenceDecoding, WhitespaceWord, LOGITS_OUTPUT_TENSOR_NAME,
};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

const SMALL_LABELS: [&str; 5] = ["O", "B-a", "I-a", "B-b", "I-b"];

#[derive(Debug, Deserialize)]
struct GoldenCase {
    labels: Vec<String>,
    logits: Vec<Vec<f64>>,
    argmax: Vec<usize>,
    bio: Vec<usize>,
}

#[derive(Debug, Deserialize)]
struct ParitySpan {
    label: String,
    start: usize,
    end: usize,
    text: String,
}

#[derive(Debug, Deserialize)]
struct ParityCase {
    family: String,
    decoding: String,
    text: String,
    argmax: Vec<String>,
    bio: Vec<String>,
    spans: Vec<ParitySpan>,
}

#[derive(Debug, Deserialize)]
struct TokenizerCase {
    text: String,
    input_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenizerFixture {
    vocab: String,
    max_sequence_length: usize,
    cases: Vec<TokenizerCase>,
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn load_fixture<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    let payload = fs::read_to_string(fixture_path(name)).expect("fixture should read");
    serde_json::from_str(&payload).expect("fixture should parse")
}

fn resource_root_or_skip() -> Option<PathBuf> {
    let resource_root = env::var_os("KALANDAR_CAPTURE_RESOURCE_ROOT").map(PathBuf::from);
    if resource_root.is_none() {
        eprintln!("skipping resource-backed capture parser test: KALANDAR_CAPTURE_RESOURCE_ROOT is not set");
    }
    resource_root
}

fn is_legal_path(labels: &[&str], path: &[usize]) -> bool {
    path.iter().enumerate().all(|(position, label_id)| {
        let label = labels[*label_id];
        match label.strip_prefix("I-") {
            None => true,
            Some(field) => {
                position > 0 && {
                    let previous = labels[path[position - 1]];
                    previous == label || previous == format!("B-{field}")
                }
            }
        }
    })
}

fn all_paths(length: usize, label_count: usize) -> Vec<Vec<usize>> {
    (0..length).fold(vec![Vec::new()], |paths, _| {
        paths
            .into_iter()
            .flat_map(|prefix| {
                (0..label_count).map(move |label| {
                    let mut path = prefix.clone();
                    path.push(label);
                    path
                })
            })
            .collect()
    })
}

fn path_score(logits: &[Vec<f64>], path: &[usize]) -> f64 {
    path.iter()
        .enumerate()
        .map(|(position, label)| logits[position][*label])
        .sum()
}

struct Lcg(u32);

impl Lcg {
    fn next_unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.0) / 4_294_967_296.0
    }
}

#[test]
fn golden_cases_match_reference_argmax_and_bio_paths() {
    let cases: Vec<GoldenCase> = load_fixture("decoder_golden.json");
    assert_eq!(cases.len(), 6);
    for case in &cases {
        assert_eq!(
            decode_argmax(&case.logits, &case.labels).expect("argmax should decode"),
            case.argmax
        );
        assert_eq!(
            decode_bio_viterbi(&case.logits, &case.labels).expect("bio should decode"),
            case.bio
        );
    }
}

#[test]
fn bio_viterbi_finds_a_best_legal_path_on_exhaustive_cases() {
    let mut random = Lcg(729);
    for length in 1..=4 {
        let candidates = all_paths(length, SMALL_LABELS.len())
            .into_iter()
            .filter(|path| is_legal_path(&SMALL_LABELS, path))
            .collect::<Vec<_>>();
        for sample in 0..25 {
            let logits = (0..length)
                .map(|_| {
                    (0..SMALL_LABELS.len())
                        .map(|_| {
                            if sample % 5 == 0 {
                                (random.next_unit() * 2.0).round()
                            } else {
                                random.next_unit() * 4.0 - 2.0
                            }
                        })
                        .collect::<Vec<f64>>()
                })
                .collect::<Vec<_>>();
            let actual = decode_bio_viterbi(&logits, &SMALL_LABELS).expect("bio should decode");
            let best = candidates
                .iter()
                .map(|path| path_score(&logits, path))
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(is_legal_path(&SMALL_LABELS, &actual));
            assert_eq!(path_score(&logits, &actual), best);
        }
    }
}

#[test]
fn bio_viterbi_handles_empty_sequences_and_ties() {
    let empty: Vec<Vec<f32>> = Vec::new();
    assert_eq!(decode_bio_viterbi(&empty, &SMALL_LABELS), Ok(Vec::new()));
    assert_eq!(decode_argmax(&empty, &SMALL_LABELS), Ok(Vec::new()));
    assert_eq!(
        decode_bio_viterbi(&[[0.0_f32; 5]; 3], &SMALL_LABELS),
        Ok(vec![0, 0, 0])
    );
    assert_eq!(
        decode_bio_viterbi(
            &[[0.0_f32, 3.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0, 5.0]],
            &SMALL_LABELS
        ),
        Ok(vec![3, 4])
    );
}

#[test]
fn decoders_reject_invalid_input() {
    assert_eq!(
        decode_bio_viterbi(&[[0.0_f32, 0.0]], &["O"]),
        Err(SequenceDecoderError::InvalidLogits)
    );
    assert_eq!(
        decode_bio_viterbi(&[[f32::NAN]], &["O"]),
        Err(SequenceDecoderError::InvalidLogits)
    );
    assert_eq!(
        decode_argmax(&[[f32::INFINITY, 0.0]], &["O", "B-a"]),
        Err(SequenceDecoderError::InvalidLogits)
    );
    for labels in [["O", "I-a"], ["O", "X-a"], ["O", "O"], ["B-a", "I-a"]] {
        assert_eq!(
            decode_bio_viterbi(&[[1.0_f32, 2.0]], &labels),
            Err(SequenceDecoderError::InvalidLabels)
        );
    }
}

#[test]
fn whitespace_words_use_python_whitespace_and_byte_offsets() {
    assert_eq!(
        whitespace_words("  caf\u{e9}\u{a0}with\tsara\u{1f}x\n"),
        vec![
            WhitespaceWord { start: 2, end: 7 },
            WhitespaceWord { start: 9, end: 13 },
            WhitespaceWord { start: 14, end: 18 },
            WhitespaceWord { start: 19, end: 20 },
        ]
    );
    assert!(whitespace_words(" \t ").is_empty());
}

#[test]
fn first_pieces_skip_special_tokens_and_truncated_words() {
    let words = whitespace_words("pick up groceries, today");
    let offsets = [
        (0, 0),
        (0, 4),
        (5, 7),
        (8, 13),
        (13, 17),
        (17, 18),
        (19, 24),
        (0, 0),
    ];
    assert_eq!(
        align_first_pieces(&words, &offsets),
        vec![Some(1), Some(2), Some(3), Some(6)]
    );
    let words = whitespace_words("one two three");
    assert_eq!(
        align_first_pieces(&words, &[(0, 0), (0, 3), (0, 0)]),
        vec![Some(1), None, None]
    );
}

#[test]
fn spans_rebuild_from_whitespace_words() {
    let text = "a b c d e f";
    let spans = rebuild_word_spans(
        text,
        &whitespace_words(text),
        &["B-x", "B-x", "I-x", "O", "I-x", "I-y"],
    );
    let summary = spans
        .iter()
        .map(|span| (span.label.as_str(), span.text.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        summary,
        vec![("x", "a"), ("x", "b c"), ("x", "e"), ("y", "f")]
    );

    let text = "lunch with s\u{e9}ra tomorrow at noon, downtown";
    let spans = rebuild_word_spans(
        text,
        &whitespace_words(text),
        &[
            "B-title",
            "I-title",
            "I-title",
            "B-start_time",
            "I-start_time",
            "I-start_time",
            "B-location",
        ],
    );
    assert_eq!(
        spans,
        vec![
            DecodedSpan {
                label: "title".into(),
                start: 0,
                end: 16,
                text: "lunch with s\u{e9}ra".into()
            },
            DecodedSpan {
                label: "start_time".into(),
                start: 17,
                end: 34,
                text: "tomorrow at noon,".into()
            },
            DecodedSpan {
                label: "location".into(),
                start: 35,
                end: 43,
                text: "downtown".into()
            },
        ]
    );
}

#[test]
fn first_piece_decoding_ignores_later_subword_pieces() {
    let labels = ["O", "B-title", "I-title", "B-due_date", "I-due_date"];
    let text = "finish reports tomorrow";
    let offsets = [(0, 0), (0, 6), (7, 13), (13, 14), (15, 23), (0, 0)];
    let rows: [[f32; 5]; 6] = [
        [9.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 9.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 9.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 9.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 9.0],
        [9.0, 0.0, 0.0, 0.0, 0.0],
    ];
    let logits = rows.concat();
    let argmax =
        decode_first_piece_spans(text, &offsets, &logits, &labels, SequenceDecoding::Argmax)
            .expect("argmax should decode");
    let bio = decode_first_piece_spans(text, &offsets, &logits, &labels, SequenceDecoding::Bio)
        .expect("bio should decode");
    let summarize = |spans: &[DecodedSpan]| {
        spans
            .iter()
            .map(|span| format!("{}={}", span.label, span.text))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        summarize(&argmax),
        vec!["title=finish reports", "due_date=tomorrow"]
    );
    assert_eq!(summarize(&bio), vec!["title=finish reports"]);
}

#[test]
fn sidecar_validation_accepts_matching_rules_and_rejects_drift() {
    let labels = SMALL_LABELS;
    let legal = |allowed: bool| if allowed { Some(0.0) } else { None };
    let transitions = labels
        .iter()
        .map(|previous| {
            labels
                .iter()
                .map(|next| {
                    legal(match next.strip_prefix("I-") {
                        Some(field) => previous == next || *previous == format!("B-{field}"),
                        None => true,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let sidecar = |transitions: &Vec<Vec<Option<f64>>>| {
        serde_json::json!({
            "algorithm": "BIO Viterbi",
            "first_subword_piece_only": true,
            "accumulator": "float64",
            "start_scores": labels.iter().map(|label| legal(!label.starts_with("I-"))).collect::<Vec<_>>(),
            "transition_scores_previous_by_next": transitions,
            "end_scores": labels.iter().map(|_| 0.0).collect::<Vec<_>>(),
            "null_score_means": "negative infinity",
            "tie_break": "lowest predecessor ID, then lowest final ID",
            "labels": labels,
            "runtime_max_tokens": 128
        })
        .to_string()
    };
    assert_eq!(
        validate_sequence_decoder_sidecar(sidecar(&transitions).as_bytes(), &labels),
        Ok(())
    );
    let mut tampered = transitions.clone();
    tampered[1][4] = Some(0.0);
    assert_eq!(
        validate_sequence_decoder_sidecar(sidecar(&tampered).as_bytes(), &labels),
        Err(SequenceDecoderError::InvalidSidecar)
    );
    assert_eq!(
        validate_sequence_decoder_sidecar(sidecar(&transitions).as_bytes(), &["O", "B-a", "I-a"]),
        Err(SequenceDecoderError::InvalidSidecar)
    );
}

#[test]
fn bundled_manifest_declares_family_decodings() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    assert_eq!(manifest.suite_version, "kalandar-capture-parser-onnx-v2");
    for (family, decoding) in [
        ("task", SequenceDecoding::Argmax),
        ("calendar", SequenceDecoding::Bio),
        ("reminder", SequenceDecoding::Argmax),
        ("intent", SequenceDecoding::Argmax),
    ] {
        let bundle = manifest.bundle(family).expect("bundle should exist");
        assert_eq!(bundle.sequence_decoding(), Ok(decoding), "{family}");
    }
    let calendar = manifest
        .bundle("calendar")
        .expect("calendar bundle should exist");
    assert!(calendar
        .files
        .iter()
        .any(|file| file.logical_name == "sequence_decoder"));
}

#[test]
fn bundled_manifest_rejects_a_tampered_sequence_decoder() {
    let Some(resource_source) = resource_root_or_skip() else {
        return;
    };
    let temp = tempdir().expect("temp dir should be created");
    let resource_root = temp.path().join("capture-parser");
    fs::create_dir_all(&resource_root).expect("resource root should be created");
    for entry in ["manifest.json", "intent", "task", "reminder", "calendar"] {
        let source = resource_source.join(entry);
        let destination = resource_root.join(entry);
        if source.is_dir() {
            fs::create_dir_all(&destination).expect("family directory should be created");
            for file in fs::read_dir(&source).expect("family directory should read") {
                let file = file.expect("directory entry should read");
                fs::copy(file.path(), destination.join(file.file_name()))
                    .expect("resource file should copy");
            }
        } else {
            fs::copy(&source, &destination).expect("manifest should copy");
        }
    }
    fs::write(
        resource_root.join("calendar").join("sequence_decoder.json"),
        "{}\n",
    )
    .expect("sidecar should be overwritten");
    assert_eq!(
        load_capture_parser_artifact_manifest_from_root(resource_root.as_path()),
        Err(CaptureParserError::ArtifactIntegrityMismatch)
    );
}

#[test]
fn bundled_models_reproduce_reference_first_piece_decoding() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let cases: Vec<ParityCase> = load_fixture("repair_model_parity.json");
    assert!(cases.len() >= 24);
    for family in ["task", "calendar", "reminder"] {
        let bundle = manifest.bundle(family).expect("bundle should exist");
        let decoding = bundle.sequence_decoding().expect("decoding should parse");
        let tokenizer =
            CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");
        let mut session = load_ort_session(&manifest, bundle).expect("ort session should load");
        for case in cases.iter().filter(|case| case.family == family) {
            assert_eq!(case.decoding, decoding.as_str(), "{}", case.text);
            let encoded = tokenizer.encode(&case.text);
            let request = build_token_classification_request(&manifest, bundle, &encoded)
                .expect("tensor request should build");
            let output = session.run(&request).expect("ort inference should run");
            let RuntimeTensorData::Float32(logits) =
                &output.outputs[LOGITS_OUTPUT_TENSOR_NAME].data
            else {
                panic!("logits output should be float32");
            };
            for (mode, expected) in [
                (SequenceDecoding::Argmax, &case.argmax),
                (SequenceDecoding::Bio, &case.bio),
            ] {
                let actual = decode_first_piece_labels(
                    &case.text,
                    &encoded.offsets,
                    logits,
                    &bundle.labels,
                    mode,
                )
                .expect("first-piece labels should decode");
                assert_eq!(
                    &actual,
                    expected,
                    "{family} {} {}",
                    mode.as_str(),
                    case.text
                );
            }
            let spans = decode_first_piece_spans(
                &case.text,
                &encoded.offsets,
                logits,
                &bundle.labels,
                decoding,
            )
            .expect("spans should decode");
            let expected = case
                .spans
                .iter()
                .map(|span| DecodedSpan {
                    label: span.label.clone(),
                    start: span.start,
                    end: span.end,
                    text: span.text.clone(),
                })
                .collect::<Vec<_>>();
            assert_eq!(spans, expected, "{family} {}", case.text);
        }
    }
}

#[test]
fn tokenizer_matches_hugging_face_ids() {
    let fixture: TokenizerFixture = load_fixture("tokenizer_hf_parity.json");
    let vocab_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(&fixture.vocab);
    let Ok(vocab_text) = fs::read_to_string(&vocab_path) else {
        eprintln!(
            "skipping tokenizer parity test: {} is missing",
            vocab_path.display()
        );
        return;
    };
    let tokenizer = CaptureWordPieceTokenizer::from_vocab(
        &vocab_text,
        CaptureTokenizerConfig {
            max_tokens: fixture.max_sequence_length,
            do_lower_case: true,
            unk_token: "[UNK]".to_string(),
            cls_token: "[CLS]".to_string(),
            sep_token: "[SEP]".to_string(),
            pad_token: "[PAD]".to_string(),
        },
    )
    .expect("tokenizer should build");
    for case in &fixture.cases {
        assert_eq!(
            tokenizer.encode(&case.text).input_ids,
            case.input_ids,
            "{:?}",
            case.text
        );
    }
}
