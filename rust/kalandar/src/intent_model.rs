use super::CaptureParserError;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TinyIntentModel {
    pub(crate) version: u32,
    pub(crate) labels: Vec<String>,
    pub(crate) word_bins: usize,
    pub(crate) char_bins: usize,
    pub(crate) intercepts: Vec<f64>,
    pub(crate) classes: Vec<TinyIntentClass>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TinyIntentClass {
    pub(crate) label: String,
    pub(crate) indices: Vec<usize>,
    pub(crate) values: Vec<f64>,
}


pub(crate) fn embedded_tiny_intent_model() -> &'static TinyIntentModel {
    static MODEL: OnceLock<TinyIntentModel> = OnceLock::new();
    MODEL.get_or_init(|| {
        let model: TinyIntentModel =
            serde_json::from_str(include_str!("../resources/tiny_intent_model.json"))
                .expect("embedded tiny intent model should parse");
        validate_model(&model).expect("embedded tiny intent model should be valid");
        model
    })
}

pub(crate) fn classify_probabilities(model: &TinyIntentModel, normalized: &str) -> [f64; 4] {
    let features = featurize(model, normalized);
    let logits = model
        .classes
        .iter()
        .enumerate()
        .map(|(index, class_model)| {
            dot_sparse(&features, &class_model.indices, &class_model.values)
                + model.intercepts.get(index).copied().unwrap_or(0.0)
        })
        .collect::<Vec<_>>();
    let probabilities = softmax(&logits);

    let mut scores = [0.0; 4];
    for (index, label) in model.labels.iter().enumerate() {
        let slot = match label.as_str() {
            "task" => 0,
            "calendar" => 1,
            "reminder" => 2,
            "none" => 3,
            _ => continue,
        };
        scores[slot] = probabilities.get(index).copied().unwrap_or(0.0);
    }
    scores
}

pub(crate) fn validate_model(model: &TinyIntentModel) -> Result<(), CaptureParserError> {
    let feature_bins = model
        .word_bins
        .checked_add(model.char_bins)
        .ok_or(CaptureParserError::ManifestInvalid)?;
    if model.version == 0
        || model.word_bins == 0
        || model.char_bins == 0
        || model.labels.is_empty()
        || model.intercepts.len() != model.labels.len()
        || model.classes.len() != model.labels.len()
    {
        return Err(CaptureParserError::ManifestInvalid);
    }

    for (index, class_model) in model.classes.iter().enumerate() {
        if class_model.label != model.labels[index]
            || class_model.indices.len() != class_model.values.len()
            || class_model
                .indices
                .iter()
                .any(|feature_index| *feature_index >= feature_bins)
        {
            return Err(CaptureParserError::ManifestInvalid);
        }
    }

    Ok(())
}

fn featurize(model: &TinyIntentModel, normalized: &str) -> BTreeMap<usize, f64> {
    let tokens = normalized
        .split(' ')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::new();

    for token in &tokens {
        let index = (fnv1a32(&format!("w:{token}")) as usize) % model.word_bins;
        *counts.entry(index).or_insert(0.0) += 1.0;
    }

    for window in tokens.windows(2) {
        let bigram = format!("{} {}", window[0], window[1]);
        let index = (fnv1a32(&format!("wb:{bigram}")) as usize) % model.word_bins;
        *counts.entry(index).or_insert(0.0) += 1.0;
    }

    let char_offset = model.word_bins;
    for gram in collect_char_ngrams(normalized, 3, 5) {
        let index = char_offset + ((fnv1a32(&format!("c:{gram}")) as usize) % model.char_bins);
        *counts.entry(index).or_insert(0.0) += 1.0;
    }

    let l2 = counts
        .values()
        .fold(0.0_f64, |total, value| total + value * value)
        .sqrt();
    let norm = if l2 == 0.0 || l2.is_nan() { 1.0 } else { l2 };
    counts
        .into_iter()
        .map(|(index, value)| (index, value / norm))
        .collect()
}

fn collect_char_ngrams(text: &str, minimum: usize, maximum: usize) -> Vec<String> {
    let mut grams = Vec::new();
    for token in text.split(' ').filter(|token| !token.is_empty()) {
        let padded = format!(" {token} ").encode_utf16().collect::<Vec<_>>();
        for size in minimum..=maximum {
            grams.extend(padded.windows(size).map(String::from_utf16_lossy));
        }
    }
    grams
}

fn fnv1a32(value: &str) -> u32 {
    let mut hashed = 0x811c9dc5_u32;
    for byte in value.as_bytes() {
        hashed ^= u32::from(*byte);
        hashed = hashed.wrapping_mul(0x01000193);
    }
    hashed
}

fn dot_sparse(features: &BTreeMap<usize, f64>, indices: &[usize], values: &[f64]) -> f64 {
    indices
        .iter()
        .zip(values.iter())
        .fold(0.0, |total, (feature_index, weight)| {
            features
                .get(feature_index)
                .map_or(total, |feature_value| total + feature_value * weight)
        })
}

fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exps = logits
        .iter()
        .map(|value| (value - max).exp())
        .collect::<Vec<_>>();
    let sum = exps.iter().fold(0.0, |total, value| total + value);
    let sum = if sum == 0.0 || sum.is_nan() { 1.0 } else { sum };
    exps.into_iter().map(|value| value / sum).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    fn valid_model() -> TinyIntentModel {
        TinyIntentModel {
            version: 1,
            labels: vec![
                "task".to_string(),
                "calendar".to_string(),
                "none".to_string(),
            ],
            word_bins: 8,
            char_bins: 8,
            intercepts: vec![0.1, 0.2, 0.3],
            classes: vec![
                TinyIntentClass {
                    label: "task".to_string(),
                    indices: vec![0, 3, 7],
                    values: vec![0.1, 0.2, 0.3],
                },
                TinyIntentClass {
                    label: "calendar".to_string(),
                    indices: vec![1, 4, 8],
                    values: vec![0.1, 0.2, 0.3],
                },
                TinyIntentClass {
                    label: "none".to_string(),
                    indices: vec![2, 5, 9],
                    values: vec![0.1, 0.2, 0.3],
                },
            ],
        }
    }

    #[test]
    fn validate_model_rejects_reordered_class_entries() {
        let mut model = valid_model();
        model.classes.swap(0, 1);

        assert_eq!(
            validate_model(&model),
            Err(CaptureParserError::ManifestInvalid)
        );
    }

    #[test]
    fn validate_model_rejects_out_of_range_feature_indices() {
        let mut model = valid_model();
        model.classes[0]
            .indices
            .push(model.word_bins + model.char_bins);
        model.classes[0].values.push(0.4);

        assert_eq!(
            validate_model(&model),
            Err(CaptureParserError::ManifestInvalid)
        );
    }
}
