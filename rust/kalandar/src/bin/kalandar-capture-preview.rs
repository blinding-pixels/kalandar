use kalandar::{CaptureParser, CaptureParserError, CapturePreviewRequest, CapturePreviewResult};
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
    let options = parse_options()?;
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read request JSON from stdin: {error}"))?;
    let request: CapturePreviewRequest = serde_json::from_str(&input)
        .map_err(|error| format!("invalid capture preview request JSON: {error}"))?;

    let parser = match options.manifest_sha256 {
        Some(manifest_sha256) => {
            CaptureParser::with_manifest_sha256(options.resource_root, &manifest_sha256)
        }
        None => CaptureParser::new(options.resource_root),
    };
    let result = match parser {
        Ok(parser) => parser.preview_capture(request),
        Err(error) => unsupported_from_error(error),
    };
    let output = serde_json::to_string(&result)
        .map_err(|error| format!("failed to serialize capture preview result: {error}"))?;
    println!("{output}");
    Ok(())
}

struct CliOptions {
    resource_root: PathBuf,
    manifest_sha256: Option<String>,
}

fn parse_options() -> Result<CliOptions, String> {
    let mut args = env::args().skip(1);
    let mut resource_root = env::var_os("KALANDAR_CAPTURE_RESOURCE_ROOT").map(PathBuf::from);
    let mut manifest_sha256 = env::var("KALANDAR_CAPTURE_MANIFEST_SHA256").ok();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--resource-root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--resource-root requires a path".to_string())?;
                resource_root = Some(PathBuf::from(value));
            }
            "--manifest-sha256" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--manifest-sha256 requires a digest".to_string())?;
                manifest_sha256 = Some(value);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: kalandar-capture-preview --resource-root <path> [--manifest-sha256 <digest>] < request.json"
                        .to_string(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let resource_root = resource_root.ok_or_else(|| {
        "missing --resource-root or KALANDAR_CAPTURE_RESOURCE_ROOT for capture parser artifacts"
            .to_string()
    })?;
    Ok(CliOptions {
        resource_root,
        manifest_sha256,
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
