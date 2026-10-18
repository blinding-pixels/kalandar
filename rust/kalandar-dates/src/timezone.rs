use std::collections::HashMap;

use chrono::{Datelike, Timelike};

use crate::jsdate::{JsDate, SystemZone};
use crate::types::{DstRule, TimezoneAbbr, Weekday};

const FIXED_TIMEZONE_ABBRS: &[(&str, i32)] = &[
    ("ACDT", 630),
    ("ACST", 570),
    ("ADT", -180),
    ("AEDT", 660),
    ("AEST", 600),
    ("AFT", 270),
    ("AKDT", -480),
    ("AKST", -540),
    ("ALMT", 360),
    ("AMST", -180),
    ("AMT", -240),
    ("ANAST", 720),
    ("ANAT", 720),
    ("AQTT", 300),
    ("ART", -180),
    ("AST", -240),
    ("AWDT", 540),
    ("AWST", 480),
    ("AZOST", 0),
    ("AZOT", -60),
    ("AZST", 300),
    ("AZT", 240),
    ("BNT", 480),
    ("BOT", -240),
    ("BRST", -120),
    ("BRT", -180),
    ("BST", 60),
    ("BTT", 360),
    ("CAST", 480),
    ("CAT", 120),
    ("CCT", 390),
    ("CDT", -300),
    ("CEST", 120),
    ("timezoneOffsetNonDst", 60),
    ("CHADT", 825),
    ("CHAST", 765),
    ("CKT", -600),
    ("CLST", -180),
    ("CLT", -240),
    ("COT", -300),
    ("CST", -360),
    ("CVT", -60),
    ("CXT", 420),
    ("ChST", 600),
    ("DAVT", 420),
    ("EASST", -300),
    ("EAST", -360),
    ("EAT", 180),
    ("ECT", -300),
    ("EDT", -240),
    ("EEST", 180),
    ("EET", 120),
    ("EGST", 0),
    ("EGT", -60),
    ("EST", -300),
    ("FJST", 780),
    ("FJT", 720),
    ("FKST", -180),
    ("FKT", -240),
    ("FNT", -120),
    ("GALT", -360),
    ("GAMT", -540),
    ("GET", 240),
    ("GFT", -180),
    ("GILT", 720),
    ("GMT", 0),
    ("GST", 240),
    ("GYT", -240),
    ("HAA", -180),
    ("HAC", -300),
    ("HADT", -540),
    ("HAE", -240),
    ("HAP", -420),
    ("HAR", -360),
    ("HAST", -600),
    ("HAT", -90),
    ("HAY", -480),
    ("HKT", 480),
    ("HLV", -210),
    ("HNA", -240),
    ("HNC", -360),
    ("HNE", -300),
    ("HNP", -480),
    ("HNR", -420),
    ("HNT", -150),
    ("HNY", -540),
    ("HOVT", 420),
    ("ICT", 420),
    ("IDT", 180),
    ("IOT", 360),
    ("IRDT", 270),
    ("IRKST", 540),
    ("IRKT", 540),
    ("IRST", 210),
    ("IST", 330),
    ("JST", 540),
    ("KGT", 360),
    ("KRAST", 480),
    ("KRAT", 480),
    ("KST", 540),
    ("KUYT", 240),
    ("LHDT", 660),
    ("LHST", 630),
    ("LINT", 840),
    ("MAGST", 720),
    ("MAGT", 720),
    ("MART", -510),
    ("MAWT", 300),
    ("MDT", -360),
    ("MESZ", 120),
    ("MEZ", 60),
    ("MHT", 720),
    ("MMT", 390),
    ("MSD", 240),
    ("MSK", 180),
    ("MST", -420),
    ("MUT", 240),
    ("MVT", 300),
    ("MYT", 480),
    ("NCT", 660),
    ("NDT", -90),
    ("NFT", 690),
    ("NOVST", 420),
    ("NOVT", 360),
    ("NPT", 345),
    ("NST", -150),
    ("NUT", -660),
    ("NZDT", 780),
    ("NZST", 720),
    ("OMSST", 420),
    ("OMST", 420),
    ("PDT", -420),
    ("PET", -300),
    ("PETST", 720),
    ("PETT", 720),
    ("PGT", 600),
    ("PHOT", 780),
    ("PHT", 480),
    ("PKT", 300),
    ("PMDT", -120),
    ("PMST", -180),
    ("PONT", 660),
    ("PST", -480),
    ("PWT", 540),
    ("PYST", -180),
    ("PYT", -240),
    ("RET", 240),
    ("SAMT", 240),
    ("SAST", 120),
    ("SBT", 660),
    ("SCT", 240),
    ("SGT", 480),
    ("SRT", -180),
    ("SST", -660),
    ("TAHT", -600),
    ("TFT", 300),
    ("TJT", 300),
    ("TKT", 780),
    ("TLT", 540),
    ("TMT", 300),
    ("TVT", 720),
    ("ULAT", 480),
    ("UTC", 0),
    ("UYST", -120),
    ("UYT", -180),
    ("UZT", 300),
    ("VET", -210),
    ("VLAST", 660),
    ("VLAT", 660),
    ("VUT", 660),
    ("WAST", 120),
    ("WAT", 60),
    ("WEST", 60),
    ("WESZ", 60),
    ("WET", 0),
    ("WEZ", 0),
    ("WFT", 720),
    ("WGST", -120),
    ("WGT", -180),
    ("WIB", 420),
    ("WIT", 540),
    ("WITA", 480),
    ("WST", 780),
    ("WT", 0),
    ("YAKST", 600),
    ("YAKT", 600),
    ("YAPT", 600),
    ("YEKST", 360),
    ("YEKT", 360),
];

