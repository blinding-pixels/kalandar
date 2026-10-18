#![cfg(feature = "tz")]

use chrono::{TimeZone, Utc};
use kalandar_dates::{chrono_tz, parse, Component, ParsingOption, ParsingReference, SystemZone};

#[test]
fn readme_example() {
    let now = Utc.with_ymd_and_hms(2026, 10, 18, 13, 0, 0).unwrap();
    let reference =
        ParsingReference::new(now).with_system_zone(SystemZone::Iana(chrono_tz::America::New_York));
    let results = parse(
        "lunch with Sam tomorrow at 1",
        &reference,
        &ParsingOption::forward(),
    );
    let first = &results[0];
    assert_eq!(first.text, "tomorrow at 1");
    assert_eq!(first.start.get(Component::Day), Some(19));
    assert_eq!(first.start.get(Component::Hour), Some(1));
    assert!(first.start.is_certain(Component::Hour));
    assert!(!first.start.is_certain(Component::Second));
}
