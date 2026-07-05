use super::{CaptureParserArtifactManifest, CaptureParserError, TINY_LINEAR_RUNTIME_KIND};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

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

pub(crate) fn load_tiny_intent_model(
    manifest: &CaptureParserArtifactManifest,
) -> Result<Arc<TinyIntentModel>, CaptureParserError> {
    let bundle = manifest
        .bundle("intent")
        .ok_or(CaptureParserError::ManifestInvalid)?;
    if bundle.runtime_kind != TINY_LINEAR_RUNTIME_KIND {
        return Err(CaptureParserError::ManifestInvalid);
    }

    let model_file = bundle
        .files
        .iter()
        .find(|file| file.logical_name == "model_json")
        .ok_or(CaptureParserError::ManifestInvalid)?;
    let canonical = manifest
        .resolve_file_path(model_file)
        .canonicalize()
        .map_err(|_| CaptureParserError::ArtifactMissing)?;

    let cache = intent_model_cache()
        .lock()
        .expect("intent model cache should lock");
    if let Some(model) = cache.get(&canonical) {
        return Ok(Arc::clone(model));
    }
    drop(cache);

    let bytes = fs::read(&canonical).map_err(|_| CaptureParserError::ArtifactMissing)?;
    let parsed: TinyIntentModel =
        serde_json::from_slice(&bytes).map_err(|_| CaptureParserError::ManifestInvalid)?;
    validate_model(&parsed)?;
    if parsed.labels != bundle.labels {
        return Err(CaptureParserError::ManifestInvalid);
    }
    let parsed = Arc::new(parsed);

    let mut cache = intent_model_cache()
        .lock()
        .expect("intent model cache should lock");
    cache.insert(canonical, Arc::clone(&parsed));
    Ok(parsed)
}

pub(crate) fn classify_probabilities(model: &TinyIntentModel, normalized: &str) -> [f64; 3] {
    // The current tiny intent model is intentionally 3-way: task, calendar, none.
    // Reminder routing stays heuristic-owned until a retrained 4-way artifact set lands.
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

    let mut task = 0.0;
    let mut calendar = 0.0;
    let mut none = 0.0;
    for (index, label) in model.labels.iter().enumerate() {
        match label.as_str() {
            "task" => task = probabilities.get(index).copied().unwrap_or(0.0),
            "calendar" => calendar = probabilities.get(index).copied().unwrap_or(0.0),
            "none" => none = probabilities.get(index).copied().unwrap_or(0.0),
            _ => {}
        }
    }

    [task, calendar, none]
}

fn validate_model(model: &TinyIntentModel) -> Result<(), CaptureParserError> {
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
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    let norm = if l2 == 0.0 { 1.0 } else { l2 };
    counts
        .into_iter()
        .map(|(index, value)| (index, value / norm))
        .collect()
}

fn collect_char_ngrams(text: &str, minimum: usize, maximum: usize) -> Vec<String> {
    let mut grams = Vec::new();
    for token in text.split(' ').filter(|token| !token.is_empty()) {
        let padded = format!(" {token} ");
        let chars = padded.chars().collect::<Vec<_>>();
        for size in minimum..=maximum {
            if chars.len() < size {
                continue;
            }
            for index in 0..=(chars.len() - size) {
                grams.push(chars[index..index + size].iter().collect());
            }
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
        .filter_map(|(feature_index, weight)| {
            features
                .get(feature_index)
                .map(|feature_value| feature_value * weight)
        })
        .sum()
}

fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits
        .iter()
        .copied()
        .max_by(|left, right| left.total_cmp(right))
        .unwrap_or(0.0);
    let exps = logits
        .iter()
        .map(|value| (value - max).exp())
        .collect::<Vec<_>>();
    let sum = exps.iter().sum::<f64>();
    if sum == 0.0 {
        return vec![0.0; logits.len()];
    }
    exps.into_iter().map(|value| value / sum).collect()
}

fn intent_model_cache() -> &'static Mutex<BTreeMap<PathBuf, Arc<TinyIntentModel>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Arc<TinyIntentModel>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
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
