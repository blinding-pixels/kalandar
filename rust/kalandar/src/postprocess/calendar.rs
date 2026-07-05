use super::{
    normalize_whitespace, strip_leading_title_filler, strip_reminder_lead_prefix,
    strip_residual_temporal_tokens, strip_trailing_dangling_preposition,
};
use regex::Regex;

pub(crate) fn finalize_calendar_title(input: &str, raw_input: &str) -> String {
    let base = clean_title_noise(input);
    let title_base = if Regex::new(r"(?i)^\s*(?:remind\s+me|don['\u{2019}]?t\s+forget)\b")
        .expect("calendar reminder lead regex should compile")
        .is_match(raw_input)
    {
        strip_reminder_lead_prefix(&base)
    } else {
        base
    };
    let mention_and_priority_stripped = strip_mention_noise(&strip_priority_noise(&title_base));
    let temporal_stripped = strip_calendar_temporal_noise(&mention_and_priority_stripped);
    let temporal_content_removed = temporal_stripped != mention_and_priority_stripped;
    normalize_whitespace(
        trim_edge_prepositions(&strip_trailing_temporal_glue(&strip_leading_title_filler(
            &strip_trailing_dangling_preposition(&temporal_stripped, temporal_content_removed),
        )))
        .trim_matches(|char: char| ",.;:-".contains(char)),
    )
}

fn clean_title_noise(input: &str) -> String {
    input
        .replace(['(', ')', '[', ']'], " ")
        .replace([',', ';'], " ")
        .replace(['–', '—'], " ")
        .replace(" - ", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn trim_edge_prepositions(input: &str) -> String {
    Regex::new(r"(?i)^(?:on|at|with|from|to|for|and|of)\s+")
        .expect("calendar leading edge preposition regex should compile")
        .replace(
            &Regex::new(r"(?i)\s+(?:on|at|with|from|to|for|and)$")
                .expect("calendar trailing edge preposition regex should compile")
                .replace(input, " "),
            " ",
        )
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_trailing_temporal_glue(input: &str) -> String {
    Regex::new(r"(?i)\s+(?:till|until|every)\s*[.,;:!?-]*\s*$")
        .expect("calendar trailing temporal glue regex should compile")
        .replace(
            &Regex::new(r"[.]+$")
                .expect("calendar trailing dot regex should compile")
                .replace(input, ""),
            "",
        )
        .trim()
        .to_string()
}

fn strip_priority_noise(input: &str) -> String {
    Regex::new(r"(?i)\b(?:high|medium|low)\s+priority\b")
        .expect("calendar priority cleanup regex should compile")
        .replace_all(input, " ")
        .to_string()
}

fn strip_mention_noise(input: &str) -> String {
    Regex::new(r"(?i)\b(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b")
        .expect("calendar mention-phrase cleanup regex should compile")
        .replace_all(
            &Regex::new(r"(?i)@[a-z0-9_.+-]+\b")
                .expect("calendar mention cleanup regex should compile")
                .replace_all(input, " "),
            " ",
        )
        .to_string()
}

fn strip_calendar_temporal_noise(input: &str) -> String {
    strip_residual_temporal_tokens(input)
}
