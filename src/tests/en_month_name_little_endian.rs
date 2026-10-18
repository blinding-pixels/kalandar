use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run("casual", "10 August 2012", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("10 August 2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "10 August 2012");
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    let r1 = run("casual", "3rd Feb 82", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(1982));
    assert_eq!(r1[0].start.get(Component::Month), Some(2));
    assert_eq!(r1[0].start.get(Component::Day), Some(3));
    assert_eq!(index16("3rd Feb 82", &r1[0]), 0);
    assert_eq!(r1[0].text, "3rd Feb 82");
    assert_eq!(millis(r1[0].start.date()), Some(381585600000));
    let r2 = run("casual", "Sun 15Sep", at(1376092800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("Sun 15Sep", &r2[0]), 0);
    assert_eq!(r2[0].text, "Sun 15Sep");
    assert_eq!(r2[0].start.get(Component::Year), Some(2013));
    assert_eq!(r2[0].start.get(Component::Month), Some(9));
    assert_eq!(r2[0].start.get(Component::Day), Some(15));
    assert_eq!(millis(r2[0].start.date()), Some(1379246400000));
    let r3 = run("casual", "SUN 15SEP", at(1376092800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("SUN 15SEP", &r3[0]), 0);
    assert_eq!(r3[0].text, "SUN 15SEP");
    assert_eq!(r3[0].start.get(Component::Year), Some(2013));
    assert_eq!(r3[0].start.get(Component::Month), Some(9));
    assert_eq!(r3[0].start.get(Component::Day), Some(15));
    assert_eq!(millis(r3[0].start.date()), Some(1379246400000));
    let r4 = run(
        "casual",
        "The Deadline is 10 August",
        at(1344556800000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("The Deadline is 10 August", &r4[0]), 16);
    assert_eq!(r4[0].text, "10 August");
    assert_eq!(r4[0].start.get(Component::Year), Some(2012));
    assert_eq!(r4[0].start.get(Component::Month), Some(8));
    assert_eq!(r4[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r4[0].start.date()), Some(1344600000000));
    let r5 = run(
        "casual",
        "The Deadline is Tuesday, 10 January",
        at(1344556800000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("The Deadline is Tuesday, 10 January", &r5[0]), 16);
    assert_eq!(r5[0].text, "Tuesday, 10 January");
    assert_eq!(r5[0].start.get(Component::Year), Some(2013));
    assert_eq!(r5[0].start.get(Component::Month), Some(1));
    assert_eq!(r5[0].start.get(Component::Day), Some(10));
    assert_eq!(r5[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r5[0].start.date()), Some(1357819200000));
    let r6 = run(
        "casual",
        "The Deadline is Tue, 10 January",
        at(1344556800000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("The Deadline is Tue, 10 January", &r6[0]), 16);
    assert_eq!(r6[0].text, "Tue, 10 January");
    assert_eq!(r6[0].start.get(Component::Year), Some(2013));
    assert_eq!(r6[0].start.get(Component::Month), Some(1));
    assert_eq!(r6[0].start.get(Component::Day), Some(10));
    assert_eq!(r6[0].start.get(Component::Weekday), Some(2));
    assert_eq!(millis(r6[0].start.date()), Some(1357819200000));
    let r7 = run("casual", "31st March, 2016", at(1344556800000), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("31st March, 2016", &r7[0]), 0);
    assert_eq!(r7[0].text, "31st March, 2016");
    assert_eq!(r7[0].start.get(Component::Year), Some(2016));
    assert_eq!(r7[0].start.get(Component::Month), Some(3));
    assert_eq!(r7[0].start.get(Component::Day), Some(31));
    assert_eq!(millis(r7[0].start.date()), Some(1459425600000));
    let r8 = run(
        "casual",
        "23rd february, 2016",
        at(1344556800000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("23rd february, 2016", &r8[0]), 0);
    assert_eq!(r8[0].text, "23rd february, 2016");
    assert_eq!(r8[0].start.get(Component::Year), Some(2016));
    assert_eq!(r8[0].start.get(Component::Month), Some(2));
    assert_eq!(r8[0].start.get(Component::Day), Some(23));
    assert_eq!(millis(r8[0].start.date()), Some(1456228800000));
}

#[test]
fn single_expression_with_separators() {
    let r0 = run("casual", "10-August 2012", at(1344384000000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "10-August 2012");
    assert_eq!(millis(r0[0].date()), Some(1344600000000));
    let r1 = run("casual", "10-August-2012", at(1344384000000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "10-August-2012");
    assert_eq!(millis(r1[0].date()), Some(1344600000000));
    let r2 = run("casual", "10/August 2012", at(1344384000000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "10/August 2012");
    assert_eq!(millis(r2[0].date()), Some(1344600000000));
    let r3 = run("casual", "10/August/2012", at(1344384000000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "10/August/2012");
    assert_eq!(millis(r3[0].date()), Some(1344600000000));
    let r4 = run("casual", "09-JAN-2017", at(1502150400000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "09-JAN-2017");
    assert_eq!(millis(r4[0].date()), Some(1483963200000));
    let r5 = run("casual", "09/JAN/2017", at(1502150400000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "09/JAN/2017");
    assert_eq!(millis(r5[0].date()), Some(1483963200000));
    let r6 = run("casual", "21-APR-16", at(1502150400000), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "21-APR-16");
    assert_eq!(millis(r6[0].date()), Some(1461240000000));
}

#[test]
fn range_expression() {
    let r0 = run(
        "casual",
        "10 - 22 August 2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("10 - 22 August 2012", &r0[0]), 0);
    assert_eq!(r0[0].text, "10 - 22 August 2012");
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
        "10 to 22 August 2012",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("10 to 22 August 2012", &r1[0]), 0);
    assert_eq!(r1[0].text, "10 to 22 August 2012");
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
        "10 August - 12 September",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("10 August - 12 September", &r2[0]), 0);
    assert_eq!(r2[0].text, "10 August - 12 September");
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
        Some(9)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r2[0].end.as_ref().expect("end").date()),
        Some(1347451200000)
    );
    let r3 = run(
        "casual",
        "10 August - 12 September 2013",
        at(1344556800000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("10 August - 12 September 2013", &r3[0]), 0);
    assert_eq!(r3[0].text, "10 August - 12 September 2013");
    assert_eq!(r3[0].start.get(Component::Year), Some(2013));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(10));
    assert_eq!(millis(r3[0].start.date()), Some(1376136000000));
    assert!(r3[0].end.is_some());
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2013)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(9)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r3[0].end.as_ref().expect("end").date()),
        Some(1378987200000)
    );
    let r4 = run(
        "casual",
        "10 August 2013 - 12 September",
        at(1344556800000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("10 August 2013 - 12 September", &r4[0]), 0);
    assert_eq!(r4[0].text, "10 August 2013 - 12 September");
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
        Some(9)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Day),
        Some(12)
    );
    assert_eq!(
        millis(r4[0].end.as_ref().expect("end").date()),
        Some(1378987200000)
    );
    let r5 = run(
        "casual",
        " 17 August 2013 to 19 August 2013",
        at(1344556800000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "17 August 2013 to 19 August 2013");
    assert_eq!(r5[0].start.get(Component::Year), Some(2013));
    assert_eq!(r5[0].start.get(Component::Month), Some(8));
    assert_eq!(r5[0].start.get(Component::Day), Some(17));
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Year),
        Some(2013)
    );
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r5[0].end.as_ref().expect("end").get(Component::Day),
        Some(19)
    );
}

#[test]
fn combined_expression() {
    let r0 = run(
        "casual",
        "12th of July at 19:00",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("12th of July at 19:00", &r0[0]), 0);
    assert_eq!(r0[0].text, "12th of July at 19:00");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Day), Some(12));
    assert_eq!(millis(r0[0].start.date()), Some(1342119600000));
    let r1 = run("casual", "5 May 12:00", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("5 May 12:00", &r1[0]), 0);
    assert_eq!(r1[0].text, "5 May 12:00");
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    assert_eq!(r1[0].start.get(Component::Day), Some(5));
    assert_eq!(millis(r1[0].start.date()), Some(1336219200000));
    let r2 = run("casual", "7 May 11:00", at(1344556800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("7 May 11:00", &r2[0]), 0);
    assert_eq!(r2[0].text, "7 May 11:00");
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(r2[0].start.get(Component::Day), Some(7));
    assert_eq!(r2[0].start.get(Component::Hour), Some(11));
    assert_eq!(millis(r2[0].start.date()), Some(1336388400000));
}

#[test]
fn ordinal_words() {
    let r0 = run(
        "casual",
        "Twenty-fourth of May",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "Twenty-fourth of May");
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(24));
    let r1 = run(
        "casual",
        "Eighth to eleventh May 2010",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Eighth to eleventh May 2010");
    assert_eq!(r1[0].start.get(Component::Year), Some(2010));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    assert_eq!(r1[0].start.get(Component::Day), Some(8));
    assert!(r1[0].end.is_some());
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2010)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(5)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(11)
    );
}

#[test]
fn little_endian_date_followed_by_time() {
    let r0 = run("casual", "24th October, 9 am", at(1499439600000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "24th October, 9 am");
    assert_eq!(r0[0].start.get(Component::Day), Some(24));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(9));
    let r1 = run("casual", "24th October, 9 pm", at(1499439600000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "24th October, 9 pm");
    assert_eq!(r1[0].start.get(Component::Day), Some(24));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Hour), Some(21));
    let r2 = run("casual", "24 October, 9 pm", at(1499439600000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "24 October, 9 pm");
    assert_eq!(r2[0].start.get(Component::Day), Some(24));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Hour), Some(21));
    let r3 = run("casual", "24 October, 9 p.m.", at(1499439600000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "24 October, 9 p.m.");
    assert_eq!(r3[0].start.get(Component::Day), Some(24));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Hour), Some(21));
    let r4 = run(
        "casual",
        "24 October 10 o clock",
        at(1499439600000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "24 October 10 o clock");
    assert_eq!(r4[0].start.get(Component::Day), Some(24));
    assert_eq!(r4[0].start.get(Component::Month), Some(10));
    assert_eq!(r4[0].start.get(Component::Hour), Some(10));
}

