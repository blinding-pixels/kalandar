use std::io::{self, BufRead, Write};

use chrono_english_wasm::{parse_json, resolve_capture_json, resolve_line_json, resolve_unlabelled_json};
use serde_json::Value;

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let line = line.expect("line");
        if line.trim().is_empty() {
            continue;
        }
        let call: Value = serde_json::from_str(&line).expect("json");
        let request = call["request"].to_string();
        let result = match call["fn"].as_str().unwrap_or("") {
            "resolveLine" => resolve_line_json(&request),
            "resolveCapture" => resolve_capture_json(&request),
            "resolveUnlabelled" => resolve_unlabelled_json(&request),
            _ => parse_json(&request),
        };
        writeln!(out, "{result}").expect("write");
    }
}
