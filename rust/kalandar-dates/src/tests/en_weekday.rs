use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run("casual", "Monday", at(1344470400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("Monday", &r0[0]), 0);
    assert_eq!(r0[0].text, "Monday");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(6));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(1));
    assert!(!r0[0].start.is_certain(Component::Day));
    assert!(!r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r0[0].start.date()), Some(1344254400000));
    let r1 = run("casual", "Thursday", at(1344470400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("Thursday", &r1[0]), 0);
    assert_eq!(r1[0].text, "Thursday");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(9));
    assert_eq!(r1[0].start.get(Component::Weekday), Some(4));
    assert_eq!(millis(r1[0].start.date()), Some(1344513600000));
    let r2 = run("casual", "Sunday", at(1344470400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("Sunday", &r2[0]), 0);
    assert_eq!(r2[0].text, "Sunday");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(12));
    assert_eq!(r2[0].start.get(Component::Weekday), Some(0));
    assert_eq!(millis(r2[0].start.date()), Some(1344772800000));
    let r3 = run(
        "casual",
        "The Deadline is last Friday...",
        at(1344470400000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("The Deadline is last Friday...", &r3[0]), 16);
    assert_eq!(r3[0].text, "last Friday");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(3));
    assert_eq!(r3[0].start.get(Component::Weekday), Some(5));
    assert_eq!(millis(r3[0].start.date()), Some(1343995200000));
    let r4 = run(
        "casual",
        "The Deadline is past Friday...",
        at(1344470400000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("The Deadline is past Friday...", &r4[0]), 16);
    assert_eq!(r4[0].text, "past Friday");
    assert_eq!(r4[0].start.get(Component::Year), Some(2012));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    assert_eq!(r4[0].start.get(Component::Day), Some(3));
    assert_eq!(r4[0].start.get(Component::Weekday), Some(5));
    assert_eq!(millis(r4[0].start.date()), Some(1343995200000));
    let r5 = run(
        "casual",
        "Let's have a meeting on Friday next week",
        at(1429315200000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(
        index16("Let's have a meeting on Friday next week", &r5[0]),
        21
    );
    assert_eq!(r5[0].text, "on Friday next week");
    assert_eq!(r5[0].start.get(Component::Year), Some(2015));
    assert_eq!(r5[0].start.get(Component::Month), Some(4));
    assert_eq!(r5[0].start.get(Component::Day), Some(24));
    assert_eq!(r5[0].start.get(Component::Weekday), Some(5));
    assert_eq!(millis(r5[0].start.date()), Some(1429876800000));
    let r6 = run(
        "casual",
        "I plan on taking the day off on Tuesday, next week",
        at(1429315200000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(
        index16("I plan on taking the day off on Tuesday, next week", &r6[0]),
        29
    );
    assert_eq!(r6[0].text, "on Tuesday, next week");
    assert_eq!(r6[0].start.get(Component::Year), Some(2015));
    assert_eq!(r6[0].start.get(Component::Month), Some(4));
    assert_eq!(r6[0].start.get(Component::Day), Some(21));
    assert_eq!(r6[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r6[0].start.date()), Some(1429617600000));
}

#[test]
fn weekday_casual_this_guessing() {
    let r0 = run("casual", "This Saturday", at(1659398400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Day), Some(6));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    let r1 = run("casual", "This Sunday", at(1659398400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Day), Some(7));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    let r2 = run("casual", "This Wednesday", at(1659398400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Day), Some(3));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
    let r3 = run("casual", "This Saturday", at(1659830400000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Day), Some(13));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Year), Some(2022));
    let r4 = run("casual", "This Sunday", at(1659830400000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Day), Some(7));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    assert_eq!(r4[0].start.get(Component::Year), Some(2022));
    let r5 = run("casual", "This Wednesday", at(1659830400000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].start.get(Component::Day), Some(10));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Year), Some(2022));
}

