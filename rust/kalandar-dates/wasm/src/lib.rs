use chrono::{DateTime, NaiveTime, Utc, Weekday};
use kalandar_dates::capture::{
    holiday_data_info, resolve_capture, resolve_line, resolve_unlabelled, CaptureKind, Clock, Context, HolidaySetting,
    LabelledSpan, Role, Settings,
};
use kalandar_dates::{Component, ParsedComponents, ParsingOption, ParsingReference, SystemZone};
use serde_json::{json, Map, Value};
use wasm_bindgen::prelude::wasm_bindgen;

fn error(message: impl std::fmt::Display) -> String {
    json!({ "error": message.to_string() }).to_string()
}

fn utf16_to_byte(text: &str, units: usize) -> usize {
    let mut count = 0;
    for (index, c) in text.char_indices() {
        if count >= units {
            return index;
        }
        count += c.len_utf16();
    }
    text.len()
}

fn byte_to_utf16(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

fn kind_of(value: &Value) -> CaptureKind {
    match value.as_str() {
        Some("task") => CaptureKind::Task,
        Some("reminder") => CaptureKind::Reminder,
        _ => CaptureKind::CalendarEvent,
    }
}

fn time_of(value: &Value) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.as_str()?, "%H:%M").ok()
}

fn weekday_of(value: &Value) -> Option<Weekday> {
    value.as_str()?.parse().ok()
}

fn settings_of(value: &Value) -> Settings {
    let mut s = Settings::default();
    if value.is_null() {
        return s;
    }
    if value["clock"].as_str() == Some("24h") || value["clock"].as_str() == Some("twenty-four-hour") {
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
                    region: region.get("region").and_then(Value::as_str).map(str::to_string),
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

fn context_of(request: &Value) -> Result<Context, String> {
    let now: DateTime<Utc> = request["now"]
        .as_str()
        .ok_or("missing now")?
        .parse()
        .map_err(|e| format!("bad now: {e}"))?;
    let zone: chrono_tz::Tz = request["zone"]
        .as_str()
        .unwrap_or("UTC")
        .parse()
        .map_err(|e| format!("bad zone: {e}"))?;
    Ok(Context::new(now, zone).with_settings(settings_of(&request["settings"])))
}

fn spans_of(line: &str, request: &Value) -> Vec<LabelledSpan> {
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

fn run(request: &str, mode: fn(&str, CaptureKind, &[LabelledSpan], &Context) -> String) -> String {
    let request: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return error(e),
    };
    let ctx = match context_of(&request) {
        Ok(c) => c,
        Err(e) => return error(e),
    };
    let line = request["line"].as_str().unwrap_or("");
    let spans = spans_of(line, &request);
    mode(line, kind_of(&request["kind"]), &spans, &ctx)
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(error)
}

#[wasm_bindgen(js_name = resolveLine)]
pub fn resolve_line_json(request: &str) -> String {
    run(request, |line, kind, spans, ctx| to_json(&resolve_line(line, kind, spans, ctx)))
}

#[wasm_bindgen(js_name = resolveCapture)]
pub fn resolve_capture_json(request: &str) -> String {
    run(request, |line, kind, spans, ctx| to_json(&resolve_capture(line, kind, spans, ctx)))
}

#[wasm_bindgen(js_name = resolveUnlabelled)]
pub fn resolve_unlabelled_json(request: &str) -> String {
    run(request, |line, kind, _, ctx| to_json(&resolve_unlabelled(line, kind, ctx)))
}

fn components_json(components: &ParsedComponents) -> Value {
    let mut known = Map::new();
    let mut implied = Map::new();
    for component in Component::ALL {
        if let Some(value) = components.get(component) {
            let target = if components.is_certain(component) { &mut known } else { &mut implied };
            target.insert(component.name().to_string(), json!(value));
        }
    }
    json!({ "known": known, "implied": implied, "date": components.date().map(|d| d.timestamp_millis()) })
}

#[wasm_bindgen(js_name = holidayData)]
pub fn holiday_data_json() -> String {
    to_json(&holiday_data_info())
}

#[wasm_bindgen(js_name = parse)]
pub fn parse_json(request: &str) -> String {
    let request: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return error(e),
    };
    let Some(instant) = request["ref"].as_i64().and_then(DateTime::<Utc>::from_timestamp_millis) else {
        return error("missing ref (milliseconds since the epoch)");
    };
    let text = request["text"].as_str().unwrap_or("");
    let mut reference = ParsingReference::new(instant);
    if let Some(zone) = request["zone"].as_str().and_then(|z| z.parse::<chrono_tz::Tz>().ok()) {
        reference = reference.with_system_zone(SystemZone::Iana(zone));
    }
    match &request["tz"] {
        Value::Number(n) => reference = reference.with_timezone_offset(i32::try_from(n.as_i64().unwrap_or(0)).unwrap_or(0)),
        Value::String(s) => reference = reference.with_timezone_abbreviation(s),
        _ => {}
    }
    let option = ParsingOption {
        forward_date: request["forward"].as_bool().unwrap_or(false),
        ..ParsingOption::default()
    };
    let chrono = match request["config"].as_str() {
        Some("strict") => kalandar_dates::strict(),
        Some("gb") => kalandar_dates::gb(),
        _ => kalandar_dates::casual(),
    };
    let results: Vec<Value> = chrono
        .parse(text, &reference, &option)
        .iter()
        .map(|r| {
            let mut tags: Vec<&str> = r.tags().into_iter().collect();
            tags.sort_unstable();
            json!({
                "index": byte_to_utf16(text, r.index),
                "text": r.text,
                "start": components_json(&r.start),
                "end": r.end.as_ref().map(components_json),
                "tags": tags,
            })
        })
        .collect();
    Value::Array(results).to_string()
}
