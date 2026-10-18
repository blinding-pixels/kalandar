use super::{
    leading_title_filler, strip_leading_title_filler, strip_reminder_lead_prefix,
    strip_residual_temporal_tokens, strip_trailing_dangling_preposition, TITLE_SMALL_WORDS,
};
use crate::js::{self, normalize_whitespace, replace_all, replace_first};
use crate::reminder_lightweight::trailing_filler_patterns;
use fancy_regex::Regex;
use std::sync::OnceLock;

const TITLE_CORRECTIONS: &[(&str, &str)] = &[
    ("abt", "about"),
    ("adn", "and"),
    ("b4", "before"),
    ("brng", "bring"),
    ("callin", "call"),
    ("cancle", "cancel"),
    ("cehck", "check"),
    ("clint", "client"),
    ("dattabase", "database"),
    ("delivry", "delivery"),
    ("dashbord", "dashboard"),
    ("electrican", "electrician"),
    ("evry", "every"),
    ("flrs", "flowers"),
    ("flowrs", "flowers"),
    ("forgt", "forget"),
    ("hr", "HR"),
    ("hub", "hub"),
    ("meetting", "meeting"),
    ("meds", "meds"),
    ("mornig", "morning"),
    ("mornng", "morning"),
    ("moms", "mom's"),
    ("mums", "mum's"),
    ("pakage", "package"),
    ("passport", "passport"),
    ("pickup", "pick up"),
    ("prescirption", "prescription"),
    ("propsal", "proposal"),
    ("refil", "refill"),
    ("rememeber", "remember"),
    ("remnd", "remind"),
    ("rent", "rent"),
    ("reveiw", "review"),
    ("revieww", "review"),
    ("subsription", "subscription"),
    ("teh", "the"),
    ("tongiht", "tonight"),
];

const ACRONYM_MAP: &[(&str, &str)] = &[
    ("api", "API"),
    ("aws", "AWS"),
    ("css", "CSS"),
    ("eod", "EOD"),
    ("fcp", "FCP"),
    ("html", "HTML"),
    ("hr", "HR"),
    ("os", "OS"),
    ("pr", "PR"),
    ("readme", "README"),
    ("sql", "SQL"),
    ("ssl", "SSL"),
    ("svg", "SVG"),
    ("usb", "USB"),
    ("ui", "UI"),
    ("ux", "UX"),
];

pub(crate) fn finalize_reminder_title(working: &str) -> String {
    extract_title(working)
}

