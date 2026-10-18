use kalandar::{CaptureParser, CapturePreviewRequest};
use std::io::{self, BufRead, Write};

fn main() {
    let root = std::env::args().nth(1).expect("resource root argument");
    let parser = CaptureParser::new(root).expect("capture parser should load");
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<CapturePreviewRequest>(&line) else {
            continue;
        };
        let result = parser.preview_capture(request);
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string(&result).unwrap_or_default()
        );
    }
}
