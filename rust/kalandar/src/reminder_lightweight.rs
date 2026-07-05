use super::{
    postprocess::finalize_reminder_title, CapturePreviewRecurrenceException,
    CapturePreviewReminderDraft, CapturePreviewReminderRecurrence, CapturePreviewRequest,
    CapturePreviewWarning,
};
use regex::Regex;
use std::sync::OnceLock;

pub struct ReminderParseOutput {
    pub draft: CapturePreviewReminderDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub confidence: f64,
}

pub fn parse_reminder_preview(text: &str, _request: &CapturePreviewRequest) -> ReminderParseOutput {
    let mut working = normalize_whitespace(text);
    working = strip_prefix_filler(&working);

    let recurrence = extract_recurrence(&mut working);
    let recurrence_exceptions = if recurrence.is_some() {
        extract_recurrence_exceptions(&mut working)
    } else {
        Vec::new()
    };
    let recurrence_start_text = if recurrence.is_some() {
        consume_regex(&mut working, recurrence_start_regex(), false, None)
    } else {
        None
    };
    let recurrence_window_text = if recurrence.is_some() {
        consume_regex(&mut working, recurrence_window_regex(), false, None)
    } else {
        None
    };
    let recurrence_end_text = if recurrence.is_some() {
        consume_regex(&mut working, recurrence_end_regex(), false, None)
    } else {
        None
    };
    if recurrence.is_some() {
        let _ = consume_regex(&mut working, monthly_anchor_regex(), false, None);
    }

    let mut context_hint = None;

    let remind_at_text = if recurrence.is_some() {
        let remind_at_text = consume_regex(&mut working, explicit_time_regex(), true, Some(1));
        if raw_recurring_context_hint_regex().is_match(text) {
            context_hint = extract_context_hint(&mut working);
        }
        remind_at_text
    } else {
        let time_text = consume_regex(&mut working, explicit_time_regex(), true, Some(1));
        let date_text = extract_one_off_date(&mut working, &mut context_hint);
        match (date_text, time_text) {
            (Some(date), Some(time)) => Some(format!("{date} {time}")),
            (Some(date), None) => Some(date),
            (None, Some(time)) => Some(time),
            (None, None) => None,
        }
    };

    let title = build_reminder_title(&working);

    let mut warnings = Vec::new();
    if remind_at_text.is_none() && recurrence.is_none() {
        warnings.push(warning(
            "no-remind-time",
            "warning",
            "No time specified for reminder.",
        ));
    }

    let confidence = if recurrence.is_some() {
        if recurrence_exceptions.is_empty()
            && recurrence_start_text.is_none()
            && recurrence_end_text.is_none()
            && recurrence_window_text.is_none()
            && context_hint.is_none()
        {
            0.9
        } else {
            0.96
        }
    } else if remind_at_text.is_some() {
        0.88
    } else if title != "Reminder" {
        0.62
    } else {
        0.4
    };

    ReminderParseOutput {
        draft: CapturePreviewReminderDraft {
            title,
            remind_at_text,
            recurrence,
            recurrence_start_text,
            recurrence_end_text,
            recurrence_exceptions,
            recurrence_window_text,
            location: None,
            context_hint,
        },
        warnings,
        confidence,
    }
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

fn strip_prefix_filler(text: &str) -> String {
    let mut output = normalize_whitespace(text);
    for pattern in prefix_filler_patterns() {
        output = normalize_whitespace(&pattern.replace(&output, " "));
    }
    output
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

fn extract_recurrence(working: &mut String) -> Option<CapturePreviewReminderRecurrence> {
    let multi_day_regex = Regex::new(
        r"(?i)\bevery\s+((?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)(?:\s*(?:,|and)\s*(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))+)\b",
    )
    .expect("multi-day recurrence regex should compile");
    if let Some(captures) = multi_day_regex.captures(working.as_str()) {
        let full = captures.get(0).expect("recurrence full match should exist");
        let group = captures.get(1).expect("recurrence group should exist");
        let days = split_weekday_list(group.as_str());
        remove_segment(working, full.start(), full.end());
        return Some(CapturePreviewReminderRecurrence {
            frequency: "weekly".to_string(),
            interval: 1,
            days: (!days.is_empty()).then_some(days),
        });
    }

    let recurrence_rules = [
        (
            Regex::new(r"(?i)\bevery\s+weekday\b").unwrap(),
            "weekly",
            Some(1u32),
            Some(vec![
                "monday".to_string(),
                "tuesday".to_string(),
                "wednesday".to_string(),
                "thursday".to_string(),
                "friday".to_string(),
            ]),
        ),
        (
            Regex::new(r"(?i)\bevery\s+other\s+day\b").unwrap(),
            "daily",
            Some(2u32),
            None,
        ),
        (
            Regex::new(r"(?i)\bevery\s+other\s+week\b").unwrap(),
            "weekly",
            Some(2u32),
            None,
        ),
    ];

    for (regex, frequency, interval, days) in recurrence_rules {
        if let Some(full) = regex.find(working.as_str()) {
            let recurrence = CapturePreviewReminderRecurrence {
                frequency: frequency.to_string(),
                interval: interval.expect("fixed recurrence interval should exist"),
                days,
            };
            remove_segment(working, full.start(), full.end());
            return Some(recurrence);
        }
    }

    if let Some(captures) = every_other_weekday_regex().captures(working.as_str()) {
        let full = captures
            .get(0)
            .expect("every-other weekday full match should exist");
        let day = normalize_weekday(
            captures
                .get(1)
                .expect("every-other weekday capture should exist")
                .as_str(),
        );
        remove_segment(working, full.start(), full.end());
        return Some(CapturePreviewReminderRecurrence {
            frequency: "weekly".to_string(),
            interval: 2,
            days: day.map(|value| vec![value]),
        });
    }

    if let Some(captures) = interval_recurrence_regex().captures(working.as_str()) {
        let full = captures
            .get(0)
            .expect("interval recurrence full match should exist");
        let amount = captures
            .get(1)
            .and_then(|value| value.as_str().parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1);
        let unit = captures
            .get(2)
            .expect("interval recurrence unit should exist")
            .as_str()
            .to_lowercase();
        remove_segment(working, full.start(), full.end());
        let frequency = if unit.starts_with("month") {
            "monthly"
        } else if unit.starts_with("week") {
            "weekly"
        } else {
            "daily"
        };
        return Some(CapturePreviewReminderRecurrence {
            frequency: frequency.to_string(),
            interval: amount,
            days: None,
        });
    }

    if let Some(captures) = weekly_on_weekday_regex().captures(working.as_str()) {
        let full = captures
            .get(0)
            .expect("weekly-on weekday full match should exist");
        let day = normalize_weekday(
            captures
                .get(1)
                .expect("weekly-on weekday capture should exist")
                .as_str(),
        );
        remove_segment(working, full.start(), full.end());
        return Some(CapturePreviewReminderRecurrence {
            frequency: "weekly".to_string(),
            interval: 1,
            days: day.map(|value| vec![value]),
        });
    }

    if let Some(captures) = every_weekday_regex().captures(working.as_str()) {
        let full = captures
            .get(0)
            .expect("every weekday full match should exist");
        let day = normalize_weekday(
            captures
                .get(1)
                .expect("every weekday capture should exist")
                .as_str(),
        );
        remove_segment(working, full.start(), full.end());
        return Some(CapturePreviewReminderRecurrence {
            frequency: "weekly".to_string(),
            interval: 1,
            days: day.map(|value| vec![value]),
        });
    }

    for (regex, frequency) in [
        (Regex::new(r"(?i)\bevery\s+day\b").unwrap(), "daily"),
        (Regex::new(r"(?i)\bevery\s+month\b").unwrap(), "monthly"),
        (Regex::new(r"(?i)^\s*daily\b").unwrap(), "daily"),
        (Regex::new(r"(?i)^\s*weekly\b").unwrap(), "weekly"),
        (Regex::new(r"(?i)^\s*monthly\b").unwrap(), "monthly"),
        (Regex::new(r"(?i)^\s*yearly\b").unwrap(), "yearly"),
    ] {
        if let Some(full) = regex.find(working.as_str()) {
            remove_segment(working, full.start(), full.end());
            return Some(CapturePreviewReminderRecurrence {
                frequency: frequency.to_string(),
                interval: 1,
                days: None,
            });
        }
    }

    None
}

fn split_weekday_list(value: &str) -> Vec<String> {
    value
        .split([',', '&'])
        .flat_map(|entry| entry.split("and"))
        .filter_map(normalize_weekday)
        .fold(Vec::<String>::new(), |mut acc, day| {
            if !acc.contains(&day) {
                acc.push(day);
            }
            acc
        })
}

fn normalize_weekday(value: &str) -> Option<String> {
    let normalized = value.trim().to_lowercase();
    if normalized.starts_with("mon") {
        Some("monday".to_string())
    } else if normalized.starts_with("tue") {
        Some("tuesday".to_string())
    } else if normalized.starts_with("wed") {
        Some("wednesday".to_string())
    } else if normalized.starts_with("thu") {
        Some("thursday".to_string())
    } else if normalized.starts_with("fri") {
        Some("friday".to_string())
    } else if normalized.starts_with("sat") {
        Some("saturday".to_string())
    } else if normalized.starts_with("sun") {
        Some("sunday".to_string())
    } else {
        None
    }
}

fn extract_recurrence_exceptions(working: &mut String) -> Vec<CapturePreviewRecurrenceException> {
    let mut exceptions = Vec::new();

    for (rule_id, regex) in [
        ("unless", Regex::new(r"(?i)\bunless\b").unwrap()),
        ("except", Regex::new(r"(?i)\bexcept(?:\s+for)?\b").unwrap()),
        ("but_skip", Regex::new(r"(?i)\bbut\s+skip\b").unwrap()),
        ("but_not", Regex::new(r"(?i)\bbut\s+not\b").unwrap()),
        (
            "skip_first",
            Regex::new(r"(?i)\bskip\s+the\s+first\s+one\b").unwrap(),
        ),
    ] {
        let _ = rule_id;
        while let Some((start, end, text)) = extract_bounded_clause(working.as_str(), &regex) {
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
        .expect("exception punctuation regex should compile")
        .find(&working[end..])
    {
        end += punctuation.end();
    }

    let text = normalize_whitespace(
        working[matched.start()..end].trim_end_matches(&['.', ',', ';', ':', '!', '?'][..]),
    );
    (!text.is_empty()).then_some((matched.start(), end, text))
}

fn extract_context_hint(working: &mut String) -> Option<String> {
    let captures = trailing_context_hint_regex().captures(working.as_str())?;
    let full = captures.get(0)?;
    let group = captures.get(1)?;
    let value = group.as_str().trim().to_lowercase();
    remove_segment(working, full.start(), full.end());
    Some(value)
}

fn extract_one_off_date(working: &mut String, context_hint: &mut Option<String>) -> Option<String> {
    for (regex, normalized_context) in [
        (month_day_regex(), None),
        (relative_clockless_regex(), None),
        (next_month_regex(), Some("next month")),
        (end_of_month_regex(), Some("end of month")),
        (end_of_week_regex(), Some("end of week")),
        (end_of_day_regex(), Some("end of day")),
        (named_day_regex(), None),
        (named_month_regex(), None),
        (ordinal_date_regex(), None),
    ] {
        if let Some(text) = consume_regex(working, regex, false, None) {
            if let Some(value) = normalized_context {
                *context_hint = Some(value.to_string());
            }
            return Some(text);
        }
    }

    None
}

fn build_reminder_title(input: &str) -> String {
    finalize_reminder_title(input)
}

fn prefix_filler_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)^\s*(?:please|pls)\b[\s,.-]*").unwrap(),
            Regex::new(r"(?i)[\s,.-]+\b(?:please|pls|lol|maybe|idk)\b\.?$").unwrap(),
        ]
    })
}

