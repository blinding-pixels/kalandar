mod artifacts;
mod calendar_lightweight;
mod errors;
mod inference;
mod intent;
#[cfg(feature = "onnx")]
mod intent_manifest;
mod intent_model;
mod intent_rules;
#[cfg(feature = "onnx")]
mod ort_runtime;
mod postprocess;
mod preview_runtime;
mod reminder_lightweight;
mod schedule;
mod sequence_decoder;
mod task_lightweight;
mod tokenizer;
mod types;

pub use artifacts::*;
pub use errors::*;
pub use inference::*;
pub use intent::{
    classify_intent_embedded, route_capture_intent, CaptureIntent, IntentKind, IntentRoute,
};
#[cfg(feature = "onnx")]
pub use ort_runtime::*;
pub use preview_runtime::preview_capture_rules;
pub use sequence_decoder::*;
pub use tokenizer::*;
pub use types::*;

#[cfg(feature = "onnx")]
use std::path::{Path, PathBuf};

#[cfg(feature = "onnx")]
#[derive(Debug, Clone)]
pub struct CaptureParser {
    manifest: CaptureParserArtifactManifest,
}

#[cfg(feature = "onnx")]
impl CaptureParser {
    pub fn new(resource_root: impl Into<PathBuf>) -> Result<Self, CaptureParserError> {
        let manifest =
            load_capture_parser_artifact_manifest_from_root(resource_root.into().as_path())?;
        Ok(Self { manifest })
    }

    pub fn resource_root(&self) -> &Path {
        self.manifest.root.as_path()
    }

    pub fn preview_capture(&self, request: CapturePreviewRequest) -> CapturePreviewResult {
        preview_runtime::preview_capture(&self.manifest, request)
    }
}
