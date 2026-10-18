use super::calendar_lightweight::parse_calendar_preview;
use super::intent::{classify_intent_embedded, CaptureIntent, IntentKind, IntentRoute};
use super::intent_rules::normalize_js_whitespace;
use super::reminder_lightweight::parse_reminder_preview;
use super::task_lightweight::parse_task_preview;
#[cfg(feature = "onnx")]
use super::{
    build_token_classification_request, intent_manifest::classify_intent, load_ort_session,
    CaptureParserArtifactBundle, CaptureParserArtifactManifest, CaptureWordPieceTokenizer,
};
use super::{
    CaptureParserError, CapturePreviewCandidate, CapturePreviewRequest, CapturePreviewResult,
    CapturePreviewWarning,
};
#[cfg(feature = "onnx")]
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

#[cfg(feature = "onnx")]
const TASK_CAPABILITY_SAMPLE: &str = "finish report tomorrow";
#[cfg(feature = "onnx")]
const REMINDER_CAPABILITY_SAMPLE: &str = "remind me to water plants tomorrow at 9";
#[cfg(feature = "onnx")]
const CALENDAR_CAPABILITY_SAMPLE: &str = "lunch with sara tomorrow at noon";

#[cfg(feature = "onnx")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParserCapabilityKey {
    root: PathBuf,
    parser_family: String,
    artifact_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Task,
    Reminder,
    Calendar,
}

impl Family {
    #[cfg(feature = "onnx")]
    fn parser_family(self) -> &'static str {
        match self {
            Family::Task => "task",
            Family::Reminder => "reminder",
            Family::Calendar => "calendar",
        }
    }
}

#[cfg(feature = "onnx")]
pub fn preview_capture(
    manifest: &CaptureParserArtifactManifest,
    request: CapturePreviewRequest,
) -> CapturePreviewResult {
    preview_with(
        request,
        |text| classify_intent(manifest, text),
        |family| verify_family_inference(manifest, family.parser_family()),
    )
}

pub fn preview_capture_rules(request: CapturePreviewRequest) -> CapturePreviewResult {
    preview_with(
        request,
        |text| Ok(classify_intent_embedded(text)),
        |_| Ok(()),
    )
}

fn preview_with(
    request: CapturePreviewRequest,
    route: impl Fn(&str) -> Result<IntentRoute, CaptureParserError>,
    verify: impl Fn(Family) -> Result<(), CaptureParserError>,
) -> CapturePreviewResult {
    let text = normalize_js_whitespace(&request.text);
    if text.is_empty() {
        return text_required();
    }

    for (prefix, family) in [
        ("task:", Family::Task),
        ("reminder:", Family::Reminder),
        ("event:", Family::Calendar),
    ] {
        if let Some(payload) = strip_prefix_case_insensitive(&text, prefix) {
            if payload.is_empty() {
                return text_required();
            }
            return family_result(
                family,
                payload,
                &request,
                0.99,
                Vec::new(),
                Vec::new(),
                &verify,
            );
        }
    }

    let route = match route(&text) {
        Ok(route) => route,
        Err(error) => return unsupported_from_error(error),
    };
    let CaptureIntent {
        confidence,
        candidates,
        warnings,
        ..
    } = route.capture_intent();
    let family = match route.intent {
        IntentKind::Task => Family::Task,
        IntentKind::Calendar => Family::Calendar,
        IntentKind::Reminder => Family::Reminder,
        IntentKind::None => {
            let mut result = CapturePreviewResult::thought(text, confidence, candidates);
            result.warnings = warnings;
            return result;
        }
    };
    family_result(
        family, &text, &request, confidence, candidates, warnings, &verify,
    )
}

fn family_result(
    family: Family,
    text: &str,
    request: &CapturePreviewRequest,
    confidence: f64,
    candidates: Vec<CapturePreviewCandidate>,
    intent_warnings: Vec<CapturePreviewWarning>,
    verify: &impl Fn(Family) -> Result<(), CaptureParserError>,
) -> CapturePreviewResult {
    if let Err(error) = verify(family) {
        return unsupported_from_error(error);
    }
    let mut result = match family {
        Family::Task => {
            let parsed = parse_task_preview(text, request);
            CapturePreviewResult::task(parsed.draft, confidence, candidates, parsed.warnings)
                .with_schedule(parsed.schedule)
        }
        Family::Reminder => {
            let parsed = parse_reminder_preview(text, request);
            CapturePreviewResult::reminder(parsed.draft, confidence, candidates, parsed.warnings)
                .with_schedule(parsed.schedule)
        }
        Family::Calendar => {
            let mut result = parse_calendar_preview(text, request);
            result.confidence = confidence;
            result.candidates = candidates;
            result
        }
    };
    result.warnings = intent_warnings.into_iter().chain(result.warnings).collect();
    result
}

fn text_required() -> CapturePreviewResult {
    CapturePreviewResult::unsupported(
        "capture-text-required",
        "Capture preview requires non-empty text.",
    )
}

#[cfg(feature = "onnx")]
fn verify_family_inference(
    manifest: &CaptureParserArtifactManifest,
    parser_family: &str,
) -> Result<(), CaptureParserError> {
    let bundle = manifest
        .bundle(parser_family)
        .ok_or(CaptureParserError::ManifestInvalid)?;
    let key = ParserCapabilityKey {
        root: manifest.root.clone(),
        parser_family: parser_family.to_string(),
        artifact_id: bundle.artifact_id.clone(),
    };
    let cache = parser_capability_cache()
        .lock()
        .expect("parser capability cache should lock");
    if cache.contains(&key) {
        return Ok(());
    }
    drop(cache);

    run_inference_smoke(manifest, bundle, capability_probe_sample(parser_family)).map(|_| {
        parser_capability_cache()
            .lock()
            .expect("parser capability cache should lock")
            .insert(key);
    })
}

#[cfg(feature = "onnx")]
fn run_inference_smoke(
    manifest: &CaptureParserArtifactManifest,
    bundle: &CaptureParserArtifactBundle,
    text: &str,
) -> Result<(), CaptureParserError> {
    let tokenizer = CaptureWordPieceTokenizer::load(manifest, bundle)?;
    let tokenized = tokenizer.encode(text);
    let request = build_token_classification_request(manifest, bundle, &tokenized)?;
    let mut session = load_ort_session(manifest, bundle)?;
    let _ = session.run(&request)?;
    Ok(())
}

fn strip_prefix_case_insensitive<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| super::js::trim(&text[prefix.len()..]))
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

#[cfg(feature = "onnx")]
fn capability_probe_sample(parser_family: &str) -> &'static str {
    match parser_family {
        "task" => TASK_CAPABILITY_SAMPLE,
        "reminder" => REMINDER_CAPABILITY_SAMPLE,
        "calendar" => CALENDAR_CAPABILITY_SAMPLE,
        _ => "preview smoke",
    }
}

#[cfg(feature = "onnx")]
fn parser_capability_cache() -> &'static Mutex<BTreeSet<ParserCapabilityKey>> {
    static CACHE: OnceLock<Mutex<BTreeSet<ParserCapabilityKey>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeSet::new()))
}