fn exception_boundaries() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\s+(?:but\s+(?:skip|not)\b|unless\b|except(?:\s+for)?\b)")
                .unwrap(),
            Regex::new(r"(?i)\s+(?:starting|beginning|from)\b").unwrap(),
            Regex::new(r"(?i)\s+for\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b")
                .unwrap(),
            Regex::new(r"(?i)\s+(?:until|till|through|ending(?:\s+on)?)\b").unwrap(),
            Regex::new(r"(?i)\s+(?:and)\s+(?:add|attach|bring|buy|call|check|clean|email|file|finish|fix|get|mail|pay|pick|plan|prep|prepare|refill|review|send|submit|water)\b")
                .unwrap(),
            Regex::new(r"(?i)\s*[.,;:!?](?:\s|$)").unwrap(),
        ]
    })
}

fn explicit_time_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:at\s+)?((?:[1-9]|1[0-2])(?::[0-5]\d)?\s*(?:am|pm)|noon|midnight)\b")
            .expect("explicit time regex should compile")
    })
}

fn named_day_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:today|tomorrow|tonight|next\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|this\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?))\b",
        )
        .expect("named day regex should compile")
    })
}

fn month_day_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:on\s+)?(?:(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}(?:st|nd|rd|th)?\s+(?:of\s+)?(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*)\b",
        )
        .expect("month-day regex should compile")
    })
}

