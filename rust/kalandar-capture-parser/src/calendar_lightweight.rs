use super::{
    postprocess::finalize_calendar_title, CapturePreviewCalendarEventDraft,
    CapturePreviewRecurrenceException, CapturePreviewRequest, CapturePreviewResult,
    CapturePreviewWarning,
};
use regex::Regex;
use std::sync::OnceLock;

pub fn parse_calendar_preview(
    text: &str,
    _request: &CapturePreviewRequest,
) -> CapturePreviewResult {
    let mut working = normalize_whitespace(text);

    let attendees = extract_attendees(text);
    let recurrence_text = extract_recurrence_text(&mut working);
    let recurrence_exceptions = if recurrence_text.is_some() {
        extract_recurrence_exceptions(&mut working)
    } else {
        Vec::new()
    };
    let recurrence_start_text = if recurrence_text.is_some() {
        consume_regex(&mut working, recurrence_start_regex(), false, None)
    } else {
        None
    };
    let recurrence_end_text = if recurrence_text.is_some() {
        consume_regex(&mut working, recurrence_end_regex(), false, None)
    } else {
        None
    };

    let time_range = extract_time_range(&mut working);
    let time_text = time_range
        .as_ref()
        .map(|range| range.0.clone())
        .or_else(|| consume_regex(&mut working, time_regex(), true, Some(1)));
    let end_time_text = time_range.as_ref().and_then(|range| range.1.clone());
    let explicit_duration = consume_regex(&mut working, duration_regex(), true, Some(1));
    let duration_minutes = explicit_duration
        .as_deref()
        .and_then(parse_duration_minutes)
        .or_else(|| parse_time_range_duration(time_text.as_deref(), end_time_text.as_deref()))
        .or_else(|| time_text.as_ref().map(|_| 60));
    let date_text = consume_regex(&mut working, date_regex(), false, None);
    let location = extract_location(&mut working);
    let title = build_calendar_title(&working, text);

    let mut warnings = Vec::new();
    if date_text.is_none() && time_text.is_none() && recurrence_text.is_none() {
        warnings.push(warning(
            "calendar-time-missing",
            "warning",
            "No date, time, or recurrence extracted.",
        ));
    }

    CapturePreviewResult::calendar_event(
        CapturePreviewCalendarEventDraft {
            title: (!title.is_empty()).then_some(title),
            date_text,
            time_text,
            end_time_text,
            duration_minutes,
            location,
            recurrence_text,
            recurrence_start_text,
            recurrence_end_text,
            recurrence_exceptions,
            alert_offset_minutes: Vec::new(),
            attendees,
        },
        0.9,
        Vec::new(),
        warnings,
    )
}