#[test]
fn weekday_casual_last_guessing() {
    let r0 = run("casual", "Last Saturday", at(1659398400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Day), Some(30));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    let r1 = run("casual", "Last Sunday", at(1659398400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Day), Some(31));
    assert_eq!(r1[0].start.get(Component::Month), Some(7));
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    let r2 = run("casual", "Last Wednesday", at(1659398400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Day), Some(27));
    assert_eq!(r2[0].start.get(Component::Month), Some(7));
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
}

#[test]
fn weekday_casual_next_guessing() {
    let r0 = run("casual", "Next Saturday", at(1659398400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Day), Some(13));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    let r1 = run("casual", "Next Sunday", at(1659398400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Day), Some(14));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    let r2 = run("casual", "Next Wednesday", at(1659398400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Day), Some(10));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
    let r3 = run("casual", "Next Saturday", at(1659744000000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Day), Some(13));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Year), Some(2022));
    let r4 = run("casual", "Next Sunday", at(1659744000000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Day), Some(14));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    assert_eq!(r4[0].start.get(Component::Year), Some(2022));
    let r5 = run("casual", "Next Wednesday", at(1659744000000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].start.get(Component::Day), Some(10));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Year), Some(2022));
    let r6 = run("casual", "Next Saturday", at(1659830400000), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].start.get(Component::Day), Some(13));
    assert_eq!(r6[0].start.get(Component::Month), Some(8));
    assert_eq!(r6[0].start.get(Component::Year), Some(2022));
    let r7 = run("casual", "Next Sunday", at(1659830400000), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].start.get(Component::Day), Some(14));
    assert_eq!(r7[0].start.get(Component::Month), Some(8));
    assert_eq!(r7[0].start.get(Component::Year), Some(2022));
    let r8 = run("casual", "Next Wednesday", at(1659830400000), options());
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].start.get(Component::Day), Some(10));
    assert_eq!(r8[0].start.get(Component::Month), Some(8));
    assert_eq!(r8[0].start.get(Component::Year), Some(2022));
}

#[test]
fn casual_weekend_and_weekday() {
    let r0 = run("casual", "last weekend", at(1729252800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "last weekend");
    assert_eq!(millis(r0[0].start.date()), Some(1728820800000));
    let r1 = run("casual", "this weekend", at(1729252800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "this weekend");
    assert_eq!(millis(r1[0].start.date()), Some(1729339200000));
    let r2 = run("casual", "next weekend", at(1729252800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "next weekend");
    assert_eq!(millis(r2[0].start.date()), Some(1729944000000));
}

#[test]
fn casual_weekday_mentioning() {
    let r0 = run("casual", "last weekday", at(1729252800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "last weekday");
    assert_eq!(millis(r0[0].start.date()), Some(1729166400000));
    let r1 = run("casual", "next weekday", at(1729252800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "next weekday");
    assert_eq!(millis(r1[0].start.date()), Some(1729512000000));
    let r2 = run("casual", "last weekday", at(1729339200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "last weekday");
    assert_eq!(millis(r2[0].start.date()), Some(1729252800000));
    let r3 = run("casual", "next weekday", at(1729339200000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "next weekday");
    assert_eq!(millis(r3[0].start.date()), Some(1729512000000));
}

#[test]
fn weekday_with_casual_time() {
    let r0 = run(
        "casual",
        "Lets meet on Tuesday morning",
        at(1429315200000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("Lets meet on Tuesday morning", &r0[0]), 10);
    assert_eq!(r0[0].text, "on Tuesday morning");
    assert_eq!(r0[0].start.get(Component::Year), Some(2015));
    assert_eq!(r0[0].start.get(Component::Month), Some(4));
    assert_eq!(r0[0].start.get(Component::Day), Some(21));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(2));
    assert_eq!(r0[0].start.get(Component::Hour), Some(6));
    assert_eq!(millis(r0[0].start.date()), Some(1429596000000));
}

#[test]
fn weekday_overlap() {
    let r0 = run(
        "casual",
        "Sunday, December 7, 2014",
        at(1344470400000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("Sunday, December 7, 2014", &r0[0]), 0);
    assert_eq!(r0[0].text, "Sunday, December 7, 2014");
    assert_eq!(r0[0].start.get(Component::Year), Some(2014));
    assert_eq!(r0[0].start.get(Component::Month), Some(12));
    assert_eq!(r0[0].start.get(Component::Day), Some(7));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(0));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r0[0].start.date()), Some(1417953600000));
    let r1 = run("casual", "Sunday 12/7/2014", at(1344470400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("Sunday 12/7/2014", &r1[0]), 0);
    assert_eq!(r1[0].text, "Sunday 12/7/2014");
    assert_eq!(r1[0].start.get(Component::Year), Some(2014));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
    assert_eq!(r1[0].start.get(Component::Day), Some(7));
    assert_eq!(r1[0].start.get(Component::Weekday), Some(0));
    assert!(r1[0].start.is_certain(Component::Day));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r1[0].start.date()), Some(1417953600000));
}

#[test]
fn weekday_range() {
    let r0 = run("casual", "Friday to Monday", at(1680998400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(4));
    assert_eq!(r0[0].start.get(Component::Day), Some(7));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(5));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(4)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Weekday),
        Some(1)
    );
    let r1 = run("casual", "Monday to Friday", at(1680998400000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(4));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(r1[0].start.get(Component::Weekday), Some(1));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2023)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(4)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(14)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Weekday),
        Some(5)
    );
}

