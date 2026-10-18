use super::support::*;

fn minutes_apart(actual: Option<i64>, expected: i64) -> i64 {
    (actual.expect("date") - expected).abs() / 60_000
}

#[test]
fn minimize_diff_because_of_local_timezone_pre_1900() {
    let cases = [
        ("1900-01-01T00:00:00-00:00", now(), -2_208_988_800_000, 60),
        ("1900-01-01T00:00:00-01:00", now(), -2_208_985_200_000, 120),
        ("1900-01-01T00:00:00+08:00", now(), -2_209_017_600_000, 120),
        (
            "1900-01-01T00:00",
            now().with_timezone_abbreviation("JST"),
            -2_209_017_600_000,
            120,
        ),
    ];
    for (text, reference, expected, tolerance) in cases {
        let results = run("casual", text, reference, options());
        assert_eq!(results.len(), 1, "{text}");
        assert!(
            minutes_apart(millis(results[0].date()), expected) <= tolerance,
            "{text}"
        );
    }
}
