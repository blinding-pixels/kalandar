use chrono::{DateTime, Utc};
use kalandar_dates::capture::{
    holiday_data_info, resolve_capture, resolve_line, resolve_unlabelled, CaptureKind, Context, LabelledSpan,
};
use kalandar_dates::json::{
    byte_to_utf16, capture_schedule_json, context_of, error, kind_of, spans_of,
};
use kalandar_dates::{Component, ParsedComponents, ParsingOption, ParsingReference, SystemZone};
use serde_json::{json, Map, Value};
use wasm_bindgen::prelude::wasm_bindgen;

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

#[wasm_bindgen(js_name = captureSchedule)]
pub fn capture_schedule_wasm(request: &str) -> String {
    capture_schedule_json(request)
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
