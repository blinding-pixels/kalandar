use super::calendar_lightweight::parse_calendar_preview;
#[cfg(feature = "onnx")]
use super::capture_spans::ModelCaptureSpans;
use super::capture_spans::{CaptureSpanSource, RuleCaptureSpans};
use super::intent::{classify_intent_embedded, CaptureIntent, IntentKind, IntentRoute};
use super::intent_rules::normalize_js_whitespace;
use super::postprocess::finalize_reminder_title;
use super::reminder_lightweight::parse_reminder_preview;
use super::task_lightweight::parse_task_preview;
#[cfg(feature = "onnx")]
use super::{intent_manifest::classify_intent, CaptureParserArtifactManifest};
use super::{
    CaptureParserError, CapturePreviewCandidate, CapturePreviewRequest, CapturePreviewResult,
    CapturePreviewWarning,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Task,
    Reminder,
    Calendar,
}

#[cfg(feature = "onnx")]
pub fn preview_capture(
    manifest: &CaptureParserArtifactManifest,
    request: CapturePreviewRequest,
) -> CapturePreviewResult {
    let spans = ModelCaptureSpans::new(manifest);
    let result = preview_with(request, |text| classify_intent(manifest, text), &spans);
    match spans.take_failure() {
        Some(error) => unsupported_from_error(error),
        None => result,
    }
}

pub fn preview_capture_rules(request: CapturePreviewRequest) -> CapturePreviewResult {
    preview_with(
        request,
        |text| Ok(classify_intent_embedded(text)),
        &RuleCaptureSpans,
    )
}

fn preview_with(
    request: CapturePreviewRequest,
    route: impl Fn(&str) -> Result<IntentRoute, CaptureParserError>,
    spans: &dyn CaptureSpanSource,
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
                spans,
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
        family, &text, &request, confidence, candidates, warnings, spans,
    )
}

fn family_result(
    family: Family,
    text: &str,
    request: &CapturePreviewRequest,
    confidence: f64,
    candidates: Vec<CapturePreviewCandidate>,
    intent_warnings: Vec<CapturePreviewWarning>,
    spans: &dyn CaptureSpanSource,
) -> CapturePreviewResult {
    let mut result = match family {
        Family::Task => {
            let parsed = parse_task_preview(text, request, spans);
            CapturePreviewResult::task(parsed.draft, confidence, candidates, parsed.warnings)
                .with_schedule(parsed.schedule)
        }
        Family::Reminder => {
            let parsed = parse_reminder_preview(text, request, spans);
            CapturePreviewResult::reminder(parsed.draft, confidence, candidates, parsed.warnings)
                .with_schedule(parsed.schedule)
        }
        Family::Calendar => {
            let mut result = parse_calendar_preview(text, request, spans);
            result.confidence = confidence;
            result.candidates = candidates;
            result
        }
    };
    if let Some(title) = spans.model_title() {
        if let Some(task) = result.task.as_mut() {
            task.title = title;
        } else if let Some(reminder) = result.reminder.as_mut() {
            let finalized = finalize_reminder_title(&title);
            if !finalized.is_empty() {
                reminder.title = finalized;
            }
        } else if let Some(event) = result.event.as_mut() {
            event.title = Some(title);
        }
    }
    result.warnings = intent_warnings.into_iter().chain(result.warnings).collect();
    result
}

fn text_required() -> CapturePreviewResult {
    CapturePreviewResult::unsupported(
        "capture-text-required",
        "Capture preview requires non-empty text.",
    )
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
