use super::{
    replace_all, replace_first, strip_leading_title_filler, strip_reminder_lead_prefix,
    strip_trailing_dangling_preposition,
};
use crate::calendar_lightweight::js::{cached, compile, test, trim, words};
use fancy_regex::Regex;
use std::sync::OnceLock;

const TITLE_GLUE_WORDS: [&str; 13] = [
    "on", "at", "with", "every", "reminder", "from", "to", "for", "starting", "invite", "w/",
    "the", "and",
];

const TITLE_SMALL_WORDS: [&str; 19] = [
    "a", "an", "the", "and", "but", "or", "nor", "for", "to", "in", "on", "at", "by", "of", "with",
    "from", "up", "as", "is",
];

type Steps = Vec<(Regex, bool, &'static str)>;

fn apply(text: &str, steps: &Steps) -> String {
    steps
        .iter()
        .fold(text.to_string(), |output, (regex, global, replacement)| {
            if *global {
                replace_all(regex, &output, replacement)
            } else {
                replace_first(regex, &output, replacement)
            }
        })
}

fn compile_steps(entries: &[(&str, &str, &'static str)]) -> Steps {
    entries
        .iter()
        .map(|(source, flags, replacement)| {
            (compile(source, flags), flags.contains('g'), *replacement)
        })
        .collect()
}

fn clean_title_noise(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"[()[\]]", "g", " "),
            (r"[,;]+", "g", " "),
            (r"[–—]+", "g", " "),
            (r"\s+-\s+", "g", " "),
            (r"\s+", "g", " "),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn trim_edge_prepositions(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"^(?:on|at|with|from|to|for|and)\s+", "i", ""),
            (r"\s+(?:on|at|with|from|to|for|and)$", "i", ""),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn strip_trailing_temporal_glue(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"\s+(?:till|until)\s*[.,;:!?-]*\s*$", "i", ""),
            (r"[.]+$", "g", ""),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn strip_priority_noise(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"\b(?:high|medium|low)\s+priority\b", "gi", " "),
            (r"\s+", "g", " "),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn strip_mention_noise(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"\b(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b", "gi", " "),
            (r"@[a-z0-9_.+-]+\b", "gi", " "),
            (r"\s+", "g", " "),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn strip_temporal_noise(input: &str) -> String {
    static STEPS: OnceLock<Steps> = OnceLock::new();
    let steps = STEPS.get_or_init(|| {
        compile_steps(&[
            (r"\bin\s+\d+\s+(?:day|days|week|weeks|month|months)\b", "gi", " "),
            (r"\bevery\s+other\s+(?:day|week)\b", "gi", " "),
            (r"\b(?:next|this)\s+(?:week|month)\b", "gi", " "),
            (
                r"\b(?:today|tomorrow|tonight|tmr|yesterday|starting|weekly|daily|monthly|yearly|annually)\b",
                "gi",
                " ",
            ),
            (r"\b(?:morning|afternoon|evening|noon)\b", "gi", " "),
            (
                r"\b(?:end of (?:the )?day|end of (?:the )?week|end of (?:the )?month|eod|eow)\b",
                "gi",
                " ",
            ),
            (r"\b(?:am|pm)\b", "gi", " "),
            (
                r"\b(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)\b",
                "gi",
                " ",
            ),
            (r"\b\d{4}-\d{2}-\d{2}\b", "g", " "),
            (r"\bfrom\s+now\b", "gi", " "),
            (r"\s+", "g", " "),
        ])
    });
    trim(&apply(input, steps)).to_string()
}

fn capitalize(lower: &str) -> String {
    let mut chars = lower.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn smart_calendar_title_case(input: &str) -> String {
    static ACRONYM: OnceLock<Regex> = OnceLock::new();
    let acronym = cached(&ACRONYM, r"^[A-Z]{2,4}$", "");
    let tokens: Vec<&str> = words(input).collect();
    let last = tokens.len().saturating_sub(1);
    tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            if test(acronym, token) {
                return (*token).to_string();
            }
            let lower = token.to_lowercase();
            if index > 0 && index < last && TITLE_SMALL_WORDS.contains(&lower.as_str()) {
                return lower;
            }
            capitalize(&lower)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_edge_glue_words(input: &str) -> String {
    let is_glue = |token: &&str| TITLE_GLUE_WORDS.contains(&token.to_lowercase().as_str());
    let tokens: Vec<&str> = words(input).collect();
    let start = tokens
        .iter()
        .position(|token| !is_glue(token))
        .unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .rposition(|token| !is_glue(token))
        .map_or(start, |index| index + 1);
    tokens[start..end.max(start)].join(" ")
}

pub(crate) fn finalize_calendar_title(masked_input: &str, cleaned_input: &str) -> String {
    static REMINDER_LEAD: OnceLock<Regex> = OnceLock::new();
    let base = clean_title_noise(masked_input);
    let should_strip_reminder_lead_prefix = test(
        cached(
            &REMINDER_LEAD,
            r"^\s*(?:remind\s+me|don['’]?t\s+forget)\b",
            "i",
        ),
        cleaned_input,
    );
    let title_base = if should_strip_reminder_lead_prefix {
        strip_reminder_lead_prefix(&base)
    } else {
        base
    };
    let mention_and_priority_stripped = strip_mention_noise(&strip_priority_noise(&title_base));
    let temporal_stripped = strip_temporal_noise(&mention_and_priority_stripped);
    let temporal_content_removed =
        temporal_stripped != mention_and_priority_stripped || masked_input != cleaned_input;
    strip_edge_glue_words(&smart_calendar_title_case(
        &strip_trailing_dangling_preposition(
            &strip_trailing_temporal_glue(&strip_leading_title_filler(&trim_edge_prepositions(
                &temporal_stripped,
            ))),
            temporal_content_removed,
        ),
    ))
}
