use super::{
    js::{self, normalize_whitespace, replace_first},
    postprocess::finalize_reminder_title,
    recurrence_exceptions::{classify_recurrence_exception, to_recurrence_exceptions},
    schedule::{context_for, ScheduleOutput},
    CapturePreviewReminderDraft, CapturePreviewRequest, CapturePreviewWarning, PreviewSchedule,
};
use crate::capture_spans::CaptureSpanSource;
use fancy_regex::Regex;
use kalandar_dates::capture::{capture_schedule, CaptureKind, Settings};
use std::sync::OnceLock;

pub struct ReminderParseOutput {
    pub draft: CapturePreviewReminderDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub schedule: Option<PreviewSchedule>,
}

pub fn parse_reminder_preview(
    text: &str,
    request: &CapturePreviewRequest,
    spans: &dyn CaptureSpanSource,
) -> ReminderParseOutput {
    let original = normalize_whitespace(text);
    let context = context_for(request).with_settings(Settings::default());
    let (found, title) = match spans.schedule_spans(CaptureKind::Reminder, &original) {
        Some(labelled) => {
            let found = ScheduleOutput::new(
                capture_schedule(&original, CaptureKind::Reminder, Some(&labelled), &context),
                &original,
            );
            let title = finalize_reminder_title(&prefix_pass(&found.remaining));
            (found, title)
        }
        None => {
            let working = prefix_pass(&original);
            let found = ScheduleOutput::new(
                capture_schedule(&working, CaptureKind::Reminder, None, &context),
                &working,
            );
            let title = finalize_reminder_title(&found.remaining);
            (found, title)
        }
    };

    let recurrence_exception = found
        .texts
        .recurrence_exceptions
        .iter()
        .map(|raw| classify_recurrence_exception(raw, &context))
        .collect::<Vec<_>>();
    let recurrence_exceptions =
        to_recurrence_exceptions(&recurrence_exception, found.schedule.as_ref());
    let remind_at_text = found.texts.moment.clone().or_else(|| found.clock_time());
    let recurrence = found.reminder_recurrence();
    let context_hint = found.context_hint();
    let warnings = finalize_reminder_warnings(
        remind_at_text.as_deref(),
        recurrence.is_some(),
        found.warnings,
    );

    ReminderParseOutput {
        draft: CapturePreviewReminderDraft {
            title,
            remind_at_text,
            recurrence,
            recurrence_start_text: found.texts.recurrence_start,
            recurrence_end_text: found.texts.recurrence_end,
            recurrence_exceptions,
            recurrence_window_text: found.texts.recurrence_window,
            location: None,
            context_hint,
        },
        warnings,
        schedule: found.schedule,
    }
}

pub(crate) fn trailing_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)[\s,.-]+\blol\b[!?.]*$",
            r"(?i)[\s,.-]+\bpls\b[!?.]*$",
            r"(?i)[\s,.-]+\bplease\b[!?.]*$",
            r"(?i)[!?.]+$",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn prefix_pass(working: &str) -> String {
    strip_prefixes(working)
}

fn strip_prefixes(input: &str) -> String {
    let mut working = input.to_string();
    for pattern in prefix_filler_patterns() {
        working = replace_first(pattern, &working, "");
    }
    for pattern in prefix_patterns() {
        working = replace_first(pattern, &working, " ");
    }
    for pattern in trailing_filler_patterns() {
        working = replace_first(pattern, &working, "");
    }
    normalize_whitespace(&working)
}

fn build_reminder_warnings(
    remind_at: Option<&str>,
    has_recurrence: bool,
) -> Vec<CapturePreviewWarning> {
    if remind_at.is_some_and(|text| !text.is_empty()) || has_recurrence {
        return Vec::new();
    }
    vec![CapturePreviewWarning {
        code: "no_remind_time".to_string(),
        severity: "warning".to_string(),
        message: "No time specified for reminder.".to_string(),
    }]
}

fn finalize_reminder_warnings(
    remind_at: Option<&str>,
    has_recurrence: bool,
    schedule_warnings: Vec<CapturePreviewWarning>,
) -> Vec<CapturePreviewWarning> {
    let mut warnings = build_reminder_warnings(remind_at, has_recurrence);
    warnings.extend(schedule_warnings);
    warnings
}

fn prefix_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"^[\p{Emoji}\p{Emoji_Modifier}\p{Emoji_Component}\x{FE0F}\x{200D}\s!?.,:;-]+",
            r"(?i)^\s*hey siri i mean claude\b[\s,:-]*",
            r"(?i)^\s*(?:hey|yo|pls|please)\b[\s,:-]*",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn prefix_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)\bpls\s+remind\s+me\s+to\b",
            r"(?i)\bpls\s+remind\s+me\b",
            r"(?i)\bplease\s+to\s+remind\s+me\s+about\b",
            r"(?i)\bplease\s+to\s+remind\s+me\b",
            r"(?i)\bhey\s+remind\s+me\b",
            r"(?i)\byo\s+remind\s+me\b",
            r"(?i)\bgotta\s+remember\s+to\b",
            r"(?i)\bneed\s+to\s+remember\b",
            r"(?i)\bneed\s+to\s+remembr\s+to\b",
            r"(?i)\bi should probably remember to\b",
            r"(?i)\bremind\s*me\s+to\b",
            r"(?i)\bremind\s*me\s+about\b",
            r"(?i)\bremind\s*me\s+that\b",
            r"(?i)\bremind\s*me\s+2\b",
            r"(?i)\bremind\s*me\s+two\b",
            r"(?i)\bremind\s*me\b",
            r"(?i)\bremindme\s+to\b",
            r"(?i)\bremindme\b",
            r"(?i)\brmind\s+me\b",
            r"(?i)\bremnd me\b",
            r"(?i)\bremmber\s+to\b",
            r"(?i)\brememeber\b",
            r"(?i)\bremember\s+to\b",
            r"(?i)\bremember\b",
            r"(?i)\bdon'?t\s+let\s+me\s+forget\s+about\b",
            r"(?i)\bdon'?t\s+forget\s+to\b",
            r"(?i)\bdon'?t\s+forget\b",
            r"(?i)\bdont\s+forget\s+to\b",
            r"(?i)\bdont\s+forget\b",
            r"(?i)\bdont\s+foreget\b",
            r"(?i)\bdont\s+fore?get\b",
            r"(?i)\bdont\s+let\s+me\s+forgt\b",
            r"(?i)\bdont\s+let\s+me\s+forget\b",
            r"(?i)\bdont\s+4get\b",
            r"(?i)\bnote\s+to\s+self\b",
            r"(?i)\bping\s+me\s+about\b",
            r"(?i)\bping\s+me\s+to\b",
            r"(?i)\bping\s+me\b",
            r"(?i)\bheads\s+up\b",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}
