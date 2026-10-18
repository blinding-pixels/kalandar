fn main() {
    let mut args = std::env::args().skip(1);
    let kind = args.next().unwrap_or_default();
    for text in args {
        let request = serde_json::json!({
            "text": text,
            "kind": kind,
            "now": "2026-05-16T12:00:00Z",
            "timezone": "America/New_York",
            "offsets": "utf8",
        });
        println!(
            "{}",
            kalandar_dates::json::capture_schedule_json(&request.to_string())
        );
    }
}
