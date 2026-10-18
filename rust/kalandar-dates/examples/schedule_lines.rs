use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut out = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let _ = writeln!(
            out,
            "{}",
            kalandar_dates::json::capture_schedule_json(&line)
        );
    }
}
