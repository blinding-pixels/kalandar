mod calendar;
mod reminder;
mod task;

use fancy_regex::Regex;
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

pub(crate) fn replace_first(regex: &Regex, text: &str, replacement: &str) -> String {
    regex.replace(text, replacement).into_owned()
}

pub(crate) fn replace_all(regex: &Regex, text: &str, replacement: &str) -> String {
    regex.replace_all(text, replacement).into_owned()
}

pub(crate) fn js_regex(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|error| panic!("rule regex should compile: {pattern}: {error}"))
}

fn repeat_until_stable(text: &str, regex: &Regex) -> String {
    let mut output = normalize_whitespace(text);
    let mut previous = String::new();
    while output != previous {
        previous = output.clone();
        output = normalize_whitespace(&replace_first(regex, &output, " "));
    }
    output
}

pub(crate) fn strip_reminder_lead_prefix(text: &str) -> String {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = REGEX.get_or_init(|| {
        js_regex(r"(?i)^(?:remind\s+me\s+to|remind\s+me|don['\x{2019}]?t\s+forget\s+to|don['\x{2019}]?t\s+let\s+me\s+forget\s+to)\b[\s,:-]*")
    });
    repeat_until_stable(text, regex)
}

pub(crate) fn strip_trailing_dangling_preposition(text: &str, enabled: bool) -> String {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    if !enabled {
        return normalize_whitespace(text);
    }
    let regex =
        REGEX.get_or_init(|| js_regex(r"(?i)(?:^|\s)(?:for|to|by|with|at|on|in|from)[.,;:!?-]*$"));
    repeat_until_stable(text, regex)
}

pub(crate) fn strip_leading_title_filler(text: &str) -> String {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = REGEX.get_or_init(|| js_regex(r"(?i)^(?:to(?:\s+the)?|that(?:\s+i)?|about)\b[\s,:-]*"));
    repeat_until_stable(text, regex)
}

pub(crate) fn strip_residual_temporal_tokens(text: &str) -> String {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    let regexes = REGEXES.get_or_init(|| {
        [
            r"(?i)\b(?:eod|eow|tmr)\b",
            r"(?i)\bin\s+\d+\s+(?:day|days|week|weeks|month|months)\b",
            r"(?i)\bevery\s+other\s+(?:day|week)\b",
            r"(?i)\bevery\s+\d+\s+(?:days|weeks|months)\b",
            r"(?i)\b(?:next|this|last)\s+(?:month|week)\b",
            r"(?i)\b(?:next|this|last)\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
            r"(?i)\bend of (?:the )?month\b",
            r"(?i)\bend of (?:the )?week\b",
            r"(?i)\bend of (?:the )?day\b",
            r"(?i)\b(?:today|tomorrow|tonight|yesterday)\b",
        ]
        .into_iter()
        .map(js_regex)
        .collect()
    });
    let output = regexes
        .iter()
        .fold(text.to_string(), |output, regex| replace_all(regex, &output, " "));
    normalize_whitespace(&output)
}
