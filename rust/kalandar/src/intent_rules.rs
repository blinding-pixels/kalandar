use regex::Regex;
use std::{
    collections::BTreeSet,
    sync::{LazyLock, OnceLock},
};

pub(crate) fn normalize_intent_input(input: &str) -> String {
    let shadow = normalize_intent_shadow_text(input);
    let collapsed = normalize_js_whitespace(&shadow)
        .to_lowercase()
        .replace('’', "'");
    let without_emoji = emoji_regex().replace_all(&collapsed, " ");
    let without_punctuation = repeated_punctuation_regex().replace_all(&without_emoji, " ");
    let without_brackets = bracket_noise_regex().replace_all(&without_punctuation, " ");

    tokens(&normalize_js_whitespace(&without_brackets))
        .into_iter()
        .filter(|token| {
            let alpha_only = token
                .chars()
                .filter(char::is_ascii_lowercase)
                .collect::<String>();
            !FILLER_WORDS.contains(alpha_only.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn normalize_js_whitespace(value: &str) -> String {
    js_whitespace_regex()
        .replace_all(value, " ")
        .trim_matches(is_js_whitespace)
        .to_string()
}

pub(crate) fn tokens(value: &str) -> Vec<String> {
    value
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn is_explicit_reminder(normalized: &str) -> bool {
    explicit_reminder_patterns()
        .iter()
        .any(|pattern| pattern.is_match(normalized))
}

pub(crate) fn looks_like_reflective_note(normalized: &str, tokens: &[String]) -> bool {
    explicit_none_note_patterns()
        .iter()
        .any(|pattern| pattern.is_match(normalized))
        && !starts_with_task_lead(tokens)
}

pub(crate) fn looks_like_soft_action_idea(normalized: &str, tokens: &[String]) -> bool {
    explicit_none_soft_action_patterns()
        .iter()
        .any(|pattern| pattern.is_match(normalized))
        && !has_time_signal(normalized)
        && !has_date_signal(normalized)
        && !has_recurrence_signal(normalized)
        && !has_strong_calendar_frame(tokens)
}

pub(crate) fn starts_with_task_lead(tokens: &[String]) -> bool {
    TASK_LEAD_WORDS.contains(tokens.first().map(String::as_str).unwrap_or_default())
}

pub(crate) fn looks_like_time_blocked_task(normalized: &str, tokens: &[String]) -> bool {
    starts_with_task_lead(tokens)
        && has_time_signal(normalized)
        && has_duration_signal(normalized)
        && time_blocked_task_regex().is_match(&normalized.to_ascii_lowercase())
        && !has_strong_calendar_frame(tokens)
}

pub(crate) fn looks_like_dated_task(normalized: &str, tokens: &[String]) -> bool {
    DATE_TASK_LEAD_WORDS.contains(tokens.first().map(String::as_str).unwrap_or_default())
        && (has_date_signal(normalized) || has_recurrence_signal(normalized))
        && !has_strong_calendar_frame(tokens)
}

pub(crate) fn is_short_appointment_ambiguous(normalized: &str, tokens: &[String]) -> bool {
    tokens.len() <= 4
        && has_time_signal(normalized)
        && tokens
            .iter()
            .any(|token| APPOINTMENT_WORDS.contains(token.as_str()))
}

pub(crate) fn has_time_signal(value: &str) -> bool {
    time_regex().is_match(&value.to_ascii_lowercase())
}

pub(crate) fn has_date_signal(value: &str) -> bool {
    date_regex().is_match(&value.to_ascii_lowercase())
}

pub(crate) fn has_recurrence_signal(value: &str) -> bool {
    recurrence_regex().is_match(&value.to_ascii_lowercase())
}

fn has_duration_signal(value: &str) -> bool {
    duration_regex().is_match(&value.to_ascii_lowercase())
}

fn has_strong_calendar_frame(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| STRONG_CALENDAR_WORDS.contains(token.as_str()))
}

fn normalize_intent_shadow_text(input: &str) -> String {
    let mut output = input.to_string();
    for (pattern, replacement) in shadow_replacements() {
        output = replace_ascii_case_insensitive(pattern, &output, replacement);
    }
    let ranged = shadow_numeric_range_regex().replace_all(&output, "${1} to ${2}");
    normalize_js_whitespace(&ranged.replace(['–', '—'], "-"))
}

fn replace_ascii_case_insensitive(pattern: &Regex, input: &str, replacement: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut last = 0;
    for captures in pattern.captures_iter(&lowered) {
        let whole = captures.get_match();
        output.push_str(&input[last..whole.start()]);
        captures.expand(replacement, &mut output);
        last = whole.end();
    }
    output.push_str(&input[last..]);
    output
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

static FILLER_WORDS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "lol", "lmao", "pls", "plz", "yo", "hey", "uhhh", "uhh", "ummm", "umm", "um",
    ])
});

static TASK_LEAD_WORDS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "finish",
        "fix",
        "build",
        "write",
        "send",
        "update",
        "deploy",
        "review",
        "submit",
        "complete",
        "refactor",
        "cancel",
        "order",
        "buy",
        "get",
        "prepare",
        "draft",
        "ship",
        "call",
        "check",
        "book",
        "renew",
        "follow",
        "prep",
        "organize",
        "schedule",
        "grab",
        "pick",
        "clean",
        "set",
        "sign",
        "water",
        "email",
        "plan",
        "brainstorm",
    ])
});