fn named_month_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\b",
        )
        .expect("named month regex should compile")
    })
}

fn ordinal_date_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bon\s+the\s+\d{1,2}(?:st|nd|rd|th)\b")
            .expect("ordinal date regex should compile")
    })
}

fn relative_clockless_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bin\s+\d+\s+(?:minutes?|hours?|days?|weeks?|months?)\b")
            .expect("relative clockless regex should compile")
    })
}

fn end_of_month_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bend of (?:the )?month\b").expect("end of month regex should compile")
    })
}

fn end_of_week_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:end of (?:the )?week|eow)\b")
            .expect("end of week regex should compile")
    })
}

fn end_of_day_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:end of day|eod)\b").expect("end of day regex should compile")
    })
}

fn next_month_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bnext\s+month\b").expect("next month regex should compile")
    })
}

fn monthly_anchor_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bon\s+the\s+\d{1,2}(?:st|nd|rd|th)\b")
            .expect("monthly anchor regex should compile")
    })
}

fn recurrence_start_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:starting|beginning|from)\s+(?:today|tomorrow|tonight|next\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|this\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|\d{1,2}(?:st|nd|rd|th)?\s+(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*|(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?|\d{1,2}[/-]\d{1,2})\b",
        )
        .expect("recurrence start regex should compile")
    })
}

