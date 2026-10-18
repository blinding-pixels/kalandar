use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run(
        "casual",
        "She is getting married soon (July 2017).",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2017));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(
        index16("She is getting married soon (July 2017).", &r0[0]),
        29
    );
    assert_eq!(r0[0].text, "July 2017");
    assert_eq!(millis(r0[0].start.date()), Some(1498910400000));
    let r1 = run(
        "casual",
        "She is leaving in August.",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(index16("She is leaving in August.", &r1[0]), 18);
    assert_eq!(r1[0].text, "August");
    assert_eq!(millis(r1[0].start.date()), Some(1343822400000));
    let r2 = run(
        "casual",
        "I am arriving sometime in August, 2012, probably.",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(
        index16("I am arriving sometime in August, 2012, probably.", &r2[0]),
        26
    );
    assert_eq!(r2[0].text, "August, 2012");
    assert_eq!(millis(r2[0].start.date()), Some(1343822400000));
    let r3 = run("casual", "August 10, 2012", at(1344556800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("August 10, 2012", &r3[0]), 0);
    assert_eq!(r3[0].text, "August 10, 2012");
    assert_eq!(millis(r3[0].start.date()), Some(1344600000000));
    let r4 = run("casual", "Nov 12, 2011", at(1344556800000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Year), Some(2011));
    assert_eq!(r4[0].start.get(Component::Month), Some(11));
    assert_eq!(r4[0].start.get(Component::Day), Some(12));
    assert_eq!(index16("Nov 12, 2011", &r4[0]), 0);
    assert_eq!(r4[0].text, "Nov 12, 2011");
    assert_eq!(millis(r4[0].start.date()), Some(1321099200000));
    let r5 = run(
        "casual",
        "The Deadline is August 10",
        at(1344556800000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].start.get(Component::Year), Some(2012));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("The Deadline is August 10", &r5[0]), 16);
    assert_eq!(r5[0].text, "August 10");
    assert_eq!(millis(r5[0].start.date()), Some(1344600000000));
    let r6 = run(
        "casual",
        "The Deadline is August 10 2555 BE",
        at(1344556800000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("The Deadline is August 10 2555 BE", &r6[0]), 16);
    assert_eq!(r6[0].text, "August 10 2555 BE");
    assert_eq!(millis(r6[0].start.date()), Some(1344600000000));
    let r7 = run(
        "casual",
        "The Deadline is August 10, 345 BC",
        at(1344556800000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("The Deadline is August 10, 345 BC", &r7[0]), 16);
    assert_eq!(r7[0].text, "August 10, 345 BC");
    assert_eq!(millis(r7[0].start.date()), Some(-73035172800000));
    let r8 = run(
        "casual",
        "The Deadline is August 10, 8 AD",
        at(1344556800000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("The Deadline is August 10, 8 AD", &r8[0]), 16);
    assert_eq!(r8[0].text, "August 10, 8 AD");
    assert_eq!(millis(r8[0].start.date()), Some(-61895534400000));
    let r9 = run(
        "casual",
        "The Deadline is Tuesday, January 10",
        at(1344556800000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(r9[0].text, "Tuesday, January 10");
    assert_eq!(r9[0].start.get(Component::Year), Some(2013));
    assert_eq!(r9[0].start.get(Component::Month), Some(1));
    assert_eq!(r9[0].start.get(Component::Day), Some(10));
    assert_eq!(r9[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r9[0].start.date()), Some(1357819200000));
    let r10 = run("casual", "Sun, Mar. 6, 2016", at(1344556800000), options());
    assert_eq!(r10.len(), 1);
    assert_eq!(r10[0].start.get(Component::Year), Some(2016));
    assert_eq!(r10[0].start.get(Component::Month), Some(3));
    assert_eq!(r10[0].start.get(Component::Day), Some(6));
    let r11 = run("casual", "Sun, March 6, 2016", at(1344556800000), options());
    assert_eq!(r11.len(), 1);
    assert_eq!(r11[0].start.get(Component::Year), Some(2016));
    assert_eq!(r11[0].start.get(Component::Month), Some(3));
    assert_eq!(r11[0].start.get(Component::Day), Some(6));
    let r12 = run(
        "casual",
        "Sun., March 6, 2016",
        at(1344556800000),
        options(),
    );
    assert_eq!(r12.len(), 1);
    assert_eq!(r12[0].start.get(Component::Year), Some(2016));
    assert_eq!(r12[0].start.get(Component::Month), Some(3));
    assert_eq!(r12[0].start.get(Component::Day), Some(6));
    let r13 = run(
        "casual",
        "Sunday, March 6, 2016",
        at(1344556800000),
        options(),
    );
    assert_eq!(r13.len(), 1);
    assert_eq!(r13[0].start.get(Component::Year), Some(2016));
    assert_eq!(r13[0].start.get(Component::Month), Some(3));
    assert_eq!(r13[0].start.get(Component::Day), Some(6));
    let r14 = run(
        "casual",
        "Sunday, March 6, 2016",
        at(1344556800000),
        options(),
    );
    assert_eq!(r14.len(), 1);
    assert_eq!(r14[0].start.get(Component::Year), Some(2016));
    assert_eq!(r14[0].start.get(Component::Month), Some(3));
    assert_eq!(r14[0].start.get(Component::Day), Some(6));
    let r15 = run(
        "casual",
        "Sunday, March, 6th 2016",
        at(1344556800000),
        options(),
    );
    assert_eq!(r15.len(), 1);
    assert_eq!(r15[0].text, "Sunday, March, 6th 2016");
    assert_eq!(r15[0].start.get(Component::Year), Some(2016));
    assert_eq!(r15[0].start.get(Component::Month), Some(3));
    assert_eq!(r15[0].start.get(Component::Day), Some(6));
    let r16 = run(
        "casual",
        "Wed, Jan 20th, 2016             ",
        at(1344556800000),
        options(),
    );
    assert_eq!(r16.len(), 1);
    assert_eq!(r16[0].text, "Wed, Jan 20th, 2016");
    assert_eq!(r16[0].start.get(Component::Year), Some(2016));
    assert_eq!(r16[0].start.get(Component::Month), Some(1));
    assert_eq!(r16[0].start.get(Component::Day), Some(20));
    let r17 = run("casual", "Dec. 21", at(1344556800000), options());
    assert_eq!(r17.len(), 1);
    assert_eq!(r17[0].text, "Dec. 21");
    assert_eq!(r17[0].start.get(Component::Year), Some(2012));
    assert_eq!(r17[0].start.get(Component::Month), Some(12));
    assert_eq!(r17[0].start.get(Component::Day), Some(21));
}

#[test]
fn single_expression_with_separators() {
    let r0 = run("casual", "August-10, 2012", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    let r1 = run("casual", "August/10, 2012", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].start.date()), Some(1344600000000));
    let r2 = run("casual", "August/10/2012", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].start.date()), Some(1344600000000));
    let r3 = run("casual", "August-10-2012", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].start.date()), Some(1344600000000));
}

#[test]
fn range_expression() {
    let r0 = run(
        "casual",
        "August 10 - 22, 2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("August 10 - 22, 2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "August 10 - 22, 2012");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
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
        Some(22)
    );
    assert_eq!(
        millis(r0[0].end.as_ref().expect("end").date()),
        Some(1345636800000)
    );
    let r1 = run(
        "casual",
        "August 10 to 22, 2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("August 10 to 22, 2012", &r1[0]), 0);
    assert_eq!(r1[0].text, "August 10 to 22, 2012");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
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
        Some(22)
    );
    assert_eq!(
        millis(r1[0].end.as_ref().expect("end").date()),
        Some(1345636800000)
    );
    let r2 = run(
        "casual",
        "August 10 - November 12",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("August 10 - November 12", &r2[0]), 0);
    assert_eq!(r2[0].text, "August 10 - November 12");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r2[0].start.date()), Some(1344600000000));
    assert!(r2[0].end.is_some());
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2012)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(11)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r2[0].end.as_ref().expect("end").date()),
        Some(1352721600000)
    );
    let r3 = run("casual", "Aug 10 to Nov 12", at(1344556800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("Aug 10 to Nov 12", &r3[0]), 0);
    assert_eq!(r3[0].text, "Aug 10 to Nov 12");
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r3[0].start.date()), Some(1344600000000));
    assert!(r3[0].end.is_some());
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2012)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(11)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r3[0].end.as_ref().expect("end").date()),
        Some(1352721600000)
    );
    let r4 = run(
        "casual",
        "Aug 10 - Nov 12, 2013",
        at(1344556800000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("Aug 10 - Nov 12, 2013", &r4[0]), 0);
    assert_eq!(r4[0].text, "Aug 10 - Nov 12, 2013");
    assert_eq!(r4[0].start.get(Component::Year), Some(2013));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    assert_eq!(r4[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r4[0].start.date()), Some(1376136000000));
    assert!(r4[0].end.is_some());
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Year),
        Some(2013)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Month),
        Some(11)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r4[0].end.as_ref().expect("end").date()),
        Some(1384257600000)
    );
    let r5 = run(
        "casual",
        "Aug 10 - Nov 12, 2011",
        at(1344556800000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("Aug 10 - Nov 12, 2011", &r5[0]), 0);
    assert_eq!(r5[0].text, "Aug 10 - Nov 12, 2011");
    assert_eq!(r5[0].start.get(Component::Year), Some(2011));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r5[0].start.date()), Some(1312977600000));
    assert!(r5[0].end.is_some());
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Year),
        Some(2011)
    );
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Month),
        Some(11)
    );
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r5[0].end.as_ref().expect("end").date()),
        Some(1321099200000)
    );
}

