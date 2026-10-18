use super::{
    postprocess::{finalize_reminder_title, normalize_whitespace},
    schedule::{context_for, ScheduleOutput},
    CapturePreviewRecurrenceException, CapturePreviewReminderDraft, CapturePreviewRequest,
    CapturePreviewWarning, PreviewSchedule,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use kalandar_dates::capture::{
    capture_schedule, CaptureKind, CaptureSchedule, ScheduleSpan, Settings,
};
use kalandar_dates::chrono_tz::Tz;
use regex::Regex;
use std::sync::OnceLock;

pub struct ReminderParseOutput {
    pub draft: CapturePreviewReminderDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub confidence: f64,
    pub schedule: Option<PreviewSchedule>,
}

const WEEKDAY_TOKEN: &str = "(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)";
const MONTH_TOKEN: &str = "(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sep|sept|october|oct|november|nov|december|dec)";

const MONTHS: &[(&str, u32)] = &[
    ("jan", 1),
    ("january", 1),
    ("feb", 2),
    ("february", 2),
    ("mar", 3),
    ("march", 3),
    ("apr", 4),
    ("april", 4),
    ("may", 5),
    ("jun", 6),
    ("june", 6),
    ("jul", 7),
    ("july", 7),
    ("aug", 8),
    ("august", 8),
    ("sep", 9),
    ("sept", 9),
    ("september", 9),
    ("oct", 10),
    ("october", 10),
    ("nov", 11),
    ("november", 11),
    ("dec", 12),
    ("december", 12),
];

pub fn parse_reminder_preview(text: &str, request: &CapturePreviewRequest) -> ReminderParseOutput {
    let working = normalize_whitespace(text);
    let working = prefix_pass(&working);
    let found = schedule_pass(&working, request);
    let title = finalize_reminder_title(&found.remaining);
    let reference = reference_date(request);

    let recurrence_exceptions = to_recurrence_exceptions(
        &found.texts.recurrence_exceptions,
        found.schedule.as_ref(),
        reference,
    );
    let remind_at_text = found.texts.moment.clone().or_else(|| found.clock_time());
    let recurrence = found.reminder_recurrence();
    let context_hint = found.context_hint();
    let warnings = finalize_reminder_warnings(
        remind_at_text.as_deref(),
        recurrence.is_some(),
        found.warnings,
    );

    ReminderParseOutput {
        draft: CapturePreviewReminderDraft {
            title,
            remind_at_text,
            recurrence,
            recurrence_start_text: found.texts.recurrence_start,
            recurrence_end_text: found.texts.recurrence_end,
            recurrence_exceptions,
            recurrence_window_text: found.texts.recurrence_window,
            location: None,
            context_hint,
        },
        warnings,
        confidence: 0.0,
        schedule: found.schedule,
    }
}

pub(crate) fn ascii_regex(pattern: &str) -> Regex {
    let translated = pattern.replace(r"\b", r"(?-u:\b)").replace(r"\d", "[0-9]");
    Regex::new(&translated)
        .unwrap_or_else(|error| panic!("rule regex should compile: {pattern}: {error}"))
}

pub(crate) fn trailing_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)[\s,.-]+\blol\b[!?.]*$",
            r"(?i)[\s,.-]+\bpls\b[!?.]*$",
            r"(?i)[\s,.-]+\bplease\b[!?.]*$",
            r"(?i)[!?.]+$",
        ]
        .into_iter()
        .map(ascii_regex)
        .collect()
    })
}

fn prefix_pass(working: &str) -> String {
    strip_prefixes(working)
}

fn strip_prefixes(input: &str) -> String {
    let mut working = input.to_string();
    for pattern in prefix_filler_patterns() {
        working = pattern.replace(&working, "").into_owned();
    }
    for pattern in prefix_patterns() {
        working = pattern.replace(&working, " ").into_owned();
    }
    for pattern in trailing_filler_patterns() {
        working = pattern.replace(&working, "").into_owned();
    }
    normalize_whitespace(&working)
}

