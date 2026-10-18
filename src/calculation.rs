use crate::jsdate::JsDate;
use crate::results::{ParsedComponents, ParsedResult, ReferenceWithTimezone};
use crate::types::{Component, Duration, Meridiem, Timeunit, Weekday};

pub(crate) fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

fn carry(duration: &mut Duration, unit: Timeunit, amount: f64) {
    let current = duration.get(unit).unwrap_or(0.0);
    duration.set(unit, current + amount);
}

pub(crate) fn add_duration(reference: JsDate, duration: &mut Duration) -> JsDate {
    let mut date = reference;
    if let Some(years) = duration.get(Timeunit::Year) {
        let floor = years.floor();
        date.set_full_year(f64::from(date.get_full_year()) + floor);
        let fraction = years - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Month, fraction * 12.0);
        }
    }
    if let Some(quarters) = duration.get(Timeunit::Quarter) {
        let floor = quarters.floor();
        date.set_month(f64::from(date.get_month()) + floor * 3.0);
    }
    if let Some(months) = duration.get(Timeunit::Month) {
        let floor = months.floor();
        date.set_month(f64::from(date.get_month()) + floor);
        let fraction = months - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Week, fraction * 4.0);
        }
    }
    if let Some(weeks) = duration.get(Timeunit::Week) {
        let floor = weeks.floor();
        date.set_date(f64::from(date.get_date()) + floor * 7.0);
        let fraction = weeks - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Day, js_round(fraction * 7.0));
        }
    }
    if let Some(days) = duration.get(Timeunit::Day) {
        let floor = days.floor();
        date.set_date(f64::from(date.get_date()) + floor);
        let fraction = days - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Hour, js_round(fraction * 24.0));
        }
    }
    if let Some(hours) = duration.get(Timeunit::Hour) {
        let floor = hours.floor();
        date.set_hours(f64::from(date.get_hours()) + floor);
        let fraction = hours - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Minute, js_round(fraction * 60.0));
        }
    }
    if let Some(minutes) = duration.get(Timeunit::Minute) {
        let floor = minutes.floor();
        date.set_minutes(f64::from(date.get_minutes()) + floor);
        let fraction = minutes - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Second, js_round(fraction * 60.0));
        }
    }
    if let Some(seconds) = duration.get(Timeunit::Second) {
        let floor = seconds.floor();
        date.set_seconds(f64::from(date.get_seconds()) + floor);
        let fraction = seconds - floor;
        if fraction > 0.0 {
            carry(duration, Timeunit::Millisecond, js_round(fraction * 1000.0));
        }
    }
    if let Some(ms) = duration.get(Timeunit::Millisecond) {
        let floor = ms.floor();
        date.set_milliseconds(f64::from(date.get_milliseconds()) + floor);
    }
    date
}

pub(crate) fn add_days(date: JsDate, days: f64) -> JsDate {
    add_duration(date, &mut Duration::new().with(Timeunit::Day, days))
}

