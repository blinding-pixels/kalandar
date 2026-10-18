use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run(
        "casual",
        "The Deadline is now",
        at(1344586150011),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("The Deadline is now", &r0[0]), 16);
    assert_eq!(r0[0].text, "now");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(8));
    assert_eq!(r0[0].start.get(Component::Minute), Some(9));
    assert_eq!(r0[0].start.get(Component::Second), Some(10));
    assert_eq!(r0[0].start.get(Component::Millisecond), Some(11));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(millis(r0[0].start.date()), Some(1344586150011));
    assert_eq!(millis(r0[0].start.date()), Some(1344586150011));
    let r1 = run(
        "casual",
        "The Deadline is now, without implicit local timezone",
        at(1637674343000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "now");
    assert_eq!(millis(r1[0].start.date()), Some(1637674343000));
    let r2 = run(
        "casual",
        "The Deadline is today",
        at(1344607920000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("The Deadline is today", &r2[0]), 16);
    assert_eq!(r2[0].text, "today");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r2[0].start.date()), Some(1344607920000));
    let r3 = run(
        "casual",
        "The Deadline is Tomorrow",
        at(1344618600000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("The Deadline is Tomorrow", &r3[0]), 16);
    assert_eq!(r3[0].text, "Tomorrow");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(11));
    assert_eq!(millis(r3[0].start.date()), Some(1344705000000));
    let r4 = run(
        "casual",
        "The Deadline is Tomorrow",
        at(1344560400000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(millis(r4[0].start.date()), Some(1344646800000));
    let r5 = run(
        "casual",
        "The Deadline was yesterday",
        at(1344600000000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("The Deadline was yesterday", &r5[0]), 17);
    assert_eq!(r5[0].text, "yesterday");
    assert_eq!(r5[0].start.get(Component::Year), Some(2012));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Day), Some(9));
    assert_eq!(millis(r5[0].start.date()), Some(1344513600000));
    let r6 = run(
        "casual",
        "The Deadline was last night ",
        at(1344600000000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("The Deadline was last night ", &r6[0]), 17);
    assert_eq!(r6[0].text, "last night");
    assert_eq!(r6[0].start.get(Component::Year), Some(2012));
    assert_eq!(r6[0].start.get(Component::Month), Some(8));
    assert_eq!(r6[0].start.get(Component::Day), Some(9));
    assert_eq!(r6[0].start.get(Component::Hour), Some(0));
    assert_eq!(millis(r6[0].start.date()), Some(1344470400000));
    let r7 = run(
        "casual",
        "The Deadline was this morning ",
        at(1344600000000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("The Deadline was this morning ", &r7[0]), 17);
    assert_eq!(r7[0].text, "this morning");
    assert_eq!(r7[0].start.get(Component::Year), Some(2012));
    assert_eq!(r7[0].start.get(Component::Month), Some(8));
    assert_eq!(r7[0].start.get(Component::Day), Some(10));
    assert_eq!(r7[0].start.get(Component::Hour), Some(6));
    assert_eq!(millis(r7[0].start.date()), Some(1344578400000));
    let r8 = run(
        "casual",
        "The Deadline was this afternoon ",
        at(1344600000000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("The Deadline was this afternoon ", &r8[0]), 17);
    assert_eq!(r8[0].text, "this afternoon");
    assert_eq!(r8[0].start.get(Component::Year), Some(2012));
    assert_eq!(r8[0].start.get(Component::Month), Some(8));
    assert_eq!(r8[0].start.get(Component::Day), Some(10));
    assert_eq!(r8[0].start.get(Component::Hour), Some(15));
    assert_eq!(millis(r8[0].start.date()), Some(1344610800000));
    let r9 = run(
        "casual",
        "The Deadline was this evening ",
        at(1344600000000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(index16("The Deadline was this evening ", &r9[0]), 17);
    assert_eq!(r9[0].text, "this evening");
    assert_eq!(r9[0].start.get(Component::Year), Some(2012));
    assert_eq!(r9[0].start.get(Component::Month), Some(8));
    assert_eq!(r9[0].start.get(Component::Day), Some(10));
    assert_eq!(r9[0].start.get(Component::Hour), Some(20));
    assert_eq!(millis(r9[0].start.date()), Some(1344628800000));
    let r10 = run(
        "casual",
        "The Deadline is midnight ",
        at(1344600000000),
        options(),
    );
    assert_eq!(r10.len(), 1);
    assert_eq!(r10[0].text, "midnight");
    assert_eq!(r10[0].start.get(Component::Year), Some(2012));
    assert_eq!(r10[0].start.get(Component::Month), Some(8));
    assert_eq!(r10[0].start.get(Component::Day), Some(11));
    assert_eq!(r10[0].start.get(Component::Hour), Some(0));
    let r11 = run(
        "casual",
        "The Deadline was midnight ",
        at(1344560400000),
        options(),
    );
    assert_eq!(r11.len(), 1);
    assert_eq!(r11[0].text, "midnight");
    assert_eq!(r11[0].start.get(Component::Year), Some(2012));
    assert_eq!(r11[0].start.get(Component::Month), Some(8));
    assert_eq!(r11[0].start.get(Component::Day), Some(10));
    assert_eq!(r11[0].start.get(Component::Hour), Some(0));
    assert_eq!(r11[0].start.get(Component::Minute), Some(0));
    assert_eq!(r11[0].start.get(Component::Second), Some(0));
    assert_eq!(r11[0].start.get(Component::Millisecond), Some(0));
    let r12 = run(
        "casual",
        "The Deadline was midnight ",
        at(1344560400000),
        options().forward(),
    );
    assert_eq!(r12.len(), 1);
    assert_eq!(r12[0].text, "midnight");
    assert_eq!(r12[0].start.get(Component::Year), Some(2012));
    assert_eq!(r12[0].start.get(Component::Month), Some(8));
    assert_eq!(r12[0].start.get(Component::Day), Some(11));
    assert_eq!(r12[0].start.get(Component::Hour), Some(0));
    assert_eq!(r12[0].start.get(Component::Minute), Some(0));
    assert_eq!(r12[0].start.get(Component::Second), Some(0));
    assert_eq!(r12[0].start.get(Component::Millisecond), Some(0));
}

#[test]
fn combined_expression() {
    let r0 = run(
        "casual",
        "The Deadline is today 5PM",
        at(1344600000000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("The Deadline is today 5PM", &r0[0]), 16);
    assert_eq!(r0[0].text, "today 5PM");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(17));
    assert_eq!(millis(r0[0].start.date()), Some(1344618000000));
    let r1 = run("casual", "Tomorrow at noon", at(1344607200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(11));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(millis(r1[0].start.date()), Some(1344686400000));
}

#[test]
fn casual_date_range() {
    let r0 = run(
        "casual",
        "The event is today - next friday",
        at(1344081600000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("The event is today - next friday", &r0[0]), 13);
    assert_eq!(r0[0].text, "today - next friday");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(4));
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(millis(r0[0].start.date()), Some(1344081600000));
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
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(12)
    );
    assert_eq!(
        millis(r0[0].end.as_ref().expect("end").date()),
        Some(1344600000000)
    );
    let r1 = run(
        "casual",
        "The event is today - next friday",
        at(1344600000000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("The event is today - next friday", &r1[0]), 13);
    assert_eq!(r1[0].text, "today - next friday");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(millis(r1[0].start.date()), Some(1344600000000));
    assert!(r1[0].end.is_some());
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2012)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(17)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(12)
    );
    assert_eq!(
        millis(r1[0].end.as_ref().expect("end").date()),
        Some(1345204800000)
    );
}

#[test]
fn casual_time_implication() {
    let r0 = run(
        "casual",
        "annual leave from today morning to tomorrow",
        at(1344081600000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "today morning to tomorrow");
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(4));
    assert_eq!(r0[0].start.get(Component::Hour), Some(6));
    assert!(!r0[0].start.is_certain(Component::Hour));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(5)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(12)
    );
    assert!(!r0[0].end.as_ref().expect("end").is_certain(Component::Hour));
    let r1 = run(
        "casual",
        "annual leave from today to tomorrow afternoon",
        at(1344081600000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "today to tomorrow afternoon");
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(4));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert!(!r1[0].start.is_certain(Component::Hour));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(5)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(15)
    );
    assert!(!r1[0].end.as_ref().expect("end").is_certain(Component::Hour));
}

#[test]
fn random_text() {
    let r0 = run("casual", "tonight", at(1325419200000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "tonight");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(22));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    let r1 = run("casual", "tonight 8pm", at(1325419200000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "tonight 8pm");
    assert_eq!(r1[0].start.get(Component::Hour), Some(20));
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(1));
    let r2 = run("casual", "tonight at 8", at(1325419200000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "tonight at 8");
    assert_eq!(r2[0].start.get(Component::Hour), Some(20));
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(1));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(1));
    let r3 = run(
        "casual",
        "tomorrow before 4pm",
        at(1325419200000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "tomorrow before 4pm");
    assert_eq!(r3[0].start.get(Component::Hour), Some(16));
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(1));
    assert_eq!(r3[0].start.get(Component::Day), Some(2));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(1));
    let r4 = run("casual", "tomorrow after 4pm", at(1325419200000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "tomorrow after 4pm");
    assert_eq!(r4[0].start.get(Component::Hour), Some(16));
    assert_eq!(r4[0].start.get(Component::Year), Some(2012));
    assert_eq!(r4[0].start.get(Component::Month), Some(1));
    assert_eq!(r4[0].start.get(Component::Day), Some(2));
    assert_eq!(r4[0].start.get(Component::Meridiem), Some(1));
    let r5 = run("casual", "thurs", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "thurs");
    assert_eq!(r5[0].start.get(Component::Weekday), Some(4));
    let r6 = run("casual", "thurs", now(), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "thurs");
    assert_eq!(r6[0].start.get(Component::Weekday), Some(4));
    let r7 = run("casual", "this evening", at(1475280000000), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "this evening");
    assert_eq!(r7[0].start.get(Component::Year), Some(2016));
    assert_eq!(r7[0].start.get(Component::Month), Some(10));
    assert_eq!(r7[0].start.get(Component::Day), Some(1));
    assert_eq!(r7[0].start.get(Component::Hour), Some(20));
    let r8 = run(
        "casual",
        "yesterday afternoon",
        at(1475280000000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].text, "yesterday afternoon");
    assert_eq!(r8[0].start.get(Component::Year), Some(2016));
    assert_eq!(r8[0].start.get(Component::Month), Some(9));
    assert_eq!(r8[0].start.get(Component::Day), Some(30));
    assert_eq!(r8[0].start.get(Component::Hour), Some(15));
    let r9 = run("casual", "tomorrow morning", at(1475308800000), options());
    assert_eq!(r9.len(), 1);
    assert_eq!(r9[0].text, "tomorrow morning");
    assert_eq!(r9[0].start.get(Component::Year), Some(2016));
    assert_eq!(r9[0].start.get(Component::Month), Some(10));
    assert_eq!(r9[0].start.get(Component::Day), Some(2));
    assert_eq!(r9[0].start.get(Component::Hour), Some(6));
    let r10 = run(
        "casual",
        "this afternoon at 3",
        at(1475308800000),
        options(),
    );
    assert_eq!(r10.len(), 1);
    assert_eq!(r10[0].text, "this afternoon at 3");
    assert_eq!(r10[0].start.get(Component::Year), Some(2016));
    assert_eq!(r10[0].start.get(Component::Month), Some(10));
    assert_eq!(r10[0].start.get(Component::Day), Some(1));
    assert_eq!(r10[0].start.get(Component::Hour), Some(15));
    let r11 = run(
        "casual",
        "at midnight on 12th August",
        at(1344610800000),
        options(),
    );
    assert_eq!(r11.len(), 1);
    assert_eq!(r11[0].start.get(Component::Year), Some(2012));
    assert_eq!(r11[0].start.get(Component::Month), Some(8));
    assert_eq!(r11[0].start.get(Component::Day), Some(12));
    assert_eq!(r11[0].start.get(Component::Hour), Some(0));
    assert_eq!(r11[0].start.get(Component::Minute), Some(0));
    assert_eq!(r11[0].start.get(Component::Second), Some(0));
    assert_eq!(r11[0].start.get(Component::Millisecond), Some(0));
}

#[test]
fn casual_time_with_timezone() {
    let r0 = run("casual", "Jan 1, 2020 Morning UTC", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Jan 1, 2020 Morning UTC");
    assert_eq!(r0[0].start.get(Component::Year), Some(2020));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(6));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(millis(r0[0].date()), Some(1577858400000));
    let r1 = run("casual", "Jan 1, 2020 Evening JST", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Jan 1, 2020 Evening JST");
    assert_eq!(r1[0].start.get(Component::Year), Some(2020));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(20));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(540));
    assert_eq!(millis(r1[0].date()), Some(1577876400000));
}

#[test]
fn random_negative_text() {
    let r0 = run("casual", "notoday", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "tdtmr", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "xyesterday", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "nowhere", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "noway", now(), options());
    assert_eq!(r4.len(), 0);
    let r5 = run("casual", "knowledge", now(), options());
    assert_eq!(r5.len(), 0);
    let r6 = run("casual", "mar", now(), options());
    assert_eq!(r6.len(), 0);
    let r7 = run("casual", "jan", now(), options());
    assert_eq!(r7.len(), 0);
    let r8 = run("casual", "do I have the money", now(), options());
    assert_eq!(r8.len(), 0);
    let r9 = run(
        "casual",
        "I may by here. May the force be with you. Theresa may become PM soon.",
        now(),
        options(),
    );
    assert_eq!(r9.len(), 0);
    let r10 = run(
        "casual",
        "XXX is set to be released in the second half of 2025",
        now(),
        options(),
    );
    assert_eq!(r10.len(), 0);
    let r11 = run("casual", "do I have the money", now(), options());
    assert_eq!(r11.len(), 0);
    let r12 = run("gb", "do I have the money", now(), options());
    assert_eq!(r12.len(), 0);
}
