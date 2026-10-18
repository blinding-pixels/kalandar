use super::support::*;
use crate::types::Component;

#[test]
fn year_numbers_with_bce_ce_era_label() {
    let r0 = run("casual", "10 August 234 BCE", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("10 August 234 BCE", &r0[0]), 0);
    assert_eq!(r0[0].text, "10 August 234 BCE");
    assert_eq!(r0[0].start.get(Component::Year), Some(-234));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r0[0].start.date()), Some(-69532344000000));
    let r1 = run("casual", "10 August 88 CE", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("10 August 88 CE", &r1[0]), 0);
    assert_eq!(r1[0].text, "10 August 88 CE");
    assert_eq!(r1[0].start.get(Component::Year), Some(88));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r1[0].start.date()), Some(-59370926400000));
}

#[test]
fn year_numbers_with_bc_ad_era_label() {
    let r0 = run("casual", "10 August 234 BC", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("10 August 234 BC", &r0[0]), 0);
    assert_eq!(r0[0].text, "10 August 234 BC");
    assert_eq!(r0[0].start.get(Component::Year), Some(-234));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r0[0].start.date()), Some(-69532344000000));
    let r1 = run("casual", "10 August 88 AD", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("10 August 88 AD", &r1[0]), 0);
    assert_eq!(r1[0].text, "10 August 88 AD");
    assert_eq!(r1[0].start.get(Component::Year), Some(88));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r1[0].start.date()), Some(-59370926400000));
}

#[test]
fn year_numbers_with_buddhist_era_label() {
    let r0 = run("casual", "10 August 2555 BE", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("10 August 2555 BE", &r0[0]), 0);
    assert_eq!(r0[0].text, "10 August 2555 BE");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
}

#[test]
fn year_number_after_date_time_expression() {
    let r0 = run(
        "casual",
        "Thu Oct 26 11:00:09 2023",
        at(1475308800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(26));
    assert_eq!(r0[0].start.get(Component::Hour), Some(11));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::Second), Some(9));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
    let r1 = run(
        "casual",
        "Thu Oct 26 11:00:09 EDT 2023",
        at(1475308800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(26));
    assert_eq!(r1[0].start.get(Component::Hour), Some(11));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::Second), Some(9));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-240));
}

#[test]
fn year_number_after_date_time_range_expression() {
    let r0 = run(
        "casual",
        "Thu Oct 26 - 28, 11:00:09 2023",
        at(1475308800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(26));
    assert_eq!(r0[0].start.get(Component::Hour), Some(11));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::Second), Some(9));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(28)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(11)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Minute),
        Some(0)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Second),
        Some(9)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(0)
    );
    let r1 = run(
        "casual",
        "Thu Oct 26, 10:00 - 11:00:09 2023",
        at(1475308800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(26));
    assert_eq!(r1[0].start.get(Component::Hour), Some(10));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::Second), Some(0));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(26)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(11)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Minute),
        Some(0)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Second),
        Some(9)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(0)
    );
}
