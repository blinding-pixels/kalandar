use super::support::*;
use crate::types::Component;

#[test]
fn month_year_expression() {
    let r0 = run("casual", "September 2012", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "September 2012");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert!(r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Day));
    assert_eq!(millis(r0[0].start.date()), Some(1346500800000));
    let r1 = run("casual", "Sept 2012", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Sept 2012");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(9));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert!(r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(!r1[0].start.is_certain(Component::Day));
    assert_eq!(millis(r1[0].start.date()), Some(1346500800000));
    let r2 = run("casual", "Sep 2012", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "Sep 2012");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert!(r2[0].start.is_certain(Component::Year));
    assert!(r2[0].start.is_certain(Component::Month));
    assert!(!r2[0].start.is_certain(Component::Day));
    assert_eq!(millis(r2[0].start.date()), Some(1346500800000));
    let r3 = run("casual", "Sep. 2012", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "Sep. 2012");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(9));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert!(r3[0].start.is_certain(Component::Year));
    assert!(r3[0].start.is_certain(Component::Month));
    assert!(!r3[0].start.is_certain(Component::Day));
    assert_eq!(millis(r3[0].start.date()), Some(1346500800000));
    let r4 = run("casual", "Sep-2012", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Year), Some(2012));
    assert_eq!(r4[0].start.get(Component::Month), Some(9));
    assert_eq!(index16("Sep-2012", &r4[0]), 0);
    assert_eq!(r4[0].text, "Sep-2012");
    assert_eq!(millis(r4[0].start.date()), Some(1346500800000));
    let r5 = run("casual", "in June of 2022", now(), options());
    assert_eq!(r5.len(), 1);
    assert!(r5[0].text.contains("June of 2022"));
    assert_eq!(r5[0].start.get(Component::Year), Some(2022));
    assert_eq!(r5[0].start.get(Component::Month), Some(6));
    let r6 = run(
        "casual",
        "Statement of comprehensive income for the year ended Dec. 2021",
        now(),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "Dec. 2021");
    assert_eq!(r6[0].start.get(Component::Year), Some(2021));
    assert_eq!(r6[0].start.get(Component::Month), Some(12));
    assert!(r6[0].start.is_certain(Component::Year));
    assert!(r6[0].start.is_certain(Component::Month));
    assert!(!r6[0].start.is_certain(Component::Day));
}

#[test]
fn month_only_expression() {
    let r0 = run("casual", "In January", at(1606003200000), options());
    assert_eq!(r0.len(), 1);
    assert!(r0[0].text.contains("January"));
    assert_eq!(r0[0].start.get(Component::Year), Some(2021));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert!(!r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Day));
    assert_eq!(millis(r0[0].start.date()), Some(1609502400000));
    let r1 = run("casual", "in Jan", at(1606003200000), options());
    assert_eq!(r1.len(), 1);
    assert!(r1[0].text.contains("Jan"));
    assert_eq!(r1[0].start.get(Component::Year), Some(2021));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert!(!r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(!r1[0].start.is_certain(Component::Day));
    assert_eq!(millis(r1[0].start.date()), Some(1609502400000));
    let r2 = run("casual", "May", at(1606003200000), options());
    assert_eq!(r2.len(), 1);
    assert!(r2[0].text.contains("May"));
    assert_eq!(r2[0].start.get(Component::Year), Some(2021));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert!(!r2[0].start.is_certain(Component::Year));
    assert!(r2[0].start.is_certain(Component::Month));
    assert!(!r2[0].start.is_certain(Component::Day));
    assert_eq!(millis(r2[0].start.date()), Some(1619870400000));
}

