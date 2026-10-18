use super::support::*;
use crate::types::Component;

#[test]
fn date_time_expression() {
    let r0 = run(
        "casual",
        "Something happen on 2014-04-18 13:00 - 16:00 as",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2014-04-18 13:00 - 16:00");
    assert_eq!(millis(r0[0].start.date()), Some(1397826000000));
    assert_eq!(
        millis(r0[0].end.as_ref().expect("end").date()),
        Some(1397836800000)
    );
}

#[test]
fn time_expression() {
    let r0 = run(
        "casual",
        "between 3:30-4:30pm",
        at(1593993600000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "3:30-4:30pm");
    assert_eq!(millis(r0[0].start.date()), Some(1594049400000));
    assert_eq!(
        millis(r0[0].end.as_ref().expect("end").date()),
        Some(1594053000000)
    );
    let r1 = run("casual", "9:00 PST", at(1593993600000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "9:00 PST");
    assert_eq!(r1[0].start.get(Component::Hour), Some(9));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-480));
}

#[test]
fn quoted_expressions() {
    let r0 = run(
        "casual",
        "Want to meet for dinner (5pm EST)?",
        at(1593993600000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert!(r0[0].text.contains("5pm EST"));
    let r1 = run(
        "casual",
        "between '3:30-4:30pm'",
        at(1593993600000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert!(r1[0].text.contains("3:30-4:30pm"));
    assert_eq!(millis(r1[0].start.date()), Some(1594049400000));
    assert_eq!(
        millis(r1[0].end.as_ref().expect("end").date()),
        Some(1594053000000)
    );
    let r2 = run("casual", "The date is '2014-04-18'", now(), options());
    assert_eq!(r2.len(), 1);
    assert!(r2[0].text.contains("2014-04-18"));
    assert_eq!(millis(r2[0].start.date()), Some(1397822400000));
}

#[test]
fn strict_mode() {
    let r0 = run("strict", "Tuesday", now(), options());
    assert_eq!(r0.len(), 0);
}

#[test]
fn built_in_english_variants() {
    let r0 = run("casual", "6/10/2018", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].start.date()), Some(1528632000000));
    let r1 = run("gb", "6/10/2018", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].start.date()), Some(1538827200000));
}

#[test]
fn random_text() {
    let r0 = run(
        "casual",
        "Adam <Adam@supercalendar.com> написал(а):\nThe date is 02.07.2013",
        now(),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "02.07.2013");
    let r1 = run(
        "casual",
        "174 November 1,2001- March 31,2002",
        now(),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "November 1,2001- March 31,2002");
    let r2 = run(
        "casual",
        "...Thursday, December 15, 2011 Best Available Rate ",
        now(),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "Thursday, December 15, 2011");
    assert_eq!(r2[0].start.get(Component::Year), Some(2011));
    let r3 = run("casual", "SUN 15SEP 11:05 AM - 12:50 PM", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "SUN 15SEP 11:05 AM - 12:50 PM");
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Hour),
        Some(12)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Minute),
        Some(50)
    );
    let r4 = run(
        "casual",
        "FRI 13SEP 1:29 PM - FRI 13SEP 3:29 PM",
        now(),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "FRI 13SEP 1:29 PM - FRI 13SEP 3:29 PM");
    assert_eq!(r4[0].start.get(Component::Day), Some(13));
    assert_eq!(r4[0].start.get(Component::Hour), Some(13));
    assert_eq!(r4[0].start.get(Component::Minute), Some(29));
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Day),
        Some(13)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Hour),
        Some(15)
    );
    assert_eq!(
        r4[0].end.as_ref().expect("end").get(Component::Minute),
        Some(29)
    );
    let r5 = run(
        "casual",
        "9:00 AM to 5:00 PM, Tuesday, 20 May 2013",
        now(),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "9:00 AM to 5:00 PM, Tuesday, 20 May 2013");
    assert_eq!(millis(r5[0].start.date()), Some(1369040400000));
    assert_eq!(
        millis(r5[0].end.as_ref().expect("end").date()),
        Some(1369069200000)
    );
    let r6 = run(
        "casual",
        "Monday afternoon to last night",
        at(1499385600000),
        options(),
    );
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "Monday afternoon to last night");
    assert_eq!(r6[0].start.get(Component::Day), Some(3));
    assert_eq!(r6[0].start.get(Component::Month), Some(7));
    assert_eq!(
        r6[0].end.as_ref().expect("end").get(Component::Day),
        Some(7)
    );
    assert_eq!(
        r6[0].end.as_ref().expect("end").get(Component::Month),
        Some(7)
    );
    let r7 = run(
        "casual",
        "07-27-2022, 02:00 AM",
        at(1499385600000),
        options(),
    );
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "07-27-2022, 02:00 AM");
    assert_eq!(r7[0].start.get(Component::Day), Some(27));
    assert_eq!(r7[0].start.get(Component::Month), Some(7));
    assert_eq!(r7[0].start.get(Component::Year), Some(2022));
    assert_eq!(r7[0].start.get(Component::Hour), Some(2));
    assert_eq!(r7[0].start.get(Component::Meridiem), Some(0));
}

