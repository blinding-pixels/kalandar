use super::support::*;
use crate::types::Component;
use crate::types::TimezoneAbbr;

#[test]
fn using_reference_with_timezone() {
    let r0 = run(
        "casual",
        "Friday at 4pm",
        at(1623240000000).with_timezone_abbreviation("CDT"),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1623445200000));
    assert_eq!(millis(r0[0].date()), Some(1623445200000));
    let r1 = run(
        "casual",
        "1am",
        at(1653548220000).with_timezone_abbreviation("CDT"),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    assert_eq!(r1[0].start.get(Component::Day), Some(26));
    assert_eq!(r1[0].start.get(Component::Hour), Some(1));
}

#[test]
fn using_reference_with_missing_default_timezone() {
    let r0 = run("casual", "Friday at 4pm", at(1623222000000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1623427200000));
    let r1 = run("casual", "Friday at 4pm", at(1623222000000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1623427200000));
    let r2 = run("casual", "Friday at 4pm", at(1623222000000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].date()), Some(1623427200000));
    let r3 = run(
        "casual",
        "Friday at 4pm",
        at(1623222000000).with_timezone_abbreviation(""),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].date()), Some(1623427200000));
}

#[test]
fn using_reference_instance_with_different_timezone() {
    let r0 = run(
        "casual",
        "At 4pm tomorrow",
        at(1622973600000).with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1623078000000));
    let r1 = run(
        "casual",
        "At 4pm tomorrow",
        at(1622973600000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1623049200000));
}

#[test]
fn using_reference_instance_with_custom_timezones() {
    let r0 = run(
        "casual",
        "At 4pm tomorrow",
        at(1622973600000).with_timezone_abbreviation("BBB"),
        options()
            .timezone("JJJ", TimezoneAbbr::Fixed(540))
            .timezone("BBB", TimezoneAbbr::Fixed(60)),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1623078000000));
    let r1 = run(
        "casual",
        "At 4pm tomorrow",
        at(1622973600000).with_timezone_abbreviation("JJJ"),
        options()
            .timezone("JJJ", TimezoneAbbr::Fixed(540))
            .timezone("BBB", TimezoneAbbr::Fixed(60)),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1623049200000));
}

#[test]
fn timezone_difference_on_reference_date_2() {
    let r0 = run(
        "casual",
        "yesterday 18:00",
        at(1708462800000).with_timezone_offset(780),
        options(),
    );
    assert_eq!(r0.len(), 1);
}

#[test]
fn timezone_difference_on_written_date() {
    let r0 = run(
        "casual",
        "Sun Jun 06 2021 19:00:00",
        now().with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1622973600000));
    let r1 = run(
        "casual",
        "Sun Jun 06 2021 11:00:00",
        now().with_timezone_abbreviation("BST"),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1622973600000));
    let r2 = run(
        "casual",
        "Sun Jun 06 2021 11:00:00",
        now().with_timezone_offset(60),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].date()), Some(1622973600000));
}

#[test]
fn precise_now_mentioned() {
    let r0 = run("casual", "now", at(1615612934000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1615612934000));
    let r1 = run("casual", "now", at(1615612934000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1615612934000));
    let r2 = run(
        "casual",
        "now",
        at(1615612934000).with_timezone_offset(540),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].date()), Some(1615612934000));
    let r3 = run(
        "casual",
        "now",
        at(1615612934000).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].date()), Some(1615612934000));
    let r4 = run(
        "casual",
        "now",
        at(1615612934000).with_timezone_offset(-300),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(millis(r4[0].date()), Some(1615612934000));
}