#[test]
fn ordinal_words() {
    let r0 = run("casual", "May eighth, 2010", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("May eighth, 2010", &r0[0]), 0);
    assert_eq!(r0[0].text, "May eighth, 2010");
    assert_eq!(r0[0].start.get(Component::Year), Some(2010));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(8));
    let r1 = run("casual", "May twenty-fourth", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("May twenty-fourth", &r1[0]), 0);
    assert_eq!(r1[0].text, "May twenty-fourth");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    assert_eq!(r1[0].start.get(Component::Day), Some(24));
    let r2 = run(
        "casual",
        "May eighth - tenth, 2010",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("May eighth - tenth, 2010", &r2[0]), 0);
    assert_eq!(r2[0].text, "May eighth - tenth, 2010");
    assert_eq!(r2[0].start.get(Component::Year), Some(2010));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(r2[0].start.get(Component::Day), Some(8));
    assert!(r2[0].end.is_some());
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2010)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(10)
    );
}

#[test]
fn forward_option() {
    let r0 = run("casual", "January 1st", at(1455494400000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    let r1 = run(
        "casual",
        "January 1st",
        at(1455494400000),
        options().forward(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2017));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
}

#[test]
fn year_90_s_parsing() {
    let r0 = run("casual", "Aug 9, 96", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Aug 9, 96");
    assert_eq!(r0[0].start.get(Component::Year), Some(1996));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(9));
    let r1 = run("casual", "Aug 9 96", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Aug 9 96");
    assert_eq!(r1[0].start.get(Component::Year), Some(1996));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(9));
}

#[test]
fn skip_year_like_on_little_endian_configuration() {
    let r0 = run("casual", "Dec. 21", at(1704844800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2023));
    assert_eq!(r0[0].start.get(Component::Month), Some(12));
    assert_eq!(r0[0].start.get(Component::Day), Some(21));
    let r1 = run("gb", "Dec. 21", at(1704844800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2021));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
}

#[test]
fn impossible_dates_strict_mode() {
    let r0 = run("strict", "August 32, 2014", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict", "February 29, 2014", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "August 32", at(1344556800000), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("strict", "February 29", at(1407628800000), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("strict", "February 151998", at(1407628800000), options());
    assert_eq!(r4.len(), 0);
}
