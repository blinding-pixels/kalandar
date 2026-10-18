use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run(
        "casual",
        "5 days ago, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(5));
    assert_eq!(index16("5 days ago, we did something", &r0[0]), 0);
    assert_eq!(r0[0].text, "5 days ago");
    assert!(r0[0].tags().contains("result/relativeDate"));
    assert_eq!(millis(r0[0].start.date()), Some(1344124800000));
    let r1 = run(
        "casual",
        "10 days ago, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(7));
    assert_eq!(r1[0].start.get(Component::Day), Some(31));
    assert_eq!(index16("10 days ago, we did something", &r1[0]), 0);
    assert_eq!(r1[0].text, "10 days ago");
    assert_eq!(millis(r1[0].start.date()), Some(1343692800000));
    let r2 = run("casual", "15 minute ago", at(1344600840000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("15 minute ago", &r2[0]), 0);
    assert_eq!(r2[0].text, "15 minute ago");
    assert!(r2[0].tags().contains("result/relativeDate"));
    assert!(r2[0].tags().contains("result/relativeDateAndTime"));
    assert_eq!(r2[0].start.get(Component::Hour), Some(11));
    assert_eq!(r2[0].start.get(Component::Minute), Some(59));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r2[0].start.date()), Some(1344599940000));
    let r3 = run("casual", "15 minute earlier", at(1344600840000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(index16("15 minute earlier", &r3[0]), 0);
    assert_eq!(r3[0].text, "15 minute earlier");
    assert_eq!(r3[0].start.get(Component::Hour), Some(11));
    assert_eq!(r3[0].start.get(Component::Minute), Some(59));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r3[0].start.date()), Some(1344599940000));
    let r4 = run("casual", "15 minute before", at(1344600840000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(index16("15 minute before", &r4[0]), 0);
    assert_eq!(r4[0].text, "15 minute before");
    assert_eq!(r4[0].start.get(Component::Hour), Some(11));
    assert_eq!(r4[0].start.get(Component::Minute), Some(59));
    assert_eq!(r4[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r4[0].start.date()), Some(1344599940000));
    let r5 = run("casual", "   12 hours ago", at(1344600840000), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(index16("   12 hours ago", &r5[0]), 3);
    assert_eq!(r5[0].text, "12 hours ago");
    assert_eq!(r5[0].start.get(Component::Hour), Some(0));
    assert_eq!(r5[0].start.get(Component::Minute), Some(14));
    assert_eq!(r5[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r5[0].start.date()), Some(1344557640000));
    let r6 = run("casual", "1h ago", at(1344600840000), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(index16("1h ago", &r6[0]), 0);
    assert_eq!(r6[0].text, "1h ago");
    assert_eq!(r6[0].start.get(Component::Hour), Some(11));
    assert_eq!(r6[0].start.get(Component::Minute), Some(14));
    assert_eq!(r6[0].start.get(Component::Meridiem), Some(0));
    let r7 = run("casual", "1hr ago", at(1344600840000), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(index16("1hr ago", &r7[0]), 0);
    assert_eq!(r7[0].text, "1hr ago");
    assert_eq!(r7[0].start.get(Component::Hour), Some(11));
    assert_eq!(r7[0].start.get(Component::Minute), Some(14));
    assert_eq!(r7[0].start.get(Component::Meridiem), Some(0));
    let r8 = run(
        "casual",
        "   half an hour ago",
        at(1344600840000),
        options(),
    );
    assert_eq!(r8.len(), 1);
    assert_eq!(index16("   half an hour ago", &r8[0]), 3);
    assert_eq!(r8[0].text, "half an hour ago");
    assert_eq!(r8[0].start.get(Component::Hour), Some(11));
    assert_eq!(r8[0].start.get(Component::Minute), Some(44));
    assert_eq!(r8[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r8[0].start.date()), Some(1344599040000));
    let r9 = run(
        "casual",
        "12 hours ago I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r9.len(), 1);
    assert_eq!(index16("12 hours ago I did something", &r9[0]), 0);
    assert_eq!(r9[0].text, "12 hours ago");
    assert_eq!(r9[0].start.get(Component::Hour), Some(0));
    assert_eq!(r9[0].start.get(Component::Minute), Some(14));
    assert_eq!(r9[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r9[0].start.date()), Some(1344557640000));
    let r10 = run(
        "casual",
        "12 seconds ago I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r10.len(), 1);
    assert_eq!(index16("12 seconds ago I did something", &r10[0]), 0);
    assert_eq!(r10[0].text, "12 seconds ago");
    assert_eq!(r10[0].start.get(Component::Hour), Some(12));
    assert_eq!(r10[0].start.get(Component::Minute), Some(13));
    assert_eq!(r10[0].start.get(Component::Second), Some(48));
    assert_eq!(r10[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r10[0].start.date()), Some(1344600828000));
    let r11 = run(
        "casual",
        "three seconds ago I did something",
        at(1344600840000),
        options(),
    );
    assert_eq!(r11.len(), 1);
    assert_eq!(index16("three seconds ago I did something", &r11[0]), 0);
    assert_eq!(r11[0].text, "three seconds ago");
    assert_eq!(r11[0].start.get(Component::Hour), Some(12));
    assert_eq!(r11[0].start.get(Component::Minute), Some(13));
    assert_eq!(r11[0].start.get(Component::Second), Some(57));
    assert_eq!(r11[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r11[0].start.date()), Some(1344600837000));
    let r12 = run(
        "casual",
        "5 Days ago, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r12.len(), 1);
    assert_eq!(r12[0].start.get(Component::Year), Some(2012));
    assert_eq!(r12[0].start.get(Component::Month), Some(8));
    assert_eq!(r12[0].start.get(Component::Day), Some(5));
    assert_eq!(index16("5 Days ago, we did something", &r12[0]), 0);
    assert_eq!(r12[0].text, "5 Days ago");
    assert_eq!(millis(r12[0].start.date()), Some(1344124800000));
    let r13 = run(
        "casual",
        "   half An hour ago",
        at(1344600840000),
        options(),
    );
    assert_eq!(r13.len(), 1);
    assert_eq!(index16("   half An hour ago", &r13[0]), 3);
    assert_eq!(r13[0].text, "half An hour ago");
    assert_eq!(r13[0].start.get(Component::Hour), Some(11));
    assert_eq!(r13[0].start.get(Component::Minute), Some(44));
    assert_eq!(r13[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(millis(r13[0].start.date()), Some(1344599040000));
    let r14 = run(
        "casual",
        "A days ago, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r14.len(), 1);
    assert_eq!(r14[0].start.get(Component::Year), Some(2012));
    assert_eq!(r14[0].start.get(Component::Month), Some(8));
    assert_eq!(r14[0].start.get(Component::Day), Some(9));
    assert_eq!(index16("A days ago, we did something", &r14[0]), 0);
    assert_eq!(r14[0].text, "A days ago");
    assert_eq!(millis(r14[0].start.date()), Some(1344470400000));
    let r15 = run("casual", "a min before", at(1344600840000), options());
    assert_eq!(r15.len(), 1);
    assert_eq!(index16("a min before", &r15[0]), 0);
    assert_eq!(r15[0].text, "a min before");
    assert_eq!(r15[0].start.get(Component::Hour), Some(12));
    assert_eq!(r15[0].start.get(Component::Minute), Some(13));
    assert_eq!(r15[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r15[0].start.date()), Some(1344600780000));
    let r16 = run("casual", "the min before", at(1344600840000), options());
    assert_eq!(r16.len(), 1);
    assert_eq!(index16("the min before", &r16[0]), 0);
    assert_eq!(r16[0].text, "the min before");
    assert_eq!(r16[0].start.get(Component::Hour), Some(12));
    assert_eq!(r16[0].start.get(Component::Minute), Some(13));
    assert_eq!(r16[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(millis(r16[0].start.date()), Some(1344600780000));
}

#[test]
fn single_expression_casual() {
    let r0 = run(
        "casual",
        "5 months ago, we did something",
        at(1349827200000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("5 months ago, we did something", &r0[0]), 0);
    assert_eq!(r0[0].text, "5 months ago");
    assert_eq!(millis(r0[0].start.date()), Some(1336608000000));
    let r1 = run(
        "casual",
        "5 years ago, we did something",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2007));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("5 years ago, we did something", &r1[0]), 0);
    assert_eq!(r1[0].text, "5 years ago");
    assert_eq!(millis(r1[0].start.date()), Some(1186704000000));
    let r2 = run(
        "casual",
        "a week ago, we did something",
        at(1343952000000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(7));
    assert_eq!(r2[0].start.get(Component::Day), Some(27));
    assert_eq!(index16("a week ago, we did something", &r2[0]), 0);
    assert_eq!(r2[0].text, "a week ago");
    assert_eq!(millis(r2[0].start.date()), Some(1343347200000));
    let r3 = run(
        "casual",
        "a few days ago, we did something",
        at(1343952000000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(7));
    assert_eq!(r3[0].start.get(Component::Day), Some(31));
    assert_eq!(index16("a few days ago, we did something", &r3[0]), 0);
    assert_eq!(r3[0].text, "a few days ago");
    assert_eq!(millis(r3[0].start.date()), Some(1343692800000));
}

#[test]
fn nested_time_ago() {
    let r0 = run(
        "casual",
        "15 hours 29 min ago",
        at(1344637800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "15 hours 29 min ago");
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(r0[0].start.get(Component::Hour), Some(7));
    assert_eq!(r0[0].start.get(Component::Minute), Some(1));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
    let r1 = run(
        "casual",
        "1 day 21 hours ago ",
        at(1344637800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "1 day 21 hours ago");
    assert_eq!(r1[0].start.get(Component::Day), Some(9));
    assert_eq!(r1[0].start.get(Component::Hour), Some(1));
    assert_eq!(r1[0].start.get(Component::Minute), Some(30));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(0));
    let r2 = run("casual", "1d 21 h 25m ago ", at(1344637800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "1d 21 h 25m ago");
    assert_eq!(r2[0].start.get(Component::Day), Some(9));
    assert_eq!(r2[0].start.get(Component::Hour), Some(1));
    assert_eq!(r2[0].start.get(Component::Minute), Some(5));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(0));
    let r3 = run("casual", "3 min 49 sec ago ", at(1344637800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "3 min 49 sec ago");
    assert_eq!(r3[0].start.get(Component::Day), Some(10));
    assert_eq!(r3[0].start.get(Component::Hour), Some(22));
    assert_eq!(r3[0].start.get(Component::Minute), Some(26));
    assert_eq!(r3[0].start.get(Component::Second), Some(11));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(1));
    let r4 = run("casual", "3m 49s ago ", at(1344637800000), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "3m 49s ago");
    assert_eq!(r4[0].start.get(Component::Day), Some(10));
    assert_eq!(r4[0].start.get(Component::Hour), Some(22));
    assert_eq!(r4[0].start.get(Component::Minute), Some(26));
    assert_eq!(r4[0].start.get(Component::Second), Some(11));
    assert_eq!(r4[0].start.get(Component::Meridiem), Some(1));
}

#[test]
fn before_with_reference() {
    let r0 = run("casual", "2 day before today", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(8));
    let r1 = run(
        "casual",
        "the day before yesterday",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2012));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(8));
    let r2 = run(
        "casual",
        "2 day before yesterday",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2012));
    assert_eq!(r2[0].start.get(Component::Month), Some(8));
    assert_eq!(r2[0].start.get(Component::Day), Some(7));
    let r3 = run(
        "casual",
        "a week before yesterday",
        at(1344556800000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2012));
    assert_eq!(r3[0].start.get(Component::Month), Some(8));
    assert_eq!(r3[0].start.get(Component::Day), Some(2));
}

#[test]
fn strict_mode() {
    let r0 = run("strict", "5 minutes ago", at(1344600840000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Hour), Some(12));
    assert_eq!(r0[0].start.get(Component::Minute), Some(9));
    assert_eq!(millis(r0[0].start.date()), Some(1344600540000));
    let r1 = run("strict", "5m ago", at(1344600840000), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "5hr before", at(1344600840000), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("strict", "5 h ago", at(1344600840000), options());
    assert_eq!(r3.len(), 0);
}

#[test]
fn forward_date() {
    let r0 = run(
        "casual",
        "2 days ago",
        at(1725969600000),
        options().forward(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2024));
    assert_eq!(r0[0].start.get(Component::Month), Some(9));
    assert_eq!(r0[0].start.get(Component::Day), Some(8));
    let r1 = run(
        "casual",
        "2 weeks ago",
        at(1725969600000),
        options().forward(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2024));
    assert_eq!(r1[0].start.get(Component::Month), Some(8));
    assert_eq!(r1[0].start.get(Component::Day), Some(27));
    let r2 = run(
        "casual",
        "2 months ago",
        at(1725969600000),
        options().forward(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2024));
    assert_eq!(r2[0].start.get(Component::Month), Some(7));
    assert_eq!(r2[0].start.get(Component::Day), Some(10));
    let r3 = run(
        "casual",
        "2 years ago",
        at(1725969600000),
        options().forward(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2022));
    assert_eq!(r3[0].start.get(Component::Month), Some(9));
    assert_eq!(r3[0].start.get(Component::Day), Some(10));
}

#[test]
fn negative_cases() {
    let r0 = run("casual", "15 hours 29 min", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "a few hour", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "5 days", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("casual", "am ago", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("casual", "them ago", now(), options());
    assert_eq!(r4.len(), 0);
}
