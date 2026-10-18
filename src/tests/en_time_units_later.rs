use super::support::*;
use crate::types::Component;

#[test]
fn later_expression() {
    let r0 = run("casual", "2 days later", at(1344600000000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(12));
    assert_eq!(index16("2 days later", &r0[0]), 0);
    assert_eq!(r0[0].text, "2 days later");
    assert!(r0[0].tags().contains("result/relativeDate"));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert_eq!(millis(r0[0].start.date()), Some(1344772800000));
    let r1 = run("casual", "5 minutes later", at(1344592800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(r1[0].start.get(Component::Hour), Some(10));
    assert_eq!(r1[0].start.get(Component::Minute), Some(5));
    assert_eq!(index16("5 minutes later", &r1[0]), 0);
    assert_eq!(r1[0].text, "5 minutes later");
    assert!(r1[0].start.is_certain(Component::Hour));
    assert!(r1[0].start.is_certain(Component::Minute));
    assert_eq!(millis(r1[0].start.date()), Some(1344593100000));
    let r2 = run("casual", "3 week later", at(1341914400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(7));
    assert_eq!(r2[0].start.get(Component::Day), Some(31));
    assert_eq!(index16("3 week later", &r2[0]), 0);
    assert_eq!(r2[0].text, "3 week later");
    assert_eq!(millis(r2[0].start.date()), Some(1343728800000));
    let r3 = run("casual", "3w later", at(1341914400000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(7));
    assert_eq!(r3[0].start.get(Component::Day), Some(31));
    assert_eq!(index16("3w later", &r3[0]), 0);
    assert_eq!(r3[0].text, "3w later");
    let r4 = run("casual", "3mo later", at(1341914400000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Year), Some(2012));
    assert_eq!(r4[0].start.get(Component::Month), Some(10));
    assert_eq!(r4[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("3mo later", &r4[0]), 0);
    assert_eq!(r4[0].text, "3mo later");
}

#[test]
fn from_now_expression() {
    let r0 = run(
        "casual",
        "5 days from now, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(15));
    assert_eq!(index16("5 days from now, we did something", &r0[0]), 0);
    assert_eq!(r0[0].text, "5 days from now");
    assert_eq!(millis(r0[0].start.date()), Some(1344988800000));
    let r1 = run(
        "casual",
        "10 days from now, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(20));
    assert_eq!(index16("10 days from now, we did something", &r1[0]), 0);
    assert_eq!(r1[0].text, "10 days from now");
    assert_eq!(millis(r1[0].start.date()), Some(1345420800000));
    let r2 = run("casual", "15 minute from now", at(1344600840000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("15 minute from now", &r2[0]), 0);
    assert_eq!(r2[0].text, "15 minute from now");
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    assert_eq!(r2[0].start.get(Component::Minute), Some(29));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r2[0].start.date()), Some(1344601740000));
    let r3 = run("casual", "15 minutes earlier", at(1344600840000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("15 minutes earlier", &r3[0]), 0);
    assert_eq!(r3[0].text, "15 minutes earlier");
    assert_eq!(r3[0].start.get(Component::Hour), Some(11));
    assert_eq!(r3[0].start.get(Component::Minute), Some(59));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r3[0].start.date()), Some(1344599940000));
    let r4 = run("casual", "15 minute out", at(1344600840000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("15 minute out", &r4[0]), 0);
    assert_eq!(r4[0].text, "15 minute out");
    assert_eq!(r4[0].start.get(Component::Hour), Some(12));
    assert_eq!(r4[0].start.get(Component::Minute), Some(29));
    assert_eq!(r4[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r4[0].start.date()), Some(1344601740000));
    let r5 = run(
        "casual",
        "   12 hours from now",
        at(1344600840000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("   12 hours from now", &r5[0]), 3);
    assert_eq!(r5[0].text, "12 hours from now");
    assert_eq!(r5[0].start.get(Component::Day), Some(11));
    assert_eq!(r5[0].start.get(Component::Hour), Some(0));
    assert_eq!(r5[0].start.get(Component::Minute), Some(14));
    assert_eq!(millis(r5[0].start.date()), Some(1344644040000));
    let r6 = run("casual", "   12 hrs from now", at(1344600840000), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("   12 hrs from now", &r6[0]), 3);
    assert_eq!(r6[0].text, "12 hrs from now");
    assert_eq!(r6[0].start.get(Component::Day), Some(11));
    assert_eq!(r6[0].start.get(Component::Hour), Some(0));
    assert_eq!(r6[0].start.get(Component::Minute), Some(14));
    assert_eq!(r6[0].start.get(Component::Meridiem), Some(0));
    let r7 = run(
        "casual",
        "   half an hour from now",
        at(1344600840000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("   half an hour from now", &r7[0]), 3);
    assert_eq!(r7[0].text, "half an hour from now");
    assert_eq!(r7[0].start.get(Component::Hour), Some(12));
    assert_eq!(r7[0].start.get(Component::Minute), Some(44));
    assert_eq!(r7[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r7[0].start.date()), Some(1344602640000));
    let r8 = run(
        "casual",
        "12 hours from now I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("12 hours from now I did something", &r8[0]), 0);
    assert_eq!(r8[0].text, "12 hours from now");
    assert_eq!(r8[0].start.get(Component::Day), Some(11));
    assert_eq!(r8[0].start.get(Component::Hour), Some(0));
    assert_eq!(r8[0].start.get(Component::Minute), Some(14));
    assert_eq!(millis(r8[0].start.date()), Some(1344644040000));
    let r9 = run(
        "casual",
        "12 seconds from now I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(index16("12 seconds from now I did something", &r9[0]), 0);
    assert_eq!(r9[0].text, "12 seconds from now");
    assert_eq!(r9[0].start.get(Component::Hour), Some(12));
    assert_eq!(r9[0].start.get(Component::Minute), Some(14));
    assert_eq!(r9[0].start.get(Component::Second), Some(12));
    assert_eq!(millis(r9[0].start.date()), Some(1344600852000));
    let r10 = run(
        "casual",
        "three seconds from now I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r10.len(), 1);
    assert_eq!(
        index16("three seconds from now I did something", &r10[0]),
        0
    );
    assert_eq!(r10[0].text, "three seconds from now");
    assert_eq!(r10[0].start.get(Component::Hour), Some(12));
    assert_eq!(r10[0].start.get(Component::Minute), Some(14));
    assert_eq!(r10[0].start.get(Component::Second), Some(3));
    assert_eq!(millis(r10[0].start.date()), Some(1344600843000));
    let r11 = run(
        "casual",
        "5 Days from now, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r11.len(), 1);
    assert_eq!(r11[0].start.get(Component::Year), Some(2012));
    assert_eq!(r11[0].start.get(Component::Month), Some(8));
    assert_eq!(r11[0].start.get(Component::Day), Some(15));
    assert_eq!(index16("5 Days from now, we did something", &r11[0]), 0);
    assert_eq!(r11[0].text, "5 Days from now");
    assert_eq!(millis(r11[0].start.date()), Some(1344988800000));
    let r12 = run(
        "casual",
        "   half An hour from now",
        at(1344600840000),
        options(),
    );
    assert_eq!(r12.len(), 1);
    assert_eq!(index16("   half An hour from now", &r12[0]), 3);
    assert_eq!(r12[0].text, "half An hour from now");
    assert_eq!(r12[0].start.get(Component::Hour), Some(12));
    assert_eq!(r12[0].start.get(Component::Minute), Some(44));
    assert_eq!(millis(r12[0].start.date()), Some(1344602640000));
    let r13 = run(
        "casual",
        "A days from now, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r13.len(), 1);
    assert_eq!(r13[0].start.get(Component::Year), Some(2012));
    assert_eq!(r13[0].start.get(Component::Month), Some(8));
    assert_eq!(r13[0].start.get(Component::Day), Some(11));
    assert_eq!(index16("A days from now, we did something", &r13[0]), 0);
    assert_eq!(r13[0].text, "A days from now");
    assert_eq!(millis(r13[0].start.date()), Some(1344643200000));
    let r14 = run("casual", "a min out", at(1344600840000), options());
    assert_eq!(r14.len(), 1);
    assert_eq!(index16("a min out", &r14[0]), 0);
    assert_eq!(r14[0].text, "a min out");
    assert_eq!(r14[0].start.get(Component::Hour), Some(12));
    assert_eq!(r14[0].start.get(Component::Minute), Some(15));
    assert_eq!(millis(r14[0].start.date()), Some(1344600900000));
    let r15 = run("casual", "in 1 hour", at(1344600840000), options());
    assert_eq!(r15.len(), 1);
    assert_eq!(index16("in 1 hour", &r15[0]), 0);
    assert_eq!(r15[0].text, "in 1 hour");
    assert_eq!(r15[0].start.get(Component::Hour), Some(13));
    assert_eq!(r15[0].start.get(Component::Minute), Some(14));
    assert_eq!(millis(r15[0].start.date()), Some(1344604440000));
    let r16 = run("casual", "in 1 mon", at(1344600840000), options());
    assert_eq!(r16.len(), 1);
    assert_eq!(index16("in 1 mon", &r16[0]), 0);
    assert_eq!(r16[0].text, "in 1 mon");
    assert_eq!(r16[0].start.get(Component::Month), Some(9));
    assert_eq!(millis(r16[0].start.date()), Some(1347279240000));
    let r17 = run("casual", "in 1.5 hours", at(1344602400000), options());
    assert_eq!(r17.len(), 1);
    assert_eq!(index16("in 1.5 hours", &r17[0]), 0);
    assert_eq!(r17[0].text, "in 1.5 hours");
    assert_eq!(r17[0].start.get(Component::Hour), Some(14));
    assert_eq!(r17[0].start.get(Component::Minute), Some(10));
    assert_eq!(millis(r17[0].start.date()), Some(1344607800000));
}

#[test]
fn the_later_expression_with_multiple_time_units() {
    let r0 = run("casual", "in 1d 2hr 5min", at(1344602400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("in 1d 2hr 5min", &r0[0]), 0);
    assert_eq!(r0[0].text, "in 1d 2hr 5min");
    assert_eq!(r0[0].start.get(Component::Day), Some(11));
    assert_eq!(r0[0].start.get(Component::Hour), Some(14));
    assert_eq!(r0[0].start.get(Component::Minute), Some(45));
    assert_eq!(millis(r0[0].start.date()), Some(1344696300000));
    let r1 = run(
        "casual",
        "in 1d, 2hr, and 5min",
        at(1344602400000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("in 1d, 2hr, and 5min", &r1[0]), 0);
    assert_eq!(r1[0].text, "in 1d, 2hr, and 5min");
    assert_eq!(r1[0].start.get(Component::Day), Some(11));
    assert_eq!(r1[0].start.get(Component::Hour), Some(14));
    assert_eq!(r1[0].start.get(Component::Minute), Some(45));
    assert_eq!(millis(r1[0].start.date()), Some(1344696300000));
}

#[test]
fn strict_mode() {
    let r0 = run("casual", "the min after", at(1344600840000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("the min after", &r0[0]), 0);
    assert_eq!(r0[0].text, "the min after");
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(r0[0].start.get(Component::Minute), Some(15));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r0[0].start.date()), Some(1344600900000));
    let r1 = run(
        "strict",
        "15 minutes from now",
        at(1344600840000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "15 minutes from now");
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(r1[0].start.get(Component::Minute), Some(29));
    assert_eq!(millis(r1[0].start.date()), Some(1344601740000));
    let r2 = run("strict", "25 minutes later", at(1344602400000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("25 minutes later", &r2[0]), 0);
    assert_eq!(r2[0].text, "25 minutes later");
    assert_eq!(r2[0].start.get(Component::Hour), Some(13));
    assert_eq!(r2[0].start.get(Component::Minute), Some(5));
    assert_eq!(millis(r2[0].start.date()), Some(1344603900000));
    let r3 = run("strict", "15m from now", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("strict", "15s later", now(), options());
    assert_eq!(r4.len(), 0);
}

#[test]
fn after_with_reference() {
    let r0 = run("casual", "2 day after today", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(12));
    let r1 = run(
        "casual",
        "the day after tomorrow",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(12));
    let r2 = run(
        "casual",
        "2 day after tomorrow",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2 day after tomorrow");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(13));
    let r3 = run(
        "casual",
        "a week after tomorrow",
        at(1344556800000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "a week after tomorrow");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(18));
}

#[test]
fn plus_after_reference() {
    let r0 = run(
        "casual",
        "next tuesday +10 days",
        at(1703808000000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2024));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(12));
    let r1 = run("casual", "2023-12-29 -10days", at(1703808000000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2023));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
    assert_eq!(r1[0].start.get(Component::Day), Some(19));
    let r2 = run("casual", "now + 40minutes", at(1703838600000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2023));
    assert_eq!(r2[0].start.get(Component::Month), Some(12));
    assert_eq!(r2[0].start.get(Component::Day), Some(29));
    assert_eq!(r2[0].start.get(Component::Hour), Some(9));
    assert_eq!(r2[0].start.get(Component::Minute), Some(10));
}

#[test]
fn negative_cases() {
    let r0 = run("casual", "tell them later", now(), options());
    assert_eq!(r0.len(), 0);
}
