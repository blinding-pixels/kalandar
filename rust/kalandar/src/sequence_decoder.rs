use serde::Deserialize;

pub const SEQUENCE_DECODER_LOGICAL_NAME: &str = "sequence_decoder";
pub const SEQUENCE_DECODER_ALGORITHM: &str = "BIO Viterbi";
pub const SEQUENCE_DECODER_TIE_BREAK: &str = "lowest predecessor ID, then lowest final ID";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SequenceDecoding {
    #[default]
    Argmax,
    Bio,
}

impl SequenceDecoding {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "argmax" => Some(Self::Argmax),
            "bio" => Some(Self::Bio),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Argmax => "argmax",
            Self::Bio => "bio",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceDecoderError {
    InvalidLabels,
    InvalidLogits,
    InvalidSidecar,
}

impl SequenceDecoderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidLabels => "sequence-decoder-invalid-labels",
            Self::InvalidLogits => "sequence-decoder-invalid-logits",
            Self::InvalidSidecar => "sequence-decoder-invalid-sidecar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhitespaceWord {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSpan {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct SequenceDecoderSidecar {
    algorithm: String,
    first_subword_piece_only: bool,
    accumulator: String,
    start_scores: Vec<Option<f64>>,
    transition_scores_previous_by_next: Vec<Vec<Option<f64>>>,
    end_scores: Vec<Option<f64>>,
    null_score_means: String,
    tie_break: String,
    labels: Vec<String>,
}

fn is_python_whitespace(value: char) -> bool {
    value.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&value)
}

pub fn whitespace_words(text: &str) -> Vec<WhitespaceWord> {
    let mut words = Vec::new();
    let mut start = None;
    for (index, value) in text.char_indices() {
        match (is_python_whitespace(value), start) {
            (true, Some(word_start)) => {
                words.push(WhitespaceWord {
                    start: word_start,
                    end: index,
                });
                start = None;
            }
            (false, None) => start = Some(index),
            _ => {}
        }
    }
    if let Some(word_start) = start {
        words.push(WhitespaceWord {
            start: word_start,
            end: text.len(),
        });
    }
    words
}

pub fn align_first_pieces(
    words: &[WhitespaceWord],
    offsets: &[(usize, usize)],
) -> Vec<Option<usize>> {
    let mut first_pieces = vec![None; words.len()];
    let mut word_index = 0;
    for (token_index, &(start, end)) in offsets.iter().enumerate() {
        if end <= start {
            continue;
        }
        while word_index < words.len() && words[word_index].end <= start {
            word_index += 1;
        }
        let Some(word) = words.get(word_index) else {
            break;
        };
        if start >= word.start && first_pieces[word_index].is_none() {
            first_pieces[word_index] = Some(token_index);
        }
    }
    first_pieces
}

pub fn validate_bio_labels<S: AsRef<str>>(labels: &[S]) -> Result<(), SequenceDecoderError> {
    let labels = labels.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let mut seen = std::collections::BTreeSet::new();
    if !labels.iter().all(|label| seen.insert(*label)) || !labels.contains(&"O") {
        return Err(SequenceDecoderError::InvalidLabels);
    }
    for label in &labels {
        if *label == "O" {
            continue;
        }
        let field = label
            .strip_prefix("B-")
            .or_else(|| label.strip_prefix("I-"))
            .ok_or(SequenceDecoderError::InvalidLabels)?;
        if field.is_empty() {
            return Err(SequenceDecoderError::InvalidLabels);
        }
        if label.starts_with("I-") && !labels.contains(&format!("B-{field}").as_str()) {
            return Err(SequenceDecoderError::InvalidLabels);
        }
    }
    Ok(())
}

pub fn bio_start_allowed<S: AsRef<str>>(labels: &[S]) -> Vec<bool> {
    labels
        .iter()
        .map(|label| !label.as_ref().starts_with("I-"))
        .collect()
}

