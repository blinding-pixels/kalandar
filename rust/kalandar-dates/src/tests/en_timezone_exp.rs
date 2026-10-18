use super::support::*;
use crate::types::Component;
use crate::types::TimezoneAbbr;
use crate::types::{DstRule, Weekday};

#[test]
fn parsing_date_time_with_utc_offset() {
    let r0 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am utc+02:45 ",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(
        r0[0].text,
        "wednesday, september 16, 2020 at 11 am utc+02:45"
    );
    assert_eq!(r0[0].start.get(Component::Hour), Some(11));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(165));
    let r1 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am utc+0245 ",
        now(),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(
        r1[0].text,
        "wednesday, september 16, 2020 at 11 am utc+0245"
    );
    assert_eq!(r1[0].start.get(Component::Hour), Some(11));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(165));
    let r2 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am utc+02 ",
        now(),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "wednesday, september 16, 2020 at 11 am utc+02");
    assert_eq!(r2[0].start.get(Component::Hour), Some(11));
    assert_eq!(r2[0].start.get(Component::Minute), Some(0));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(120));
}

#[test]
fn parsing_date_time_with_numeric_offset() {
    let r0 = run(
        "casual",
        "wednesday, september 16, 2020 at 23.00+14",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "wednesday, september 16, 2020 at 23.00+14");
    assert!(r0[0].start.is_certain(Component::TimezoneOffset));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(840));
    let r1 = run(
        "casual",
        "wednesday, september 16, 2020 at 23.00+1400",
        now(),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "wednesday, september 16, 2020 at 23.00+1400");
    assert!(r1[0].start.is_certain(Component::TimezoneOffset));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(840));
    let r2 = run(
        "casual",
        "wednesday, september 16, 2020 at 23.00+15",
        now(),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "wednesday, september 16, 2020 at 23.00");
    assert!(!r2[0].start.is_certain(Component::TimezoneOffset));
}

#[test]
fn parsing_date_time_with_gmt_offset() {
    let r0 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am GMT -08:45 ",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(
        r0[0].text,
        "wednesday, september 16, 2020 at 11 am GMT -08:45"
    );
    assert_eq!(r0[0].start.get(Component::Hour), Some(11));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(-525));
    let r1 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am gmt+02 ",
        now(),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "wednesday, september 16, 2020 at 11 am gmt+02");
    assert_eq!(r1[0].start.get(Component::Hour), Some(11));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(120));
    let r2 = run("casual", "published: 10:30 (gmt-2:30).", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "10:30 (gmt-2:30)");
    assert_eq!(r2[0].start.get(Component::Hour), Some(10));
    assert_eq!(r2[0].start.get(Component::Minute), Some(30));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(-150));
}

#[test]
fn parsing_date_time_with_timezone_abbreviation() {
    let r0 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "wednesday, september 16, 2020 at 11 am");
    assert_eq!(r0[0].start.get(Component::Year), Some(2020));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(r0[0].start.get(Component::Day), Some(16));
    assert_eq!(r0[0].start.get(Component::Hour), Some(11));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), None);
    let r1 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am JST",
        now(),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "wednesday, september 16, 2020 at 11 am JST");
    assert_eq!(r1[0].start.get(Component::Year), Some(2020));
    assert_eq!(r1[0].start.get(Component::Month), Some(9));
    assert_eq!(r1[0].start.get(Component::Day), Some(16));
    assert_eq!(r1[0].start.get(Component::Hour), Some(11));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(540));
    let r2 = run(
        "casual",
        "wednesday, september 16, 2020 at 11 am GMT+0900 (JST)",
        now(),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(
        r2[0].text,
        "wednesday, september 16, 2020 at 11 am GMT+0900 (JST)"
    );
    assert_eq!(r2[0].start.get(Component::Year), Some(2020));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(16));
    assert_eq!(r2[0].start.get(Component::Hour), Some(11));
    assert_eq!(r2[0].start.get(Component::Minute), Some(0));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(540));
    let r3 = run("casual", "10:30 pst today", at(1475308800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "10:30 pst today");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(10));
    assert_eq!(r3[0].start.get(Component::Minute), Some(30));
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(-480));
}

#[test]
fn parsing_date_range_with_timezone_abbreviation() {
    let r0 = run(
        "casual",
        "10:30 JST today to 10:30 pst tomorrow ",
        at(1475308800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "10:30 JST today to 10:30 pst tomorrow");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(10));
    assert_eq!(r0[0].start.get(Component::Minute), Some(30));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(540));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(2)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Minute),
        Some(30)
    );
    assert_eq!(
        r0[0]
            .end
            .as_ref()
            .expect("end")
            .get(Component::TimezoneOffset),
        Some(-480)
    );
}

