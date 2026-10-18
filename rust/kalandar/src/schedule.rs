use super::{
    CapturePreviewRecurrenceException, CapturePreviewReminderRecurrence, CapturePreviewRequest,
    CapturePreviewWarning,
};
use kalandar_dates::capture::{
    capture_schedule, CaptureKind, Context, Frequency, PreviewSchedule, ScheduleTexts, Settings,
};
use kalandar_dates::chrono_tz::Tz;

pub struct ScheduleOutput {
    pub remaining: String,
    pub texts: ScheduleTexts,
    pub schedule: Option<PreviewSchedule>,
    pub warnings: Vec<CapturePreviewWarning>,
}

impl ScheduleOutput {
    pub fn recurrence_exceptions(&self) -> Vec<CapturePreviewRecurrenceException> {
        let resolved = self
            .schedule
            .as_ref()
            .and_then(|s| s.recurrence.as_ref())
            .map(|r| r.exceptions.as_slice())
            .unwrap_or(&[]);
        let same_count = resolved.len() == self.texts.recurrence_exceptions.len();
        self.texts
            .recurrence_exceptions
            .iter()
            .enumerate()
            .map(|(index, raw)| {
                let matched = if same_count {
                    resolved.get(index)
                } else {
                    resolved
                        .iter()
                        .find(|e| e.raw.contains(raw.as_str()) || raw.contains(e.raw.as_str()))
                };
                CapturePreviewRecurrenceException {
                    raw: raw.clone(),
                    condition: matched.and_then(|e| e.condition.clone()),
                    action: matched.and_then(|e| e.action.clone()),
                }
            })
            .collect()
    }

    pub fn clock_time(&self) -> Option<String> {
        self.texts.time.clone().filter(|t| {
            t.chars().any(|c| c.is_ascii_digit())
                || t.eq_ignore_ascii_case("noon")
                || t.eq_ignore_ascii_case("midnight")
        })
    }

    pub fn reminder_recurrence(&self) -> Option<CapturePreviewReminderRecurrence> {
        let recurrence = self.schedule.as_ref()?.recurrence.as_ref()?;
        let frequency = match recurrence.frequency {
            Frequency::Minutely => "minutely",
            Frequency::Hourly => "hourly",
            Frequency::Daily => "daily",
            Frequency::Weekly => "weekly",
            Frequency::Monthly => "monthly",
            Frequency::Yearly => "yearly",
        };
        let days = recurrence.days.clone().or_else(|| {
            let names: Vec<&str> = recurrence
                .weekdays
                .iter()
                .filter(|d| d.position.is_none())
                .map(|d| d.weekday)
                .collect();
            (!names.is_empty()).then_some(names)
        });
        Some(CapturePreviewReminderRecurrence {
            frequency: frequency.to_string(),
            interval: recurrence.interval,
            days: days.map(|d| d.into_iter().map(str::to_string).collect()),
        })
    }

    pub fn alert_offsets(&self) -> Vec<u32> {
        self.schedule
            .as_ref()
            .and_then(|s| s.alert_minutes_before.or(s.lead_minutes))
            .and_then(|m| u32::try_from(m).ok())
            .into_iter()
            .collect()
    }

    pub fn context_hint(&self) -> Option<String> {
        static HINT: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let hint = HINT.get_or_init(|| {
            regex::Regex::new(
                r"(?i)\b(end of (?:the )?(?:month|week|day)|next month|morning|noon|afternoon|evening|night)\b",
            )
            .expect("context hint regex should compile")
        });
        [&self.texts.moment, &self.texts.time, &self.texts.recurrence]
            .into_iter()
            .flatten()
            .find_map(|text| hint.find(text))
            .map(|m| m.as_str().to_lowercase().replace("end of the ", "end of "))
    }

    pub fn duration_minutes(&self) -> Option<u32> {
        self.schedule
            .as_ref()
            .and_then(|s| s.duration_minutes)
            .and_then(|m| u32::try_from(m).ok())
    }
}

pub fn context_for(request: &CapturePreviewRequest) -> Context {
    let now = request
        .now_iso
        .as_deref()
        .and_then(|iso| iso.parse::<chrono::DateTime<chrono::Utc>>().ok())
        .unwrap_or_else(chrono::Utc::now);
    let zone: Tz = request.timezone.parse().unwrap_or(Tz::UTC);
    let settings = Settings {
        locale: request.locale.clone(),
        ..Settings::default()
    };
    Context::new(now, zone).with_settings(settings)
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn extract_schedule(
    text: &str,
    kind: CaptureKind,
    request: &CapturePreviewRequest,
) -> ScheduleOutput {
    let found = capture_schedule(text, kind, None, &context_for(request));
    let mut ranges: Vec<(usize, usize)> = found.spans.iter().map(|s| (s.start, s.end)).collect();
    ranges.sort_unstable();
    let mut remaining = String::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if start >= cursor {
            remaining.push_str(&text[cursor..start]);
            remaining.push(' ');
        }
        cursor = cursor.max(end);
    }
    remaining.push_str(&text[cursor..]);
    let warnings = found
        .warnings
        .iter()
        .map(|w| CapturePreviewWarning {
            code: w.code.to_string(),
            severity: "info".to_string(),
            message: w.message.clone(),
        })
        .collect();
    ScheduleOutput {
        remaining: normalize_whitespace(&remaining),
        texts: found.texts,
        schedule: found.schedule,
        warnings,
    }
}