static DATE_TASK_LEAD_WORDS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "finish",
        "fix",
        "build",
        "write",
        "send",
        "update",
        "deploy",
        "submit",
        "complete",
        "refactor",
        "cancel",
        "order",
        "buy",
        "get",
        "prepare",
        "draft",
        "ship",
        "check",
        "renew",
        "follow",
        "organize",
        "grab",
        "pick",
        "clean",
        "set",
        "sign",
        "water",
        "email",
        "brainstorm",
    ])
});

static APPOINTMENT_WORDS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from(["dentist", "doctor", "appointment", "appt", "haircut", "gym"])
});

static STRONG_CALENDAR_WORDS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    BTreeSet::from([
        "meeting",
        "session",
        "standup",
        "sync",
        "1on1",
        "interview",
        "lunch",
        "dinner",
        "brunch",
        "coffee",
        "drinks",
        "breakfast",
        "appointment",
        "appt",
        "flight",
        "rehearsal",
        "class",
        "yoga",
        "reservation",
        "workshop",
        "party",
        "birthday",
        "offsite",
        "town",
        "hall",
        "concert",
        "doctor",
        "dentist",
        "haircut",
        "gym",
        "trip",
    ])
});

const JS_WHITESPACE_CLASS: &str = r"[\t\n\x0B\x0C\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]";

const WEEKDAY_PATTERN: &str = r"(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)";

const MONTH_PATTERN: &str = r"(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)";

fn js_regex(pattern: &str) -> Regex {
    let translated = pattern
        .replace(r"\d", "[0-9]")
        .replace(r"\b", r"(?-u:\b)")
        .replace(r"\s", JS_WHITESPACE_CLASS);
    Regex::new(&translated).expect("intent regex should compile")
}

fn js_regexes(patterns: &[&str]) -> Vec<Regex> {
    patterns.iter().copied().map(js_regex).collect()
}

fn js_whitespace_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| js_regex(r"\s+"))
}

fn emoji_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?:\p{Extended_Pictographic}|\p{Emoji_Presentation}|\x{FE0F})")
            .expect("intent emoji regex should compile")
    })
}

fn repeated_punctuation_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"[!?.,]{2,}").expect("intent punctuation regex should compile"))
}

fn bracket_noise_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"[()\[\]{}"]"#).expect("intent bracket regex should compile"))
}

fn shadow_replacements() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    REPLACEMENTS.get_or_init(|| {
        [
            (r"\btmrw\b", "tomorrow"),
            (r"\btmr\b", "tomorrow"),
            (r"\btomorow\b", "tomorrow"),
            (r"\btommorow\b", "tomorrow"),
            (r"\btmorow\b", "tomorrow"),
            (r"\btonite\b", "tonight"),
            (r"\b2\s*hrs?\b", "2 hours"),
            (r"\b(\d+)\s*hrs?\b", "${1} hours"),
            (r"\b(\d+)\s*mins?\b", "${1} minutes"),
            (r"\b(\d+)\s*min\b", "${1} minutes"),
            (r"\b(\d+)\s*secs?\b", "${1} seconds"),
            (r"\btom(?:orrow)?\s+night\b", "tomorrow night"),
            (r"\btom(?:orrow)?\s+morn(?:ing)?\b", "tomorrow morning"),
            (r"\bfri\s+eve\b", "friday evening"),
            (r"\bfri\s+morn\b", "friday morning"),
            (r"\bmon\b", "monday"),
            (r"\btue\b", "tuesday"),
            (r"\btues\b", "tuesday"),
            (r"\bwed\b", "wednesday"),
            (r"\bthu\b", "thursday"),
            (r"\bthur\b", "thursday"),
            (r"\bthurs\b", "thursday"),
            (r"\bfri\b", "friday"),
            (r"\bsat\b", "saturday"),
            (r"\bsun\b", "sunday"),
        ]
        .into_iter()
        .map(|(pattern, replacement)| (js_regex(pattern), replacement))
        .collect()
    })
}

