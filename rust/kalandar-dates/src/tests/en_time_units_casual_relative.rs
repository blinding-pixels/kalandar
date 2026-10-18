use super::support::*;
use crate::types::Component;

#[test]
fn positive_time_units() {
    let r0 = run("casual", "next 2 weeks", at(1475323200000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "next 2 weeks");
    assert!(r0[0].tags().contains("result/relativeDate"));
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(15));
    let r1 = run("casual", "next 2 days", at(1475323200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "next 2 days");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(3));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "next two years", at(1475323200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "next two years");
    assert_eq!(r2[0].start.get(Component::Year), Some(2018));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    let r3 = run(
        "casual",
        "next 2 weeks 3 days",
        at(1475323200000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "next 2 weeks 3 days");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(18));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
    let r4 = run("casual", "after a year", at(1475323200000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "after a year");
    assert_eq!(r4[0].start.get(Component::Year), Some(2017));
    assert_eq!(r4[0].start.get(Component::Month), Some(10));
    assert_eq!(r4[0].start.get(Component::Day), Some(1));
    assert_eq!(r4[0].start.get(Component::Hour), Some(12));
    let r5 = run("casual", "after an hour", at(1475334000000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "after an hour");
    assert!(r5[0].tags().contains("result/relativeDate"));
    assert!(r5[0].tags().contains("result/relativeDateAndTime"));
    assert_eq!(r5[0].start.get(Component::Year), Some(2016));
    assert_eq!(r5[0].start.get(Component::Month), Some(10));
    assert_eq!(r5[0].start.get(Component::Day), Some(1));
    assert_eq!(r5[0].start.get(Component::Hour), Some(16));
}

#[test]
fn negative_time_units() {
    let r0 = run("casual", "last 2 weeks", at(1475323200000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "last 2 weeks");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(r0[0].start.get(Component::Day), Some(17));
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    let r1 = run("casual", "last two weeks", at(1475323200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "last two weeks");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(9));
    assert_eq!(r1[0].start.get(Component::Day), Some(17));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "past 2 days", at(1475323200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "past 2 days");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(29));
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    let r3 = run("casual", "+2 months, 5 days", at(1475323200000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "+2 months, 5 days");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(12));
    assert_eq!(r3[0].start.get(Component::Day), Some(6));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
}

#[test]
fn plus_sign() {
    let r0 = run("casual", "+15 minutes", at(1341922440000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "+15 minutes");
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(r0[0].start.get(Component::Minute), Some(29));
    assert_eq!(millis(r0[0].start.date()), Some(1341923340000));
    let r1 = run("casual", "+15min", at(1341922440000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "+15min");
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(r1[0].start.get(Component::Minute), Some(29));
    assert_eq!(millis(r1[0].start.date()), Some(1341923340000));
    let r2 = run("casual", "+1 day 2 hour", at(1341922440000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "+1 day 2 hour");
    assert_eq!(r2[0].start.get(Component::Day), Some(11));
    assert_eq!(r2[0].start.get(Component::Hour), Some(14));
    assert_eq!(r2[0].start.get(Component::Minute), Some(14));
    assert_eq!(millis(r2[0].start.date()), Some(1342016040000));
    let r3 = run("casual", "+1m", at(1341922440000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "+1m");
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
    assert_eq!(r3[0].start.get(Component::Minute), Some(15));
    assert_eq!(millis(r3[0].start.date()), Some(1341922500000));
}

#[test]
fn minus_sign() {
    let r0 = run("casual", "-3y", at(1436530440000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "-3y");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(r0[0].start.get(Component::Minute), Some(14));
    assert_eq!(millis(r0[0].start.date()), Some(1341922440000));
    let r1 = run("casual", "-2hr5min", at(1475323200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "-2hr5min");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(9));
    assert_eq!(r1[0].start.get(Component::Minute), Some(55));
    let r2 = run("casual", "-5d 00", at(1475323200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "-5d 00");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(26));
    assert_eq!(r2[0].start.get(Component::Hour), Some(0));
    assert_eq!(r2[0].start.get(Component::Minute), Some(0));
    assert_eq!(millis(r2[0].start.date()), Some(1474848000000));
}

#[test]
fn without_custom_parser_without_abbreviations() {
    let r0 = run("strict+casualRelativeNoAbbr", "-3y", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict+casualRelativeNoAbbr", "last 2m", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run(
        "strict+casualRelativeNoAbbr",
        "-2 hours 5 minutes",
        at(1475323200000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "-2 hours 5 minutes");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(9));
    assert_eq!(r2[0].start.get(Component::Minute), Some(55));
}

#[test]
fn negative_cases() {
    let r0 = run("casual", "3y", at(1436530440000), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "1 m", at(1436530440000), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "the day", at(1436530440000), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "a day", at(1436530440000), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "+am", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "+them", now(), options());
    assert_eq!(r5.len(), 0);
}