#[test]
fn weekday_of_next_last_this_week_connector() {
    let r0 = run(
        "casual",
        "Tuesday of next week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Tuesday of next week");
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(9));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r0[0].start.date()), Some(1660046400000));
    let r1 = run(
        "casual",
        "Friday of next week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Friday of next week");
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(12));
    assert_eq!(r1[0].start.get(Component::Weekday), Some(5));
    assert_eq!(millis(r1[0].start.date()), Some(1660305600000));
    let r2 = run(
        "casual",
        "Monday of next week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "Monday of next week");
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(8));
    assert_eq!(r2[0].start.get(Component::Weekday), Some(1));
    assert_eq!(millis(r2[0].start.date()), Some(1659960000000));
    let r3 = run(
        "casual",
        "Tuesday of last week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "Tuesday of last week");
    assert_eq!(r3[0].start.get(Component::Year), Some(2022));
    assert_eq!(r3[0].start.get(Component::Month), Some(7));
    assert_eq!(r3[0].start.get(Component::Day), Some(26));
    assert_eq!(r3[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r3[0].start.date()), Some(1658836800000));
    let r4 = run(
        "casual",
        "Friday of last week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "Friday of last week");
    assert_eq!(r4[0].start.get(Component::Year), Some(2022));
    assert_eq!(r4[0].start.get(Component::Month), Some(7));
    assert_eq!(r4[0].start.get(Component::Day), Some(29));
    assert_eq!(r4[0].start.get(Component::Weekday), Some(5));
    assert_eq!(millis(r4[0].start.date()), Some(1659096000000));
    let r5 = run(
        "casual",
        "Wednesday of this week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "Wednesday of this week");
    assert_eq!(r5[0].start.get(Component::Year), Some(2022));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Day), Some(3));
    assert_eq!(r5[0].start.get(Component::Weekday), Some(3));
    assert_eq!(millis(r5[0].start.date()), Some(1659528000000));
    let r6 = run(
        "casual",
        "Tuesday of this week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "Tuesday of this week");
    assert_eq!(r6[0].start.get(Component::Year), Some(2022));
    assert_eq!(r6[0].start.get(Component::Month), Some(8));
    assert_eq!(r6[0].start.get(Component::Day), Some(2));
    assert_eq!(r6[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r6[0].start.date()), Some(1659441600000));
    let r7 = run(
        "casual",
        "Tuesday of next week after 2pm",
        at(1659398400000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "Tuesday of next week after 2pm");
    assert_eq!(r7[0].start.get(Component::Year), Some(2022));
    assert_eq!(r7[0].start.get(Component::Month), Some(8));
    assert_eq!(r7[0].start.get(Component::Day), Some(9));
    assert_eq!(r7[0].start.get(Component::Hour), Some(14));
    assert_eq!(millis(r7[0].start.date()), Some(1660053600000));
    let r8 = run(
        "casual",
        "Friday of next week at 9am",
        at(1659398400000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].text, "Friday of next week at 9am");
    assert_eq!(r8[0].start.get(Component::Year), Some(2022));
    assert_eq!(r8[0].start.get(Component::Month), Some(8));
    assert_eq!(r8[0].start.get(Component::Day), Some(12));
    assert_eq!(r8[0].start.get(Component::Hour), Some(9));
    assert_eq!(millis(r8[0].start.date()), Some(1660294800000));
    let r9 = run(
        "casual",
        "Let's sync on Tuesday of next week",
        at(1659398400000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(r9[0].text, "on Tuesday of next week");
    assert_eq!(r9[0].start.get(Component::Day), Some(9));
    assert_eq!(r9[0].start.get(Component::Month), Some(8));
    assert_eq!(r9[0].start.get(Component::Year), Some(2022));
}

#[test]
fn forward_dates_only_option() {
    let r0 = run(
        "casual",
        "Monday (forward dates only)",
        at(1344470400000),
        options().forward(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("Monday (forward dates only)", &r0[0]), 0);
    assert_eq!(r0[0].text, "Monday");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(13));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(1));
    assert!(!r0[0].start.is_certain(Component::Day));
    assert!(!r0[0].start.is_certain(Component::Month));
    assert!(!r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r0[0].start.date()), Some(1344859200000));
    let r1 = run(
        "casual",
        "this Friday to this Monday",
        at(1470268800000),
        options().forward(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("this Friday to this Monday", &r1[0]), 0);
    assert_eq!(r1[0].text, "this Friday to this Monday");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(5));
    assert_eq!(r1[0].start.get(Component::Weekday), Some(5));
    assert!(!r1[0].start.is_certain(Component::Day));
    assert!(!r1[0].start.is_certain(Component::Month));
    assert!(!r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r1[0].start.date()), Some(1470398400000));
    assert!(r1[0].end.is_some());
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(8)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Weekday),
        Some(1)
    );
    assert!(!r1[0].end.as_ref().expect("end").is_certain(Component::Day));
    assert!(!r1[0]
        .end
        .as_ref()
        .expect("end")
        .is_certain(Component::Month));
    assert!(!r1[0].end.as_ref().expect("end").is_certain(Component::Year));
    assert!(r1[0]
        .end
        .as_ref()
        .expect("end")
        .is_certain(Component::Weekday));
    assert_eq!(
        millis(r1[0].end.as_ref().expect("end").date()),
        Some(1470657600000)
    );
    let r2 = run(
        "casual",
        "sunday morning",
        at(1629057600000),
        options().forward(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("sunday morning", &r2[0]), 0);
    assert_eq!(r2[0].text, "sunday morning");
    assert_eq!(r2[0].start.get(Component::Year), Some(2021));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(22));
    assert_eq!(r2[0].start.get(Component::Weekday), Some(0));
    assert!(!r2[0].start.is_certain(Component::Day));
    assert!(!r2[0].start.is_certain(Component::Month));
    assert!(!r2[0].start.is_certain(Component::Year));
    assert!(r2[0].start.is_certain(Component::Weekday));
    assert_eq!(millis(r2[0].start.date()), Some(1629612000000));
    let r3 = run(
        "casual",
        "vacation monday - friday",
        at(1560384000000),
        options().forward(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "monday - friday");
    assert_eq!(r3[0].start.get(Component::Year), Some(2019));
    assert_eq!(r3[0].start.get(Component::Month), Some(6));
    assert_eq!(r3[0].start.get(Component::Day), Some(17));
    assert!(r3[0].end.is_some());
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2019)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(6)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Day),
        Some(21)
    );
    let r4 = run(
        "casual",
        "timeoff monday 7 to 9am",
        at(1560384000000),
        options().forward(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "monday 7 to 9am");
    assert_eq!(r4[0].start.get(Component::Year), Some(2019));
    assert_eq!(r4[0].start.get(Component::Month), Some(6));
    assert_eq!(r4[0].start.get(Component::Day), Some(17));
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Year),
        Some(2019)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Month),
        Some(6)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Day),
        Some(17)
    );
}
