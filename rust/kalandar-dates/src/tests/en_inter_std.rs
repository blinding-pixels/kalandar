use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run(
        "casual",
        "Let's finish this before this 2013-2-7.",
        at(1344384000000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2013));
    assert_eq!(r0[0].start.get(Component::Month), Some(2));
    assert_eq!(r0[0].start.get(Component::Day), Some(7));
    assert_eq!(millis(r0[0].start.date()), Some(1360238400000));
    let r1 = run(
        "casual",
        "1994-11-05T08:15:30-05:30",
        at(1344384000000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(1994));
    assert_eq!(r1[0].start.get(Component::Month), Some(11));
    assert_eq!(r1[0].start.get(Component::Day), Some(5));
    assert_eq!(r1[0].start.get(Component::Hour), Some(8));
    assert_eq!(r1[0].start.get(Component::Minute), Some(15));
    assert_eq!(r1[0].start.get(Component::Second), Some(30));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-330));
    assert_eq!(r1[0].text, "1994-11-05T08:15:30-05:30");
    assert_eq!(millis(r1[0].start.date()), Some(784043130000));
    let r2 = run(
        "casual",
        "1994-11-05T13:15:30Z",
        at(1344384000000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(1994));
    assert_eq!(r2[0].start.get(Component::Month), Some(11));
    assert_eq!(r2[0].start.get(Component::Day), Some(5));
    assert_eq!(r2[0].start.get(Component::Hour), Some(13));
    assert_eq!(r2[0].start.get(Component::Minute), Some(15));
    assert_eq!(r2[0].start.get(Component::Second), Some(30));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(r2[0].text, "1994-11-05T13:15:30Z");
    assert_eq!(millis(r2[0].start.date()), Some(784041330000));
    let r3 = run(
        "casual",
        "1994-11-05T13:15:30Z",
        at(1344384000000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(1994));
    assert_eq!(r3[0].start.get(Component::Month), Some(11));
    assert_eq!(r3[0].start.get(Component::Day), Some(5));
    assert_eq!(r3[0].start.get(Component::Hour), Some(13));
    assert_eq!(r3[0].start.get(Component::Minute), Some(15));
    assert_eq!(r3[0].start.get(Component::Second), Some(30));
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(r3[0].text, "1994-11-05T13:15:30Z");
    assert_eq!(millis(r3[0].start.date()), Some(784041330000));
    let r4 = run(
        "casual",
        "- 1994-11-05T13:15:30Z",
        at(1344384000000),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].start.get(Component::Year), Some(1994));
    assert_eq!(r4[0].start.get(Component::Month), Some(11));
    assert_eq!(r4[0].start.get(Component::Day), Some(5));
    assert_eq!(r4[0].start.get(Component::Hour), Some(13));
    assert_eq!(r4[0].start.get(Component::Minute), Some(15));
    assert_eq!(r4[0].start.get(Component::Second), Some(30));
    assert_eq!(r4[0].start.get(Component::TimezoneOffset), Some(0));
    assert_eq!(index16("- 1994-11-05T13:15:30Z", &r4[0]), 2);
    assert_eq!(r4[0].text, "1994-11-05T13:15:30Z");
    assert_eq!(millis(r4[0].start.date()), Some(784041330000));
    let r5 = run(
        "strict",
        "2016-05-07T23:45:00.487+01:00",
        at(1344384000000),
        options(),
    );
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].start.get(Component::Year), Some(2016));
    assert_eq!(r5[0].start.get(Component::Month), Some(5));
    assert_eq!(r5[0].start.get(Component::Day), Some(7));
    assert_eq!(r5[0].start.get(Component::Hour), Some(23));
    assert_eq!(r5[0].start.get(Component::Minute), Some(45));
    assert_eq!(r5[0].start.get(Component::Second), Some(0));
    assert_eq!(r5[0].start.get(Component::TimezoneOffset), Some(60));
    assert_eq!(r5[0].text, "2016-05-07T23:45:00.487+01:00");
    assert_eq!(millis(r5[0].start.date()), Some(1462661100487));
}

#[test]
fn single_expression_with_local_timezeon() {
    let r0 = run(
        "casual",
        "1994-11-05T13:15:30",
        at(1344384000000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "1994-11-05T13:15:30");
    assert_eq!(r0[0].start.get(Component::Year), Some(1994));
    assert_eq!(r0[0].start.get(Component::Month), Some(11));
    assert_eq!(r0[0].start.get(Component::Day), Some(5));
    assert_eq!(r0[0].start.get(Component::Hour), Some(13));
    assert_eq!(r0[0].start.get(Component::Minute), Some(15));
    assert_eq!(r0[0].start.get(Component::Second), Some(30));
    assert_eq!(r0[0].start.get(Component::Millisecond), Some(0));
    assert!(!r0[0].start.is_certain(Component::TimezoneOffset));
    assert_eq!(millis(r0[0].start.date()), Some(784041330000));
    let r1 = run(
        "casual",
        "2015-07-31T12:00:00",
        at(1344384000000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2015-07-31T12:00:00");
    assert_eq!(r1[0].start.get(Component::Year), Some(2015));
    assert_eq!(r1[0].start.get(Component::Month), Some(7));
    assert_eq!(r1[0].start.get(Component::Day), Some(31));
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::Second), Some(0));
    assert_eq!(r1[0].start.get(Component::Millisecond), Some(0));
    assert!(!r1[0].start.is_certain(Component::TimezoneOffset));
    assert_eq!(millis(r1[0].start.date()), Some(1438344000000));
}
