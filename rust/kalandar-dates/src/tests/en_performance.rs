use super::support::*;

#[test]
fn benchmarking_against_whitespace_backtracking() {
    let started = std::time::Instant::now();
    let r0 = run("casual", "BGR3                                                                                                                                                                                 186                                                days                                                                                                                                                                                                                                                                                18                                                hours                                                                                                                                                                                                                                                                                37                                                minutes                                                                                                                                                                                                                                                                                01                                                seconds", now(), options());
    assert_eq!(r0.len(), 0);
    let elapsed = started.elapsed().as_millis();
    assert!(elapsed < 1000);
}
