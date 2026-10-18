use super::support::*;
use crate::types::Component;

#[test]
fn parsing_text_offset() {
    let r0 = run("casual", "  11 AM ", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(index16("  11 AM ", &r0[0]), 2);
    assert_eq!(r0[0].text, "11 AM");
    let r1 = run("casual", "2020 at  11 AM ", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(index16("2020 at  11 AM ", &r1[0]), 5);
    assert_eq!(r1[0].text, "at  11 AM");
}

#[test]
fn time_expression() {
    let r0 = run("casual", "20:32:13", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "20:32:13");
    assert_eq!(r0[0].start.get(Component::Hour), Some(20));
    assert_eq!(r0[0].start.get(Component::Minute), Some(32));
    assert_eq!(r0[0].start.get(Component::Second), Some(13));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    assert!(r0[0].tags().contains("parser/ENTimeExpressionParser"));
    assert!(r0[0].start.tags().contains("parser/ENTimeExpressionParser"));
}

#[test]
fn time_expression_with_clues() {
    let r0 = run("casual", "1 at night", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "1 at night");
    assert_eq!(r0[0].start.get(Component::Hour), Some(1));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
    let r1 = run("casual", "1 in the afternoon", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "1 in the afternoon");
    assert_eq!(r1[0].start.get(Component::Hour), Some(13));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(1));
}

#[test]
fn time_expression_after_date() {
    let r0 = run("casual", "05/31/2024 14:15", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "05/31/2024 14:15");
    assert_eq!(r0[0].start.get(Component::Year), Some(2024));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(31));
    assert_eq!(r0[0].start.get(Component::Hour), Some(14));
    assert_eq!(r0[0].start.get(Component::Minute), Some(15));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    assert!(r0[0].tags().contains("parser/ENTimeExpressionParser"));
    assert!(r0[0].start.tags().contains("parser/ENTimeExpressionParser"));
    let r1 = run("casual", "05/31/2024.14:15", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "05/31/2024.14:15");
    assert_eq!(r1[0].start.get(Component::Year), Some(2024));
    assert_eq!(r1[0].start.get(Component::Month), Some(5));
    assert_eq!(r1[0].start.get(Component::Day), Some(31));
    assert_eq!(r1[0].start.get(Component::Hour), Some(14));
    assert_eq!(r1[0].start.get(Component::Minute), Some(15));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(1));
    let r2 = run("casual", "05/31/2024:14:15", at(1475308800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "05/31/2024:14:15");
    assert_eq!(r2[0].start.get(Component::Year), Some(2024));
    assert_eq!(r2[0].start.get(Component::Month), Some(5));
    assert_eq!(r2[0].start.get(Component::Day), Some(31));
    assert_eq!(r2[0].start.get(Component::Hour), Some(14));
    assert_eq!(r2[0].start.get(Component::Minute), Some(15));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(1));
    let r3 = run("casual", "05/31/2024-14:15", at(1475308800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "05/31/2024-14:15");
    assert_eq!(r3[0].start.get(Component::Year), Some(2024));
    assert_eq!(r3[0].start.get(Component::Month), Some(5));
    assert_eq!(r3[0].start.get(Component::Day), Some(31));
    assert_eq!(r3[0].start.get(Component::Hour), Some(14));
    assert_eq!(r3[0].start.get(Component::Minute), Some(15));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(1));
}

#[test]
fn time_expression_before_date() {
    let r0 = run("casual", "14:15 05/31/2024", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "14:15 05/31/2024");
    assert_eq!(r0[0].start.get(Component::Year), Some(2024));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(31));
    assert_eq!(r0[0].start.get(Component::Hour), Some(14));
    assert_eq!(r0[0].start.get(Component::Minute), Some(15));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    let r1 = run("casual", "8:23 AM, Jul 9", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "8:23 AM, Jul 9");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(7));
    assert_eq!(r1[0].start.get(Component::Day), Some(9));
    assert_eq!(r1[0].start.get(Component::Hour), Some(8));
    assert_eq!(r1[0].start.get(Component::Minute), Some(23));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(0));
    let r2 = run("casual", "8:23 AM ∙ Jul 9", at(1475308800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "8:23 AM ∙ Jul 9");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(7));
    assert_eq!(r2[0].start.get(Component::Day), Some(9));
    assert_eq!(r2[0].start.get(Component::Hour), Some(8));
    assert_eq!(r2[0].start.get(Component::Minute), Some(23));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(0));
}

