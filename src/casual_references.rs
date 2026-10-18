use crate::results::{ParsedComponents, ReferenceWithTimezone};
use crate::types::{Component, Duration, Meridiem, Timeunit};

pub(crate) fn now(reference: ReferenceWithTimezone) -> ParsedComponents {
    let target = reference.date_with_adjusted_timezone();
    let mut component = ParsedComponents::new(reference);
    component.assign_similar_date(&target);
    component.assign_similar_time(&target);
    component.assign(Component::TimezoneOffset, reference.timezone_offset());
    component.add_tag("casualReference/now");
    component
}

pub(crate) fn today(reference: ReferenceWithTimezone) -> ParsedComponents {
    let target = reference.date_with_adjusted_timezone();
    let mut component = ParsedComponents::new(reference);
    component.assign_similar_date(&target);
    component.imply_similar_time(&target);
    component.delete(&[Component::Meridiem]);
    component.add_tag("casualReference/today");
    component
}

pub(crate) fn yesterday(reference: ReferenceWithTimezone) -> ParsedComponents {
    let mut component = the_day_after(reference, -1);
    component.add_tag("casualReference/yesterday");
    component
}

pub(crate) fn tomorrow(reference: ReferenceWithTimezone) -> ParsedComponents {
    let mut component = the_day_after(reference, 1);
    component.add_tag("casualReference/tomorrow");
    component
}

pub(crate) fn the_day_after(reference: ReferenceWithTimezone, days: i32) -> ParsedComponents {
    let mut target = reference.date_with_adjusted_timezone();
    let mut component = ParsedComponents::new(reference);
    target.set_date(f64::from(target.get_date()) + f64::from(days));
    component.assign_similar_date(&target);
    component.imply_similar_time(&target);
    component.delete(&[Component::Meridiem]);
    component
}

pub(crate) fn tonight(reference: ReferenceWithTimezone) -> ParsedComponents {
    let target = reference.date_with_adjusted_timezone();
    let mut component = ParsedComponents::new(reference);
    component.assign_similar_date(&target);
    component.imply(Component::Hour, 22);
    component.imply(Component::Meridiem, Meridiem::Pm as i32);
    component.add_tag("casualReference/tonight");
    component
}

pub(crate) fn evening(reference: ReferenceWithTimezone) -> ParsedComponents {
    let mut component = ParsedComponents::new(reference);
    component.imply(Component::Meridiem, Meridiem::Pm as i32);
    component.imply(Component::Hour, 20);
    component.add_tag("casualReference/evening");
    component
}

pub(crate) fn midnight(reference: ReferenceWithTimezone) -> ParsedComponents {
    let mut component = ParsedComponents::new(reference);
    if reference.date_with_adjusted_timezone().get_hours() > 2 {
        component.add_duration_as_implied(&Duration::new().with(Timeunit::Day, 1.0));
    }
    component.assign(Component::Hour, 0);
    component.imply(Component::Minute, 0);
    component.imply(Component::Second, 0);
    component.imply(Component::Millisecond, 0);
    component.add_tag("casualReference/midnight");
    component
}

fn part_of_day(
    reference: ReferenceWithTimezone,
    meridiem: Meridiem,
    hour: i32,
    tag: &'static str,
) -> ParsedComponents {
    let mut component = ParsedComponents::new(reference);
    component.imply(Component::Meridiem, meridiem as i32);
    component.imply(Component::Hour, hour);
    component.imply(Component::Minute, 0);
    component.imply(Component::Second, 0);
    component.imply(Component::Millisecond, 0);
    component.add_tag(tag);
    component
}

pub(crate) fn morning(reference: ReferenceWithTimezone) -> ParsedComponents {
    part_of_day(reference, Meridiem::Am, 6, "casualReference/morning")
}

pub(crate) fn afternoon(reference: ReferenceWithTimezone) -> ParsedComponents {
    part_of_day(reference, Meridiem::Pm, 15, "casualReference/afternoon")
}

pub(crate) fn noon(reference: ReferenceWithTimezone) -> ParsedComponents {
    let mut component = ParsedComponents::new(reference);
    component.imply(Component::Meridiem, Meridiem::Am as i32);
    component.assign(Component::Hour, 12);
    component.imply(Component::Minute, 0);
    component.imply(Component::Second, 0);
    component.imply(Component::Millisecond, 0);
    component.add_tag("casualReference/noon");
    component
}