const US_DST_START: DstRule = DstRule::NthWeekdayOfMonth {
    month: 3,
    weekday: Weekday::Sunday,
    n: 2,
    hour: 2,
};

const US_DST_END: DstRule = DstRule::NthWeekdayOfMonth {
    month: 11,
    weekday: Weekday::Sunday,
    n: 1,
    hour: 2,
};

fn us_ambiguous(offset_during_dst: i32, offset_non_dst: i32) -> TimezoneAbbr {
    TimezoneAbbr::Ambiguous {
        offset_during_dst,
        offset_non_dst,
        dst_start: US_DST_START,
        dst_end: US_DST_END,
    }
}

pub fn default_timezone_abbr(abbr: &str) -> Option<TimezoneAbbr> {
    match abbr {
        "CET" => Some(TimezoneAbbr::Ambiguous {
            offset_during_dst: 120,
            offset_non_dst: 60,
            dst_start: DstRule::LastWeekdayOfMonth {
                month: 3,
                weekday: Weekday::Sunday,
                hour: 2,
            },
            dst_end: DstRule::LastWeekdayOfMonth {
                month: 10,
                weekday: Weekday::Sunday,
                hour: 3,
            },
        }),
        "CT" => Some(us_ambiguous(-300, -360)),
        "ET" => Some(us_ambiguous(-240, -300)),
        "MT" => Some(us_ambiguous(-360, -420)),
        "PT" => Some(us_ambiguous(-420, -480)),
        _ => FIXED_TIMEZONE_ABBRS
            .iter()
            .find(|(name, _)| *name == abbr)
            .map(|&(_, offset)| TimezoneAbbr::Fixed(offset)),
    }
}

pub fn nth_weekday_of_month(
    zone: SystemZone,
    year: i32,
    month: u32,
    weekday: Weekday,
    n: u32,
    hour: u32,
) -> JsDate {
    let mut day_of_month = 0;
    let mut count = 0;
    while count < n {
        day_of_month += 1;
        let date = JsDate::local(zone, year, month as i32 - 1, day_of_month, 0, 0, 0, 0);
        if date.get_day() == weekday as i32 {
            count += 1;
        }
    }
    JsDate::local(
        zone,
        year,
        month as i32 - 1,
        day_of_month,
        hour as i32,
        0,
        0,
        0,
    )
}

pub fn last_weekday_of_month(
    zone: SystemZone,
    year: i32,
    month: u32,
    weekday: Weekday,
    hour: u32,
) -> JsDate {
    let one_indexed_weekday = if weekday == Weekday::Sunday {
        7
    } else {
        weekday as i32
    };
    let mut date = JsDate::local(zone, year, month as i32, 1, 12, 0, 0, 0);
    let first_weekday_next_month = if date.get_day() == 0 {
        7
    } else {
        date.get_day()
    };
    let day_diff = if first_weekday_next_month == one_indexed_weekday {
        7
    } else if first_weekday_next_month < one_indexed_weekday {
        7 + first_weekday_next_month - one_indexed_weekday
    } else {
        first_weekday_next_month - one_indexed_weekday
    };
    date.set_date(f64::from(date.get_date() - day_diff));
    JsDate::local(
        zone,
        year,
        month as i32 - 1,
        date.get_date(),
        hour as i32,
        0,
        0,
        0,
    )
}

fn resolve_rule(rule: DstRule, zone: SystemZone, year: i32) -> JsDate {
    match rule {
        DstRule::NthWeekdayOfMonth {
            month,
            weekday,
            n,
            hour,
        } => nth_weekday_of_month(zone, year, month, weekday, n, hour),
        DstRule::LastWeekdayOfMonth {
            month,
            weekday,
            hour,
        } => last_weekday_of_month(zone, year, month, weekday, hour),
        DstRule::Custom(rule) => {
            let local = rule(year);
            JsDate::local(
                zone,
                local.year(),
                local.month0() as i32,
                local.day() as i32,
                local.hour() as i32,
                local.minute() as i32,
                local.second() as i32,
                (local.nanosecond() / 1_000_000) as i32,
            )
        }
    }
}

pub(crate) fn to_timezone_offset(
    input: &str,
    date: Option<JsDate>,
    overrides: &HashMap<String, TimezoneAbbr>,
) -> Option<i32> {
    let matched = overrides
        .get(input)
        .copied()
        .or_else(|| default_timezone_abbr(input))?;
    match matched {
        TimezoneAbbr::Fixed(offset) => Some(offset),
        TimezoneAbbr::Ambiguous {
            offset_during_dst,
            offset_non_dst,
            dst_start,
            dst_end,
        } => {
            let date = date?;
            let year = date.get_full_year();
            let start = resolve_rule(dst_start, date.zone(), year);
            let end = resolve_rule(dst_end, date.zone(), year);
            if date > start && date.partial_cmp(&end) != Some(std::cmp::Ordering::Greater) {
                Some(offset_during_dst)
            } else {
                Some(offset_non_dst)
            }
        }
    }
}