#[test]
fn time_range_expression() {
    let r0 = run(
        "casual",
        "10:00:00 - 21:45:00",
        at(1475308800000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "10:00:00 - 21:45:00");
    assert!(r0[0].tags().contains("parser/ENTimeExpressionParser"));
    assert_eq!(r0[0].start.get(Component::Hour), Some(10));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::Second), Some(0));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
    assert!(r0[0].start.tags().contains("parser/ENTimeExpressionParser"));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(21)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Minute),
        Some(45)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Second),
        Some(0)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
    assert!(r0[0]
        .end
        .as_ref()
        .expect("end")
        .tags()
        .contains("parser/ENTimeExpressionParser"));
    let r1 = run(
        "casual",
        "10:00:00 until 21:45:00",
        at(1475319600000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "10:00:00 until 21:45:00");
    assert_eq!(r1[0].start.get(Component::Hour), Some(10));
    assert_eq!(r1[0].start.get(Component::Minute), Some(0));
    assert_eq!(r1[0].start.get(Component::Second), Some(0));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(21)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Minute),
        Some(45)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Second),
        Some(0)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
    let r2 = run(
        "casual",
        "10:00:00 till 21:45:00",
        at(1475319600000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "10:00:00 till 21:45:00");
    assert_eq!(r2[0].start.get(Component::Hour), Some(10));
    assert_eq!(r2[0].start.get(Component::Minute), Some(0));
    assert_eq!(r2[0].start.get(Component::Second), Some(0));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Hour),
        Some(21)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Minute),
        Some(45)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Second),
        Some(0)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
    let r3 = run(
        "casual",
        "10:00:00 through 21:45:00",
        at(1475319600000),
        options(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "10:00:00 through 21:45:00");
    assert_eq!(r3[0].start.get(Component::Hour), Some(10));
    assert_eq!(r3[0].start.get(Component::Minute), Some(0));
    assert_eq!(r3[0].start.get(Component::Second), Some(0));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Hour),
        Some(21)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Minute),
        Some(45)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Second),
        Some(0)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
}

#[test]
fn time_non_range_expression() {
    let r0 = run("casual", "10:00:00 - 15/15", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "10:00:00");
    assert_eq!(r0[0].start.get(Component::Hour), Some(10));
    assert_eq!(r0[0].start.get(Component::Minute), Some(0));
    assert_eq!(r0[0].start.get(Component::Second), Some(0));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(0));
}

#[test]
fn casual_time_number_expression() {
    let r0 = run("casual", "11 at night", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "11 at night");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(23));
    let r1 = run("casual", "11 tonight", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "11 tonight");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(23));
    let r2 = run("casual", "6 in the morning", at(1475308800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "6 in the morning");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(6));
    assert_eq!(r2[0].start.get(Component::Minute), Some(0));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(0));
    let r3 = run("casual", "6 in the afternoon", at(1475308800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "6 in the afternoon");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(18));
    assert_eq!(r3[0].start.get(Component::Minute), Some(0));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(1));
}

#[test]
fn time_range_s_meridiem_handling() {
    let r0 = run("casual", "10 - 11 at night", at(1475308800000), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "10 - 11 at night");
    assert_eq!(r0[0].start.get(Component::Year), Some(2016));
    assert_eq!(r0[0].start.get(Component::Month), Some(10));
    assert_eq!(r0[0].start.get(Component::Day), Some(1));
    assert_eq!(r0[0].start.get(Component::Hour), Some(22));
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Day),
        Some(1)
    );
    assert_eq!(
        r0[0].end.as_ref().expect("end").get(Component::Hour),
        Some(23)
    );
    let r1 = run("casual", "8pm - 11", at(1475308800000), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "8pm - 11");
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(1));
    assert_eq!(r1[0].start.get(Component::Hour), Some(20));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Day),
        Some(1)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Hour),
        Some(23)
    );
    assert_eq!(
        r1[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
    let r2 = run("casual", "8 - 11pm", at(1475308800000), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "8 - 11pm");
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(1));
    assert_eq!(r2[0].start.get(Component::Hour), Some(20));
    assert_eq!(r2[0].start.get(Component::Meridiem), Some(1));
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(1)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Hour),
        Some(23)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(1)
    );
    let r3 = run("casual", "7 - 8", at(1475308800000), options());
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].text, "7 - 8");
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(1));
    assert_eq!(r3[0].start.get(Component::Hour), Some(7));
    assert_eq!(r3[0].start.get(Component::Meridiem), Some(0));
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Day),
        Some(1)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Hour),
        Some(8)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Meridiem),
        Some(0)
    );
}

#[test]
fn time_range_to_the_next_day() {
    let r0 = run(
        "casual",
        "December 31, 2022 10:00 pm - 1:00 am",
        at(1499385600000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Day), Some(31));
    assert_eq!(r0[0].start.get(Component::Month), Some(12));
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    assert_eq!(r0[0].start.get(Component::Hour), Some(22));
    assert_eq!(r0[0].start.get(Component::Meridiem), Some(1));
    let r1 = run(
        "casual",
        "December 31, 2022 10:00 pm - 12:00 am",
        at(1499385600000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Day), Some(31));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
    assert_eq!(r1[0].start.get(Component::Year), Some(2022));
    assert_eq!(r1[0].start.get(Component::Hour), Some(22));
    assert_eq!(r1[0].start.get(Component::Meridiem), Some(1));
}

