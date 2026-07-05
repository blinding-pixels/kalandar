mod calendar;
mod reminder;
mod task;

use regex::Regex;
use std::sync::OnceLock;

pub(crate) use calendar::finalize_calendar_title;
pub(crate) use reminder::finalize_reminder_title;
pub(crate) use task::finalize_task_title;

pub(crate) fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn clean_title_punctuation(input: &str) -> String {
    normalize_whitespace(
        &input
            .replace(['“', '”', '"'], "")
            .replace(['?', '!'], " ")
            .replace([':', '-', ','], " "),
    )
}

pub(crate) fn strip_reminder_lead_prefix(text: &str) -> String {
    let mut output = normalize_whitespace(text);
    let mut previous = String::new();
    while output != previous {
        previous = output.clone();
        output = normalize_whitespace(&reminder_prefix_regex().replace(&output, " "));
    }
    output
}

pub(crate) fn strip_residual_temporal_tokens(text: &str) -> String {
    let output = text
        .replace("eod", " ")
        .replace("eow", " ")
        .replace("tmr", " ");
    let output = Regex::new(r"(?i)\bin\s+\d+\s+(?:minutes?|hours?|days?|weeks?|months?)\b")
        .expect("relative temporal cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    let output = Regex::new(
        r"(?i)\bevery\s+other\s+(?:day|week|month|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
    )
    .expect("every-other temporal cleanup regex should compile")
    .replace_all(&output, " ")
    .to_string();
    let output = Regex::new(r"(?i)\bevery\s+\d+\s+(?:days?|weeks?|months?)\b")
        .expect("interval temporal cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    let output = Regex::new(r"(?i)\b(?:next|this|last)\s+(?:month|week|year)\b")
        .expect("named period cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    let output = Regex::new(
        r"(?i)\b(?:next|this|last)\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
    )
    .expect("named weekday cleanup regex should compile")
    .replace_all(&output, " ")
    .to_string();
    let output = Regex::new(r"(?i)\bend of (?:the )?(?:month|week|day)\b")
        .expect("end-of period cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    let output = Regex::new(r"(?i)\b(?:today|tomorrow|tonight|yesterday)\b")
        .expect("named temporal cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    let output = Regex::new(r"(?i)\b\d{1,2}(?::\d{2})?\s*(?:am|pm|a|p)\b")
        .expect("clock cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    normalize_whitespace(&output)
}

pub(crate) fn strip_trailing_dangling_preposition(text: &str, enabled: bool) -> String {
    if !enabled {
        return normalize_whitespace(text);
    }

    let mut output = normalize_whitespace(text);
    let mut previous = String::new();
    while output != previous {
        previous = output.clone();
        output = normalize_whitespace(&trailing_preposition_regex().replace(&output, " "));
    }
    output
}

pub(crate) fn strip_leading_title_filler(text: &str) -> String {
    let mut output = normalize_whitespace(text);
    let mut previous = String::new();
    while output != previous {
        previous = output.clone();
        output = normalize_whitespace(&leading_title_filler_regex().replace(&output, " "));
    }
    output
}

fn reminder_prefix_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)^(?:remind\s+me\s+to|remind\s+me|don't\s+forget\s+to|dont\s+forget\s+to|don't\s+let\s+me\s+forget\s+to|don’t\s+forget\s+to)\b[\s,:-]*",
        )
        .expect("reminder prefix regex should compile")
    })
}

fn trailing_preposition_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:for|to|by|with|at|on|in|from)\b[.,;:!?-]*$")
            .expect("trailing preposition regex should compile")
    })
}

fn leading_title_filler_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^(?:to(?:\s+the)?|that(?:\s+i)?|about)\b[\s,:-]*")
            .expect("leading title filler regex should compile")
    })
}