#[test]
fn month_only_range_expression() {
    let r0 = run(
        "casual",
        "From May to December",
        at(1680998400000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(12)
    );
    let r1 = run(
        "casual",
        "From December to May",
        at(1680998400000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
    let r2 = run(
        "casual",
        "From May to December, 2022",
        at(1680998400000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2022)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(12)
    );
    let r3 = run(
        "casual",
        "From December to May 2022",
        at(1680998400000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2021));
    assert_eq!(r3[0].start.get(Component::Month), Some(12));
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2022)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
    let r4 = run(
        "casual",
        "From December to May 2020",
        at(1680998400000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Year), Some(2019));
    assert_eq!(r4[0].start.get(Component::Month), Some(12));
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Year),
        Some(2020)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
    let r5 = run(
        "casual",
        "From December to May 2025",
        at(1680998400000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].start.get(Component::Year), Some(2024));
    assert_eq!(r5[0].start.get(Component::Month), Some(12));
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Year),
        Some(2025)
    );
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
}

#[test]
fn month_with_farward_date_option() {
    let r0 = run(
        "casual",
        "in December",
        at(1680998400000),
        options().forward(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(12));
    let r1 = run("casual", "in May", at(1680998400000), options().forward());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    let r2 = run(
        "casual",
        "From May to December",
        at(1680998400000),
        options().forward(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2023));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(12)
    );
    let r3 = run(
        "casual",
        "From December to May",
        at(1680998400000),
        options().forward(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2023));
    assert_eq!(r3[0].start.get(Component::Month), Some(12));
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2024)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
}

#[test]
fn month_expression_in_context() {
    let r0 = run(
        "casual",
        "The date is Sep 2012 is the date",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("The date is Sep 2012 is the date", &r0[0]), 12);
    assert_eq!(r0[0].text, "Sep 2012");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(millis(r0[0].start.date()), Some(1346500800000));
    let r1 = run("casual", "By Angie Mar November 2019", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "November 2019");
    assert_eq!(r1[0].start.get(Component::Year), Some(2019));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(millis(r1[0].start.date()), Some(1572609600000));
}

#[test]
fn month_slash_expression() {
    let r0 = run("casual", "9/2012", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(index16("9/2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "9/2012");
    assert_eq!(millis(r0[0].start.date()), Some(1346500800000));
    let r1 = run("casual", "09/2012", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(9));
    assert_eq!(index16("09/2012", &r1[0]), 0);
    assert_eq!(r1[0].text, "09/2012");
    assert_eq!(millis(r1[0].start.date()), Some(1346500800000));
}

#[test]
fn year_90_s_parsing() {
    let r0 = run("casual", "Aug 96", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Aug 96");
    assert_eq!(r0[0].start.get(Component::Year), Some(1996));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    let r1 = run("casual", "96 Aug 96", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Aug 96");
    assert_eq!(r1[0].start.get(Component::Year), Some(1996));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
}

#[test]
fn month_should_not_have_timezone() {
    let r0 = run("casual", "People visiting Buñol towards the end of August get a good chance to participate in La Tomatina (under normal circumstances)", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "August");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
}

#[test]
fn month_only_in_different_context() {
    let r0 = run("casual", "May", now(), options());
    assert_eq!(r0.len(), 1);
    assert!(r0[0].text.contains("May"));
    let r1 = run("casual", "in May", now(), options());
    assert_eq!(r1.len(), 1);
    assert!(r1[0].text.contains("May"));
    let r2 = run("casual", "The mountain may not move", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "May not be correct", now(), options());
    assert_eq!(r3.len(), 0);
}

#[test]
fn year_month_expression() {
    let r0 = run("casual", "2024 Aug", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2024 Aug");
    assert_eq!(r0[0].start.get(Component::Year), Some(2024));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert!(r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Day));
    let r1 = run("casual", "2024 August", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2024 August");
    assert_eq!(r1[0].start.get(Component::Year), Some(2024));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    let r2 = run("casual", "2023 Sept", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2023 Sept");
    assert_eq!(r2[0].start.get(Component::Year), Some(2023));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    let r3 = run("casual", "2024-August", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "2024-August");
    assert_eq!(r3[0].start.get(Component::Year), Some(2024));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    let r4 = run("casual", "2024/August", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "2024/August");
    assert_eq!(r4[0].start.get(Component::Year), Some(2024));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    let r5 = run("casual", "2024 AD August", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "2024 AD August");
    assert_eq!(r5[0].start.get(Component::Year), Some(2024));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
}
