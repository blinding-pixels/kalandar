use std::io::{self, BufRead, Write};

use chrono::{DateTime, Utc};
use kalandar_dates::capture::{
    resolve_capture, resolve_line, resolve_unlabelled, Alert, CaptureKind, Context, LabelledSpan,
    LineResolution, Moment, Role,
};
use serde_json::{json, Value};

fn byte_offset(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(i, _)| i)
}

fn moment(m: &Option<Moment>) -> Value {
    match m {
        Some(m) => match m.local {
            Some(l) if !m.all_day => json!(l.format("%Y-%m-%dT%H:%M").to_string()),
            _ => json!(m.date.format("%Y-%m-%d").to_string()),
        },
        None => Value::Null,
    }
}

fn summary(r: &LineResolution) -> Value {
    let recurrence = r.recurrence.as_ref().map(|rec| {
        json!({
            "frequency": rec.rule.frequency,
            "interval": rec.rule.interval,
            "weekdays": rec.rule.weekdays,
            "month_days": rec.rule.month_days,
            "months": rec.rule.months,
            "count": rec.rule.count,
            "start_date": rec.start_date,
            "end_date": rec.end_date,
            "exceptions": rec.exceptions.iter().map(|e| e.raw.clone()).collect::<Vec<_>>(),
            "time": rec.rule.time.map(|t| t.format("%H:%M").to_string()),
            "rrule": rec.rrule,
        })
    });
    let (alert_at, alerts_off, alert_lead) = match &r.alert {
        Some(Alert::Absolute { at }) => (moment(&Some(at.clone())), false, None),
        Some(Alert::Off) => (Value::Null, true, None),
        Some(Alert::Relative { minutes_before, .. }) => (Value::Null, false, Some(*minutes_before)),
        None => (Value::Null, false, None),
    };
    let mut warnings: Vec<&str> = Vec::new();
    for m in [&r.start, &r.end].into_iter().flatten() {
        warnings.extend(m.warnings.iter().map(|w| w.code.as_str()));
    }
    warnings.extend(r.warnings.iter().map(|w| w.code.as_str()));
    json!({
        "start": moment(&r.start),
        "end": moment(&r.end),
        "recurrence": recurrence,
        "lead_minutes": r.lead_minutes.or(alert_lead),
        "alert_at": alert_at,
        "alerts_off": alerts_off,
        "task_signal": r.task_signal.is_some(),
        "warnings": warnings,
    })
}

fn kind_of(name: &str) -> CaptureKind {
    match name {
        "task" => CaptureKind::Task,
        "reminder" => CaptureKind::Reminder,
        _ => CaptureKind::CalendarEvent,
    }
}

fn spans_from(text: &str, list: &Value) -> Vec<LabelledSpan> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|e| {
                    let role = Role::from_label(e["label"].as_str()?)?;
                    match (e["start"].as_u64(), e["end"].as_u64()) {
                        (Some(s), Some(en)) => Some(LabelledSpan::new(
                            role,
                            byte_offset(text, s as usize),
                            byte_offset(text, en as usize),
                        )),
                        _ => LabelledSpan::find(text, role, e["text"].as_str()?),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn main() {
    let now: DateTime<Utc> = "2026-10-18T17:00:00Z".parse().expect("now");
    let ctx = Context::new(now, chrono_tz::America::Los_Angeles);
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let line = line.expect("line");
        if line.trim().is_empty() {
            continue;
        }
        let case: Value = serde_json::from_str(&line).expect("json");
        let text = case["text"].as_str().unwrap_or("");
        let kind = kind_of(case["kind"].as_str().unwrap_or("calendar"));
        let gold_spans = spans_from(text, &case["entities"]);
        let model_spans = spans_from(text, &case["model_spans"]);
        let with_gold = resolve_line(text, kind, &gold_spans, &ctx);
        let with_model = resolve_line(text, kind, &model_spans, &ctx);
        let unlabelled = resolve_unlabelled(text, kind, &ctx);
        let repaired = resolve_capture(text, kind, &model_spans, &ctx);
        writeln!(
            out,
            "{}",
            json!({
                "id": case["id"],
                "gold_spans": summary(&with_gold),
                "model_spans": summary(&with_model),
                "no_spans": summary(&unlabelled),
                "model_capture": summary(&repaired),
            })
        )
        .expect("write");
    }
}
