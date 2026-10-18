use super::{js, CapturePreviewReminderRecurrence, CapturePreviewRequest, CapturePreviewWarning};
use fancy_regex::Regex;
use kalandar_dates::capture::{
    CaptureSchedule, Context, Frequency, PreviewSchedule, ScheduleSpan, ScheduleTexts, Settings,
};
use kalandar_dates::chrono_tz::Tz;
use std::sync::OnceLock;

pub struct ScheduleOutput {
    pub remaining: String,
    pub texts: ScheduleTexts,
    pub schedule: Option<PreviewSchedule>,
    pub warnings: Vec<CapturePreviewWarning>,
}

impl ScheduleOutput {
    pub fn new(capture: CaptureSchedule, text: &str) -> Self {
        Self {
            remaining: remove_schedule_spans(text, &capture.spans),
            warnings: capture
                .warnings
                .iter()
                .map(|warning| CapturePreviewWarning {
                    code: warning.code.to_string(),
                    severity: "info".to_string(),
                    message: warning.message.clone(),
                })
                .collect(),
            texts: capture.texts,
            schedule: capture.schedule,
        }
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
        static HINT: OnceLock<Regex> = OnceLock::new();
        let hint = js::cached(
            &HINT,
            r"\b(end of (?:the )?(?:month|week|day)|next month|morning|noon|afternoon|evening|night)\b",
            "i",
        );
        [&self.texts.moment, &self.texts.time, &self.texts.recurrence]
            .into_iter()
            .flatten()
            .find_map(|text| hint.find(text).ok().flatten())
            .map(|m| {
                m.as_str()
                    .to_lowercase()
                    .replacen("end of the ", "end of ", 1)
            })
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

fn remove_schedule_spans(text: &str, spans: &[ScheduleSpan]) -> String {
    let mut ordered: Vec<&ScheduleSpan> = spans.iter().collect();
    ordered.sort_by_key(|span| (span.start, span.end));
    let mut output = String::new();
    let mut cursor = 0;
    for span in ordered {
        if span.start >= cursor {
            output.push_str(js::slice(text, cursor, span.start));
            output.push(' ');
        }
        cursor = cursor.max(span.end);
    }
    output.push_str(js::slice(text, cursor, text.len()));
    js::normalize_whitespace(&output)
}