fn schedule_pass(working: &str, request: &CapturePreviewRequest) -> ScheduleOutput {
    let context = context_for(request).with_settings(Settings::default());
    let found = capture_schedule(working, CaptureKind::Reminder, None, &context);
    let remaining = remove_schedule_spans(working, &found.spans);
    apply_reminder_schedule(found, remaining)
}

fn apply_reminder_schedule(found: CaptureSchedule, remaining: String) -> ScheduleOutput {
    let warnings = schedule_warnings(&found);
    ScheduleOutput {
        remaining,
        texts: found.texts,
        schedule: found.schedule,
        warnings,
    }
}

fn schedule_warnings(found: &CaptureSchedule) -> Vec<CapturePreviewWarning> {
    found
        .warnings
        .iter()
        .map(|warning| CapturePreviewWarning {
            code: warning.code.to_string(),
            severity: "info".to_string(),
            message: warning.message.clone(),
        })
        .collect()
}

fn remove_schedule_spans(text: &str, spans: &[ScheduleSpan]) -> String {
    let mut ordered: Vec<(usize, usize)> =
        spans.iter().map(|span| (span.start, span.end)).collect();
    ordered.sort_unstable();
    let slice = |start: usize, end: usize| {
        text.get(start.min(text.len())..end.min(text.len()))
            .unwrap_or_default()
    };
    let mut output = String::new();
    let mut cursor = 0;
    for (start, end) in ordered {
        if start >= cursor {
            output.push_str(slice(cursor, start));
            output.push(' ');
        }
        cursor = cursor.max(end);
    }
    output.push_str(slice(cursor, text.len()));
    normalize_whitespace(&output)
}

fn build_reminder_warnings(
    remind_at: Option<&str>,
    has_recurrence: bool,
) -> Vec<CapturePreviewWarning> {
    if remind_at.is_some_and(|text| !text.is_empty()) || has_recurrence {
        return Vec::new();
    }
    vec![CapturePreviewWarning {
        code: "no_remind_time".to_string(),
        severity: "warning".to_string(),
        message: "No time specified for reminder.".to_string(),
    }]
}

fn finalize_reminder_warnings(
    remind_at: Option<&str>,
    has_recurrence: bool,
    schedule_warnings: Vec<CapturePreviewWarning>,
) -> Vec<CapturePreviewWarning> {
    let mut warnings = build_reminder_warnings(remind_at, has_recurrence);
    warnings.extend(schedule_warnings);
    warnings
}

fn reference_date(request: &CapturePreviewRequest) -> NaiveDate {
    let now = request
        .now_iso
        .as_deref()
        .and_then(|iso| iso.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);
    let zone: Tz = request.timezone.parse().unwrap_or(Tz::UTC);
    now.with_timezone(&zone).date_naive()
}

fn to_recurrence_exceptions(
    raws: &[String],
    schedule: Option<&PreviewSchedule>,
    reference: NaiveDate,
) -> Vec<CapturePreviewRecurrenceException> {
    let resolved = schedule
        .and_then(|schedule| schedule.recurrence.as_ref())
        .map(|recurrence| recurrence.exceptions.as_slice())
        .unwrap_or(&[]);
    let same_count = resolved.len() == raws.len();
    raws.iter()
        .enumerate()
        .map(|(index, raw)| {
            if resolved.is_empty() {
                let (condition, action) = classify_recurrence_exception(raw, reference);
                return CapturePreviewRecurrenceException {
                    raw: raw.clone(),
                    condition: condition.map(str::to_string),
                    action: action.map(str::to_string),
                };
            }
            let matched = if same_count {
                resolved.get(index)
            } else {
                resolved.iter().find(|exception| {
                    exception.raw.contains(raw.as_str()) || raw.contains(exception.raw.as_str())
                })
            };
            CapturePreviewRecurrenceException {
                raw: raw.clone(),
                condition: matched.and_then(|exception| exception.condition.clone()),
                action: matched.and_then(|exception| exception.action.clone()),
            }
        })
        .collect()
}

