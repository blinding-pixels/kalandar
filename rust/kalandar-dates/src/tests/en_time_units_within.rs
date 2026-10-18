use super::support::*;
use crate::types::Component;

#[test]
fn the_normal_within_expression() {
    let r0 = run(
        "casual",
        "we have to make something in 5 days.",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("we have to make something in 5 days.", &r0[0]), 26);
    assert_eq!(r0[0].text, "in 5 days");
    assert!(r0[0].tags().contains("result/relativeDate"));
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(15));
    assert_eq!(millis(r0[0].start.date()), Some(1344988800000));
    let r1 = run(
        "casual",
        "we have to make something in five days.",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(
        index16("we have to make something in five days.", &r1[0]),
        26
    );
    assert_eq!(r1[0].text, "in five days");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(15));
    assert_eq!(millis(r1[0].start.date()), Some(1344988800000));
    let r2 = run(
        "casual",
        "we have to make something within 10 day",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(
        index16("we have to make something within 10 day", &r2[0]),
        26
    );
    assert_eq!(r2[0].text, "within 10 day");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(20));
    assert_eq!(millis(r2[0].start.date()), Some(1345420800000));
    let r3 = run("casual", "in 5 minutes", at(1344600840000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("in 5 minutes", &r3[0]), 0);
    assert_eq!(r3[0].text, "in 5 minutes");
    assert_eq!(millis(r3[0].start.date()), Some(1344601140000));
    let r4 = run("casual", "wait for 5 minutes", at(1344600840000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("wait for 5 minutes", &r4[0]), 5);
    assert_eq!(r4[0].text, "for 5 minutes");
    assert_eq!(millis(r4[0].start.date()), Some(1344601140000));
    let r5 = run("casual", "within 1 hour", at(1344600840000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("within 1 hour", &r5[0]), 0);
    assert_eq!(r5[0].text, "within 1 hour");
    assert_eq!(millis(r5[0].start.date()), Some(1344604440000));
    let r6 = run(
        "casual",
        "In 5 minutes I will go home",
        at(1344600840000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("In 5 minutes I will go home", &r6[0]), 0);
    assert_eq!(r6[0].text, "In 5 minutes");
    assert_eq!(millis(r6[0].start.date()), Some(1344601140000));
    let r7 = run(
        "casual",
        "In 5 minutes A car need to move",
        at(1344600840000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("In 5 minutes A car need to move", &r7[0]), 0);
    assert_eq!(r7[0].text, "In 5 minutes");
    assert_eq!(millis(r7[0].start.date()), Some(1344601140000));
    let r8 = run(
        "casual",
        "In 5 seconds A car need to move",
        at(1344600840000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("In 5 seconds A car need to move", &r8[0]), 0);
    assert_eq!(r8[0].text, "In 5 seconds");
    assert_eq!(millis(r8[0].start.date()), Some(1344600845000));
    let r9 = run(
        "casual",
        "within half an hour",
        at(1344600840000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(index16("within half an hour", &r9[0]), 0);
    assert_eq!(r9[0].text, "within half an hour");
    assert_eq!(millis(r9[0].start.date()), Some(1344602640000));
    let r10 = run("casual", "within two weeks", at(1344600840000), options());
    assert_eq!(r10.len(), 1);
    assert_eq!(index16("within two weeks", &r10[0]), 0);
    assert_eq!(r10[0].text, "within two weeks");
    assert_eq!(millis(r10[0].start.date()), Some(1345810440000));
    let r11 = run("casual", "within a month", at(1344600840000), options());
    assert_eq!(r11.len(), 1);
    assert_eq!(index16("within a month", &r11[0]), 0);
    assert_eq!(r11[0].text, "within a month");
    assert_eq!(millis(r11[0].start.date()), Some(1347279240000));
    let r12 = run(
        "casual",
        "within a few months",
        at(1341922440000),
        options(),
    );
    assert_eq!(r12.len(), 1);
    assert_eq!(index16("within a few months", &r12[0]), 0);
    assert_eq!(r12[0].text, "within a few months");
    assert_eq!(millis(r12[0].start.date()), Some(1349871240000));
    let r13 = run("casual", "within one year", at(1344600840000), options());
    assert_eq!(r13.len(), 1);
    assert_eq!(index16("within one year", &r13[0]), 0);
    assert_eq!(r13[0].text, "within one year");
    assert_eq!(millis(r13[0].start.date()), Some(1376136840000));
    let r14 = run("casual", "within one Year", at(1344600840000), options());
    assert_eq!(r14.len(), 1);
    assert_eq!(index16("within one Year", &r14[0]), 0);
    assert_eq!(r14[0].text, "within one Year");
    assert_eq!(millis(r14[0].start.date()), Some(1376136840000));
    let r15 = run("casual", "within One year", at(1344600840000), options());
    assert_eq!(r15.len(), 1);
    assert_eq!(index16("within One year", &r15[0]), 0);
    assert_eq!(r15[0].text, "within One year");
    assert_eq!(millis(r15[0].start.date()), Some(1376136840000));
    let r16 = run(
        "casual",
        "In 5 Minutes A car need to move",
        at(1344600840000),
        options(),
    );
    assert_eq!(r16.len(), 1);
    assert_eq!(index16("In 5 Minutes A car need to move", &r16[0]), 0);
    assert_eq!(r16[0].text, "In 5 Minutes");
    assert_eq!(millis(r16[0].start.date()), Some(1344601140000));
    let r17 = run(
        "casual",
        "In 5 mins a car need to move",
        at(1344600840000),
        options(),
    );
    assert_eq!(r17.len(), 1);
    assert_eq!(index16("In 5 mins a car need to move", &r17[0]), 0);
    assert_eq!(r17[0].text, "In 5 mins");
    assert_eq!(millis(r17[0].start.date()), Some(1344601140000));
    let r18 = run("casual", "in a week", at(1475280000000), options());
    assert_eq!(r18.len(), 1);
    assert_eq!(r18[0].text, "in a week");
    assert_eq!(r18[0].start.get(Component::Year), Some(2016));
    assert_eq!(r18[0].start.get(Component::Month), Some(10));
    assert_eq!(r18[0].start.get(Component::Day), Some(8));
    let r19 = run("casual", "In around 5 hours", at(1475326800000), options());
    assert_eq!(r19.len(), 1);
    assert_eq!(r19[0].text, "In around 5 hours");
    assert_eq!(r19[0].start.get(Component::Year), Some(2016));
    assert_eq!(r19[0].start.get(Component::Month), Some(10));
    assert_eq!(r19[0].start.get(Component::Day), Some(1));
    assert_eq!(r19[0].start.get(Component::Hour), Some(18));
    let r20 = run("casual", "In about ~5 hours", at(1475326800000), options());
    assert_eq!(r20.len(), 1);
    assert_eq!(r20[0].start.get(Component::Year), Some(2016));
    assert_eq!(r20[0].start.get(Component::Month), Some(10));
    assert_eq!(r20[0].start.get(Component::Day), Some(1));
    assert_eq!(r20[0].start.get(Component::Hour), Some(18));
    let r21 = run("casual", "in 1 month", at(1475333520000), options());
    assert_eq!(r21.len(), 1);
    assert_eq!(r21[0].text, "in 1 month");
    assert_eq!(r21[0].start.get(Component::Year), Some(2016));
    assert_eq!(r21[0].start.get(Component::Month), Some(11));
    assert_eq!(r21[0].start.get(Component::Day), Some(1));
}

#[test]
fn the_within_expression_with_multiple_time_units() {
    let r0 = run(
        "casual",
        "set a timer for 5 minutes 30 seconds",
        at(1344600840000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("set a timer for 5 minutes 30 seconds", &r0[0]), 12);
    assert_eq!(r0[0].text, "for 5 minutes 30 seconds");
    assert_eq!(millis(r0[0].start.date()), Some(1344601170000));
    let r1 = run(
        "casual",
        "set a timer for 5 minutes, 30 seconds",
        at(1344600840000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("set a timer for 5 minutes, 30 seconds", &r1[0]), 12);
    assert_eq!(r1[0].text, "for 5 minutes, 30 seconds");
    assert_eq!(millis(r1[0].start.date()), Some(1344601170000));
    let r2 = run(
        "casual",
        "set a timer for 1 hour, 5 minutes, 30 seconds",
        at(1344600840000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(
        index16("set a timer for 1 hour, 5 minutes, 30 seconds", &r2[0]),
        12
    );
    assert_eq!(r2[0].text, "for 1 hour, 5 minutes, 30 seconds");
    assert_eq!(millis(r2[0].start.date()), Some(1344604770000));
    let r3 = run(
        "casual",
        "set a timer for 5 minutes and 30 seconds",
        at(1344600840000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(
        index16("set a timer for 5 minutes and 30 seconds", &r3[0]),
        12
    );
    assert_eq!(r3[0].text, "for 5 minutes and 30 seconds");
    assert_eq!(millis(r3[0].start.date()), Some(1344601170000));
    let r4 = run(
        "casual",
        "set a timer for 1 hour, 5 minutes, and 30 seconds",
        at(1344600840000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(
        index16("set a timer for 1 hour, 5 minutes, and 30 seconds", &r4[0]),
        12
    );
    assert_eq!(r4[0].text, "for 1 hour, 5 minutes, and 30 seconds");
    assert_eq!(millis(r4[0].start.date()), Some(1344604770000));
}

#[test]
fn the_within_expression_with_certain_keywords() {
    let r0 = run("casual", "In  about 5 hours", at(1344602940000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "In  about 5 hours");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(17));
    assert_eq!(r0[0].start.get(Component::Minute), Some(49));
    let r1 = run(
        "casual",
        "within around 3 hours",
        at(1344602940000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "within around 3 hours");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(r1[0].start.get(Component::Hour), Some(15));
    assert_eq!(r1[0].start.get(Component::Minute), Some(49));
    let r2 = run("casual", "In several hours", at(1344602940000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "In several hours");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(10));
    assert_eq!(r2[0].start.get(Component::Hour), Some(19));
    assert_eq!(r2[0].start.get(Component::Minute), Some(49));
    let r3 = run(
        "casual",
        "In a couple of days",
        at(1344602940000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "In a couple of days");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(12));
    assert_eq!(r3[0].start.get(Component::Hour), Some(12));
    assert_eq!(r3[0].start.get(Component::Minute), Some(49));
}

#[test]
fn single_expression_implied() {
    let r0 = run("casual", "within 30 days", at(1344600840000), options());
    assert_eq!(r0.len(), 1);
}

#[test]
fn implied_time_values() {
    let r0 = run("casual", "in 24 hours", at(1594383240000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(r0[0].start.get(Component::Minute), Some(14));
    assert_eq!(r0[0].start.get(Component::Day), Some(11));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Year), Some(2020));
    let r1 = run("casual", "in one day", at(1594383240000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(r1[0].start.get(Component::Minute), Some(14));
    assert_eq!(r1[0].start.get(Component::Day), Some(11));
    assert_eq!(r1[0].start.get(Component::Month), Some(7));
    assert_eq!(r1[0].start.get(Component::Year), Some(2020));
}

#[test]
fn time_units_certainty() {
    let r0 = run("casual", "in 2 minute", at(1475333520000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "in 2 minute");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(14));
    assert_eq!(r0[0].start.get(Component::Minute), Some(54));
    assert!(r0[0].start.is_certain(Component::Year));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Hour));
    assert!(r0[0].start.is_certain(Component::Minute));
    let r1 = run("casual", "in 2hour", at(1475333520000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "in 2hour");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(16));
    assert_eq!(r1[0].start.get(Component::Minute), Some(52));
    assert!(r1[0].start.is_certain(Component::Year));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(r1[0].start.is_certain(Component::Day));
    assert!(r1[0].start.is_certain(Component::Hour));
    assert!(r1[0].start.is_certain(Component::Minute));
    let r2 = run("casual", "in a few year", at(1475333520000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "in a few year");
    assert_eq!(r2[0].start.get(Component::Year), Some(2019));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(14));
    assert_eq!(r2[0].start.get(Component::Minute), Some(52));
    assert!(!r2[0].start.is_certain(Component::Month));
    assert!(!r2[0].start.is_certain(Component::Day));
    assert!(!r2[0].start.is_certain(Component::Hour));
    assert!(!r2[0].start.is_certain(Component::Minute));
    let r3 = run("casual", "within 12 month", at(1475333520000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "within 12 month");
    assert_eq!(r3[0].start.get(Component::Year), Some(2017));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(14));
    assert_eq!(r3[0].start.get(Component::Minute), Some(52));
    assert!(r3[0].start.is_certain(Component::Year));
    assert!(r3[0].start.is_certain(Component::Month));
    assert!(!r3[0].start.is_certain(Component::Day));
    assert!(!r3[0].start.is_certain(Component::Hour));
    assert!(!r3[0].start.is_certain(Component::Minute));
    let r4 = run("casual", "within 3 days", at(1475333520000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "within 3 days");
    assert_eq!(r4[0].start.get(Component::Year), Some(2016));
    assert_eq!(r4[0].start.get(Component::Month), Some(10));
    assert_eq!(r4[0].start.get(Component::Day), Some(4));
    assert_eq!(r4[0].start.get(Component::Hour), Some(14));
    assert_eq!(r4[0].start.get(Component::Minute), Some(52));
    assert!(r4[0].start.is_certain(Component::Year));
    assert!(r4[0].start.is_certain(Component::Month));
    assert!(r4[0].start.is_certain(Component::Day));
    assert!(!r4[0].start.is_certain(Component::Hour));
    assert!(!r4[0].start.is_certain(Component::Minute));
    let r5 = run(
        "casual",
        "give it 2 months",
        at(1475333520000),
        options().forward(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "2 months");
    assert_eq!(r5[0].start.get(Component::Year), Some(2016));
    assert_eq!(r5[0].start.get(Component::Month), Some(12));
    assert_eq!(r5[0].start.get(Component::Day), Some(1));
    assert_eq!(r5[0].start.get(Component::Hour), Some(14));
    assert_eq!(r5[0].start.get(Component::Minute), Some(52));
    assert!(r5[0].start.is_certain(Component::Year));
    assert!(r5[0].start.is_certain(Component::Month));
    assert!(!r5[0].start.is_certain(Component::Day));
    assert!(!r5[0].start.is_certain(Component::Hour));
    assert!(!r5[0].start.is_certain(Component::Minute));
}

#[test]
fn strict_mode() {
    let r0 = run("casual", "in 2hour", at(1475333520000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Hour), Some(16));
    assert_eq!(r0[0].start.get(Component::Minute), Some(52));
    let r1 = run("strict", "in 15m", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "within 5hr", now(), options());
    assert_eq!(r2.len(), 0);
}

#[test]
fn forward_date_option() {
    let r0 = run("casual", "1 hour", at(1344600840000), options().forward());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "1 hour");
    assert_eq!(millis(r0[0].start.date()), Some(1344604440000));
    let r1 = run("casual", "1 month", at(1475333520000), options().forward());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "1 month");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    let r2 = run(
        "casual",
        "in 1 month",
        at(1475333520000),
        options().forward(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "in 1 month");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(11));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
}

#[test]
fn negative_cases() {
    let r0 = run("casual", "in am", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "in them", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "the second half", now(), options());
    assert_eq!(r2.len(), 0);
}
