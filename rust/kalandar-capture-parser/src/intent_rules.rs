use super::shadow_text::normalize_shadow_text;
use regex::Regex;
use std::{
    collections::BTreeSet,
    sync::{LazyLock, OnceLock},
};

pub(crate) fn normalize_intent_input(input: &str) -> String {
    let shadow = normalize_shadow_text(input);
    let collapsed = shadow
        .replace(['’', '\''], "'")
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let without_noise = repeated_punctuation_regex()
        .replace_all(&collapsed, " ")
        .into_owned();
    let without_brackets = bracket_noise_regex()
        .replace_all(&without_noise, " ")
        .into_owned();

    tokens(
        whitespace_regex()
            .replace_all(&without_brackets, " ")
            .trim(),
    )
    .into_iter()
    .filter(|token| {
        let alpha_only = token
            .chars()
            .filter(|character| character.is_ascii_lowercase())
            .collect::<String>();
        !FILLER_WORDS.contains(alpha_only.as_str())
    })
    .collect::<Vec<_>>()
    .join(" ")
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
        && !TASK_LEAD_WORDS.contains(tokens.first().map(String::as_str).unwrap_or_default())
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
        && time_blocked_task_regex().is_match(normalized)
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
    time_regex().is_match(value)
}

pub(crate) fn has_date_signal(value: &str) -> bool {
    date_regex().is_match(value)
}

pub(crate) fn has_recurrence_signal(value: &str) -> bool {
    recurrence_regex().is_match(value)
}

fn has_duration_signal(value: &str) -> bool {
    duration_regex().is_match(value)
}

fn has_strong_calendar_frame(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| STRONG_CALENDAR_WORDS.contains(token.as_str()))
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

fn repeated_punctuation_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"[!?.,]{2,}").expect("intent punctuation regex should compile"))
}

fn bracket_noise_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r#"[()\[\]{}"]"#).expect("intent bracket regex should compile"))
}

fn whitespace_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\s+").expect("intent whitespace regex should compile"))
}

fn explicit_reminder_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+me\b")
                .expect("intent regex should compile"),
            Regex::new(
                r"\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+(?:abt|about|to|2)\b",
            )
            .expect("intent regex should compile"),
            Regex::new(r"\b(?:set|create|add|make)\s+(?:me\s+)?(?:an?\s+)?reminder\b")
                .expect("intent regex should compile"),
            Regex::new(r"\breminder\s+(?:to|for|about)\b").expect("intent regex should compile"),
            Regex::new(r"\b(?:remember|remmber|rember|remeber|remembr)\b")
                .expect("intent regex should compile"),
            Regex::new(r"\bneed\s+to\s+remember\b").expect("intent regex should compile"),
            Regex::new(r"\b(?:alert|notify|tell)\s+me\b").expect("intent regex should compile"),
            Regex::new(r"\b(?:dont|don't|do not)\s+forget\b").expect("intent regex should compile"),
            Regex::new(r"\bdont\s+4get\b").expect("intent regex should compile"),
            Regex::new(r"\bdon'?t\s+let\s+me\s+forget\b").expect("intent regex should compile"),
            Regex::new(r"\bnote\s+to\s+self\b").expect("intent regex should compile"),
            Regex::new(r"\bping\s+me\b").expect("intent regex should compile"),
            Regex::new(r"\bheads\s+up\b").expect("intent regex should compile"),
            Regex::new(r"\bnudge\s+me\b").expect("intent regex should compile"),
            Regex::new(r"\bsend\s+me\s+a\s+nudge\b").expect("intent regex should compile"),
        ]
    })
}

fn explicit_none_note_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"^(?:just\s+)?(?:an?\s+)?(?:random\s+)?thought\s+(?:about|on|re|regarding)\b").expect("intent regex should compile"),
            Regex::new(r"^(?:just\s+)?(?:an?\s+)?idea\s+(?:about|for|on|re|regarding)\b").expect("intent regex should compile"),
            Regex::new(r"^(?:just\s+)?(?:an?\s+)?question\s+(?:about|for|on|re|regarding)\b").expect("intent regex should compile"),
            Regex::new(r"^(?:just\s+)?(?:some\s+)?notes?\s+(?:about|for|on|re|regarding)\b").expect("intent regex should compile"),
            Regex::new(r"^(?:just\s+)?(?:an?\s+)?(?:observation|reflection|musing)\s+(?:about|on|re|regarding)\b").expect("intent regex should compile"),
        ]
    })
}

fn explicit_none_soft_action_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"^(?:should\s+i|could\s+we|what\s+if|how\s+about)\b")
                .expect("intent regex should compile"),
            Regex::new(r"^(?:consider|think\s+about|explore)\b")
                .expect("intent regex should compile"),
            Regex::new(r"\bat\s+some\s+point\b").expect("intent regex should compile"),
            Regex::new(r"^(?:ability|option|support|feature)\s+to\b")
                .expect("intent regex should compile"),
            Regex::new(r"^(?:auto|automatic(?:ally)?)\s+[a-z]")
                .expect("intent regex should compile"),
        ]
    })
}

fn time_blocked_task_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(?:starting\s+at|start(?:ing)?\s+at|for\s+\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?))\b")
            .expect("intent time-blocked task regex should compile")
    })
}

fn time_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(?:[01]?\d(?::[0-5]\d)?\s?(?:a|p|am|pm)|[01]?\d(?:[ap])|\d{1,2}|noon|midnight|half\s+[01]?\d|\d{3,4})\b")
            .expect("intent time regex should compile")
    })
}

fn date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(?:(?:today|tonight|tomorrow|tmr|tmrw|next\s+(?:week|month|(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))|this\s+(?:week|month|(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))|in\s+a\s+fortnight|fortnight|end\s+of\s+(?:day|week)|eod)|(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}(?:-\d{1,2})?|\d{1,2}[/-]\d{1,2}|the\s+\d{1,2}(?:st|nd|rd|th)?)\b")
            .expect("intent date regex should compile")
    })
}

fn recurrence_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(?:every\s+(?:other\s+)?(?:weekday|week|day|month|(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))|weekly|daily|monthly|yearly|every\s+week\s+on\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))\b")
            .expect("intent recurrence regex should compile")
    })
}

fn duration_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?)\b")
            .expect("intent duration regex should compile")
    })
}
