use super::js::{cached, compile, exec, group, test, trim};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use fancy_regex::Regex;
use kalandar_dates::chrono_tz::Tz;
use std::sync::OnceLock;

const WEEKDAY_TOKEN: &str = "(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)";
const MONTH_TOKEN: &str = "(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sep|sept|october|oct|november|nov|december|dec)";

pub(crate) struct ClassifyOptions {
    pub now: DateTime<Utc>,
    pub timezone: Tz,
}

pub(crate) struct StructuredRecurrenceException {
    pub raw: String,
    pub condition: Option<&'static str>,
    pub action: Option<&'static str>,
}

fn month_number(token: &str) -> Option<u32> {
    match token {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn ordinal_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            compile(
                r"\bskip\s+the\s+(first|second|third|last)(?:\s+(?:one|occurrence|instance))?\b",
                "i",
            ),
            compile(r"\bbut\s+not\s+the\s+(first|second|third|last)\b", "i"),
            compile(r"\bnot\s+the\s+(first|second|third|last)\b", "i"),
            compile(
                r"\bthe\s+(first|second|third|last)\s+(?:one|occurrence|instance)\b",
                "i",
            ),
            compile(
                &format!(
                    r"\bthe\s+(first|second|third|last)\s+{WEEKDAY_TOKEN}\s+of\s+(?:the\s+|every\s+)?month\b"
                ),
                "i",
            ),
        ]
    })
}

fn normalize_ordinal(value: &str) -> Option<&'static str> {
    match trim(value).to_lowercase().as_str() {
        "first" => Some("first"),
        "second" => Some("second"),
        "third" => Some("third"),
        "last" => Some("last"),
        _ => None,
    }
}

fn zoned_date(now: DateTime<Utc>, timezone: Tz) -> (i64, i64, i64) {
    let local = now.with_timezone(&timezone);
    (
        i64::from(local.year()),
        i64::from(local.month()),
        i64::from(local.day()),
    )
}

fn is_valid_calendar_date(year: i64, month: i64, day: i64) -> bool {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || (0..=99).contains(&year) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        i32::try_from(year),
        u32::try_from(month),
        u32::try_from(day),
    ) else {
        return false;
    };
    NaiveDate::from_ymd_opt(year, month, day).is_some()
}

fn to_iso_date(year: i64, month: i64, day: i64) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}

fn normalize_iso_date_year(
    month: i64,
    day: i64,
    explicit_year: Option<i64>,
    options: &ClassifyOptions,
) -> Option<String> {
    let reference = zoned_date(options.now, options.timezone);
    let mut year = explicit_year.unwrap_or(reference.0);
    if !is_valid_calendar_date(year, month, day) {
        return None;
    }
    if explicit_year.is_none() {
        let candidate = to_iso_date(year, month, day);
        let reference = to_iso_date(reference.0, reference.1, reference.2);
        if candidate < reference && is_valid_calendar_date(year + 1, month, day) {
            year += 1;
        }
    }
    Some(to_iso_date(year, month, day))
}

fn number(text: &str) -> i64 {
    text.parse().unwrap_or(0)
}

fn extract_specific_date(raw: &str, options: &ClassifyOptions) -> Option<String> {
    static ISO: OnceLock<Regex> = OnceLock::new();
    static SLASH: OnceLock<Regex> = OnceLock::new();
    static MONTH_NAME: OnceLock<Regex> = OnceLock::new();

    if let Some(iso) = exec(cached(&ISO, r"\b(\d{4})-(\d{2})-(\d{2})\b", ""), raw) {
        let (year, month, day) = (
            number(group(&iso, 1)),
            number(group(&iso, 2)),
            number(group(&iso, 3)),
        );
        return is_valid_calendar_date(year, month, day).then(|| to_iso_date(year, month, day));
    }

    if let Some(slash) = exec(
        cached(&SLASH, r"\b(\d{1,2})[/-](\d{1,2})(?:[/-](\d{2,4}))?\b", ""),
        raw,
    ) {
        let month = number(group(&slash, 1));
        let day = number(group(&slash, 2));
        let explicit_year = slash.get(3).map(|year| {
            if year.as_str().len() == 2 {
                2000 + number(year.as_str())
            } else {
                number(year.as_str())
            }
        });
        return normalize_iso_date_year(month, day, explicit_year, options);
    }

    let month_name = exec(
        cached(
            &MONTH_NAME,
            &format!(r"\b{MONTH_TOKEN}\s+(\d{{1,2}})(?:st|nd|rd|th)?(?:,?\s+(\d{{4}}))?\b"),
            "i",
        ),
        raw,
    )?;
    let month = month_number(&group(&month_name, 1).to_lowercase())?;
    let day = number(group(&month_name, 2));
    let explicit_year = month_name.get(3).map(|year| number(year.as_str()));
    normalize_iso_date_year(i64::from(month), day, explicit_year, options)
}

