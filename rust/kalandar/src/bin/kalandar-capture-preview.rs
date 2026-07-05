use kalandar::{
    CaptureParser, CaptureParserError, CapturePreviewRequest, CapturePreviewResult,
};
use std::{
    env,
    io::{self, Read},
    path::PathBuf,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let resource_root = parse_resource_root()?;
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read request JSON from stdin: {error}"))?;
    let request: CapturePreviewRequest = serde_json::from_str(&input)
        .map_err(|error| format!("invalid capture preview request JSON: {error}"))?;

    let result = match CaptureParser::new(resource_root) {
        Ok(parser) => parser.preview_capture(request),
        Err(error) => unsupported_from_error(error),
    };
    let output = serde_json::to_string(&result)
        .map_err(|error| format!("failed to serialize capture preview result: {error}"))?;
    println!("{output}");
    Ok(())
}

fn parse_resource_root() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    let mut resource_root = env::var_os("KALANDAR_CAPTURE_RESOURCE_ROOT").map(PathBuf::from);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--resource-root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--resource-root requires a path".to_string())?;
                resource_root = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: kalandar-capture-preview --resource-root <path> < request.json"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    resource_root.ok_or_else(|| {
        "missing --resource-root or KALANDAR_CAPTURE_RESOURCE_ROOT for capture parser artifacts"
            .to_string()
    })
}

fn unsupported_from_error(error: CaptureParserError) -> CapturePreviewResult {
    CapturePreviewResult::unsupported(
        error.code(),
        match error {
            CaptureParserError::NotWired => "Capture preview is not wired.",
            CaptureParserError::ManifestMissing => "Capture parser manifest is missing.",
            CaptureParserError::ManifestInvalid => "Capture parser manifest is invalid.",
            CaptureParserError::ArtifactMissing => "Capture parser artifact is missing.",
            CaptureParserError::ArtifactIntegrityMismatch => {
                "Capture parser artifact integrity verification failed."
            }
            CaptureParserError::SessionLoadFailed => "Capture parser session could not be loaded.",
            CaptureParserError::InferenceFailed => "Capture parser inference failed.",
        },
    )
}