#[test]
fn year_90_s_parsing() {
    let r0 = run("casual", "03 Aug 96", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "03 Aug 96");
    assert_eq!(r0[0].start.get(Component::Year), Some(1996));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(3));
    let r1 = run("casual", "3 Aug 96", at(1344556800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "3 Aug 96");
    assert_eq!(r1[0].start.get(Component::Year), Some(1996));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(3));
    let r2 = run("casual", "9 Aug 96", at(1344556800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "9 Aug 96");
    assert_eq!(r2[0].start.get(Component::Year), Some(1996));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(9));
}

#[test]
fn forward_option() {
    let r0 = run("casual", "22-23 Feb at 7pm", at(1458000000000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(2));
    assert_eq!(r0[0].start.get(Component::Day), Some(22));
    assert_eq!(r0[0].start.get(Component::Hour), Some(19));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(2)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(23)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(19)
    );
    let r1 = run(
        "casual",
        "22-23 Feb at 7pm",
        at(1458000000000),
        options().forward(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2017));
    assert_eq!(r1[0].start.get(Component::Month), Some(2));
    assert_eq!(r1[0].start.get(Component::Day), Some(22));
    assert_eq!(r1[0].start.get(Component::Hour), Some(19));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2017)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(2)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(23)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(19)
    );
    let r2 = run(
        "casual",
        "17 August 2013 - 19 August 2013",
        now(),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2013));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(17));
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2013)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(8)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(19)
    );
}