#[test]
fn wikipedia_texts() {
    let r0 = run("casual", "October 7, 2011, of which details were not revealed out of respect to Jobs's family.[239] Apple announced on the same day that they had no plans for a public service, but were encouraging \"well-wishers\" to send their remembrance messages to an email address created to receive such messages.[240] Sunday, October 16, 2011", at(1344556800000), options());
    assert_eq!(r0.len(), 2);
    assert_eq!(r0[0].start.get(Component::Year), Some(2011));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(7));
    assert_eq!(index16("October 7, 2011, of which details were not revealed out of respect to Jobs's family.[239] Apple announced on the same day that they had no plans for a public service, but were encouraging \"well-wishers\" to send their remembrance messages to an email address created to receive such messages.[240] Sunday, October 16, 2011", &r0[0]), 0);
    assert_eq!(r0[0].text, "October 7, 2011");
    assert_eq!(r0[1].start.get(Component::Year), Some(2011));
    assert_eq!(r0[1].start.get(Component::Month), Some(10));
    assert_eq!(r0[1].start.get(Component::Day), Some(16));
    assert_eq!(index16("October 7, 2011, of which details were not revealed out of respect to Jobs's family.[239] Apple announced on the same day that they had no plans for a public service, but were encouraging \"well-wishers\" to send their remembrance messages to an email address created to receive such messages.[240] Sunday, October 16, 2011", &r0[1]), 297);
    assert_eq!(r0[1].text, "Sunday, October 16, 2011");
}

#[test]
fn parse_multiple_date_results() {
    let r0 = run(
        "casual",
        "I will see you at 2:30. If not I will see you somewhere between 3:30-4:30pm",
        at(1593993600000),
        options(),
    );
    assert_eq!(r0.len(), 2);
    assert_eq!(r0[0].text, "at 2:30");
    assert_eq!(r0[0].start.get(Component::Year), Some(2020));
    assert_eq!(r0[0].start.get(Component::Month), Some(7));
    assert_eq!(r0[0].start.get(Component::Day), Some(6));
    assert_eq!(r0[0].start.get(Component::Hour), Some(2));
    assert!(r0[0].end.is_none());
    assert_eq!(r0[1].text, "3:30-4:30pm");
    assert_eq!(r0[1].start.get(Component::Hour), Some(15));
    assert_eq!(r0[1].start.get(Component::Minute), Some(30));
    assert_eq!(
        r0[1].end.as_ref().expect("end").get(Component::Hour),
        Some(16)
    );
    assert_eq!(
        r0[1].end.as_ref().expect("end").get(Component::Minute),
        Some(30)
    );
}

#[test]
fn customize_by_removing_time_extraction() {
    let _r0 = run("casual-timeExpression", "Thursday 9AM", now(), options());
    let r1 = run(
        "casual-timeExpression",
        "Thursday 9AM",
        at(1606608000000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "Thursday");
    assert_eq!(r1[0].start.get(Component::Year), Some(2020));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(26));
}
