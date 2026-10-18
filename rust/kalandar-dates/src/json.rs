use chrono::{DateTime, NaiveTime, Utc, Weekday};
use serde_json::{json, Value};

use crate::capture::{
    capture_schedule, CaptureKind, Clock, Context, HolidaySetting, LabelledSpan, Role, Settings,
};

pub fn error(message: impl std::fmt::Display) -> String {
    json!({ "error": message.to_string() }).to_string()
}

pub fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut count = 0;
    for (index, c) in text.char_indices() {
        if count >= units {
            return index;
        }
        count += c.len_utf16();
    }
    text.len()
}

pub fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

pub fn kind_of(value: &Value) -> CaptureKind {
    match value.as_str() {
        Some("task") => CaptureKind::Task,
        Some("reminder") => CaptureKind::Reminder,
        Some("calendar") | Some("calendar-event") | Some("event") => CaptureKind::CalendarEvent,
        _ => CaptureKind::CalendarEvent,
    }
}

fn time_of(value: &Value) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.as_str()?, "%H:%M").ok()
}

fn weekday_of(value: &Value) -> Option<Weekday> {
    value.as_str()?.parse().ok()
}

pub fn settings_of(value: &Value) -> Settings {
    let mut s = Settings::default();
    if value.is_null() {
        return s;
    }
    if value["clock"].as_str() == Some("24h") || value["clock"].as_str() == Some("twenty-four-hour")
    {
        s.clock = Clock::TwentyFourHour;
    }
    if let Some(day) = weekday_of(&value["weekStart"]) {
        s.week_start = day;
    }
    if let Some(days) = value["weekend"].as_array() {
        let parsed: Vec<Weekday> = days.iter().filter_map(weekday_of).collect();
        if !parsed.is_empty() {
            s.weekend = parsed;
        }
    }
    let times: [(&str, &mut NaiveTime); 15] = [
        ("dayStart", &mut s.day_start),
        ("morning", &mut s.morning),
        ("afternoon", &mut s.afternoon),
        ("evening", &mut s.evening),
        ("tonight", &mut s.tonight),
        ("afterWork", &mut s.after_work),
        ("endOfDay", &mut s.end_of_day),
        ("reminderDefault", &mut s.reminder_default),
        ("defaultNotificationTime", &mut s.default_notification_time),
        ("breakfast", &mut s.breakfast),
        ("brunch", &mut s.brunch),
        ("lunch", &mut s.lunch),
        ("afternoonTea", &mut s.afternoon_tea),
        ("dinner", &mut s.dinner),
        ("drinks", &mut s.drinks),
    ];
    for (key, slot) in times {
        if let Some(t) = time_of(&value[key]) {
            *slot = t;
        }
    }
    if let Some(day_first) = value["dayFirst"].as_bool() {
        s.day_first = day_first;
    }
    if let Some(minutes) = value["defaultEventMinutes"].as_u64() {
        s.default_event_minutes = u32::try_from(minutes).unwrap_or(60);
    }
    match &value["holidays"] {
        Value::String(mode) if mode == "off" => s.holidays = HolidaySetting::Off,
        Value::Object(region) => {
            if let Some(country) = region.get("country").and_then(Value::as_str) {
                s.holidays = HolidaySetting::Region {
                    country: country.to_string(),
                    region: region
                        .get("region")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                };
            }
        }
        _ => {}
    }
    if let Some(locale) = value["locale"].as_str() {
        s.locale = Some(locale.to_string());
    }
    s
}

pub fn context_of(request: &Value) -> Result<Context, String> {
    let now: DateTime<Utc> = request["now"]
        .as_str()
        .or(request["now_iso"].as_str())
        .ok_or("missing now")?
        .parse()
        .map_err(|e| format!("bad now: {e}"))?;
    let zone: chrono_tz::Tz = request["zone"]
        .as_str()
        .or(request["timezone"].as_str())
        .unwrap_or("UTC")
        .parse()
        .map_err(|e| format!("bad zone: {e}"))?;
    let mut settings = settings_of(&request["settings"]);
    if settings.locale.is_none() {
        settings.locale = request["locale"].as_str().map(str::to_string);
    }
    Ok(Context::new(now, zone).with_settings(settings))
}

pub fn spans_of(line: &str, request: &Value) -> Vec<LabelledSpan> {
    request["spans"]
        .as_array()
        .map(|spans| {
            spans
                .iter()
                .filter_map(|span| {
                    let role = Role::from_label(span["label"].as_str().or(span["role"].as_str())?)?;
                    let start = utf16_to_byte(line, usize::try_from(span["start"].as_u64()?).ok()?);
                    let end = utf16_to_byte(line, usize::try_from(span["end"].as_u64()?).ok()?);
                    Some(LabelledSpan::new(role, start, end))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn line_of(request: &Value) -> &str {
    request["line"]
        .as_str()
        .or(request["text"].as_str())
        .unwrap_or("")
}

fn utf8_spans(request: &Value) -> bool {
    request["offsets"].as_str() == Some("utf8")
}

pub fn capture_schedule_json(request: &str) -> String {
    let request: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return error(e),
    };
    let ctx = match context_of(&request) {
        Ok(c) => c,
        Err(e) => return error(e),
    };
    let line = line_of(&request);
    let utf8 = utf8_spans(&request);
    let labelled: Option<Vec<LabelledSpan>> = request["spans"].as_array().map(|spans| {
        spans
            .iter()
            .filter_map(|span| {
                let role = Role::from_label(span["label"].as_str().or(span["role"].as_str())?)?;
                let start = usize::try_from(span["start"].as_u64()?).ok()?;
                let end = usize::try_from(span["end"].as_u64()?).ok()?;
                Some(if utf8 {
                    LabelledSpan::new(role, start.min(line.len()), end.min(line.len()))
                } else {
                    LabelledSpan::new(role, utf16_to_byte(line, start), utf16_to_byte(line, end))
                })
            })
            .collect()
    });
    let mut schedule = capture_schedule(line, kind_of(&request["kind"]), labelled.as_deref(), &ctx);
    if !utf8 {
        for span in &mut schedule.spans {
            span.start = byte_to_utf16(line, span.start);
            span.end = byte_to_utf16(line, span.end);
        }
    }
    serde_json::to_string(&schedule).unwrap_or_else(error)
}