fn shadow_numeric_range_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| js_regex(r"\b(\d{1,2})\s*-\s*(\d{1,2})\b"))
}

fn explicit_reminder_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        js_regexes(&[
            r"\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+me\b",
            r"\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+(?:abt|about|to|2)\b",
            r"\b(?:set|create|add|make)\s+(?:me\s+)?(?:an?\s+)?reminder\b",
            r"\breminder\s+(?:to|for|about)\b",
            r"\b(?:remember|remmber|rember|remeber|remembr)\b",
            r"\bneed\s+to\s+remember\b",
            r"\b(?:alert|notify|tell)\s+me\b",
            r"\b(?:dont|don't|do not)\s+forget\b",
            r"\bdont\s+4get\b",
            r"\bdon'?t\s+let\s+me\s+forget\b",
            r"\bnote\s+to\s+self\b",
            r"\bping\s+me\b",
            r"\bheads\s+up\b",
            r"\bnudge\s+me\b",
            r"\bsend\s+me\s+a\s+nudge\b",
        ])
    })
}

fn explicit_none_note_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        js_regexes(&[
            r"^(?:just\s+)?(?:an?\s+)?(?:random\s+)?thought\s+(?:about|on|re|regarding)\b",
            r"^(?:just\s+)?(?:an?\s+)?idea\s+(?:about|for|on|re|regarding)\b",
            r"^(?:just\s+)?(?:an?\s+)?question\s+(?:about|for|on|re|regarding)\b",
            r"^(?:just\s+)?(?:some\s+)?notes?\s+(?:about|for|on|re|regarding)\b",
            r"^(?:just\s+)?(?:an?\s+)?(?:observation|reflection|musing)\s+(?:about|on|re|regarding)\b",
        ])
    })
}

fn explicit_none_soft_action_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        js_regexes(&[
            r"^(?:should\s+i|could\s+we|what\s+if|how\s+about)\b",
            r"^(?:consider|think\s+about|explore)\b",
            r"\bat\s+some\s+point\b",
            r"^(?:ability|option|support|feature)\s+to\b",
            r"^(?:auto|automatic(?:ally)?)\s+[a-z]",
        ])
    })
}

fn time_blocked_task_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        js_regex(r"\b(?:starting\s+at|start(?:ing)?\s+at|for\s+\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?))\b")
    })
}

fn time_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        js_regex(r"\b(?:[01]?\d(?::[0-5]\d)?\s?(?:a|p|am|pm)|[01]?\d(?:[ap])|\d{1,2}|noon|midnight|half\s+[01]?\d|\d{3,4})\b")
    })
}

fn date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        let relative = format!(
            r"(?:today|tonight|tomorrow|tmr|tmrw|next\s+(?:week|month|{WEEKDAY_PATTERN})|this\s+(?:week|month|{WEEKDAY_PATTERN})|in\s+a\s+fortnight|fortnight|end\s+of\s+(?:day|week)|eod)"
        );
        js_regex(&format!(
            r"\b(?:{relative}|{WEEKDAY_PATTERN}|{MONTH_PATTERN}\s+\d{{1,2}}(?:-\d{{1,2}})?|\d{{1,2}}[/-]\d{{1,2}}|the\s+\d{{1,2}}(?:st|nd|rd|th)?)\b"
        ))
    })
}

fn recurrence_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        js_regex(&format!(
            r"\b(?:every\s+(?:other\s+)?(?:weekday|week|day|month|{WEEKDAY_PATTERN})|weekly|daily|monthly|yearly|every\s+week\s+on\s+{WEEKDAY_PATTERN})\b"
        ))
    })
}

fn duration_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| js_regex(r"\b\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?)\b"))
}
