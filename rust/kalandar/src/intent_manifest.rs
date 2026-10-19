use super::intent::{classify_intent_with_model, IntentRoute};
use super::intent_model::{validate_model, TinyIntentModel};
use super::{CaptureParserArtifactManifest, CaptureParserError, TINY_LINEAR_RUNTIME_KIND};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

pub fn classify_intent(
    manifest: &CaptureParserArtifactManifest,
    input: &str,
) -> Result<IntentRoute, CaptureParserError> {
    let model = load_tiny_intent_model(manifest)?;
    Ok(classify_intent_with_model(model.as_ref(), input))
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
    let key = (
        manifest.resolve_file_path(model_file),
        model_file.sha256.clone(),
    );

    let cache = intent_model_cache()
        .lock()
        .expect("intent model cache should lock");
    if let Some(model) = cache.get(&key) {
        return Ok(Arc::clone(model));
    }
    drop(cache);

    let bytes = crate::read_verified_artifact(manifest, model_file)?;
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
    cache.insert(key, Arc::clone(&parsed));
    Ok(parsed)
}

type IntentModelKey = (PathBuf, String);

fn intent_model_cache() -> &'static Mutex<BTreeMap<IntentModelKey, Arc<TinyIntentModel>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<IntentModelKey, Arc<TinyIntentModel>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}
