use super::{
    strip_leading_title_filler, strip_reminder_lead_prefix, strip_residual_temporal_tokens,
    strip_trailing_dangling_preposition,
};
use crate::js::{self, normalize_whitespace, replace_all, replace_first};
use fancy_regex::Regex;
use std::sync::OnceLock;

pub(crate) fn finalize_task_title(input: &str) -> String {
    normalize_title_text(input)
}

fn clean_title_punctuation(input: &str) -> String {
    static REGEXES: OnceLock<[Regex; 5]> = OnceLock::new();
    let [pictographic, repeated, quotes, separators, hyphens] = REGEXES.get_or_init(|| {
        [
            js::regex(r"\p{Extended_Pictographic}+"),
            js::regex(r"[!?.,]{2,}"),
            js::regex("[\u{201C}\u{201D}\"]"),
            js::regex(r"\s*[,;:]\s*"),
            js::regex(r"\s*-\s*"),
        ]
    });
    let output = replace_all(pictographic, input, " ");
    let output = replace_all(repeated, &output, " ");
    let output = replace_all(quotes, &output, "");
    let output = replace_all(separators, &output, " ");
    let output = replace_all(hyphens, &output, " ");
    normalize_whitespace(&output)
}

fn normalize_title_text(input: &str) -> String {
    static PLEASE: OnceLock<Regex> = OnceLock::new();
    static CAN_U: OnceLock<Regex> = OnceLock::new();
    static PRIORITY: OnceLock<Regex> = OnceLock::new();
    static TRAILING_IN: OnceLock<Regex> = OnceLock::new();
    static TAIL: OnceLock<[Regex; 4]> = OnceLock::new();
    let mut output = clean_title_punctuation(input);

    for pattern in leading_filler_patterns() {
        output = replace_first(pattern, &output, "");
    }
    for pattern in trailing_filler_patterns() {
        output = replace_first(pattern, &output, "");
    }

    let please = PLEASE.get_or_init(|| js::regex(r"(?i)\b(?:plz|pls)\b"));
    let can_u = CAN_U.get_or_init(|| js::regex(r"(?i)\bcan u\b"));
    let priority = PRIORITY.get_or_init(|| js::regex(r"(?i)\bpriority\b"));
    output = replace_all(please, &output, " ");
    output = replace_all(can_u, &output, " ");
    output = replace_all(priority, &output, " ");
    output = strip_reminder_lead_prefix(&output);

    for (pattern, replacement) in phrase_corrections() {
        output = replace_all(pattern, &output, replacement);
    }

    let temporal_stripped = strip_residual_temporal_tokens(&output);
    let temporal_content_removed = temporal_stripped != output;
    let preserve_trailing_in = js::test(
        TRAILING_IN.get_or_init(|| js::regex(r"(?i)\b(?:check|log|sign|drop|fill|get)\s+in$")),
        &temporal_stripped,
    );
    output = strip_leading_title_filler(&if preserve_trailing_in {
        temporal_stripped
    } else {
        strip_trailing_dangling_preposition(&temporal_stripped, temporal_content_removed)
    });

    let [conjunction, preposition, leading_punctuation, trailing_punctuation] =
        TAIL.get_or_init(|| {
            [
                js::regex(r"(?i)\b(?:both|but)\b$"),
                js::regex(r"(?i)\b(?:for|to|with|by|at|on|from)\b$"),
                js::regex(r"^[,.;:-]+\s*"),
                js::regex(r"\s+[,.;:-]\s*$"),
            ]
        });
    let output = replace_first(conjunction, &output, " ");
    let output = replace_first(preposition, &output, " ");
    let output = replace_all(leading_punctuation, &output, " ");
    let output = replace_all(trailing_punctuation, &output, " ");
    normalize_whitespace(&output)
}

fn leading_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)^\s*i think\b[\s,.-]*",
            r"(?i)^\s*we should\b[\s,.-]*",
            r"(?i)^\s*we shoud\b[\s,.-]*",
            r"(?i)^\s*we need to\b[\s,.-]*",
            r"(?i)^\s*we need\b[\s,.-]*",
            r"(?i)^\s*i need to\b[\s,.-]*",
            r"(?i)^\s*i need\b[\s,.-]*",
            r"(?i)^\s*need to\b[\s,.-]*",
            r"(?i)^\s*need\b[\s,.-]*",
            r"(?i)^\s*pls\b[\s,.-]*",
            r"(?i)^\s*please\b[\s,.-]*",
            r"(?i)^\s*but\b[\s,.-]*",
            r"(?i)^\s*can\b[\s,.-]*",
            r"(?i)^\s*task\b[\s,.:-]*",
            r"(?i)^\s*super\b[\s,.-]*",
            r"(?i)^\s*maybe\b[\s,.-]*",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn trailing_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)[\s,.-]+\bpls\b\.?$",
            r"(?i)[\s,.-]+\bplease\b\.?$",
            r"(?i)[\s,.-]+\blol\b\.?$",
            r"(?i)[\s,.-]+\bmaybe\b\.?$",
            r"(?i)[\s,.-]+\bidk\b\.?$",
            r"(?i)[\s,.-]+\bboth\b\.?$",
            r"(?i)[\s,.-]+\bbut\b\.?$",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn phrase_corrections() -> &'static [(Regex, &'static str)] {
    static REGEXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            (r"(?i)\bre-do\b", "redo"),
            (r"(?i)\bre do\b", "redo"),
            (r"(?i)\bwe dont\b", "we don't"),
            (r"(?i)\blast years\b", "last year's"),
            (r"(?i)\bhub os\b", "hub OS"),
            (r"(?i)\bprod login\b", "production login"),
            (r"(?i)\bpitch deck draft\b", "pitch deck, draft"),
            (r"(?i)\bdraft it send\b", "draft and send"),
            (r"(?i)\bto clint\b", "to client"),
            (r"(?i)\bbday\b", "bday"),
        ]
        .into_iter()
        .map(|(pattern, replacement)| (js::regex(pattern), replacement))
        .collect()
    })
}
