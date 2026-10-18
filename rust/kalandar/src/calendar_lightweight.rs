use super::{
    postprocess::finalize_calendar_title, schedule::extract_schedule,
    CapturePreviewCalendarEventDraft, CapturePreviewRequest, CapturePreviewResult,
    CapturePreviewWarning,
};
use kalandar_dates::capture::CaptureKind;
use regex::Regex;
use std::sync::OnceLock;

pub fn parse_calendar_preview(text: &str, request: &CapturePreviewRequest) -> CapturePreviewResult {
    let working = normalize_whitespace(text);

    let attendees = extract_attendees(text);
    let schedule = extract_schedule(&working, CaptureKind::CalendarEvent, request);
    let mut working = schedule.remaining.clone();
    let location = extract_location(&mut working);
    let title = build_calendar_title(&working, text);
    let date_text = schedule.texts.date.clone();
    let time_text = schedule.texts.time.clone();
    let recurrence_text = schedule.texts.recurrence.clone();

    let mut warnings = Vec::new();
    if date_text.is_none() && time_text.is_none() && recurrence_text.is_none() {
        warnings.push(warning(
            "calendar-time-missing",
            "warning",
            "No date, time, or recurrence extracted.",
        ));
    }
    warnings.extend(schedule.warnings.iter().cloned());

    CapturePreviewResult::calendar_event(
        CapturePreviewCalendarEventDraft {
            title: (!title.is_empty()).then_some(title),
            date_text,
            time_text,
            end_time_text: schedule.texts.end_time.clone(),
            duration_minutes: schedule.duration_minutes(),
            location,
            recurrence_text,
            recurrence_start_text: schedule.texts.recurrence_start.clone(),
            recurrence_end_text: schedule.texts.recurrence_end.clone(),
            recurrence_exceptions: schedule.recurrence_exceptions(),
            alert_offset_minutes: schedule.alert_offsets(),
            attendees,
        },
        0.9,
        Vec::new(),
        warnings,
    )
    .with_schedule(schedule.schedule)
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

fn remove_segment(working: &mut String, start: usize, end: usize) {
    *working = normalize_whitespace(&format!("{} {}", &working[..start], &working[end..]));
}

fn extract_attendees(text: &str) -> Vec<String> {
    attendees_regex()
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|group| {
            group
                .as_str()
                .split(&[',', '&'][..])
                .flat_map(|entry| entry.split(" and "))
                .map(|entry| entry.trim().trim_start_matches('@').to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_location(working: &mut String) -> Option<String> {
    let captures = location_regex().captures(working.as_str())?;
    let full = captures.get(0)?;
    let group = captures.get(1)?;
    let value = trim_location_candidate(group.as_str())?;
    let candidate_end = group.start() + value.len();
    remove_segment(working, full.start(), candidate_end);
    Some(normalize_whitespace(&value))
}

fn build_calendar_title(input: &str, raw_input: &str) -> String {
    finalize_calendar_title(input, raw_input)
}

fn attendees_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bwith\s+([A-Za-z@][A-Za-z0-9@'._+-]*(?:\s*(?:,|and|&)\s*[A-Za-z@][A-Za-z0-9@'._+-]*)*)")
            .expect("calendar attendees regex should compile")
    })
}

fn location_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:at|in)\s+([A-Za-z][A-Za-z0-9 '&/_-]*)")
            .expect("calendar location regex should compile")
    })
}

fn trim_location_candidate(value: &str) -> Option<String> {
    let stop_regex = Regex::new(
        r"(?i)\s+(?:today|tomorrow|tonight|next\b|this\b|every\b|daily\b|weekly\b|monthly\b|yearly\b|mon(?:day)?\b|tue(?:s|sday)?\b|wed(?:nesday)?\b|thu(?:r|rs|rsday|ursday)?\b|fri(?:day)?\b|sat(?:urday)?\b|sun(?:day)?\b|(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\b|\d{1,2}(?::\d{2})?\s*(?:am|pm)\b|noon\b|midnight\b)",
    )
    .expect("calendar location stop regex should compile");
    let trimmed = if let Some(found) = stop_regex.find(value) {
        &value[..found.start()]
    } else {
        value
    };
    let candidate = trimmed
        .trim()
        .trim_matches(|char: char| ",.;:!?".contains(char));
    if candidate.is_empty() {
        return None;
    }
    let time_like = Regex::new(r"(?i)^(?:\d{1,2}(?::\d{2})?\s*(?:am|pm)?|noon|midnight)$")
        .expect("calendar location time-like regex should compile");
    (!time_like.is_match(candidate)).then_some(candidate.to_string())
}
