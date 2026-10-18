use kalandar::{
    build_token_classification_request, decode_first_piece_labels, decode_first_piece_spans,
    load_capture_parser_artifact_manifest_from_root, load_ort_session, CaptureWordPieceTokenizer,
    RuntimeTensorData, SequenceDecoding, LOGITS_OUTPUT_TENSOR_NAME,
};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("resource root argument"));
    let manifest =
        load_capture_parser_artifact_manifest_from_root(&root).expect("manifest should load");
    let mut runners = Vec::new();
    for family in ["task", "calendar", "reminder"] {
        let bundle = manifest.bundle(family).expect("bundle").clone();
        let tokenizer = CaptureWordPieceTokenizer::load(&manifest, &bundle).expect("tokenizer");
        let session = load_ort_session(&manifest, &bundle).expect("session");
        runners.push((family, bundle, tokenizer, session));
    }
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(row) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let family = row["family"].as_str().unwrap_or("task");
        let text = row["text"].as_str().unwrap_or("");
        let Some((_, bundle, tokenizer, session)) =
            runners.iter_mut().find(|(name, ..)| *name == family)
        else {
            continue;
        };
        let encoded = tokenizer.encode(text);
        let request =
            build_token_classification_request(&manifest, bundle, &encoded).expect("request");
        let output = session.run(&request).expect("inference");
        let RuntimeTensorData::Float32(logits) = &output.outputs[LOGITS_OUTPUT_TENSOR_NAME].data
        else {
            continue;
        };
        let labels = |mode| {
            decode_first_piece_labels(text, &encoded.offsets, logits, &bundle.labels, mode)
                .unwrap_or_default()
        };
        let spans = decode_first_piece_spans(
            text,
            &encoded.offsets,
            logits,
            &bundle.labels,
            bundle.sequence_decoding().unwrap_or(SequenceDecoding::Argmax),
        )
        .unwrap_or_default();
        let _ = writeln!(
            out,
            "{}",
            json!({
                "id": row["id"],
                "family": family,
                "input_ids": encoded.input_ids,
                "offsets": encoded.offsets,
                "logits": logits,
                "argmax": labels(SequenceDecoding::Argmax),
                "bio": labels(SequenceDecoding::Bio),
                "spans": spans.iter().map(|s| json!({"label": s.label, "start": s.start, "end": s.end, "text": s.text})).collect::<Vec<_>>(),
            })
        );
    }
}