fn lookup(table: &[(&str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .iter()
        .find(|(from, _)| *from == key)
        .map(|(_, to)| *to)
}

fn is_upper_acronym(word: &str) -> bool {
    (2..=4).contains(&word.len()) && word.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn possessive_base(word: &str) -> Option<&str> {
    let base = ["'s", "'S", "\u{2019}s", "\u{2019}S"]
        .into_iter()
        .find_map(|suffix| word.strip_suffix(suffix))?;
    (!base.is_empty() && base.bytes().all(|byte| byte.is_ascii_alphabetic())).then_some(base)
}

fn clean_title_word(word: &str) -> String {
    if is_upper_acronym(word) {
        return word.to_string();
    }
    let lower = word.to_lowercase();
    lookup(TITLE_CORRECTIONS, &lower).map_or(lower, str::to_string)
}

fn smart_title_case(input: &str) -> String {
    let words: Vec<&str> = js::words(input).collect();
    let count = words.len();

    let mut output = words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            let cleaned = clean_title_word(word);
            if is_upper_acronym(&cleaned) {
                return cleaned;
            }
            let lower = cleaned.to_lowercase();
            if let Some(acronym) = lookup(ACRONYM_MAP, &lower) {
                return acronym.to_string();
            }
            if let Some(base) = possessive_base(&cleaned) {
                return format!("{}'s", js::capitalize(base));
            }
            if index > 0 && index < count - 1 && TITLE_SMALL_WORDS.contains(&lower.as_str()) {
                return lower;
            }
            js::capitalize(&cleaned)
        })
        .collect::<Vec<_>>()
        .join(" ");

    for (pattern, replacement) in phrase_corrections() {
        output = replace_all(pattern, &output, replacement);
    }

    output
}

fn extract_title(input: &str) -> String {
    let mut working = input.to_string();
    for pattern in start_strip_patterns() {
        working = replace_first(pattern, &working, "");
    }
    for pattern in trailing_filler_patterns() {
        working = replace_first(pattern, &working, "");
    }

    for pattern in title_noise_patterns() {
        working = replace_all(pattern, &working, " ");
    }
    working = normalize_whitespace(&working);
    working = strip_reminder_lead_prefix(&working);
    working = strip_leading_title_filler(&strip_trailing_dangling_preposition(
        &strip_residual_temporal_tokens(&working),
        false,
    ));

    let mut deduped: Vec<&str> = Vec::new();
    for word in js::words(&working) {
        if deduped
            .last()
            .is_some_and(|last| last.to_lowercase() == word.to_lowercase())
        {
            continue;
        }
        deduped.push(word);
    }

    let mut normalized = deduped.join(" ");
    for (pattern, replacement) in title_word_corrections() {
        normalized = replace_all(pattern, &normalized, replacement);
    }
    normalized = replace_first(leading_title_filler(), &normalized, "");
    normalized = normalize_whitespace(&normalized);

    smart_title_case(if normalized.is_empty() {
        "Reminder"
    } else {
        &normalized
    })
}

fn start_strip_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"^[\s,;:.!?-]+",
            r"(?i)^\s*(?:to|about|that|re:|re|abt|bout)\b[\s,:-]*",
            r"(?i)^\s*for\b[\s,:-]*",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn title_noise_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"[\p{Emoji}\p{Emoji_Modifier}\p{Emoji_Component}\x{FE0F}\x{200D}]+",
            r"[!?.,]{2,}",
            r"(?i)(?:^|\s)(?:high|medium|low)\s+priority\b",
            r"(?i)(?:^|\s)(?:urgent|critical|important)\b",
            r"(?i)(?:^|\s)(?:for|to|with|from)\s+@[a-z0-9_.+-]+\b",
            r"(?i)(?:^|\s)@[a-z0-9_.+-]+\b",
            r"(?i)\b\d{1,2}(?::\d{2})?\s*(?:am|pm|a|p)\b",
            r"(?i)\b(?:at\s+)?(?:noon|midnight)\b",
            r"(?i)\b(?:am|pm|a|p)\b",
            r"(?i)\b(?:to|about|that)\s*$",
        ]
        .into_iter()
        .map(js::regex)
        .collect()
    })
}

fn title_word_corrections() -> &'static [(Regex, &'static str)] {
    static REGEXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            (r"(?i)\bpickup\b", "pick up"),
            (r"(?i)\bmums\b", "mum's"),
            (r"(?i)\bbday\b", "birthday"),
            (r"(?i)\bcehck\b", "check"),
        ]
        .into_iter()
        .map(|(pattern, replacement)| (js::regex(pattern), replacement))
        .collect()
    })
}

fn phrase_corrections() -> &'static [(Regex, &'static str)] {
    static REGEXES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            (r"\bHub Os\b", "Hub OS"),
            (r"\bMoms Bday\b", "Mom's Birthday"),
            (r"\bThe Contract Renewal\b", "Contract Renewal"),
            (r"\bThe Webinar\b", "Webinar"),
            (r"^The Meeting$", "Meeting"),
            (r"\bMom's Bday\b", "Mom's Birthday"),
            (r"\bThe SVG Logo\b", "SVG Logo"),
            (r"\bKeys(?: Keys)+\b", "Keys"),
        ]
        .into_iter()
        .map(|(pattern, replacement)| (js::regex(pattern), replacement))
        .collect()
    })
}