fn classify_recurrence_exception(
    raw: &str,
    reference: NaiveDate,
) -> (Option<&'static str>, Option<&'static str>) {
    let condition = classify_recurrence_exception_condition(raw, reference);
    let mut action = classify_recurrence_exception_action(raw);
    if action.is_none() && condition.is_some() && unless_regex().is_match(raw.trim()) {
        action = Some("skip");
    }
    (condition, action)
}

fn classify_recurrence_exception_condition(
    raw: &str,
    reference: NaiveDate,
) -> Option<&'static str> {
    if holiday_regex().is_match(raw) {
        return Some("holiday");
    }
    if weekend_regex().is_match(raw) {
        return Some("weekend");
    }
    if ordinal_patterns()
        .iter()
        .any(|pattern| pattern.is_match(raw))
    {
        return Some("ordinal");
    }
    extract_specific_date(raw, reference).map(|_| "specific_date")
}

fn classify_recurrence_exception_action(raw: &str) -> Option<&'static str> {
    if shift_regex().is_match(raw) {
        return Some("shift");
    }
    if let Some(captures) = replace_regex().captures(raw) {
        let replacement = trailing_replacement_punctuation().replace_all(captures[1].trim(), "");
        if !replacement.is_empty() {
            return Some("replace");
        }
    }
    if skip_regex().is_match(raw) || but_not_regex().is_match(raw) {
        return Some("skip");
    }
    None
}

fn number<T: std::str::FromStr + Default>(digits: &str) -> T {
    digits.parse().unwrap_or_default()
}

fn calendar_date(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    if year < 100 {
        return None;
    }
    NaiveDate::from_ymd_opt(year, month, day)
}

fn normalize_iso_date_year(
    month: u32,
    day: u32,
    explicit_year: Option<i32>,
    reference: NaiveDate,
) -> Option<NaiveDate> {
    let year = explicit_year.unwrap_or_else(|| reference.year());
    let date = calendar_date(year, month, day)?;
    if explicit_year.is_none() && date < reference {
        if let Some(next) = calendar_date(year + 1, month, day) {
            return Some(next);
        }
    }
    Some(date)
}

fn extract_specific_date(raw: &str, reference: NaiveDate) -> Option<NaiveDate> {
    if let Some(captures) = iso_date_regex().captures(raw) {
        return calendar_date(
            number(&captures[1]),
            number(&captures[2]),
            number(&captures[3]),
        );
    }
    if let Some(captures) = slash_date_regex().captures(raw) {
        let explicit_year = captures.get(3).map(|year| {
            let value: i32 = number(year.as_str());
            if year.as_str().len() == 2 {
                2000 + value
            } else {
                value
            }
        });
        return normalize_iso_date_year(
            number(&captures[1]),
            number(&captures[2]),
            explicit_year,
            reference,
        );
    }
    let captures = month_name_date_regex().captures(raw)?;
    let token = captures[1].to_lowercase();
    let month = MONTHS
        .iter()
        .find(|(name, _)| *name == token)
        .map(|(_, month)| *month)?;
    normalize_iso_date_year(
        month,
        number(&captures[2]),
        captures.get(3).map(|year| number(year.as_str())),
        reference,
    )
}

fn prefix_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"^[\p{Emoji}\p{Emoji_Modifier}\p{Emoji_Component}\x{FE0F}\x{200D}\s!?.,:;-]+",
            r"(?i)^\s*hey siri i mean claude\b[\s,:-]*",
            r"(?i)^\s*(?:hey|yo|pls|please)\b[\s,:-]*",
        ]
        .into_iter()
        .map(ascii_regex)
        .collect()
    })
}