pub fn bio_transition_allowed<S: AsRef<str>>(labels: &[S]) -> Vec<Vec<bool>> {
    labels
        .iter()
        .map(|previous| {
            let previous = previous.as_ref();
            labels
                .iter()
                .map(|next| {
                    let next = next.as_ref();
                    match next.strip_prefix("I-") {
                        Some(field) => previous == next || previous == format!("B-{field}"),
                        None => true,
                    }
                })
                .collect()
        })
        .collect()
}

fn validate_logits<R, T>(logits: &[R], label_count: usize) -> Result<(), SequenceDecoderError>
where
    R: AsRef<[T]>,
    T: Copy + Into<f64>,
{
    let valid = logits.iter().all(|row| {
        let row = row.as_ref();
        row.len() == label_count && row.iter().all(|value| (*value).into().is_finite())
    });
    if valid {
        Ok(())
    } else {
        Err(SequenceDecoderError::InvalidLogits)
    }
}

fn argmax_index<T: Copy + Into<f64>>(row: &[T]) -> usize {
    row.iter()
        .enumerate()
        .fold(
            (0, f64::NEG_INFINITY),
            |(best_index, best), (index, value)| {
                let value = (*value).into();
                if value > best {
                    (index, value)
                } else {
                    (best_index, best)
                }
            },
        )
        .0
}

pub fn decode_argmax<R, T, S>(
    logits: &[R],
    labels: &[S],
) -> Result<Vec<usize>, SequenceDecoderError>
where
    R: AsRef<[T]>,
    T: Copy + Into<f64>,
    S: AsRef<str>,
{
    validate_bio_labels(labels)?;
    validate_logits(logits, labels.len())?;
    Ok(logits
        .iter()
        .map(|row| argmax_index(row.as_ref()))
        .collect())
}

pub fn decode_bio_viterbi<R, T, S>(
    logits: &[R],
    labels: &[S],
) -> Result<Vec<usize>, SequenceDecoderError>
where
    R: AsRef<[T]>,
    T: Copy + Into<f64>,
    S: AsRef<str>,
{
    validate_bio_labels(labels)?;
    validate_logits(logits, labels.len())?;
    let Some((first, rest)) = logits.split_first() else {
        return Ok(Vec::new());
    };

    let label_count = labels.len();
    let start_allowed = bio_start_allowed(labels);
    let allowed = bio_transition_allowed(labels);
    let mut scores = first
        .as_ref()
        .iter()
        .zip(&start_allowed)
        .map(|(value, legal)| {
            if *legal {
                (*value).into()
            } else {
                f64::NEG_INFINITY
            }
        })
        .collect::<Vec<f64>>();
    let mut backpointers = Vec::with_capacity(rest.len());

    for row in rest {
        let row = row.as_ref();
        let (next, pointers): (Vec<f64>, Vec<usize>) = (0..label_count)
            .map(|label| {
                let (previous, best) = (0..label_count)
                    .filter(|previous| allowed[*previous][label])
                    .fold((0, f64::NEG_INFINITY), |(best_previous, best), previous| {
                        if scores[previous] > best {
                            (previous, scores[previous])
                        } else {
                            (best_previous, best)
                        }
                    });
                (row[label].into() + best, previous)
            })
            .unzip();
        backpointers.push(pointers);
        scores = next;
    }

    let mut current = argmax_index(&scores);
    let mut path = vec![current];
    for pointers in backpointers.iter().rev() {
        current = pointers[current];
        path.push(current);
    }
    path.reverse();
    Ok(path)
}

pub fn decode_sequence<R, T, S>(
    logits: &[R],
    labels: &[S],
    decoding: SequenceDecoding,
) -> Result<Vec<usize>, SequenceDecoderError>
where
    R: AsRef<[T]>,
    T: Copy + Into<f64>,
    S: AsRef<str>,
{
    match decoding {
        SequenceDecoding::Argmax => decode_argmax(logits, labels),
        SequenceDecoding::Bio => decode_bio_viterbi(logits, labels),
    }
}

fn label_field(label: &str) -> &str {
    label
        .split_once('-')
        .map(|(_, field)| field)
        .unwrap_or(label)
}

