#![no_main]

use chrono::{DateTime, Utc};
use kalandar_dates::capture::{
    resolve_capture, resolve_line, resolve_phrase, resolve_unlabelled, CaptureKind, Context, LabelledSpan, Role,
};
use libfuzzer_sys::fuzz_target;

const ROLES: [Role; 8] = [
    Role::EventStart,
    Role::EventEnd,
    Role::DueDate,
    Role::DueTime,
    Role::Trigger,
    Role::LeadTime,
    Role::Recurrence,
    Role::Duration,
];

const ZONES: [chrono_tz::Tz; 4] = [
    chrono_tz::America::Los_Angeles,
    chrono_tz::Europe::London,
    chrono_tz::Australia::Lord_Howe,
    chrono_tz::Asia::Kathmandu,
];

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let kind = match data[0] % 3 {
        0 => CaptureKind::CalendarEvent,
        1 => CaptureKind::Task,
        _ => CaptureKind::Reminder,
    };
    let role = ROLES[usize::from(data[1]) % ROLES.len()];
    let zone = ZONES[usize::from(data[2]) % ZONES.len()];
    let seconds = 1_700_000_000 + i64::from(data[3]) * 86_400 * 3;
    let Some(now) = DateTime::<Utc>::from_timestamp(seconds, 0) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&data[4..]) else {
        return;
    };
    let ctx = Context::new(now, zone);
    let _ = resolve_phrase(text, role, kind, &ctx);
    let _ = resolve_unlabelled(text, kind, &ctx);
    let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let spans: Vec<LabelledSpan> = boundaries
        .windows(2)
        .step_by(3)
        .enumerate()
        .map(|(i, w)| LabelledSpan::new(ROLES[i % ROLES.len()], w[0], w[1]))
        .collect();
    let _ = resolve_line(text, kind, &spans, &ctx);
    let _ = resolve_capture(text, kind, &spans, &ctx);
});
