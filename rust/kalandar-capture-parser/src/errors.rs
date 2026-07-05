#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureParserError {
    NotWired,
    ManifestMissing,
    ManifestInvalid,
    ArtifactMissing,
    ArtifactIntegrityMismatch,
    SessionLoadFailed,
    InferenceFailed,
}

impl CaptureParserError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotWired => "host-capture-preview-not-wired",
            Self::ManifestMissing => "capture-parser-manifest-missing",
            Self::ManifestInvalid => "capture-parser-manifest-invalid",
            Self::ArtifactMissing => "capture-parser-artifact-missing",
            Self::ArtifactIntegrityMismatch => "capture-parser-artifact-integrity-mismatch",
            Self::SessionLoadFailed => "capture-parser-session-load-failed",
            Self::InferenceFailed => "capture-parser-inference-failed",
        }
    }
}