#[test]
fn impossible_dates_strict_mode() {
    let r0 = run("strict", "32 August 2014", at(1344556800000), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict", "29 February 2014", at(1344556800000), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "32 August", at(1344556800000), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("strict", "29 February", at(1376092800000), options());
    assert_eq!(r3.len(), 0);
}

#[test]
fn year_3000_support_issue_636() {
    let r0 = run("casual", "Jan 1 3000, 9:30", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("Jan 1 3000, 9:30", &r0[0]), 0);
    assert_eq!(r0[0].text, "Jan 1 3000, 9:30");
    assert_eq!(r0[0].start.get(Component::Year), Some(3000));
    assert_eq!(r0[0].start.get(Component::Month), Some(1));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(9));
    assert_eq!(r0[0].start.get(Component::Minute), Some(30));
    assert_eq!(millis(r0[0].start.date()), Some(32503714200000));
    let r1 = run("casual", "Jan 1 2025, 9:30", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("Jan 1 2025, 9:30", &r1[0]), 0);
    assert_eq!(r1[0].text, "Jan 1 2025, 9:30");
    assert_eq!(r1[0].start.get(Component::Year), Some(2025));
    assert_eq!(r1[0].start.get(Component::Month), Some(1));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(9));
    assert_eq!(r1[0].start.get(Component::Minute), Some(30));
    assert_eq!(millis(r1[0].start.date()), Some(1735723800000));
}
