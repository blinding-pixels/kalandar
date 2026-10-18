use std::io::{self, BufRead, Write};

use chrono::{DateTime, Utc};
use chrono_english_rs::{Component, ParsedComponents, ParsingOption, ParsingReference, SystemZone};
use serde_json::{json, Map, Value};

fn system_zone(name: &str) -> SystemZone {
    if name == "UTC" {
        SystemZone::Utc
    } else {
        SystemZone::Iana(
            name.parse()
                .unwrap_or_else(|_| panic!("unknown zone {name}")),
        )
    }
}

fn components_json(components: &ParsedComponents) -> Value {
    let mut known = Map::new();
    let mut implied = Map::new();
    for component in Component::ALL {
        if let Some(value) = components.get(component) {
            if components.is_certain(component) {
                known.insert(component.name().to_string(), json!(value));
            } else {
                implied.insert(component.name().to_string(), json!(value));
            }
        }
    }
    json!({
        "known": known,
        "implied": implied,
        "date": components.date().map(|d| d.timestamp_millis()),
    })
}

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let line = line.expect("read line");
        if line.trim().is_empty() {
            continue;
        }
        let case: Value = serde_json::from_str(&line).expect("json case");
        let text = case["text"].as_str().unwrap_or("");
        let instant = DateTime::<Utc>::from_timestamp_millis(case["ref"].as_i64().unwrap_or(0))
            .expect("instant");
        let mut reference = ParsingReference::new(instant)
            .with_system_zone(system_zone(case["zone"].as_str().unwrap_or("UTC")));
        match &case["tz"] {
            Value::Number(n) => {
                reference = reference.with_timezone_offset(n.as_i64().unwrap_or(0) as i32)
            }
            Value::String(s) => reference = reference.with_timezone_abbreviation(s),
            _ => {}
        }
        let option = ParsingOption {
            forward_date: case["forward"].as_bool().unwrap_or(false),
            ..ParsingOption::default()
        };
        let chrono = match case["config"].as_str().unwrap_or("casual") {
            "strict" => chrono_english_rs::strict(),
            "gb" => chrono_english_rs::gb(),
            _ => chrono_english_rs::casual(),
        };
        let results: Vec<Value> = chrono
            .parse(text, &reference, &option)
            .iter()
            .map(|r| {
                let mut tags: Vec<&str> = r.tags().into_iter().collect();
                tags.sort();
                json!({
                    "index": text[..r.index].encode_utf16().count(),
                    "text": r.text,
                    "start": components_json(&r.start),
                    "end": r.end.as_ref().map(components_json),
                    "tags": tags,
                })
            })
            .collect();
        writeln!(out, "{}", json!({"id": case["id"], "results": results})).expect("write");
    }
}
