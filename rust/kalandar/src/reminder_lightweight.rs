use super::{
    postprocess::finalize_reminder_title, schedule::extract_schedule, CapturePreviewReminderDraft,
    CapturePreviewRequest, CapturePreviewWarning, PreviewSchedule,
};
use kalandar_dates::capture::CaptureKind;
use regex::Regex;
use std::sync::OnceLock;

pub struct ReminderParseOutput {
    pub draft: CapturePreviewReminderDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub confidence: f64,
    pub schedule: Option<PreviewSchedule>,
}

pub fn parse_reminder_preview(text: &str, request: &CapturePreviewRequest) -> ReminderParseOutput {
    let mut working = normalize_whitespace(text);
    working = strip_prefix_filler(&working);

    let schedule = extract_schedule(&working, CaptureKind::Reminder, request);
    let recurrence = schedule.reminder_recurrence();
    let recurrence_exceptions = schedule.recurrence_exceptions();
    let recurrence_start_text = schedule.texts.recurrence_start.clone();
    let recurrence_end_text = schedule.texts.recurrence_end.clone();
    let recurrence_window_text = schedule.texts.recurrence_window.clone();
    let context_hint = schedule.context_hint();
    let remind_at_text = schedule
        .texts
        .moment
        .clone()
        .or_else(|| schedule.clock_time());
    let title = build_reminder_title(&schedule.remaining);

    let mut warnings = Vec::new();
    if remind_at_text.is_none() && recurrence.is_none() {
        warnings.push(warning(
            "no-remind-time",
            "warning",
            "No time specified for reminder.",
        ));
    }
    warnings.extend(schedule.warnings.iter().cloned());

    let confidence = if recurrence.is_some() {
        if recurrence_exceptions.is_empty()
            && recurrence_start_text.is_none()
            && recurrence_end_text.is_none()
            && recurrence_window_text.is_none()
            && context_hint.is_none()
        {
            0.9
        } else {
            0.96
        }
    } else if remind_at_text.is_some() {
        0.88
    } else if title != "Reminder" {
        0.62
    } else {
        0.4
    };

    ReminderParseOutput {
        draft: CapturePreviewReminderDraft {
            title,
            remind_at_text,
            recurrence,
            recurrence_start_text,
            recurrence_end_text,
            recurrence_exceptions,
            recurrence_window_text,
            location: None,
            context_hint,
        },
        warnings,
        confidence,
        schedule: schedule.schedule,
    }
}

fn warning(code: &str, severity: &str, message: &str) -> CapturePreviewWarning {
    CapturePreviewWarning {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_prefix_filler(text: &str) -> String {
    let mut output = normalize_whitespace(text);
    for pattern in prefix_filler_patterns() {
        output = normalize_whitespace(&pattern.replace(&output, " "));
    }
    output
}

fn build_reminder_title(input: &str) -> String {
    finalize_reminder_title(input)
}

fn prefix_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)^\s*(?:please|pls)\b[\s,.-]*").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\b(?:please|pls|lol|maybe|idk)\b\.?$").unwrap(),
        ]
    })
}
