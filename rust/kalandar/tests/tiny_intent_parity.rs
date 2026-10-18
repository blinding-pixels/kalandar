use kalandar::{route_capture_intent, CaptureIntent};
use serde::Deserialize;
use std::path::PathBuf;

const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Deserialize)]
struct Fixture {
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    text: String,
    kind: String,
    confidence: f64,
    candidates: Vec<FixtureCandidate>,
    warnings: Vec<FixtureWarning>,
    scores: FixtureScores,
    ambiguous: bool,
    top_two_gap: f64,
}

#[derive(Debug, Deserialize)]
struct FixtureCandidate {
    kind: String,
    score: f64,
}

#[derive(Debug, Deserialize)]
struct FixtureWarning {
    code: String,
    severity: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct FixtureScores {
    task: f64,
    calendar: f64,
    reminder: f64,
    none: f64,
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

fn mismatch(case: &FixtureCase, actual: &CaptureIntent) -> Option<String> {
    let route = &actual.route;
    let candidates_match = case.candidates.len() == actual.candidates.len()
        && case
            .candidates
            .iter()
            .zip(&actual.candidates)
            .all(|(expected, actual)| {
                expected.kind == actual.kind && close(expected.score, actual.score)
            });
    let warnings_match = case.warnings.len() == actual.warnings.len()
        && case
            .warnings
            .iter()
            .zip(&actual.warnings)
            .all(|(expected, actual)| {
                expected.code == actual.code
                    && expected.severity == actual.severity
                    && expected.message == actual.message
            });
    let scores_match = close(case.scores.task, route.task)
        && close(case.scores.calendar, route.calendar)
        && close(case.scores.reminder, route.reminder)
        && close(case.scores.none, route.none);

    (case.kind != actual.kind
        || !close(case.confidence, actual.confidence)
        || !candidates_match
        || !warnings_match
        || !scores_match
        || case.ambiguous != route.ambiguous
        || !close(case.top_two_gap, route.top_two_gap))
    .then(|| format!("{:?}: expected {case:?}, got {actual:?}", case.text))
}

#[test]
fn tiny_intent_routing_matches_javascript_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tiny_intent_parity.json");
    let fixture: Fixture = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("tiny intent parity fixture should be readable"),
    )
    .expect("tiny intent parity fixture should parse");
    assert!(!fixture.cases.is_empty());

    let mismatches = fixture
        .cases
        .iter()
        .filter_map(|case| mismatch(case, &route_capture_intent(&case.text)))
        .collect::<Vec<_>>();

    assert!(
        mismatches.is_empty(),
        "{} of {} cases disagree with the JS classifier:\n{}",
        mismatches.len(),
        fixture.cases.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