#[test]
fn parsing_date_time_with_ambiguous_timezone_abbreviations() {
    let r0 = run("casual", "2022-03-12 23:00 ET", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2022-03-12 23:00 ET");
    assert_eq!(r0[0].start.get(Component::Hour), Some(23));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(-300));
    let r1 = run("casual", "2022-03-13 23:00 ET", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2022-03-13 23:00 ET");
    assert_eq!(r1[0].start.get(Component::Hour), Some(23));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-240));
    let r2 = run("casual", "2021-03-13 23:00 ET", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2021-03-13 23:00 ET");
    assert_eq!(r2[0].start.get(Component::Hour), Some(23));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(-300));
    let r3 = run("casual", "2021-03-14 23:00 ET", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "2021-03-14 23:00 ET");
    assert_eq!(r3[0].start.get(Component::Hour), Some(23));
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(-240));
    let r4 = run("casual", "2021-11-06 23:00 ET", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "2021-11-06 23:00 ET");
    assert_eq!(r4[0].start.get(Component::Hour), Some(23));
    assert_eq!(r4[0].start.get(Component::TimezoneOffset), Some(-240));
    let r5 = run("casual", "2021-11-07 23:00 ET", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "2021-11-07 23:00 ET");
    assert_eq!(r5[0].start.get(Component::Hour), Some(23));
    assert_eq!(r5[0].start.get(Component::TimezoneOffset), Some(-300));
    let r6 = run("casual", "2020-10-31 23:00 ET", now(), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "2020-10-31 23:00 ET");
    assert_eq!(r6[0].start.get(Component::Hour), Some(23));
    assert_eq!(r6[0].start.get(Component::TimezoneOffset), Some(-240));
    let r7 = run("casual", "2020-11-01 23:00 ET", now(), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "2020-11-01 23:00 ET");
    assert_eq!(r7[0].start.get(Component::Hour), Some(23));
    assert_eq!(r7[0].start.get(Component::TimezoneOffset), Some(-300));
    let r8 = run("casual", "2022-03-26 23:00 CET", now(), options());
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].text, "2022-03-26 23:00 CET");
    assert_eq!(r8[0].start.get(Component::Hour), Some(23));
    assert_eq!(r8[0].start.get(Component::TimezoneOffset), Some(60));
    let r9 = run("casual", "2022-03-27 23:00 CET", now(), options());
    assert_eq!(r9.len(), 1);
    assert_eq!(r9[0].text, "2022-03-27 23:00 CET");
    assert_eq!(r9[0].start.get(Component::Hour), Some(23));
    assert_eq!(r9[0].start.get(Component::TimezoneOffset), Some(120));
    let r10 = run("casual", "2021-03-27 23:00 CET", now(), options());
    assert_eq!(r10.len(), 1);
    assert_eq!(r10[0].text, "2021-03-27 23:00 CET");
    assert_eq!(r10[0].start.get(Component::Hour), Some(23));
    assert_eq!(r10[0].start.get(Component::TimezoneOffset), Some(60));
    let r11 = run("casual", "2021-03-28 23:00 CET", now(), options());
    assert_eq!(r11.len(), 1);
    assert_eq!(r11[0].text, "2021-03-28 23:00 CET");
    assert_eq!(r11[0].start.get(Component::Hour), Some(23));
    assert_eq!(r11[0].start.get(Component::TimezoneOffset), Some(120));
    let r12 = run("casual", "2022-10-29 23:00 CET", now(), options());
    assert_eq!(r12.len(), 1);
    assert_eq!(r12[0].text, "2022-10-29 23:00 CET");
    assert_eq!(r12[0].start.get(Component::Hour), Some(23));
    assert_eq!(r12[0].start.get(Component::TimezoneOffset), Some(120));
    let r13 = run("casual", "2022-10-30 23:00 CET", now(), options());
    assert_eq!(r13.len(), 1);
    assert_eq!(r13[0].text, "2022-10-30 23:00 CET");
    assert_eq!(r13[0].start.get(Component::Hour), Some(23));
    assert_eq!(r13[0].start.get(Component::TimezoneOffset), Some(60));
    let r14 = run("casual", "2021-10-30 23:00 CET", now(), options());
    assert_eq!(r14.len(), 1);
    assert_eq!(r14[0].text, "2021-10-30 23:00 CET");
    assert_eq!(r14[0].start.get(Component::Hour), Some(23));
    assert_eq!(r14[0].start.get(Component::TimezoneOffset), Some(120));
    let r15 = run("casual", "2021-10-31 23:00 CET", now(), options());
    assert_eq!(r15.len(), 1);
    assert_eq!(r15[0].text, "2021-10-31 23:00 CET");
    assert_eq!(r15[0].start.get(Component::Hour), Some(23));
    assert_eq!(r15[0].start.get(Component::TimezoneOffset), Some(60));
}