pub(crate) fn add_years(date: JsDate, years: f64) -> JsDate {
    add_duration(date, &mut Duration::new().with(Timeunit::Year, years))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WeekdayModifier {
    This,
    Next,
    Last,
}

pub(crate) fn create_parsing_components_at_weekday(
    reference: ReferenceWithTimezone,
    weekday: i32,
    modifier: Option<WeekdayModifier>,
) -> ParsedComponents {
    let ref_date = reference.date_with_adjusted_timezone();
    let days_to_weekday = days_to_weekday(&ref_date, weekday, modifier);
    let mut components = ParsedComponents::new(reference);
    components
        .add_duration_as_implied(&Duration::new().with(Timeunit::Day, f64::from(days_to_weekday)));
    components.assign(Component::Weekday, weekday);
    components
}

pub(crate) fn days_to_weekday(
    ref_date: &JsDate,
    weekday: i32,
    modifier: Option<WeekdayModifier>,
) -> i32 {
    let ref_weekday = ref_date.get_day();
    let sunday = Weekday::Sunday as i32;
    let saturday = Weekday::Saturday as i32;
    match modifier {
        Some(WeekdayModifier::This) => days_forward_to_weekday(ref_date, weekday),
        Some(WeekdayModifier::Last) => backward_days_to_weekday(ref_date, weekday),
        Some(WeekdayModifier::Next) => {
            if ref_weekday == sunday {
                return if weekday == sunday { 7 } else { weekday };
            }
            if ref_weekday == saturday {
                if weekday == saturday {
                    return 7;
                }
                if weekday == sunday {
                    return 8;
                }
                return 1 + weekday;
            }
            if weekday < ref_weekday && weekday != sunday {
                days_forward_to_weekday(ref_date, weekday)
            } else {
                days_forward_to_weekday(ref_date, weekday) + 7
            }
        }
        None => days_to_weekday_closest(ref_date, weekday),
    }
}

pub(crate) fn days_to_weekday_closest(ref_date: &JsDate, weekday: i32) -> i32 {
    let backward = backward_days_to_weekday(ref_date, weekday);
    let forward = days_forward_to_weekday(ref_date, weekday);
    if forward < -backward {
        forward
    } else {
        backward
    }
}

pub(crate) fn days_forward_to_weekday(ref_date: &JsDate, weekday: i32) -> i32 {
    let mut forward = weekday - ref_date.get_day();
    if forward < 0 {
        forward += 7;
    }
    forward
}

pub(crate) fn backward_days_to_weekday(ref_date: &JsDate, weekday: i32) -> i32 {
    let mut backward = weekday - ref_date.get_day();
    if backward >= 0 {
        backward -= 7;
    }
    backward
}

pub(crate) fn find_most_likely_ad_year(year: i32) -> i32 {
    if year < 100 {
        if year > 50 {
            year + 1900
        } else {
            year + 2000
        }
    } else {
        year
    }
}

pub(crate) fn find_year_closest_to_ref(ref_date: JsDate, day: i32, month: i32) -> i32 {
    let mut date = ref_date;
    date.set_month(f64::from(month - 1));
    date.set_date(f64::from(day));
    let next_year = add_years(date, 1.0);
    let last_year = add_years(date, -1.0);
    let distance = |d: &JsDate| (d.time() - ref_date.time()).abs();
    if distance(&next_year) < distance(&date) {
        date = next_year;
    } else if distance(&last_year) < distance(&date) {
        date = last_year;
    }
    date.get_full_year()
}

pub(crate) fn merge_date_time_result(
    date_result: &ParsedResult,
    time_result: &ParsedResult,
) -> ParsedResult {
    let mut result = date_result.js_clone();
    let begin_date = &date_result.start;
    let begin_time = &time_result.start;

    result.start = merge_date_time_component(begin_date, begin_time);
    if date_result.end.is_some() || time_result.end.is_some() {
        let end_date = date_result.end.as_ref().unwrap_or(&date_result.start);
        let end_time = time_result.end.as_ref().unwrap_or(&time_result.start);
        let mut end_date_time = merge_date_time_component(end_date, end_time);

        if date_result.end.is_none()
            && end_date_time.js_date().time() < result.start.js_date().time()
        {
            let mut next_day = end_date_time.js_date();
            next_day.set_date(f64::from(next_day.get_date()) + 1.0);
            if end_date_time.is_certain(Component::Day) {
                end_date_time.assign_similar_date(&next_day);
            } else {
                end_date_time.imply_similar_date(&next_day);
            }
        }
        result.end = Some(end_date_time);
    }
    result
}

pub(crate) fn merge_date_time_component(
    date_component: &ParsedComponents,
    time_component: &ParsedComponents,
) -> ParsedComponents {
    let mut merged = date_component.js_clone();
    let time_value = |c: Component| time_component.value(c);

    if time_component.is_certain(Component::Hour) {
        merged.assign(Component::Hour, time_value(Component::Hour));
        merged.assign(Component::Minute, time_value(Component::Minute));
        if time_component.is_certain(Component::Second) {
            merged.assign(Component::Second, time_value(Component::Second));
            if time_component.is_certain(Component::Millisecond) {
                merged.assign(Component::Millisecond, time_value(Component::Millisecond));
            } else {
                merged.imply(Component::Millisecond, time_value(Component::Millisecond));
            }
        } else {
            merged.imply(Component::Second, time_value(Component::Second));
            merged.imply(Component::Millisecond, time_value(Component::Millisecond));
        }
    } else {
        merged.imply(Component::Hour, time_value(Component::Hour));
        merged.imply(Component::Minute, time_value(Component::Minute));
        merged.imply(Component::Second, time_value(Component::Second));
        merged.imply(Component::Millisecond, time_value(Component::Millisecond));
    }

    if time_component.is_certain(Component::TimezoneOffset) {
        merged.assign(
            Component::TimezoneOffset,
            time_value(Component::TimezoneOffset),
        );
    }

    let date_has_meaningful_meridiem = date_component.get(Component::Meridiem).is_some()
        && (date_component.is_certain(Component::Meridiem)
            || date_component
                .tags()
                .iter()
                .any(|t| t.starts_with("casualReference/")));

    if time_component.is_certain(Component::Meridiem) {
        merged.assign(Component::Meridiem, time_value(Component::Meridiem));
    } else if let Some(meridiem) = time_component.get(Component::Meridiem) {
        if !date_has_meaningful_meridiem {
            merged.imply(Component::Meridiem, meridiem);
        }
    }

    if merged.get(Component::Meridiem) == Some(Meridiem::Pm as i32)
        && merged.value(Component::Hour) < 12
    {
        let hour = merged.value(Component::Hour) + 12;
        if time_component.is_certain(Component::Hour) {
            merged.assign(Component::Hour, hour);
        } else {
            merged.imply(Component::Hour, hour);
        }
    }

    merged.add_tags(date_component.tags());
    merged.add_tags(time_component.tags());
    merged
}