fn warning(code: &str, severity: &str, message: &str) -> CapturePreviewWarning {
    CapturePreviewWarning {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_segment(working: &mut String, start: usize, end: usize) {
    *working = normalize_whitespace(&format!("{} {}", &working[..start], &working[end..]));
}

fn consume_regex(
    working: &mut String,
    regex: &Regex,
    group_only: bool,
    group_index: Option<usize>,
) -> Option<String> {
    let captures = regex.captures(working.as_str())?;
    let full = captures.get(0)?;
    let value = if group_only {
        let group = captures.get(group_index.unwrap_or(1))?;
        group.as_str().trim().to_string()
    } else {
        full.as_str().trim().to_string()
    };
    remove_segment(working, full.start(), full.end());
    Some(normalize_whitespace(&value))
}

fn extract_attendees(text: &str) -> Vec<String> {
    attendees_regex()
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|group| {
            group
                .as_str()
                .split(&[',', '&'][..])
                .flat_map(|entry| entry.split(" and "))
                .map(|entry| entry.trim().trim_start_matches('@').to_string())
                .filter(|entry| !entry.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_recurrence_text(working: &mut String) -> Option<String> {
    for regex in recurrence_patterns() {
        if let Some(full) = regex.find(working.as_str()) {
            let value = full.as_str().trim().to_string();
            remove_segment(working, full.start(), full.end());
            return Some(normalize_whitespace(&value));
        }
    }
    None
}

fn extract_recurrence_exceptions(working: &mut String) -> Vec<CapturePreviewRecurrenceException> {
    let mut exceptions = Vec::new();
    for regex in exception_rule_regexes() {
        while let Some((start, end, text)) = extract_bounded_clause(working.as_str(), regex) {
            remove_segment(working, start, end);
            exceptions.push(CapturePreviewRecurrenceException {
                raw: text,
                condition: None,
                action: None,
            });
        }
    }
    exceptions
}

fn extract_bounded_clause(working: &str, regex: &Regex) -> Option<(usize, usize, String)> {
    let matched = regex.find(working)?;
    let search_start = matched.end();
    let mut end = working.len();
    for boundary in exception_boundaries() {
        if let Some(found) = boundary.find(&working[search_start..]) {
            end = end.min(search_start + found.start());
        }
    }
    if let Some(punctuation) = Regex::new(r"^\s*[.,;:!?]+")
        .expect("calendar exception punctuation regex should compile")
        .find(&working[end..])
    {
        end += punctuation.end();
    }
    let text = normalize_whitespace(
        working[matched.start()..end].trim_end_matches(&['.', ',', ';', ':', '!', '?'][..]),
    );
    (!text.is_empty()).then_some((matched.start(), end, text))
}

fn extract_time_range(working: &mut String) -> Option<(String, Option<String>)> {
    let captures = time_range_regex().captures(working.as_str())?;
    let full = captures.get(0)?;
    let start = captures.get(1)?.as_str().trim().to_string();
    let end = captures
        .get(2)
        .map(|value| value.as_str().trim().to_string());
    remove_segment(working, full.start(), full.end());
    Some((
        normalize_whitespace(&start),
        end.map(|value| normalize_whitespace(&value)),
    ))
}

fn extract_location(working: &mut String) -> Option<String> {
    let captures = location_regex().captures(working.as_str())?;
    let full = captures.get(0)?;
    let group = captures.get(1)?;
    let value = trim_location_candidate(group.as_str())?;
    let candidate_end = group.start() + value.len();
    remove_segment(working, full.start(), candidate_end);
    Some(normalize_whitespace(&value))
}

fn build_calendar_title(input: &str, raw_input: &str) -> String {
    finalize_calendar_title(input, raw_input)
}

fn parse_duration_minutes(text: &str) -> Option<u32> {
    let captures = duration_amount_regex().captures(text)?;
    let value = captures.get(1)?.as_str().parse::<u32>().ok()?;
    let unit = captures.get(2)?.as_str().to_lowercase();
    Some(if unit.starts_with("hour") || unit.starts_with("hr") {
        value * 60
    } else {
        value
    })
}

fn parse_time_range_duration(start: Option<&str>, end: Option<&str>) -> Option<u32> {
    let (Some(start), Some(end)) = (start, end) else {
        return None;
    };
    let start_minutes = parse_clock_to_minutes(start)?;
    let end_minutes = parse_clock_to_minutes(end)?;
    (end_minutes > start_minutes).then_some((end_minutes - start_minutes) as u32)
}

fn parse_clock_to_minutes(value: &str) -> Option<i32> {
    let captures = time_component_regex().captures(value)?;
    let hour = captures.get(1)?.as_str().parse::<i32>().ok()?;
    let minute = captures
        .get(2)
        .and_then(|value| value.as_str().parse::<i32>().ok())
        .unwrap_or(0);
    let meridiem = captures
        .get(3)
        .map(|value| value.as_str().to_lowercase())
        .unwrap_or_default();
    let normalized_hour = match meridiem.as_str() {
        "pm" if hour < 12 => hour + 12,
        "am" if hour == 12 => 0,
        _ => hour,
    };
    Some(normalized_hour * 60 + minute)
}

fn recurrence_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\bevery\s+\d+\s+(?:days?|weeks?|months?|years?)\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+second\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\s+of\s+the\s+month\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+(?:first|1st|second|2nd|third|3rd|fourth|4th|last)\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\s+of\s+every\s+month\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+other\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+weekday\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+weekend\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+month\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+day\b").unwrap(),
            Regex::new(r"(?i)\b(?:daily|weekly|monthly|yearly)\b").unwrap(),
            Regex::new(r"(?i)\bevery\s+(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b").unwrap(),
        ]
    })
}

fn exception_rule_regexes() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\bunless\b").unwrap(),
            Regex::new(r"(?i)\bexcept(?:\s+for)?\b").unwrap(),
            Regex::new(r"(?i)\bbut\s+skip\b").unwrap(),
            Regex::new(r"(?i)\bbut\s+not\b").unwrap(),
            Regex::new(r"(?i)\bskip\s+the\s+first\s+one\b").unwrap(),
        ]
    })
}

