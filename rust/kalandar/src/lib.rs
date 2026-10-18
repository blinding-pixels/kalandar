mod artifacts;
mod calendar_lightweight;
mod errors;
mod inference;
mod intent;
mod intent_model;
mod intent_rules;
mod ort_runtime;
mod postprocess;
mod preview_runtime;
mod reminder_lightweight;
mod schedule;
mod sequence_decoder;
mod shadow_text;
mod task_lightweight;
mod tokenizer;
mod types;

pub use artifacts::*;
pub use errors::*;
pub use inference::*;
pub use ort_runtime::*;
pub use sequence_decoder::*;
pub use tokenizer::*;
pub use types::*;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CaptureParser {
    manifest: CaptureParserArtifactManifest,
}

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
