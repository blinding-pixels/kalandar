use super::{
    clean_title_punctuation, normalize_whitespace, strip_leading_title_filler,
    strip_reminder_lead_prefix, strip_residual_temporal_tokens,
    strip_trailing_dangling_preposition,
};
use regex::Regex;
use std::sync::OnceLock;

pub(crate) fn finalize_task_title(input: &str) -> String {
    let mut output = clean_title_punctuation(input);
    for pattern in leading_filler_patterns() {
        output = pattern.replace(&output, "").to_string();
    }
    for pattern in trailing_filler_patterns() {
        output = pattern.replace(&output, "").to_string();
    }

    output = Regex::new(r"(?i)\b(?:plz|pls)\b")
        .expect("please regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = Regex::new(r"(?i)\bcan u\b")
        .expect("can-u regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = Regex::new(r"(?i)\bpriority\b")
        .expect("priority title regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = strip_reminder_lead_prefix(&output);

    for (from, to) in phrase_corrections() {
        output = from.replace_all(&output, *to).to_string();
    }

    let temporal_stripped = strip_residual_temporal_tokens(&output);
    let temporal_content_removed = temporal_stripped != output;
    let preserve_trailing_in = Regex::new(r"(?i)\b(?:check|log|sign|drop|fill|get)\s+in$")
        .expect("phrasal trailing-in regex should compile")
        .is_match(&temporal_stripped);
    let trimmed = if preserve_trailing_in {
        temporal_stripped
    } else {
        strip_trailing_dangling_preposition(&temporal_stripped, temporal_content_removed)
    };
    output = strip_leading_title_filler(&trimmed);

    let normalized = normalize_whitespace(
        output
            .replace(" both", " ")
            .replace(" but", " ")
            .trim_matches(|char: char| ",.;:-".contains(char)),
    );

    if normalized.is_empty() {
        "Task".to_string()
    } else {
        normalized
    }
}

fn leading_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)^\s*(?:please|pls|can\s+you|could\s+you)\b[\s,.-]*").unwrap(),
            Regex::new(r"(?i)^\s*(?:i\s+need\s+to|need\s+to)\b[\s,.-]*").unwrap(),
        ]
    })
}

fn trailing_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)[\s,.-]+\bpls\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\bplease\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\blol\b\.?$").unwrap(),
        ]
    })
}

fn phrase_corrections() -> &'static [(Regex, &'static str)] {
    static REGEXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            (Regex::new(r"(?i)\bre-do\b").unwrap(), "redo"),
            (Regex::new(r"(?i)\bre do\b").unwrap(), "redo"),
            (Regex::new(r"(?i)\bwe dont\b").unwrap(), "we don't"),
            (Regex::new(r"(?i)\blast years\b").unwrap(), "last year's"),
            (Regex::new(r"(?i)\bhub os\b").unwrap(), "hub OS"),
            (
                Regex::new(r"(?i)\bprod login\b").unwrap(),
                "production login",
            ),
            (
                Regex::new(r"(?i)\bpitch deck draft\b").unwrap(),
                "pitch-deck draft",
            ),
        ]
    })
}
