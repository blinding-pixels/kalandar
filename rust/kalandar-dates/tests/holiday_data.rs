#![cfg(feature = "facets")]

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Utc};
use chrono_english_rs::facets::{
    holiday_data, holiday_data_info, resolve_unlabelled, CaptureKind, Context, HolidayKind,
    HolidaySetting, Moment, Settings, WarningCode,
};
use serde_json::Value;

fn kind_name(kind: HolidayKind) -> &'static str {
    match kind {
        HolidayKind::Public => "public",
        HolidayKind::Bank => "bank",
        HolidayKind::School => "school",
        HolidayKind::Optional => "optional",
        HolidayKind::Observance => "observance",
    }
}

fn base_name(name: &str) -> String {
    name.trim_end_matches(" (substitute day)")
        .trim()
        .to_string()
}

#[test]
fn lookups_match_date_holidays() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/holiday_samples.json"
    );
    let fixture: Value = serde_json::from_str(
        &std::fs::read_to_string(path).expect("run tools/holidays/regenerate.sh"),
    )
    .expect("samples json");
    let data = holiday_data();
    let english = data.languages.iter().position(|l| l == "en");
    let mut failures = Vec::new();
    let mut compared = 0;
    for sample in fixture["samples"].as_array().expect("samples") {
        let code = sample["region"].as_str().expect("region");
        let year = i32::try_from(sample["year"].as_i64().expect("year")).expect("year");
        let region = data
            .region(code)
            .unwrap_or_else(|| panic!("region {code} missing"));
        let mut want: BTreeMap<(String, String, bool), usize> = BTreeMap::new();
        for h in sample["holidays"].as_array().expect("holidays") {
            let key = (
                h["date"].as_str().unwrap_or("").to_string(),
                h["type"].as_str().unwrap_or("").to_string(),
                h["substitute"].as_bool().unwrap_or(false),
            );
            *want.entry(key).or_default() += 1;
        }
        let mut got: BTreeMap<(String, String, bool), usize> = BTreeMap::new();
        let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for entry in &data.regions[region].entries {
            let def = &data.definitions[entry.definition];
            if def.added {
                continue;
            }
            for o in data
                .occurrences(entry)
                .iter()
                .filter(|o| o.date.year() == year)
            {
                let key = (
                    o.date.to_string(),
                    kind_name(o.kind.unwrap_or(entry.kind)).to_string(),
                    o.substitute,
                );
                *got.entry(key).or_default() += 1;
                if !o.substitute {
                    let name = def
                        .names
                        .iter()
                        .find(|(l, _)| Some(*l) == english)
                        .or_else(|| def.names.first())
                        .map(|(_, n)| n.clone())
                        .unwrap_or_default();
                    names.entry(o.date.to_string()).or_default().push(name);
                }
            }
        }
        compared += want.values().sum::<usize>();
        if want != got {
            failures.push(format!("{code} {year}: want {want:?}, got {got:?}"));
            continue;
        }
        for h in sample["holidays"].as_array().expect("holidays") {
            if h["substitute"].as_bool().unwrap_or(false) {
                continue;
            }
            let date = h["date"].as_str().unwrap_or("");
            let name = base_name(h["name"].as_str().unwrap_or(""));
            if !names
                .get(date)
                .is_some_and(|n| n.iter().any(|x| base_name(x) == name))
            {
                failures.push(format!(
                    "{code} {date}: name {name:?} not in {:?}",
                    names.get(date)
                ));
            }
        }
    }
    assert!(compared > 5000, "only {compared} holidays compared");
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn data_header_is_exposed() {
    let info = holiday_data_info();
    assert_eq!(info.source, "date-holidays");
    assert!(info.integrity.starts_with("sha512-"), "{}", info.integrity);
    assert_eq!((info.first_year, info.last_year), (1970, 2100));
    let pin = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tools/holidays/date-holidays.pin"
    ))
    .expect("pin");
    assert!(pin.contains(&format!("version={}", info.version)));
    assert!(pin.contains(&format!("integrity={}", info.integrity)));
    assert!(chrono::NaiveDate::parse_from_str(&info.generated, "%Y-%m-%d").is_ok());
}

fn now() -> DateTime<Utc> {
    "2026-10-18T17:00:00Z".parse().expect("now")
}

fn resolve(
    line: &str,
    zone: chrono_tz::Tz,
    holidays: HolidaySetting,
) -> (Option<Moment>, Vec<WarningCode>) {
    let settings = Settings {
        holidays,
        ..Settings::default()
    };
    let r = resolve_unlabelled(
        line,
        CaptureKind::Task,
        &Context::new(now(), zone).with_settings(settings),
    );
    let mut codes: Vec<WarningCode> = r.warnings.iter().map(|w| w.code).collect();
    if let Some(s) = &r.start {
        codes.extend(s.warnings.iter().map(|w| w.code));
    }
    (r.start, codes)
}

fn date(m: &Option<Moment>) -> String {
    m.as_ref().map_or("-".to_string(), |m| m.date.to_string())
}

#[test]
fn holiday_setting_off() {
    let (start, codes) = resolve(
        "order thanksgiving turkey",
        chrono_tz::America::Los_Angeles,
        HolidaySetting::Off,
    );
    assert_eq!(date(&start), "-");
    assert!(!codes.contains(&WarningCode::HolidayOtherRegion));
    let r = resolve_unlabelled(
        "christmas party friday",
        CaptureKind::CalendarEvent,
        &Context::new(now(), chrono_tz::America::Los_Angeles).with_settings(Settings {
            holidays: HolidaySetting::Off,
            ..Settings::default()
        }),
    );
    assert_eq!(date(&r.start), "2026-10-23");
}

#[test]
fn holiday_setting_auto_follows_zone_and_locale() {
    let (us, _) = resolve(
        "thanksgiving",
        chrono_tz::America::Los_Angeles,
        HolidaySetting::Auto,
    );
    assert_eq!(date(&us), "2026-11-26");
    let (ca, codes) = resolve(
        "thanksgiving",
        chrono_tz::America::Toronto,
        HolidaySetting::Auto,
    );
    assert_eq!(date(&ca), "2027-10-11");
    assert!(!codes.contains(&WarningCode::HolidayOtherRegion));
    let settings = Settings {
        locale: Some("en-GB".to_string()),
        ..Settings::default()
    };
    let r = resolve_unlabelled(
        "mother's day lunch",
        CaptureKind::Task,
        &Context::new(now(), chrono_tz::America::Los_Angeles).with_settings(settings),
    );
    assert_eq!(date(&r.start), "2027-03-07");
}

#[test]
fn holiday_setting_explicit_region() {
    let explicit = HolidaySetting::Region {
        country: "CA".to_string(),
        region: None,
    };
    let (ca, _) = resolve("thanksgiving", chrono_tz::America::Los_Angeles, explicit);
    assert_eq!(date(&ca), "2027-10-11");
    let (boxing, codes) = resolve(
        "boxing day sale",
        chrono_tz::America::Los_Angeles,
        HolidaySetting::Region {
            country: "US".to_string(),
            region: None,
        },
    );
    assert_eq!(date(&boxing), "2026-12-26");
    assert!(codes.contains(&WarningCode::HolidayOtherRegion));
    let (diwali, codes) = resolve(
        "diwali sweets",
        chrono_tz::America::Los_Angeles,
        HolidaySetting::Region {
            country: "IN".to_string(),
            region: None,
        },
    );
    assert_eq!(date(&diwali), "2026-11-08");
    assert!(codes.contains(&WarningCode::HolidayComputed));
    assert!(!codes.contains(&WarningCode::HolidayOtherRegion));
}