pub fn rebuild_word_spans<S: AsRef<str>>(
    text: &str,
    words: &[WhitespaceWord],
    word_labels: &[S],
) -> Vec<DecodedSpan> {
    let mut spans: Vec<DecodedSpan> = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let label = word_labels.get(index).map(AsRef::as_ref).unwrap_or("O");
        if label == "O" {
            continue;
        }
        let field = label_field(label);
        let previous = index
            .checked_sub(1)
            .and_then(|previous| word_labels.get(previous))
            .map(AsRef::as_ref)
            .unwrap_or("O");
        let continues =
            label.starts_with("I-") && previous != "O" && label_field(previous) == field;
        match spans.last_mut() {
            Some(span) if continues => span.end = word.end,
            _ => spans.push(DecodedSpan {
                label: field.to_string(),
                start: word.start,
                end: word.end,
                text: String::new(),
            }),
        }
    }
    for span in &mut spans {
        span.text = text[span.start..span.end].to_string();
    }
    spans
}

pub fn decode_first_piece_labels<S: AsRef<str>>(
    text: &str,
    offsets: &[(usize, usize)],
    logits: &[f32],
    labels: &[S],
    decoding: SequenceDecoding,
) -> Result<Vec<String>, SequenceDecoderError> {
    let label_count = labels.len();
    if label_count == 0 || logits.len() != offsets.len() * label_count {
        return Err(SequenceDecoderError::InvalidLogits);
    }
    let words = whitespace_words(text);
    let first_pieces = align_first_pieces(&words, offsets);
    let represented = first_pieces
        .iter()
        .enumerate()
        .filter_map(|(word_index, token_index)| token_index.map(|token| (word_index, token)))
        .collect::<Vec<_>>();
    let rows = represented
        .iter()
        .map(|(_, token)| &logits[token * label_count..(token + 1) * label_count])
        .collect::<Vec<_>>();
    let label_ids = decode_sequence(&rows, labels, decoding)?;
    let mut word_labels = vec!["O".to_string(); words.len()];
    for ((word_index, _), label_id) in represented.iter().zip(label_ids) {
        word_labels[*word_index] = labels[label_id].as_ref().to_string();
    }
    Ok(word_labels)
}

pub fn decode_first_piece_spans<S: AsRef<str>>(
    text: &str,
    offsets: &[(usize, usize)],
    logits: &[f32],
    labels: &[S],
    decoding: SequenceDecoding,
) -> Result<Vec<DecodedSpan>, SequenceDecoderError> {
    let word_labels = decode_first_piece_labels(text, offsets, logits, labels, decoding)?;
    Ok(rebuild_word_spans(
        text,
        &whitespace_words(text),
        &word_labels,
    ))
}

pub fn validate_sequence_decoder_sidecar<S: AsRef<str>>(
    bytes: &[u8],
    labels: &[S],
) -> Result<(), SequenceDecoderError> {
    let sidecar: SequenceDecoderSidecar =
        serde_json::from_slice(bytes).map_err(|_| SequenceDecoderError::InvalidSidecar)?;
    validate_bio_labels(labels)?;
    let legal_score = |legal: &bool| if *legal { Some(0.0) } else { None };
    let expected_start = bio_start_allowed(labels)
        .iter()
        .map(legal_score)
        .collect::<Vec<_>>();
    let expected_transitions = bio_transition_allowed(labels)
        .iter()
        .map(|row| row.iter().map(legal_score).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let valid = sidecar.algorithm == SEQUENCE_DECODER_ALGORITHM
        && sidecar.first_subword_piece_only
        && sidecar.accumulator == "float64"
        && sidecar.null_score_means == "negative infinity"
        && sidecar.tie_break == SEQUENCE_DECODER_TIE_BREAK
        && sidecar.labels.len() == labels.len()
        && sidecar
            .labels
            .iter()
            .zip(labels)
            .all(|(left, right)| left == right.as_ref())
        && sidecar.start_scores == expected_start
        && sidecar.transition_scores_previous_by_next == expected_transitions
        && sidecar.end_scores.iter().all(|score| *score == Some(0.0))
        && sidecar.end_scores.len() == labels.len();
    if valid {
        Ok(())
    } else {
        Err(SequenceDecoderError::InvalidSidecar)
    }
}