#[test]
fn parsing_causal_positive_cases() {
    let r0 = run("casual", "at 1", now(), options());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "at 1");
    assert_eq!(r0[0].start.get(Component::Hour), Some(1));
    let r1 = run("casual", "at 12", now(), options());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "at 12");
    assert_eq!(r1[0].start.get(Component::Hour), Some(12));
    let r2 = run("casual", "at 12.30", now(), options());
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "at 12.30");
    assert_eq!(r2[0].start.get(Component::Hour), Some(12));
    assert_eq!(r2[0].start.get(Component::Minute), Some(30));
}

#[test]
fn parsing_negative_cases_year_like_pattern() {
    let r0 = run("casual", "2020", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "2020  ", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "2019 to 2020", now(), options());
    assert_eq!(r2.len(), 0);
}

#[test]
fn parsing_negative_cases_at_some_numbers() {
    let r0 = run("casual", "I'm at 101,194 points!", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "I'm at 101 points!", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("casual", "I'm at 10.1", now(), options());
    assert_eq!(r2.len(), 0);
}

#[test]
fn parsing_negative_cases_at_some_numbers_some_numbers() {
    let r0 = run("casual", "I'm at 10.1 - 10.12", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("casual", "I'm at 10 - 10.1", now(), options());
    assert_eq!(r1.len(), 0);
}

#[test]
fn parsing_negative_cases_strict() {
    let r0 = run("strict", "I'm at 101,194 points!", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict", "I'm at 101 points!", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "I'm at 10.1", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("strict", "I'm at 10", now(), options());
    assert_eq!(r3.len(), 0);
    let r4 = run("strict", "2020", now(), options());
    assert_eq!(r4.len(), 0);
}

#[test]
fn parsing_negative_cases_at_some_numbers_some_numbers_strict() {
    let r0 = run("strict", "I'm at 10.1 - 10.12", now(), options());
    assert_eq!(r0.len(), 0);
    let r1 = run("strict", "I'm at 10 - 10.1", now(), options());
    assert_eq!(r1.len(), 0);
    let r2 = run("strict", "I'm at 10 - 20", now(), options());
    assert_eq!(r2.len(), 0);
    let r3 = run("strict", "7-730", now(), options());
    assert_eq!(r3.len(), 0);
}

#[test]
fn forward_time_calculation_forwarddate_flag() {
    let r0 = run("casual", "1am", at(1653530220000), options().forward());
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(27));
    assert_eq!(r0[0].start.get(Component::Hour), Some(1));
    let r1 = run("casual", "11am", at(1475323200000), options().forward());
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].start.get(Component::Year), Some(2016));
    assert_eq!(r1[0].start.get(Component::Month), Some(10));
    assert_eq!(r1[0].start.get(Component::Day), Some(2));
    assert_eq!(r1[0].start.get(Component::Hour), Some(11));
    let r2 = run(
        "casual",
        "  11am to 1am  ",
        at(1475323200000),
        options().forward(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].start.get(Component::Year), Some(2016));
    assert_eq!(r2[0].start.get(Component::Month), Some(10));
    assert_eq!(r2[0].start.get(Component::Day), Some(2));
    assert_eq!(r2[0].start.get(Component::Hour), Some(11));
    assert!(r2[0].end.is_some());
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Day),
        Some(3)
    );
    assert_eq!(
        r2[0].end.as_ref().expect("end").get(Component::Hour),
        Some(1)
    );
    let r3 = run(
        "casual",
        "  10am to 12pm  ",
        at(1475319600000),
        options().forward(),
    );
    assert_eq!(r3.len(), 1);
    assert_eq!(r3[0].start.get(Component::Year), Some(2016));
    assert_eq!(r3[0].start.get(Component::Month), Some(10));
    assert_eq!(r3[0].start.get(Component::Day), Some(2));
    assert_eq!(r3[0].start.get(Component::Hour), Some(10));
    assert!(r3[0].end.is_some());
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Year),
        Some(2016)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Month),
        Some(10)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Day),
        Some(2)
    );
    assert_eq!(
        r3[0].end.as_ref().expect("end").get(Component::Hour),
        Some(12)
    );
}

#[test]
fn test_forward_time_calculation_forwarddate_flag_in_different_timezones() {
    let r0 = run(
        "casual",
        "1am",
        at(1653548220000).with_timezone_abbreviation("CDT"),
        options().forward(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    assert_eq!(r0[0].start.get(Component::Month), Some(5));
    assert_eq!(r0[0].start.get(Component::Day), Some(27));
    assert_eq!(r0[0].start.get(Component::Hour), Some(1));
}
