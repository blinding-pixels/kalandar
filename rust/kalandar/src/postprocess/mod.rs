mod calendar;
mod reminder;
mod task;

use crate::js::{self, normalize_whitespace, replace_all, replace_first};
use fancy_regex::Regex;
use std::sync::OnceLock;

pub(crate) use calendar::finalize_calendar_title;
pub(crate) use reminder::finalize_reminder_title;
pub(crate) use task::finalize_task_title;

const TITLE_SMALL_WORDS: [&str; 19] = [
    "a", "an", "the", "and", "but", "or", "nor", "for", "to", "in", "on", "at", "by", "of", "with",
    "from", "up", "as", "is",
];

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
        js::regex(r"(?i)^(?:remind\s+me\s+to|remind\s+me|don['\x{2019}]?t\s+forget\s+to|don['\x{2019}]?t\s+let\s+me\s+forget\s+to)\b[\s,:-]*")
    });
    repeat_until_stable(text, regex)
}

pub(crate) fn strip_trailing_dangling_preposition(text: &str, enabled: bool) -> String {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    if !enabled {
        return normalize_whitespace(text);
    }
    let regex =
        REGEX.get_or_init(|| js::regex(r"(?i)(?:^|\s)(?:for|to|by|with|at|on|in|from)[.,;:!?-]*$"));
    repeat_until_stable(text, regex)
}

fn leading_title_filler() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| js::regex(r"(?i)^(?:to(?:\s+the)?|that(?:\s+i)?|about)\b[\s,:-]*"))
}

pub(crate) fn strip_leading_title_filler(text: &str) -> String {
    repeat_until_stable(text, leading_title_filler())
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
        .map(js::regex)
        .collect()
    });
    let output = regexes.iter().fold(text.to_string(), |output, regex| {
        replace_all(regex, &output, " ")
    });
    normalize_whitespace(&output)
}
