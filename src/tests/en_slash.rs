use super::support::*;
use crate::types::Component;

#[test]
fn parsing_offset_expression() {
    let r0 = run("casual", "    04/2016   ", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("    04/2016   ", &r0[0]), 4);
    assert_eq!(r0[0].text, "04/2016");
}

#[test]
fn single_expression_mm_dd_yyyy() {
    let r0 = run("casual", "8/10/2012", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("8/10/2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "8/10/2012");
    assert!(r0[0].tags().contains("parser/SlashDateFormatParser"));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(r0[0].start.is_certain(Component::Year));
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    let r1 = run("casual", ": 8/1/2012", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(index16(": 8/1/2012", &r1[0]), 2);
    assert_eq!(r1[0].text, "8/1/2012");
    assert_eq!(millis(r1[0].start.date()), Some(1343822400000));
    let r2 = run(
        "casual",
        "The Deadline is 8/10/2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("The Deadline is 8/10/2012", &r2[0]), 16);
    assert_eq!(r2[0].text, "8/10/2012");
    assert_eq!(millis(r2[0].start.date()), Some(1344600000000));
    let r3 = run(
        "casual",
        "The Deadline is Tuesday 11/3/2015",
        at(1446508800000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("The Deadline is Tuesday 11/3/2015", &r3[0]), 16);
    assert_eq!(r3[0].text, "Tuesday 11/3/2015");
    assert_eq!(millis(r3[0].start.date()), Some(1446552000000));
    let r4 = run("strict", "2/28/2014", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "2/28/2014");
    let r5 = run("strict", "12-30-16", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(millis(r5[0].start.date()), Some(1483099200000));
    let r6 = run("strict", "Friday 12-30-16", now(), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "Friday 12-30-16");
    assert_eq!(millis(r6[0].date()), Some(1483099200000));
}

#[test]
fn single_expression_little_endian_dd_mm_yyyy() {
    let r0 = run("gb", "8/10/2012", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(8));
    assert_eq!(index16("8/10/2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "8/10/2012");
    assert_eq!(millis(r0[0].start.date()), Some(1349697600000));
    let r1 = run("strict", "30-12-16", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].start.date()), Some(1483099200000));
    let r2 = run("strict", "Friday 30-12-16", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "Friday 30-12-16");
    assert_eq!(millis(r2[0].date()), Some(1483099200000));
}

#[test]
fn single_expression_little_endian_with_month_name() {
    let r0 = run("gb", "8/Oct/2012", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(8));
    assert_eq!(index16("8/Oct/2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "8/Oct/2012");
    assert_eq!(millis(r0[0].start.date()), Some(1349697600000));
    let r1 = run("strict", "06/Nov/2023", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "06/Nov/2023");
    assert_eq!(millis(r1[0].date()), Some(1699272000000));
    let r2 = run("strict", "06/Nov/2023:06:36:02", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "06/Nov/2023:06:36:02");
    assert_eq!(millis(r2[0].date()), Some(1699252562000));
    let r3 = run("strict", "06/Nov/2023:06:36:02 +0200", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "06/Nov/2023:06:36:02 +0200");
    assert_eq!(millis(r3[0].date()), Some(1699245362000));
}

#[test]
fn single_expression_shorten_mm_yyyy() {
    let r0 = run(
        "casual",
        "The event is going ahead (04/2016)",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(4));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(index16("The event is going ahead (04/2016)", &r0[0]), 26);
    assert_eq!(r0[0].text, "04/2016");
    assert_eq!(millis(r0[0].start.date()), Some(1459512000000));
    let r1 = run("casual", "Published: 06/2004", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2004));
    assert_eq!(r1[0].start.get(Component::Month), Some(6));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(index16("Published: 06/2004", &r1[0]), 11);
    assert_eq!(r1[0].text, "06/2004");
    assert_eq!(millis(r1[0].start.date()), Some(1086091200000));
}

#[test]
fn single_expression_shorten_dd_mm() {
    let r0 = run("casual", "8/10", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("8/10", &r0[0]), 0);
    assert_eq!(r0[0].text, "8/10");
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Year));
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
}

#[test]
fn range_expression() {
    let r0 = run(
        "casual",
        "8/10/2012 - 8/15/2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("8/10/2012 - 8/15/2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "8/10/2012 - 8/15/2012");
    assert!(r0[0].tags().contains("parser/SlashDateFormatParser"));
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert!(r0[0].start.tags().contains("parser/SlashDateFormatParser"));
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    assert!(r0[0].end.is_some());
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2012)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(15)
    );
    assert!(r0[0]
        .end
        .as_ref()
        .expect("end")
        .tags()
        .contains("parser/SlashDateFormatParser"));
    assert_eq!(
        millis(r0[0].end.as_ref().expect("end").date()),
        Some(1345032000000)
    );
}

#[test]
fn range_expressions_with_time() {
    let r0 = run(
        "casual",
        "from 01/21/2021 10:00 to 01/01/2023 07:00",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2021));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(21));
    assert_eq!(r0[0].start.get(Component::Hour), Some(10));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert!(r0[0].end.is_some());
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(1)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(1)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(7)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Minute),
        Some(0)
    );
    let r1 = run(
        "casual",
        "08/08/2023, 09:15 AM to 08/29/2023, 09:15 AM",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(8));
    assert_eq!(r1[0].start.get(Component::Hour), Some(9));
    assert_eq!(r1[0].start.get(Component::Minute), Some(15));
    assert!(r1[0].end.is_some());
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(29)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(9)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Minute),
        Some(15)
    );
}

