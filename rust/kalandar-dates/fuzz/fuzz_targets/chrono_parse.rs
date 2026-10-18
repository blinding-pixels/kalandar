#![no_main]

use chrono::{DateTime, Utc};
use kalandar_dates::{ParsingOption, ParsingReference, SystemZone};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Some(now) = DateTime::<Utc>::from_timestamp(1_792_342_800, 0) else {
        return;
    };
    let reference = ParsingReference::new(now).with_system_zone(SystemZone::Iana(chrono_tz::Europe::Berlin));
    for result in kalandar_dates::parse(text, &reference, &ParsingOption::forward()) {
        let _ = result.date();
        let _ = &text[result.index..];
    }
});
