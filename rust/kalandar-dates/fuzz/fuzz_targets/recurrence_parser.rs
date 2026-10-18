#![no_main]

use chrono::{DateTime, Utc};
use kalandar_dates::capture::{parse_recurrence, resolve_unlabelled, CaptureKind, Context, Settings};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let settings = Settings::default();
    if let Some(rule) = parse_recurrence(text, &settings) {
        let _ = rule.rrule(None);
        let _ = rule.reminder_shape();
    }
    let Some(now) = DateTime::<Utc>::from_timestamp(1_792_342_800, 0) else {
        return;
    };
    let ctx = Context::new(now, chrono_tz::America::New_York);
    let phrase = format!("every {text}");
    let _ = resolve_unlabelled(&phrase, CaptureKind::Reminder, &ctx);
    let _ = resolve_unlabelled(text, CaptureKind::Task, &ctx);
});
