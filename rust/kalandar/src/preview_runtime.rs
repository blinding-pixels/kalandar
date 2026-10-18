use super::calendar_lightweight::parse_calendar_preview;
use super::intent::{classify_intent, IntentKind, IntentRoute};
use super::reminder_lightweight::parse_reminder_preview;
use super::shadow_text::normalize_shadow_text;
use super::task_lightweight::parse_task_preview;
use super::{
    build_token_classification_request, load_ort_session, CaptureParserArtifactBundle,
    CaptureParserArtifactManifest, CaptureParserError, CapturePreviewCandidate,
    CapturePreviewRequest, CapturePreviewResult, CaptureWordPieceTokenizer,
};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

const TASK_CAPABILITY_SAMPLE: &str = "finish report tomorrow";
const REMINDER_CAPABILITY_SAMPLE: &str = "remind me to water plants tomorrow at 9";
const CALENDAR_CAPABILITY_SAMPLE: &str = "lunch with sara tomorrow at noon";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParserCapabilityKey {
    root: PathBuf,
    parser_family: String,
    artifact_id: String,
}

pub fn preview_capture(
    manifest: &CaptureParserArtifactManifest,
    request: CapturePreviewRequest,
) -> CapturePreviewResult {
    let shadow = normalize_shadow_text(request.text.trim());
    if shadow.is_empty() {
        return CapturePreviewResult::unsupported(
            "capture-text-required",
            "Capture preview requires non-empty text.",
        );
    }

    if let Some(payload) = strip_prefix_case_insensitive(&shadow, "task:") {
        if payload.is_empty() {
            return CapturePreviewResult::unsupported(
                "capture-text-required",
                "Capture preview requires non-empty text.",
            );
        }
        return build_task_result(manifest, payload, &request, 0.99, Vec::new());
    }
    if let Some(payload) = strip_prefix_case_insensitive(&shadow, "reminder:") {
        if payload.is_empty() {
            return CapturePreviewResult::unsupported(
                "capture-text-required",
                "Capture preview requires non-empty text.",
            );
        }
        return build_reminder_result(manifest, payload, &request, 0.99, Vec::new());
    }
    if let Some(payload) = strip_prefix_case_insensitive(&shadow, "event:") {
        if payload.is_empty() {
            return CapturePreviewResult::unsupported(
                "capture-text-required",
                "Capture preview requires non-empty text.",
            );
        }
        return build_calendar_result(manifest, payload, &request, 0.99, Vec::new());
    }

    let route = match classify_intent(manifest, &shadow) {
        Ok(route) => route,
        Err(error) => return unsupported_from_error(error),
    };
    let candidates = build_candidates(&route);

    match route.intent {
        IntentKind::Task => {
            build_task_result(
                manifest,
                &shadow,
                &request,
                route.primary_score(),
                candidates,
            )
        }
        .with_ambiguous(route),
        IntentKind::Calendar => {
            build_calendar_result(
                manifest,
                &shadow,
                &request,
                route.primary_score(),
                candidates,
            )
        }
        .with_ambiguous(route),
        IntentKind::Reminder => {
            build_reminder_result(
                manifest,
                &shadow,
                &request,
                route.primary_score(),
                candidates,
            )
        }
        .with_ambiguous(route),
        IntentKind::None => {
            CapturePreviewResult::thought(shadow, route.none, candidates).with_ambiguous(route)
        }
    }
}

trait CapturePreviewResultExt {
    fn with_candidates(
        self,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
    ) -> CapturePreviewResult;

    fn with_ambiguous(self, route: IntentRoute) -> CapturePreviewResult;
}

impl CapturePreviewResultExt for CapturePreviewResult {
    fn with_candidates(
        mut self,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
    ) -> CapturePreviewResult {
        self.confidence = confidence;
        self.candidates = candidates;
        self
    }

    fn with_ambiguous(mut self, route: IntentRoute) -> CapturePreviewResult {
        if route.ambiguous {
            self.warnings.push(super::CapturePreviewWarning {
                code: "intent_ambiguous".to_string(),
                severity: "info".to_string(),
                message: if route.intent == IntentKind::None {
                    "Intent is ambiguous and was routed to thought.".to_string()
                } else {
                    "Intent is ambiguous between multiple categories.".to_string()
                },
            });
        }
        self
    }
}

fn build_candidates(route: &IntentRoute) -> Vec<CapturePreviewCandidate> {
    let mut candidates = vec![
        CapturePreviewCandidate {
            kind: "task",
            score: route.task,
        },
        CapturePreviewCandidate {
            kind: "calendar-event",
            score: route.calendar,
        },
        CapturePreviewCandidate {
            kind: "reminder",
            score: route.reminder,
        },
    ];
    candidates.sort_by(|left, right| right.score.total_cmp(&left.score));
    candidates
        .into_iter()
        .filter(|candidate| candidate.score > 0.0)
        .collect()
}

fn build_task_result(
    manifest: &CaptureParserArtifactManifest,
    text: &str,
    request: &CapturePreviewRequest,
    confidence: f64,
    candidates: Vec<CapturePreviewCandidate>,
) -> CapturePreviewResult {
    if let Err(error) = verify_family_inference(manifest, "task") {
        return unsupported_from_error(error);
    }
    let parsed = parse_task_preview(text, request);
    CapturePreviewResult::task(
        parsed.draft,
        confidence.max(parsed.confidence),
        candidates,
        parsed.warnings,
    )
    .with_schedule(parsed.schedule)
}

fn build_reminder_result(
    manifest: &CaptureParserArtifactManifest,
    text: &str,
    request: &CapturePreviewRequest,
    confidence: f64,
    candidates: Vec<CapturePreviewCandidate>,
) -> CapturePreviewResult {
    if let Err(error) = verify_family_inference(manifest, "reminder") {
        return unsupported_from_error(error);
    }
    let parsed = parse_reminder_preview(text, request);
    CapturePreviewResult::reminder(
        parsed.draft,
        confidence.max(parsed.confidence),
        candidates,
        parsed.warnings,
    )
    .with_schedule(parsed.schedule)
}

fn build_calendar_result(
    manifest: &CaptureParserArtifactManifest,
    text: &str,
    request: &CapturePreviewRequest,
    confidence: f64,
    candidates: Vec<CapturePreviewCandidate>,
) -> CapturePreviewResult {
    if let Err(error) = verify_family_inference(manifest, "calendar") {
        return unsupported_from_error(error);
    }
    parse_calendar_preview(text, request).with_candidates(confidence, candidates)
}

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
        .map(|_| text[prefix.len()..].trim())
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

fn capability_probe_sample(parser_family: &str) -> &'static str {
    match parser_family {
        "task" => TASK_CAPABILITY_SAMPLE,
        "reminder" => REMINDER_CAPABILITY_SAMPLE,
        "calendar" => CALENDAR_CAPABILITY_SAMPLE,
        _ => "preview smoke",
    }
}

fn parser_capability_cache() -> &'static Mutex<BTreeSet<ParserCapabilityKey>> {
    static CACHE: OnceLock<Mutex<BTreeSet<ParserCapabilityKey>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeSet::new()))
}
