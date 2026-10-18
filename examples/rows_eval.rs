use std::io::{self, BufRead, Write};

use chrono::{DateTime, Utc, Weekday};
use chrono_english_rs::facets::{
    resolve_unlabelled, CaptureKind, Clock, Context, HolidaySetting, Moment, Settings,
};
use serde_json::{json, Value};

fn moment(m: &Option<Moment>) -> Value {
    match m {
        Some(m) => match m.local {
            Some(l) if !m.all_day => json!(l.format("%Y-%m-%dT%H:%M").to_string()),
            _ => json!(m.date.format("%Y-%m-%d").to_string()),
        },
        None => Value::Null,
    }
}

fn settings_of(value: &Value) -> Settings {
    let mut s = Settings::default();
    if value["day_first"].as_bool() == Some(true) {
        s.day_first = true;
    }
    if value["week_start"].as_str() == Some("sun") {
        s.week_start = Weekday::Sun;
    }
    if value["clock"].as_str() == Some("24h") {
        s.clock = Clock::TwentyFourHour;
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
    s.locale = value["locale"].as_str().map(str::to_string);
    s
}

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let line = line.expect("line");
        if line.trim().is_empty() {
            continue;
        }
        let row: Value = serde_json::from_str(&line).expect("json");
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
        let ctx = Context::new(now, zone).with_settings(settings_of(&row["settings"]));
        let kind = match row["kind"].as_str() {
            Some("task") => CaptureKind::Task,
            Some("reminder") => CaptureKind::Reminder,
            _ => CaptureKind::CalendarEvent,
        };
        let r = resolve_unlabelled(row["text"].as_str().unwrap_or(""), kind, &ctx);
        let mut warnings: Vec<&str> = r.warnings.iter().map(|w| w.code.as_str()).collect();
        for m in [&r.start, &r.end].into_iter().flatten() {
            warnings.extend(m.warnings.iter().map(|w| w.code.as_str()));
        }
        let recurrence = r.recurrence.as_ref().map(|rec| {
            let all_day = rec.first.as_ref().is_some_and(|f| f.all_day);
            warnings.extend(rec.warnings.iter().map(|w| w.code.as_str()));
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
                "rrule": rec.rrule,
            })
        });
        let alternative = r
            .start
            .as_ref()
            .and_then(|s| s.alternative)
            .map(|d| d.to_string());
        writeln!(
            out,
            "{}",
            json!({
                "id": row["id"],
                "start": moment(&r.start),
                "end": moment(&r.end),
                "recurrence": recurrence,
                "lead_minutes": r.lead_minutes,
                "warnings": warnings,
                "alternative": alternative,
            })
        )
        .expect("write");
    }
}
