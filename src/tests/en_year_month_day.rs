use super::support::*;
use crate::types::Component;

#[test]
fn single_expression_yyyy_mm_dd() {
    let r0 = run("casual", "2012/8/10", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("2012/8/10", &r0[0]), 0);
    assert_eq!(r0[0].text, "2012/8/10");
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    let r1 = run(
        "casual",
        "The Deadline is 2012/8/10",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("The Deadline is 2012/8/10", &r1[0]), 16);
    assert_eq!(r1[0].text, "2012/8/10");
    assert_eq!(millis(r1[0].start.date()), Some(1344600000000));
    let r2 = run("strict", "2014/2/28", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2014/2/28");
    let r3 = run("strict", "2014/12/28", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "2014/12/28");
    assert_eq!(millis(r3[0].date()), Some(1419768000000));
    let r4 = run("strict", "2014.12.28", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "2014.12.28");
    assert_eq!(millis(r4[0].date()), Some(1419768000000));
    let r5 = run("strict", "2014 12 28", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "2014 12 28");
    assert_eq!(millis(r5[0].date()), Some(1419768000000));
}

#[test]
fn single_expression_with_month_name_yyyy_mmm_dd() {
    let r0 = run("casual", "2012/Aug/10", at(1344556800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2012));
    assert_eq!(r0[0].start.get(Component::Month), Some(8));
    assert_eq!(r0[0].start.get(Component::Day), Some(10));
    assert_eq!(index16("2012/Aug/10", &r0[0]), 0);
    assert_eq!(r0[0].text, "2012/Aug/10");
    assert_eq!(millis(r0[0].start.date()), Some(1344600000000));
    let r1 = run(
        "casual",
        "The Deadline is 2012/aug/10",
        at(1344556800000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("The Deadline is 2012/aug/10", &r1[0]), 16);
    assert_eq!(r1[0].text, "2012/aug/10");
    assert_eq!(millis(r1[0].start.date()), Some(1344600000000));
    let r2 = run(
        "casual",
        "The Deadline is 2018 March 18",
        at(1344556800000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(index16("The Deadline is 2018 March 18", &r2[0]), 16);
    assert_eq!(r2[0].text, "2018 March 18");
    assert_eq!(r2[0].start.get(Component::Year), Some(2018));
    assert_eq!(r2[0].start.get(Component::Month), Some(3));
    assert_eq!(r2[0].start.get(Component::Day), Some(18));
    assert_eq!(millis(r2[0].date()), Some(1521374400000));
}

#[test]
fn allow_swap_date_month_order_in_casual_mode() {
    let r0 = run("strict", "2024/13/1", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict", "2024-13-01", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "2024/13/1", at(1344556800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2024));
    assert_eq!(r2[0].start.get(Component::Month), Some(1));
    assert_eq!(r2[0].start.get(Component::Day), Some(13));
    let r3 = run("casual", "2024-13-01", at(1344556800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2024));
    assert_eq!(r3[0].start.get(Component::Month), Some(1));
    assert_eq!(r3[0].start.get(Component::Day), Some(13));
}

#[test]
fn not_parse_unlikely_xxxx_xx_xx_pattern() {
    let r0 = run("casual", "2012/80/10", at(1344556800000), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "2012 80 10", at(1344556800000), options());
    assert_eq!(r1.len(), 0);
}

#[test]
fn not_parse_impossible_dates() {
    let r0 = run("casual", "2014-08-32", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "2014-02-30", now(), options());
    assert_eq!(r1.len(), 0);
}