fn recurrence_window_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bfor\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b")
            .expect("recurrence window regex should compile")
    })
}

fn recurrence_end_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\b(?:until|till|through|ending(?:\s+on)?)\s+(?:today|tomorrow|tonight|next\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|this\s+(?:week|month|year|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)|\d{1,2}(?:st|nd|rd|th)?\s+(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*|(?:jan|feb|mar|apr|may|jun|jul|aug|sep(?:t)?|oct|nov|dec)[a-z]*\s+\d{1,2}(?:st|nd|rd|th)?|end of (?:the )?(?:week|month)|the week of the \d{1,2}(?:st|nd|rd|th)?)\b",
        )
        .expect("recurrence end regex should compile")
    })
}

fn trailing_context_hint_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:at\s+)?(morning|noon|afternoon|evening|night)\b[.,;:!?]*\s*$")
            .expect("trailing context hint regex should compile")
    })
}

fn raw_recurring_context_hint_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bevery\s+(?:other\s+)?(?:day|week|month|year|weekday|weekend|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\s+(?:at\s+)?(?:morning|noon|afternoon|evening|night)\b",
        )
        .expect("raw recurring context hint regex should compile")
    })
}

fn every_other_weekday_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bevery\s+other\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
        )
        .expect("every-other weekday regex should compile")
    })
}

fn interval_recurrence_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bevery\s+(\d+)\s+(days|weeks|months)\b")
            .expect("interval recurrence regex should compile")
    })
}

fn weekly_on_weekday_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bevery\s+week\s+on\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
        )
        .expect("weekly-on weekday regex should compile")
    })
}

fn every_weekday_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)\bevery\s+(mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
        )
        .expect("every weekday regex should compile")
    })
}
