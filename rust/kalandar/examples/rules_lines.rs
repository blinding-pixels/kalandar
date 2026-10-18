use kalandar::{preview_capture_rules, CapturePreviewRequest};
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<CapturePreviewRequest>(&line) else {
            continue;
        };
        let result = preview_capture_rules(request);
        let _ = writeln!(out, "{}", serde_json::to_string(&result).unwrap_or_default());
    }
}