#[test]
fn precise_date_time_mentioned() {
    let r0 = run(
        "casual",
        "Sat Mar 13 2021 14:22:14 GMT+0900",
        at(1792328625089),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(millis(r0[0].date()), Some(1615612934000));
    let r1 = run(
        "casual",
        "Sat Mar 13 2021 14:22:14 GMT+0900",
        at(1792328625089),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(millis(r1[0].date()), Some(1615612934000));
    let r2 = run(
        "casual",
        "Sat Mar 13 2021 14:22:14 GMT+0900",
        at(1792328625089).with_timezone_offset(540),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(millis(r2[0].date()), Some(1615612934000));
    let r3 = run(
        "casual",
        "Sat Mar 13 2021 14:22:14 GMT+0900",
        at(1792328625089).with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(millis(r3[0].date()), Some(1615612934000));
    let r4 = run(
        "casual",
        "Sat Mar 13 2021 14:22:14 GMT+0900",
        at(1792328625089).with_timezone_offset(-300),
        options(),
    );
    assert_eq!(r4.len(), 1);
    assert_eq!(millis(r4[0].date()), Some(1615612934000));
}

#[test]
fn minimize_diff_because_of_local_timezone_pre_1900() {
    let r0 = run("casual", "1900-01-01T00:00:00-00:00", now(), options());
    assert_eq!(r0.len(), 1);
    let r1 = run("casual", "1900-01-01T00:00:00-01:00", now(), options());
    assert_eq!(r1.len(), 1);
    let r2 = run("casual", "1900-01-01T00:00:00+08:00", now(), options());
    assert_eq!(r2.len(), 1);
    let r3 = run(
        "casual",
        "1900-01-01T00:00",
        now().with_timezone_abbreviation("JST"),
        options(),
    );
    assert_eq!(r3.len(), 1);
}

#[test]
fn timezone_dst_transitions() {
    let r0 = run("casual", "2022-03-12 23:00 CT", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2022-03-12 23:00 CT");
    assert_eq!(r0[0].start.get(Component::Hour), Some(23));
    assert_eq!(r0[0].start.get(Component::TimezoneOffset), Some(-360));
    let r1 = run("casual", "2022-03-13 23:00 CT", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2022-03-13 23:00 CT");
    assert_eq!(r1[0].start.get(Component::Hour), Some(23));
    assert_eq!(r1[0].start.get(Component::TimezoneOffset), Some(-300));
    let r2 = run("casual", "2022-11-06 23:00 CT", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2022-11-06 23:00 CT");
    assert_eq!(r2[0].start.get(Component::Hour), Some(23));
    assert_eq!(r2[0].start.get(Component::TimezoneOffset), Some(-360));
    let r3 = run("casual", "2022-03-12 23:00 MT", now(), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "2022-03-12 23:00 MT");
    assert_eq!(r3[0].start.get(Component::Hour), Some(23));
    assert_eq!(r3[0].start.get(Component::TimezoneOffset), Some(-420));
    let r4 = run("casual", "2022-03-13 23:00 MT", now(), options());
    assert_eq!(r4.len(), 1);
    assert_eq!(r4[0].text, "2022-03-13 23:00 MT");
    assert_eq!(r4[0].start.get(Component::Hour), Some(23));
    assert_eq!(r4[0].start.get(Component::TimezoneOffset), Some(-360));
    let r5 = run("casual", "2022-11-06 23:00 MT", now(), options());
    assert_eq!(r5.len(), 1);
    assert_eq!(r5[0].text, "2022-11-06 23:00 MT");
    assert_eq!(r5[0].start.get(Component::Hour), Some(23));
    assert_eq!(r5[0].start.get(Component::TimezoneOffset), Some(-420));
    let r6 = run("casual", "2022-03-12 23:00 PT", now(), options());
    assert_eq!(r6.len(), 1);
    assert_eq!(r6[0].text, "2022-03-12 23:00 PT");
    assert_eq!(r6[0].start.get(Component::Hour), Some(23));
    assert_eq!(r6[0].start.get(Component::TimezoneOffset), Some(-480));
    let r7 = run("casual", "2022-03-13 23:00 PT", now(), options());
    assert_eq!(r7.len(), 1);
    assert_eq!(r7[0].text, "2022-03-13 23:00 PT");
    assert_eq!(r7[0].start.get(Component::Hour), Some(23));
    assert_eq!(r7[0].start.get(Component::TimezoneOffset), Some(-420));
    let r8 = run("casual", "2022-11-06 23:00 PT", now(), options());
    assert_eq!(r8.len(), 1);
    assert_eq!(r8[0].text, "2022-11-06 23:00 PT");
    assert_eq!(r8[0].start.get(Component::Hour), Some(23));
    assert_eq!(r8[0].start.get(Component::TimezoneOffset), Some(-480));
}
