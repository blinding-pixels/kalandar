use super::support::*;
use crate::types::Component;

#[test]
fn this_expressions() {
    let r0 = run("casual", "this week", at(1511092800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "this week");
    assert_eq!(r0[0].start.get(Component::Year), Some(2017));
    assert_eq!(r0[0].start.get(Component::Month), Some(11));
    assert_eq!(r0[0].start.get(Component::Day), Some(19));
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    let r1 = run("casual", "this month", at(1511092800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "this month");
    assert_eq!(r1[0].start.get(Component::Year), Some(2017));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "this month", at(1509537600000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "this month");
    assert_eq!(r2[0].start.get(Component::Year), Some(2017));
    assert_eq!(r2[0].start.get(Component::Month), Some(11));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    let r3 = run("casual", "this year", at(1511092800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "this year");
    assert_eq!(r3[0].start.get(Component::Year), Some(2017));
    assert_eq!(r3[0].start.get(Component::Month), Some(1));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
}

#[test]
fn past_relative_expressions() {
    let r0 = run("casual", "last week", at(1475323200000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "last week");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(r0[0].start.get(Component::Day), Some(24));
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    let r1 = run("casual", "lastmonth", at(1475323200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "lastmonth");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(9));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "last day", at(1475323200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "last day");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(30));
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    let r3 = run("casual", "last month", at(1475323200000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "last month");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(9));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
    let r4 = run("casual", "past week", at(1475323200000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "past week");
    assert_eq!(r4[0].start.get(Component::Year), Some(2016));
    assert_eq!(r4[0].start.get(Component::Month), Some(9));
    assert_eq!(r4[0].start.get(Component::Day), Some(24));
    assert_eq!(r4[0].start.get(Component::Hour), Some(12));
}

#[test]
fn future_relative_expressions() {
    let r0 = run("casual", "next hour", at(1475323200000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "next hour");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(13));
    let r1 = run("casual", "next week", at(1475323200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "next week");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(8));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "next day", at(1475323200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "next day");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(2));
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    let r3 = run("casual", "next month", at(1475323200000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "next month");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(11));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
    assert!(r3[0].start.is_certain(Component::Year));
    assert!(r3[0].start.is_certain(Component::Month));
    assert!(!r3[0].start.is_certain(Component::Day));
    assert!(!r3[0].start.is_certain(Component::Hour));
    let r4 = run("casual", "next year", at(1606047092006), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "next year");
    assert_eq!(r4[0].start.get(Component::Year), Some(2021));
    assert_eq!(r4[0].start.get(Component::Month), Some(11));
    assert_eq!(r4[0].start.get(Component::Day), Some(22));
    assert_eq!(r4[0].start.get(Component::Hour), Some(12));
    assert_eq!(r4[0].start.get(Component::Minute), Some(11));
    assert_eq!(r4[0].start.get(Component::Second), Some(32));
    assert_eq!(r4[0].start.get(Component::Millisecond), Some(6));
    assert!(r4[0].start.is_certain(Component::Year));
    assert!(!r4[0].start.is_certain(Component::Month));
    assert!(!r4[0].start.is_certain(Component::Day));
    assert!(!r4[0].start.is_certain(Component::Hour));
    assert!(!r4[0].start.is_certain(Component::Minute));
    assert!(!r4[0].start.is_certain(Component::Second));
    assert!(!r4[0].start.is_certain(Component::Millisecond));
    assert!(!r4[0].start.is_certain(Component::TimezoneOffset));
    let r5 = run("casual", "next quarter", at(1611316800000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "next quarter");
    assert_eq!(r5[0].start.get(Component::Year), Some(2021));
    assert_eq!(r5[0].start.get(Component::Month), Some(4));
    assert_eq!(r5[0].start.get(Component::Day), Some(22));
    assert_eq!(r5[0].start.get(Component::Hour), Some(12));
    assert!(!r5[0].start.is_certain(Component::Year));
    assert!(!r5[0].start.is_certain(Component::Month));
    assert!(!r5[0].start.is_certain(Component::Day));
    assert!(!r5[0].start.is_certain(Component::Hour));
    let r6 = run("casual", "next qtr", at(1634904000000), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "next qtr");
    assert_eq!(r6[0].start.get(Component::Year), Some(2022));
    assert_eq!(r6[0].start.get(Component::Month), Some(1));
    assert_eq!(r6[0].start.get(Component::Day), Some(22));
    assert_eq!(r6[0].start.get(Component::Hour), Some(12));
    assert!(!r6[0].start.is_certain(Component::Year));
    assert!(!r6[0].start.is_certain(Component::Month));
    assert!(!r6[0].start.is_certain(Component::Day));
    assert!(!r6[0].start.is_certain(Component::Hour));
    let r7 = run("casual", "next two quarter", at(1611316800000), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "next two quarter");
    assert_eq!(r7[0].start.get(Component::Year), Some(2021));
    assert_eq!(r7[0].start.get(Component::Month), Some(7));
    assert_eq!(r7[0].start.get(Component::Day), Some(22));
    assert_eq!(r7[0].start.get(Component::Hour), Some(12));
    assert!(!r7[0].start.is_certain(Component::Year));
    assert!(!r7[0].start.is_certain(Component::Month));
    assert!(!r7[0].start.is_certain(Component::Day));
    assert!(!r7[0].start.is_certain(Component::Hour));
    let r8 = run("casual", "after this year", at(1606047092006), options());
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].text, "after this year");
    assert_eq!(r8[0].start.get(Component::Year), Some(2021));
    assert_eq!(r8[0].start.get(Component::Month), Some(11));
    assert_eq!(r8[0].start.get(Component::Day), Some(22));
    assert_eq!(r8[0].start.get(Component::Hour), Some(12));
    assert_eq!(r8[0].start.get(Component::Minute), Some(11));
    assert_eq!(r8[0].start.get(Component::Second), Some(32));
    assert_eq!(r8[0].start.get(Component::Millisecond), Some(6));
    assert!(r8[0].start.is_certain(Component::Year));
    assert!(!r8[0].start.is_certain(Component::Month));
    assert!(!r8[0].start.is_certain(Component::Day));
    assert!(!r8[0].start.is_certain(Component::Hour));
    assert!(!r8[0].start.is_certain(Component::Minute));
    assert!(!r8[0].start.is_certain(Component::Second));
    assert!(!r8[0].start.is_certain(Component::Millisecond));
    assert!(!r8[0].start.is_certain(Component::TimezoneOffset));
    let r9 = run(
        "casual",
        "Connect back after this year",
        at(1650110400000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(r9[0].start.get(Component::Year), Some(2023));
    assert_eq!(r9[0].start.get(Component::Month), Some(4));
    assert_eq!(r9[0].start.get(Component::Day), Some(16));
}

#[test]
fn relative_date_components_certainty() {
    let r0 = run("casual", "next hour", at(1475841600000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "next hour");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(7));
    assert_eq!(r0[0].start.get(Component::Hour), Some(13));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(0));
    assert!(r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Hour));
    assert!(r0[0].start.is_certain(Component::TimezoneOffset));
    let r1 = run("casual", "next month", at(1475841600000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "next month");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(7));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert!(r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(!r1[0].start.is_certain(Component::Day));
    assert!(!r1[0].start.is_certain(Component::Hour));
    assert!(!r1[0].start.is_certain(Component::TimezoneOffset));
}

#[test]
fn relative_date_when_the_timezone_is_irrelevant() {
    let mut r0 = run("casual", "now", at(1606623853000), options());
    assert_eq!(millis(r0[0].date()), Some(1606623853000));
    assert_eq!(millis(r0[0].date()), Some(1606623853000));
    r0[0].start.imply(Component::TimezoneOffset, 60);
    assert_eq!(millis(r0[0].date()), Some(1606623853000));
    assert_eq!(millis(r0[0].date()), Some(1606623853000));
    let mut r1 = run("casual", "in 10 minutes", at(1606623853000), options());
    assert_eq!(millis(r1[0].date()), Some(1606624453000));
    assert_eq!(millis(r1[0].date()), Some(1606624453000));
    r1[0].start.imply(Component::TimezoneOffset, 60);
    assert_eq!(millis(r1[0].date()), Some(1606624453000));
    assert_eq!(millis(r1[0].date()), Some(1606624453000));
    let r2 = run(
        "casual",
        "in 10 minutes",
        at(1606623853000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(millis(r2[0].date()), Some(1606624453000));
    assert_eq!(millis(r2[0].date()), Some(1606624453000));
    let r3 = run(
        "casual",
        "in 10 minutes",
        at(1606623853000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(millis(r3[0].date()), Some(1606624453000));
    assert_eq!(millis(r3[0].date()), Some(1606624453000));
    let r4 = run("casual", "in 20 minutes", at(1606623853000), options());
    assert_eq!(millis(r4[0].date()), Some(1606625053000));
    assert_eq!(millis(r4[0].date()), Some(1606625053000));
}

#[test]
fn relative_date_when_the_timezone_is_relevant_but_unknown() {
    let r0 = run("casual", "tomorrow at 5pm", at(1606656253000), options());
    assert_eq!(millis(r0[0].date()), Some(1606755600000));
    assert_eq!(r0[0].start.get(Component::Month), Some(11));
    assert_eq!(r0[0].start.get(Component::Day), Some(30));
    assert_eq!(r0[0].start.get(Component::Hour), Some(17));
}

#[test]
fn relative_date_when_the_timezone_is_relevant_and_known() {
    let mut r0 = run(
        "casual",
        "tomorrow at 5pm",
        at(1606623853000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(millis(r0[0].date()), Some(1606723200000));
    assert_eq!(r0[0].start.get(Component::Month), Some(11));
    assert_eq!(r0[0].start.get(Component::Day), Some(30));
    assert_eq!(r0[0].start.get(Component::Hour), Some(17));
    r0[0].start.imply(Component::TimezoneOffset, 60);
    assert_eq!(millis(r0[0].date()), Some(1606752000000));
    let mut r1 = run(
        "casual",
        "tomorrow at 5pm",
        at(1606623853000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(millis(r1[0].date()), Some(1606752000000));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(30));
    assert_eq!(r1[0].start.get(Component::Hour), Some(17));
    r1[0].start.imply(Component::TimezoneOffset, 540);
    assert_eq!(millis(r1[0].date()), Some(1606723200000));
    let mut r2 = run(
        "casual",
        "tomorrow at 5pm",
        at(1606623853000).with_timezone_offset(-420),
        options(),
    );
    assert_eq!(millis(r2[0].date()), Some(1606694400000));
    assert_eq!(r2[0].start.get(Component::Month), Some(11));
    assert_eq!(r2[0].start.get(Component::Day), Some(29));
    assert_eq!(r2[0].start.get(Component::Hour), Some(17));
    r2[0].start.imply(Component::TimezoneOffset, 540);
    assert_eq!(millis(r2[0].date()), Some(1606636800000));
}

#[test]
fn relative_date_when_the_timezone_is_relevant_and_known_2nd_scenario() {
    let r0 = run(
        "casual",
        "tomorrow at 9am",
        at(1740675600000).with_timezone_abbreviation("PST"),
        options(),
    );
    assert_eq!(millis(r0[0].date()), Some(1740762000000));
    assert_eq!(millis(r0[0].date()), Some(1740762000000));
    let r1 = run(
        "casual",
        "in 2 weeks at 9am",
        at(1740675600000).with_timezone_abbreviation("PST"),
        options(),
    );
    assert_eq!(millis(r1[0].date()), Some(1741885200000));
    assert_eq!(millis(r1[0].date()), Some(1741885200000));
    let r2 = run(
        "casual",
        "2 weeks ago at 9am",
        at(1740675600000).with_timezone_abbreviation("PST"),
        options(),
    );
    assert_eq!(millis(r2[0].date()), Some(1739466000000));
    assert_eq!(millis(r2[0].date()), Some(1739466000000));
    let r3 = run(
        "casual",
        "next friday at 9am",
        at(1740675600000).with_timezone_abbreviation("PST"),
        options(),
    );
    assert_eq!(millis(r3[0].date()), Some(1741366800000));
    assert_eq!(millis(r3[0].date()), Some(1741366800000));
}
