use chrono::{DateTime, Utc};
use kalandar_dates::capture::{resolve_unlabelled, CaptureKind, Context};
use kalandar_dates::{parse, ParsingOption, ParsingReference, SystemZone};

fn main() {
    let now: DateTime<Utc> = "2026-10-18T17:00:00Z".parse().expect("now");
    let zone: chrono_tz::Tz = std::env::var("FACETS_ZONE")
        .ok()
        .and_then(|z| z.parse().ok())
        .unwrap_or(chrono_tz::America::Los_Angeles);
    let ctx = Context::new(now, zone);
    let kind = match std::env::args().nth(1).as_deref() {
        Some("task") => CaptureKind::Task,
        Some("reminder") => CaptureKind::Reminder,
        _ => CaptureKind::CalendarEvent,
    };
    for line in std::env::args().skip(2) {
        let reference = ParsingReference::new(now).with_system_zone(SystemZone::Iana(zone));
        for r in parse(&line, &reference, &ParsingOption::default()) {
            println!(
                "chrono {:?} {} {:?}",
                r.text,
                r.index,
                r.start.certain_components()
            );
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&resolve_unlabelled(&line, kind, &ctx))
                .unwrap_or_default()
        );
    }
}
