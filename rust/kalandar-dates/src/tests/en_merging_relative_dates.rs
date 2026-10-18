use super::support::*;
use crate::types::Component;

#[test]
fn single_expression() {
    let r0 = run(
        "casual",
        "2 weeks after yesterday",
        at(1643760000000),
        options(),
    );
    assert_eq!(r0.len(), 1);
    assert_eq!(r0[0].text, "2 weeks after yesterday");
    assert_eq!(r0[0].start.get(Component::Year), Some(2022));
    assert_eq!(r0[0].start.get(Component::Month), Some(2));
    assert_eq!(r0[0].start.get(Component::Day), Some(15));
    assert_eq!(r0[0].start.get(Component::Weekday), Some(2));
    assert!(r0[0].start.is_certain(Component::Day));
    assert!(r0[0].start.is_certain(Component::Month));
    assert!(r0[0].start.is_certain(Component::Year));
    assert_eq!(millis(r0[0].start.date()), Some(1644883200000));
    let r1 = run(
        "casual",
        "2 months before 02/02",
        at(1643760000000),
        options(),
    );
    assert_eq!(r1.len(), 1);
    assert_eq!(r1[0].text, "2 months before 02/02");
    assert_eq!(r1[0].start.get(Component::Year), Some(2021));
    assert_eq!(r1[0].start.get(Component::Month), Some(12));
    assert_eq!(r1[0].start.get(Component::Day), Some(2));
    assert!(!r1[0].start.is_certain(Component::Day));
    assert!(r1[0].start.is_certain(Component::Month));
    assert!(r1[0].start.is_certain(Component::Year));
    assert_eq!(millis(r1[0].start.date()), Some(1638446400000));
    let r2 = run(
        "casual",
        "2 days after next Friday",
        at(1643760000000),
        options(),
    );
    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].text, "2 days after next Friday");
    assert_eq!(r2[0].start.get(Component::Year), Some(2022));
    assert_eq!(r2[0].start.get(Component::Month), Some(2));
    assert_eq!(r2[0].start.get(Component::Day), Some(13));
    assert!(r2[0].start.is_certain(Component::Day));
    assert!(r2[0].start.is_certain(Component::Month));
    assert!(r2[0].start.is_certain(Component::Year));
    assert_eq!(millis(r2[0].start.date()), Some(1644753600000));
}