fn exception_boundaries() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\s+(?:but\s+(?:skip|not)\b|unless\b|except(?:\s+for)?\b)").unwrap(),
            Regex::new(r"(?i)\s+(?:starting|beginning|from)\b").unwrap(),
            Regex::new(r"(?i)\s+(?:until|till|through|ending(?:\s+on)?)\b").unwrap(),
            Regex::new(r"(?i)\s*[.,;:!?](?:\s|$)").unwrap(),
        ]
    })
}

fn attendees_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bwith\s+([A-Za-z@][A-Za-z0-9@'._+-]*(?:\s*(?:,|and|&)\s*[A-Za-z@][A-Za-z0-9@'._+-]*)*)")
            .expect("calendar attendees regex should compile")
    })
}

fn date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(today|tomorrow|tonight|next\s+(?:week|month|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|this\s+(?:week|month|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)(?:\s+(?:morning|afternoon|evening|night|noon))?|(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}(?:st|nd|rd|th)?\s+(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*)\b",
        )
        .expect("calendar date regex should compile")
    })
}

fn time_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:at\s+)?((?:[1-9]|1[0-2])(?::[0-5]\d)?\s*(?:am|pm)|noon|midnight)\b")
            .expect("calendar time regex should compile")
    })
}

fn time_range_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b((?:[1-9]|1[0-2])(?::[0-5]\d)?\s*(?:am|pm))\s*(?:to|-)\s*((?:[1-9]|1[0-2])(?::[0-5]\d)?\s*(?:am|pm))\b")
            .expect("calendar time range regex should compile")
    })
}

fn time_component_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(\d{1,2})(?::(\d{2}))?\s*(am|pm)?\b")
            .expect("calendar time component regex should compile")
    })
}

fn duration_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bfor\s+(\d+\s*(?:minutes?|mins?|hours?|hrs?))\b")
            .expect("calendar duration regex should compile")
    })
}

fn duration_amount_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(\d+)\s*(minutes?|mins?|hours?|hrs?)\b")
            .expect("calendar duration amount regex should compile")
    })
}

fn recurrence_start_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:starting|beginning|from)\s+(?:today|tomorrow|tonight|next\s+\w+|this\s+\w+|[A-Za-z]+\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?)",
        )
        .expect("calendar recurrence start regex should compile")
    })
}

fn recurrence_end_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:until|till|through|ending(?:\s+on)?)\s+(?:today|tomorrow|tonight|next\s+\w+|this\s+\w+|[A-Za-z]+\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}(?:st|nd|rd|th)?\s+[A-Za-z]+|\d{1,2}[/-]\d{1,2}(?:[/-]\d{2,4})?)",
        )
        .expect("calendar recurrence end regex should compile")
    })
}

fn location_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:at|in)\s+([A-Za-z][A-Za-z0-9 '&/_-]*)")
            .expect("calendar location regex should compile")
    })
}

fn trim_location_candidate(value: &str) -> Option<String> {
    let stop_regex = Regex::new(
        r"(?i)\s+(?:today|tomorrow|tonight|next\b|this\b|every\b|daily\b|weekly\b|monthly\b|yearly\b|mon(?:day)?\b|tue(?:s|sday)?\b|wed(?:nesday)?\b|thu(?:r|rs|rsday|ursday)?\b|fri(?:day)?\b|sat(?:urday)?\b|sun(?:day)?\b|(?:jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\b|\d{1,2}(?::\d{2})?\s*(?:am|pm)\b|noon\b|midnight\b)",
    )
    .expect("calendar location stop regex should compile");
    let trimmed = if let Some(found) = stop_regex.find(value) {
        &value[..found.start()]
    } else {
        value
    };
    let candidate = trimmed
        .trim()
        .trim_matches(|char: char| ",.;:!?".contains(char));
    if candidate.is_empty() {
        return None;
    }
    let time_like = Regex::new(r"(?i)^(?:\d{1,2}(?::\d{2})?\s*(?:am|pm)?|noon|midnight)$")
        .expect("calendar location time-like regex should compile");
    (!time_like.is_match(candidate)).then_some(candidate.to_string())
}
