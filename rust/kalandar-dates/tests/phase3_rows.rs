#![cfg(feature = "facets")]

use std::collections::BTreeMap;

use chrono::{DateTime, Utc, Weekday};
use chrono_english_rs::facets::{
    resolve_unlabelled, CaptureKind, Clock, Context, HolidaySetting, LineResolution, Moment,
    Settings,
};
use serde_json::{json, Value};

fn show(m: &Option<Moment>) -> Value {
    match m {
        Some(m) => match m.local {
            Some(l) if !m.all_day => json!(l.format("%Y-%m-%dT%H:%M").to_string()),
            _ => json!(m.date.format("%Y-%m-%d").to_string()),
        },
        None => Value::Null,
    }
}

fn context(row: &Value) -> Context {
    let now: DateTime<Utc> = row["now"]
        .as_str()
        .unwrap_or("2026-10-18T17:00:00Z")
        .parse()
        .expect("now");
    let zone: chrono_tz::Tz = row["zone"]
        .as_str()
        .unwrap_or("America/Los_Angeles")
        .parse()
        .expect("zone");
    let mut settings = Settings::default();
    let s = &row["settings"];
    settings.day_first = s["day_first"].as_bool().unwrap_or(false);
    if s["week_start"].as_str() == Some("sun") {
        settings.week_start = Weekday::Sun;
    }
    if s["clock"].as_str() == Some("24h") {
        settings.clock = Clock::TwentyFourHour;
    }
    match &s["holidays"] {
        Value::String(mode) if mode == "off" => settings.holidays = HolidaySetting::Off,
        Value::Object(region) => {
            if let Some(country) = region.get("country").and_then(Value::as_str) {
                settings.holidays = HolidaySetting::Region {
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
    settings.locale = s["locale"].as_str().map(str::to_string);

    Context::new(now, zone).with_settings(settings)
}

fn warnings(r: &LineResolution) -> Vec<String> {
    let mut out: Vec<String> = r
        .warnings
        .iter()
        .map(|w| w.code.as_str().to_string())
        .collect();
    for m in [&r.start, &r.end].into_iter().flatten() {
        out.extend(m.warnings.iter().map(|w| w.code.as_str().to_string()));
    }
    if let Some(rec) = &r.recurrence {
        out.extend(rec.warnings.iter().map(|w| w.code.as_str().to_string()));
    }
    out
}

fn recurrence_value(r: &LineResolution) -> Value {
    let Some(rec) = &r.recurrence else {
        return Value::Null;
    };
    let all_day = rec.first.as_ref().is_some_and(|f| f.all_day);
    json!({
        "frequency": rec.rule.frequency,
        "interval": rec.rule.interval,
        "weekdays": rec.rule.weekdays,
        "month_days": rec.rule.month_days,
        "months": rec.rule.months,
        "hours": rec.rule.hours,
        "count": rec.rule.count,
        "until": rec.until,
        "start_date": rec.start_date,
        "window": rec.rule.window.map(|(a, b)| vec![a.format("%H:%M").to_string(), b.format("%H:%M").to_string()]),
        "exdates": rec.exdates.iter().map(|d| if all_day { d.format("%Y-%m-%d").to_string() } else { d.format("%Y-%m-%dT%H:%M").to_string() }).collect::<Vec<_>>(),
    })
}

fn sorted(v: &Value) -> Value {
    match v {
        Value::Array(items) if items.iter().all(|i| !i.is_array() && !i.is_object()) => {
            let mut items = items.clone();
            items.sort_by_key(|i| i.to_string());
            Value::Array(items)
        }
        Value::Null => Value::Array(Vec::new()),
        other => other.clone(),
    }
}

fn problems(row: &Value, r: &LineResolution) -> Vec<String> {
    let mut out = Vec::new();
    let expect = row["expect"].as_object().expect("expect");
    for (key, want) in expect {
        match key.as_str() {
            "start" => {
                if show(&r.start) != *want {
                    out.push(format!("start {} != {}", show(&r.start), want));
                }
            }
            "end" => {
                if show(&r.end) != *want {
                    out.push(format!("end {} != {}", show(&r.end), want));
                }
            }
            "lead_minutes" => {
                if json!(r.lead_minutes) != *want {
                    out.push(format!("lead {:?} != {}", r.lead_minutes, want));
                }
            }
            "alternative" => {
                let got = r
                    .start
                    .as_ref()
                    .and_then(|s| s.alternative)
                    .map(|d| d.to_string());
                if json!(got) != *want {
                    out.push(format!("alternative {got:?} != {want}"));
                }
            }
            "absent_warnings" => {
                let got = warnings(r);
                for w in want.as_array().into_iter().flatten() {
                    if got.iter().any(|g| Some(g.as_str()) == w.as_str()) {
                        out.push(format!("unexpected warning {w}"));
                    }
                }
            }
            "warnings" => {
                let got = warnings(r);
                for w in want.as_array().into_iter().flatten() {
                    if !got.iter().any(|g| Some(g.as_str()) == w.as_str()) {
                        out.push(format!("missing warning {w} in {got:?}"));
                    }
                }
            }
            "recurrence" => {
                let got = recurrence_value(r);
                if want.is_null() {
                    if !got.is_null() {
                        out.push("unexpected recurrence".to_string());
                    }
                    continue;
                }
                for (field, value) in want.as_object().into_iter().flatten() {
                    let g = got.get(field).cloned().unwrap_or(Value::Null);
                    let equal = if matches!(
                        field.as_str(),
                        "weekdays" | "month_days" | "months" | "hours" | "exdates"
                    ) {
                        sorted(&g) == sorted(value)
                    } else {
                        g == *value
                    };
                    if !equal {
                        out.push(format!("recurrence.{field} {g} != {value}"));
                    }
                }
            }
            other => out.push(format!("unknown key {other}")),
        }
    }
    out
}

#[test]
fn independent_phase3_rows() {
    let mut text = String::new();
    for name in [
        "phase3_rows.jsonl",
        "phase3_rows_b.jsonl",
        "phase4_rows_c.jsonl",
        "phase4_rows_d.jsonl",
        "phase4b_rows_e.jsonl",
        "phase4b_rows_f.jsonl",
        "phase5_rows_g.jsonl",
    ] {
        let path = format!("{}/facets-eval/{name}", env!("CARGO_MANIFEST_DIR"));
        text.push_str(&std::fs::read_to_string(path).expect("rows"));
        text.push('\n');
    }
    let superseded: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/facets-eval/superseded.json"
        ))
        .expect("superseded"),
    )
    .expect("superseded json");
    let mut failures = BTreeMap::new();
    let mut total = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let row: Value = serde_json::from_str(line).expect("row");
        let id = row["id"].as_str().unwrap_or("").to_string();
        if superseded.get(&id).is_some() {
            continue;
        }
        total += 1;
        let kind = match row["kind"].as_str() {
            Some("task") => CaptureKind::Task,
            Some("reminder") => CaptureKind::Reminder,
            _ => CaptureKind::CalendarEvent,
        };
        let r = resolve_unlabelled(row["text"].as_str().unwrap_or(""), kind, &context(&row));
        let p = problems(&row, &r);
        if !p.is_empty() {
            failures.insert(format!("{id}: {}", row["text"]), p);
        }
    }
    assert!(total > 700, "too few rows: {total}");
    assert!(
        failures.is_empty(),
        "{} of {total} rows failed: {failures:#?}",
        failures.len()
    );
}