#[test]
fn timezone_parsing_overrides() {
    let r0 = run("casual", "Jan 1st 2023 at 10:00 XYZ", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Jan 1st 2023 at 10:00");
    assert_eq!(millis(r0[0].start.date()), Some(1672567200000));
    let r1 = run(
        "casual",
        "Jan 1st 2023 at 10:00 XYZ",
        at(1675209600000),
        options().timezone("XYZ", TimezoneAbbr::Fixed(-180)),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Jan 1st 2023 at 10:00 XYZ");
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-180));
    let r2 = run(
        "casual",
        "Jan 1st 2023 at 10:00 XYZ",
        at(1675209600000),
        options().timezone(
            "XYZ",
            TimezoneAbbr::Ambiguous {
                offset_during_dst: -120,
                offset_non_dst: -180,
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
            },
        ),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "Jan 1st 2023 at 10:00 XYZ");
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(-180));
    let r3 = run(
        "casual",
        "Jun 1st 2023 at 10:00 XYZ",
        at(1675209600000),
        options().timezone(
            "XYZ",
            TimezoneAbbr::Ambiguous {
                offset_during_dst: -120,
                offset_non_dst: -180,
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
            },
        ),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "Jun 1st 2023 at 10:00 XYZ");
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(-120));
}

#[test]
fn parsing_date_with_timezone_abbreviation() {
    let r0 = run(
        "casual",
        "Wednesday, September 16, 2020, EST",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Wednesday, September 16, 2020, EST");
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(-300));
}

#[test]
fn not_parsing_timezone_from_relative_time() {
    let r0 = run(
        "casual",
        "in 1 hour get eggs and milk",
        at(1606623853000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "in 1 hour");
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(millis(r0[0].start.date()), Some(1606627453000));
    let r1 = run("casual", "in 1 hour GMT", at(1606623853000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].start.date()), Some(1606627453000));
    let r2 = run(
        "casual",
        "in 1 hour GMT",
        at(1606623853000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].start.date()), Some(1606627453000));
    let r3 = run(
        "casual",
        "in 1 hour GMT",
        at(1606623853000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].start.date()), Some(1606627453000));
}

#[test]
fn relative_time_now_is_not_effected_by_timezone_setting() {
    let r0 = run("casual", "now", at(1637674343000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "now");
    assert_eq!(millis(r0[0].start.date()), Some(1637674343000));
    let r1 = run("casual", "now", at(1637674343000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "now");
    assert_eq!(millis(r1[0].start.date()), Some(1637674343000));
    let r2 = run(
        "casual",
        "now",
        at(1637674343000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "now");
    assert_eq!(millis(r2[0].start.date()), Some(1637674343000));
    let r3 = run(
        "casual",
        "now",
        at(1637674343000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "now");
    assert_eq!(millis(r3[0].start.date()), Some(1637674343000));
}

#[test]
fn relative_time_2_hour_later_is_not_effected_by_timezone_setting() {
    let r0 = run("casual", "2 hour later", at(1637674343000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2 hour later");
    assert_eq!(millis(r0[0].start.date()), Some(1637681543000));
    let r1 = run("casual", "2 hour later", at(1637674343000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2 hour later");
    assert_eq!(millis(r1[0].start.date()), Some(1637681543000));
    let r2 = run(
        "casual",
        "2 hour later",
        at(1637674343000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2 hour later");
    assert_eq!(millis(r2[0].start.date()), Some(1637681543000));
    let r3 = run(
        "casual",
        "2 hour later",
        at(1637674343000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "2 hour later");
    assert_eq!(millis(r3[0].start.date()), Some(1637681543000));
}

#[test]
fn parsing_timezone_from_relative_date_when_valid() {
    let r0 = run(
        "casual",
        "in 1 day get eggs and milk",
        at(1605361702000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "in 1 day");
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(0));
    let r1 = run("casual", "in 1 day GET", at(1605361702000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "in 1 day GET");
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(240));
    let r2 = run("casual", "today EST", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "today EST");
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(-300));
    let r3 = run("casual", "next week EST", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "next week EST");
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(-300));
}