#[test]
fn splitter_variances_patterns() {
    let r0 = run("casual", "2015-05-25", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].start.date()), Some(1432555200000));
    let r1 = run("casual", "2015/05/25", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].start.date()), Some(1432555200000));
    let r2 = run("casual", "2015.05.25", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].start.date()), Some(1432555200000));
    let r3 = run("casual", "05-25-2015", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].start.date()), Some(1432555200000));
    let r4 = run("casual", "05/25/2015", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(millis(r4[0].start.date()), Some(1432555200000));
    let r5 = run("casual", "05.25.2015", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(millis(r5[0].start.date()), Some(1432555200000));
    let r6 = run("casual", "/05/25/2015", now(), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(millis(r6[0].start.date()), Some(1432555200000));
    let r7 = run("casual", "25/05/2015", now(), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(millis(r7[0].start.date()), Some(1432555200000));
}

#[test]
fn impossible_dates_and_unexpected_results() {
    let r0 = run("casual", "8/32/2014", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "8/32", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "2/29/2014", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "2014/22/29", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "2014/13/22", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "80-32-89-89", now(), options());
    assert_eq!(r5.len(), 0);
    let r6 = run("casual", "02/29/2022", now(), options());
    assert_eq!(r6.len(), 0);
    let r7 = run("casual", "06/31/2022", now(), options());
    assert_eq!(r7.len(), 0);
    let r8 = run("casual", "06/-31/2022", now(), options());
    assert_eq!(r8.len(), 0);
    let r9 = run("casual", "18/13/2022", now(), options());
    assert_eq!(r9.len(), 0);
    let r10 = run("casual", "15/28/2022", now(), options());
    assert_eq!(r10.len(), 0);
    let r11 = run("casual", "4/13/1", now(), options());
    assert_eq!(r11.len(), 0);
}

#[test]
fn forward_dates_only_option() {
    let r0 = run("casual", "5/31", at(928195200000), options().forward());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2000));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(31));
    assert_eq!(index16("5/31", &r0[0]), 0);
    assert_eq!(r0[0].text, "5/31");
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Year));
    assert_eq!(millis(r0[0].start.date()), Some(959774400000));
    let r1 = run(
        "casual",
        "1/8 at 12pm",
        at(1632571200000),
        options().forward(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "1/8 at 12pm");
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(8));
    assert_eq!(millis(r1[0].start.date()), Some(1641643200000));
}

#[test]
fn slash_date_with_an_extra_chunk() {
    let r0 = run("casual", "14/4 90", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(4));
    assert_eq!(r0[0].start.get(Component::Day), Some(14));
}