pub(crate) fn classify_recurrence_exception_condition(
    raw: &str,
    options: &ClassifyOptions,
) -> Option<&'static str> {
    static HOLIDAY: OnceLock<Regex> = OnceLock::new();
    static WEEKEND: OnceLock<Regex> = OnceLock::new();

    if test(cached(&HOLIDAY, r"\bholidays?\b", "i"), raw) {
        return Some("holiday");
    }
    if test(cached(&WEEKEND, r"\bweekends?\b", "i"), raw) {
        return Some("weekend");
    }
    for pattern in ordinal_patterns() {
        let which = exec(pattern, raw)
            .and_then(|found| found.get(1).map(|which| which.as_str()))
            .and_then(normalize_ordinal);
        if which.is_some() {
            return Some("ordinal");
        }
    }
    extract_specific_date(raw, options).map(|_| "specific_date")
}

fn normalize_weekday_token(text: &str) -> Option<&'static str> {
    let normalized = trim(text).to_lowercase();
    [
        ("mon", "monday"),
        ("tue", "tuesday"),
        ("wed", "wednesday"),
        ("thu", "thursday"),
        ("fri", "friday"),
        ("sat", "saturday"),
        ("sun", "sunday"),
    ]
    .into_iter()
    .find(|(prefix, _)| normalized.starts_with(prefix))
    .map(|(_, weekday)| weekday)
}

pub(crate) fn classify_recurrence_exception_action(raw: &str) -> Option<&'static str> {
    static SHIFT: OnceLock<Regex> = OnceLock::new();
    static REPLACE: OnceLock<Regex> = OnceLock::new();
    static TRAILING_PUNCTUATION: OnceLock<Regex> = OnceLock::new();
    static SKIP: OnceLock<Regex> = OnceLock::new();
    static BUT_NOT: OnceLock<Regex> = OnceLock::new();

    let shift = cached(
        &SHIFT,
        &format!(
            r"\b(?:then\s+)?(?:move|shift)?(?:\s+to)?\s*(?:the\s+)?(following|next|preceding|previous|prior)\s+{WEEKDAY_TOKEN}\b"
        ),
        "i",
    );
    if exec(shift, raw).is_some_and(|found| normalize_weekday_token(group(&found, 2)).is_some()) {
        return Some("shift");
    }

    if let Some(replace) = exec(
        cached(&REPLACE, r"\b(?:replace|replaced)\s+with\s+(.+?)$", "i"),
        raw,
    ) {
        let replacement = group(&replace, 1);
        if !replacement.is_empty() {
            let trailing = cached(&TRAILING_PUNCTUATION, r"[.,;]+$", "g");
            if !trailing.replace_all(trim(replacement), "").is_empty() {
                return Some("replace");
            }
        }
    }

    if test(cached(&SKIP, r"\bskip\b", "i"), raw)
        || test(cached(&BUT_NOT, r"\bbut\s+not\b", "i"), raw)
    {
        return Some("skip");
    }
    None
}

pub(crate) fn classify_recurrence_exception(
    raw: &str,
    options: &ClassifyOptions,
) -> StructuredRecurrenceException {
    static LEAD: OnceLock<Regex> = OnceLock::new();

    let condition = classify_recurrence_exception_condition(raw, options);
    let mut action = classify_recurrence_exception_action(raw);
    if action.is_none()
        && condition.is_some()
        && test(cached(&LEAD, r"^(?:unless|except)\b", "i"), trim(raw))
    {
        action = Some("skip");
    }
    StructuredRecurrenceException {
        raw: raw.to_string(),
        condition,
        action,
    }
}
