use super::{
    clean_title_punctuation, normalize_whitespace, strip_leading_title_filler,
    strip_reminder_lead_prefix, strip_residual_temporal_tokens,
    strip_trailing_dangling_preposition,
};
use regex::Regex;
use std::sync::OnceLock;

pub(crate) fn finalize_reminder_title(input: &str) -> String {
    let mut output = clean_title_punctuation(input);
    for pattern in trailing_filler_patterns() {
        output = pattern.replace(&output, "").to_string();
    }

    output = Regex::new(r"(?i)(?:^|\s)(?:high|medium|low)\s+priority\b")
        .expect("priority phrase regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = Regex::new(r"(?i)(?:^|\s)(?:urgent|critical|important)\b")
        .expect("priority token regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = Regex::new(r"(?i)(?:^|\s)(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b")
        .expect("mention phrase cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = Regex::new(r"(?i)(?:^|\s)@[a-z0-9_.+-]+\b")
        .expect("mention cleanup regex should compile")
        .replace_all(&output, " ")
        .to_string();
    output = strip_reminder_lead_prefix(&output);

    let temporal_stripped = strip_residual_temporal_tokens(&output);
    let temporal_content_removed = temporal_stripped != output;
    let preserve_trailing_in = Regex::new(r"(?i)\b(?:check|log|sign|drop|fill|get)\s+in$")
        .expect("phrasal trailing-in regex should compile")
        .is_match(&temporal_stripped);
    output = if preserve_trailing_in {
        temporal_stripped
    } else {
        strip_trailing_dangling_preposition(&temporal_stripped, temporal_content_removed)
    };
    output = strip_leading_title_filler(&output);

    for (from, to) in phrase_corrections() {
        output = from.replace_all(&output, *to).to_string();
    }

    let mut deduped = Vec::new();
    for word in normalize_whitespace(&output).split_whitespace() {
        let repeated = deduped
            .last()
            .map(|last: &String| last.eq_ignore_ascii_case(word))
            .unwrap_or(false);
        if !repeated {
            deduped.push(word.to_string());
        }
    }

    let normalized = normalize_whitespace(
        deduped
            .join(" ")
            .replace(" both", " ")
            .replace(" but", " ")
            .trim_matches(|char: char| ",.;:-".contains(char)),
    );
    if normalized.is_empty() {
        "Reminder".to_string()
    } else {
        normalized
    }
}

fn trailing_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)[\s,.-]+\bpls\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\bplease\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\blol\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\bmaybe\b\.?$").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\bidk\b\.?$").unwrap(),
        ]
    })
}

fn phrase_corrections() -> &'static [(Regex, &'static str)] {
    static REGEXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            (Regex::new(r"(?i)\bpickup\b").unwrap(), "pick up"),
            (Regex::new(r"(?i)\bmums\b").unwrap(), "mum's"),
            (Regex::new(r"(?i)\bbday\b").unwrap(), "birthday"),
            (Regex::new(r"(?i)\bcehck\b").unwrap(), "check"),
        ]
    })
}