fn prefix_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)\bpls\s+remind\s+me\s+to\b",
            r"(?i)\bpls\s+remind\s+me\b",
            r"(?i)\bplease\s+to\s+remind\s+me\s+about\b",
            r"(?i)\bplease\s+to\s+remind\s+me\b",
            r"(?i)\bhey\s+remind\s+me\b",
            r"(?i)\byo\s+remind\s+me\b",
            r"(?i)\bgotta\s+remember\s+to\b",
            r"(?i)\bneed\s+to\s+remember\b",
            r"(?i)\bneed\s+to\s+remembr\s+to\b",
            r"(?i)\bi should probably remember to\b",
            r"(?i)\bremind\s*me\s+to\b",
            r"(?i)\bremind\s*me\s+about\b",
            r"(?i)\bremind\s*me\s+that\b",
            r"(?i)\bremind\s*me\s+2\b",
            r"(?i)\bremind\s*me\s+two\b",
            r"(?i)\bremind\s*me\b",
            r"(?i)\bremindme\s+to\b",
            r"(?i)\bremindme\b",
            r"(?i)\brmind\s+me\b",
            r"(?i)\bremnd me\b",
            r"(?i)\bremmber\s+to\b",
            r"(?i)\brememeber\b",
            r"(?i)\bremember\s+to\b",
            r"(?i)\bremember\b",
            r"(?i)\bdon'?t\s+let\s+me\s+forget\s+about\b",
            r"(?i)\bdon'?t\s+forget\s+to\b",
            r"(?i)\bdon'?t\s+forget\b",
            r"(?i)\bdont\s+forget\s+to\b",
            r"(?i)\bdont\s+forget\b",
            r"(?i)\bdont\s+foreget\b",
            r"(?i)\bdont\s+fore?get\b",
            r"(?i)\bdont\s+let\s+me\s+forgt\b",
            r"(?i)\bdont\s+let\s+me\s+forget\b",
            r"(?i)\bdont\s+4get\b",
            r"(?i)\bnote\s+to\s+self\b",
            r"(?i)\bping\s+me\s+about\b",
            r"(?i)\bping\s+me\s+to\b",
            r"(?i)\bping\s+me\b",
            r"(?i)\bheads\s+up\b",
        ]
        .into_iter()
        .map(ascii_regex)
        .collect()
    })
}

fn holiday_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)\bholidays?\b"))
}

fn weekend_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)\bweekends?\b"))
}

fn ordinal_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"(?i)\bskip\s+the\s+(first|second|third|last)(?:\s+(?:one|occurrence|instance))?\b".to_string(),
            r"(?i)\bbut\s+not\s+the\s+(first|second|third|last)\b".to_string(),
            r"(?i)\bnot\s+the\s+(first|second|third|last)\b".to_string(),
            r"(?i)\bthe\s+(first|second|third|last)\s+(?:one|occurrence|instance)\b".to_string(),
            format!(r"(?i)\bthe\s+(first|second|third|last)\s+{WEEKDAY_TOKEN}\s+of\s+(?:the\s+|every\s+)?month\b"),
        ]
        .iter()
        .map(|pattern| ascii_regex(pattern))
        .collect()
    })
}

fn iso_date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"\b(\d{4})-(\d{2})-(\d{2})\b"))
}

fn slash_date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"\b(\d{1,2})[/-](\d{1,2})(?:[/-](\d{2,4}))?\b"))
}

fn month_name_date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        ascii_regex(&format!(
            r"(?i)\b{MONTH_TOKEN}\s+(\d{{1,2}})(?:st|nd|rd|th)?(?:,?\s+(\d{{4}}))?\b"
        ))
    })
}

fn shift_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        ascii_regex(&format!(
            r"(?i)\b(?:then\s+)?(?:move|shift)?(?:\s+to)?\s*(?:the\s+)?(following|next|preceding|previous|prior)\s+{WEEKDAY_TOKEN}\b"
        ))
    })
}

fn replace_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)\b(?:replace|replaced)\s+with\s+(.+?)$"))
}

fn trailing_replacement_punctuation() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"[.,;]+$"))
}

fn skip_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)\bskip\b"))
}

fn but_not_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)\bbut\s+not\b"))
}

fn unless_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"(?i)^(?:unless|except)\b"))
}
