use std::collections::BTreeSet;

use chrono::{DateTime, Utc};

use crate::calculation::add_duration;
use crate::jsdate::{JsDate, SystemZone};
use crate::timezone::to_timezone_offset;
use crate::types::{
    Component, Duration, Meridiem, ParsingOption, ParsingReference, Timeunit, TimezoneInput,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ReferenceWithTimezone {
    pub instant: JsDate,
    pub timezone_offset: Option<i32>,
}

impl ReferenceWithTimezone {
    pub fn from_date(date: JsDate) -> ReferenceWithTimezone {
        ReferenceWithTimezone {
            instant: date,
            timezone_offset: None,
        }
    }

    pub fn from_input(input: &ParsingReference, option: &ParsingOption) -> ReferenceWithTimezone {
        let instant = JsDate::from_instant(input.instant, input.system_zone);
        let timezone_offset = match &input.timezone {
            None => None,
            Some(TimezoneInput::Offset(minutes)) => Some(*minutes),
            Some(TimezoneInput::Abbreviation(abbr)) => {
                to_timezone_offset(abbr, Some(instant), &option.timezones)
            }
        };
        ReferenceWithTimezone {
            instant,
            timezone_offset,
        }
    }

    pub fn zone(&self) -> SystemZone {
        self.instant.zone()
    }

    pub fn date_with_adjusted_timezone(&self) -> JsDate {
        let mut date = self.instant;
        if self.timezone_offset.is_some() {
            let adjustment = self.system_timezone_adjustment_minute(&self.instant);
            date.set_minutes(f64::from(date.get_minutes()) - f64::from(adjustment));
        }
        date
    }

    fn system_timezone_adjustment_minute(&self, date: &JsDate) -> i32 {
        let current = -date.timezone_offset_minutes();
        let target = self.timezone_offset.unwrap_or(current);
        current - target
    }

    pub fn timezone_offset(&self) -> i32 {
        self.timezone_offset
            .unwrap_or_else(|| -self.instant.timezone_offset_minutes())
    }
}

#[derive(Clone, Debug)]
pub struct ParsedComponents {
    known: [Option<i32>; 10],
    implied: [Option<i32>; 10],
    reference: ReferenceWithTimezone,
    tags: BTreeSet<&'static str>,
}

impl ParsedComponents {
    pub(crate) fn new(reference: ReferenceWithTimezone) -> ParsedComponents {
        ParsedComponents::with_known(reference, &[])
    }

    pub(crate) fn with_known(
        reference: ReferenceWithTimezone,
        known: &[(Component, i32)],
    ) -> ParsedComponents {
        let mut components = ParsedComponents {
            known: [None; 10],
            implied: [None; 10],
            reference,
            tags: BTreeSet::new(),
        };
        for &(component, value) in known {
            components.known[component.slot()] = Some(value);
        }
        let date = reference.date_with_adjusted_timezone();
        components.imply(Component::Day, date.get_date());
        components.imply(Component::Month, date.get_month() + 1);
        components.imply(Component::Year, date.get_full_year());
        components.imply(Component::Hour, 12);
        components.imply(Component::Minute, 0);
        components.imply(Component::Second, 0);
        components.imply(Component::Millisecond, 0);
        components
    }

    pub(crate) fn create_relative_from_reference(
        reference: ReferenceWithTimezone,
        duration: &Duration,
    ) -> ParsedComponents {
        let mut duration = duration.clone();
        let date = add_duration(reference.date_with_adjusted_timezone(), &mut duration);

        let mut components = ParsedComponents::new(reference);
        components.add_tag("result/relativeDate");
        if duration.has_time_unit() {
            components.add_tag("result/relativeDateAndTime");
            components.assign_similar_time(&date);
            components.assign_similar_date(&date);
            components.assign(Component::TimezoneOffset, reference.timezone_offset());
        } else {
            components.imply_similar_time(&date);
            components.imply(Component::TimezoneOffset, reference.timezone_offset());

            if duration.has(Timeunit::Day) {
                components.assign(Component::Day, date.get_date());
                components.assign(Component::Month, date.get_month() + 1);
                components.assign(Component::Year, date.get_full_year());
                components.assign(Component::Weekday, date.get_day());
            } else if duration.has(Timeunit::Week) {
                components.assign(Component::Day, date.get_date());
                components.assign(Component::Month, date.get_month() + 1);
                components.assign(Component::Year, date.get_full_year());
                components.imply(Component::Weekday, date.get_day());
            } else {
                components.imply(Component::Day, date.get_date());
                if duration.has(Timeunit::Month) {
                    components.assign(Component::Month, date.get_month() + 1);
                    components.assign(Component::Year, date.get_full_year());
                } else {
                    components.imply(Component::Month, date.get_month() + 1);
                    if duration.has(Timeunit::Year) {
                        components.assign(Component::Year, date.get_full_year());
                    } else {
                        components.imply(Component::Year, date.get_full_year());
                    }
                }
            }
        }
        components
    }

    pub fn get(&self, component: Component) -> Option<i32> {
        self.known[component.slot()].or(self.implied[component.slot()])
    }

    pub fn is_certain(&self, component: Component) -> bool {
        self.known[component.slot()].is_some()
    }

    pub fn is_implied(&self, component: Component) -> bool {
        !self.is_certain(component) && self.implied[component.slot()].is_some()
    }

    pub fn certain_components(&self) -> Vec<Component> {
        Component::ALL
            .into_iter()
            .filter(|c| self.is_certain(*c))
            .collect()
    }

    pub fn tags(&self) -> &BTreeSet<&'static str> {
        &self.tags
    }

    pub fn date(&self) -> Option<DateTime<Utc>> {
        self.js_date().to_instant()
    }

    pub(crate) fn js_clone(&self) -> ParsedComponents {
        ParsedComponents {
            known: self.known,
            implied: self.implied,
            reference: self.reference,
            tags: BTreeSet::new(),
        }
    }

    pub(crate) fn value(&self, component: Component) -> i32 {
        self.get(component).unwrap_or(0)
    }

    pub(crate) fn imply(&mut self, component: Component, value: i32) -> &mut Self {
        if !self.is_certain(component) {
            self.implied[component.slot()] = Some(value);
        }
        self
    }

    pub(crate) fn assign(&mut self, component: Component, value: i32) -> &mut Self {
        self.known[component.slot()] = Some(value);
        self.implied[component.slot()] = None;
        self
    }

    pub(crate) fn delete(&mut self, components: &[Component]) {
        for component in components {
            self.known[component.slot()] = None;
            self.implied[component.slot()] = None;
        }
    }

    pub(crate) fn add_tag(&mut self, tag: &'static str) -> &mut Self {
        self.tags.insert(tag);
        self
    }

    pub(crate) fn add_tags(&mut self, tags: &BTreeSet<&'static str>) -> &mut Self {
        self.tags.extend(tags.iter().copied());
        self
    }

    pub(crate) fn add_duration_as_implied(&mut self, duration: &Duration) -> &mut Self {
        let mut duration = duration.clone();
        let current = self.date_without_timezone_adjustment();
        let date = add_duration(current, &mut duration);
        if duration.has(Timeunit::Day)
            || duration.has(Timeunit::Week)
            || duration.has(Timeunit::Month)
            || duration.has(Timeunit::Year)
        {
            self.delete(&[
                Component::Day,
                Component::Weekday,
                Component::Month,
                Component::Year,
            ]);
            self.imply(Component::Day, date.get_date());
            self.imply(Component::Weekday, date.get_day());
            self.imply(Component::Month, date.get_month() + 1);
            self.imply(Component::Year, date.get_full_year());
        }
        if duration.has(Timeunit::Second)
            || duration.has(Timeunit::Minute)
            || duration.has(Timeunit::Hour)
        {
            self.delete(&[Component::Second, Component::Minute, Component::Hour]);
            self.imply(Component::Second, date.get_seconds());
            self.imply(Component::Minute, date.get_minutes());
            self.imply(Component::Hour, date.get_hours());
        }
        self
    }

    pub(crate) fn is_only_date(&self) -> bool {
        !self.is_certain(Component::Hour)
            && !self.is_certain(Component::Minute)
            && !self.is_certain(Component::Second)
    }

    pub(crate) fn is_only_time(&self) -> bool {
        !self.is_certain(Component::Weekday)
            && !self.is_certain(Component::Day)
            && !self.is_certain(Component::Month)
            && !self.is_certain(Component::Year)
    }

    pub(crate) fn is_only_weekday_component(&self) -> bool {
        self.is_certain(Component::Weekday)
            && !self.is_certain(Component::Day)
            && !self.is_certain(Component::Month)
    }

    pub(crate) fn is_date_with_unknown_year(&self) -> bool {
        self.is_certain(Component::Month) && !self.is_certain(Component::Year)
    }

    fn utc_date_from_components(&self) -> JsDate {
        let mut date = JsDate::utc(
            self.reference.zone(),
            self.value(Component::Year),
            self.value(Component::Month) - 1,
            self.value(Component::Day),
            self.value(Component::Hour),
            self.value(Component::Minute),
            self.value(Component::Second),
            self.value(Component::Millisecond),
        );
        date.set_utc_full_year(f64::from(self.value(Component::Year)));
        date
    }

    pub(crate) fn is_valid_date(&self) -> bool {
        let date = self.utc_date_from_components();
        let year = self.get(Component::Year);
        let month = self.get(Component::Month).map(|m| m - 1);
        let day = self.get(Component::Day);
        if date.get_utc_full_year() != year || year.is_none() {
            return false;
        }
        if date.get_utc_month() != month || month.is_none() {
            return false;
        }
        if date.get_utc_date() != day || day.is_none() {
            return false;
        }
        if let Some(hour) = self.get(Component::Hour) {
            if date.get_utc_hours() != Some(hour) {
                return false;
            }
        }
        if let Some(minute) = self.get(Component::Minute) {
            if date.get_utc_minutes() != Some(minute) {
                return false;
            }
        }
        true
    }

    pub(crate) fn js_date(&self) -> JsDate {
        let offset = self
            .get(Component::TimezoneOffset)
            .or(self.reference.timezone_offset);
        match offset {
            None => self.date_without_timezone_adjustment(),
            Some(offset) => {
                let date = self.utc_date_from_components();
                date.plus_ms(-f64::from(offset) * 60_000.0)
            }
        }
    }

    fn date_without_timezone_adjustment(&self) -> JsDate {
        let mut date = JsDate::local(
            self.reference.zone(),
            self.value(Component::Year),
            self.value(Component::Month) - 1,
            self.value(Component::Day),
            self.value(Component::Hour),
            self.value(Component::Minute),
            self.value(Component::Second),
            self.value(Component::Millisecond),
        );
        date.set_full_year(f64::from(self.value(Component::Year)));
        date
    }

    pub(crate) fn assign_similar_date(&mut self, target: &JsDate) {
        self.assign(Component::Day, target.get_date());
        self.assign(Component::Month, target.get_month() + 1);
        self.assign(Component::Year, target.get_full_year());
    }

    pub(crate) fn assign_similar_time(&mut self, target: &JsDate) {
        self.assign(Component::Hour, target.get_hours());
        self.assign(Component::Minute, target.get_minutes());
        self.assign(Component::Second, target.get_seconds());
        self.assign(Component::Millisecond, target.get_milliseconds());
        self.assign(Component::Meridiem, meridiem_of(target.get_hours()));
    }

    pub(crate) fn imply_similar_date(&mut self, target: &JsDate) {
        self.imply(Component::Day, target.get_date());
        self.imply(Component::Month, target.get_month() + 1);
        self.imply(Component::Year, target.get_full_year());
    }

    pub(crate) fn imply_similar_time(&mut self, target: &JsDate) {
        self.imply(Component::Hour, target.get_hours());
        self.imply(Component::Minute, target.get_minutes());
        self.imply(Component::Second, target.get_seconds());
        self.imply(Component::Millisecond, target.get_milliseconds());
        self.imply(Component::Meridiem, meridiem_of(target.get_hours()));
    }
}

fn meridiem_of(hour: i32) -> i32 {
    if hour < 12 {
        Meridiem::Am as i32
    } else {
        Meridiem::Pm as i32
    }
}

#[derive(Clone, Debug)]
pub struct ParsedResult {
    pub index: usize,
    pub text: String,
    pub start: ParsedComponents,
    pub end: Option<ParsedComponents>,
    pub(crate) reference: ReferenceWithTimezone,
}

impl ParsedResult {
    pub(crate) fn new(
        reference: ReferenceWithTimezone,
        index: usize,
        text: impl Into<String>,
        start: Option<ParsedComponents>,
        end: Option<ParsedComponents>,
    ) -> ParsedResult {
        ParsedResult {
            index,
            text: text.into(),
            start: start.unwrap_or_else(|| ParsedComponents::new(reference)),
            end,
            reference,
        }
    }

    pub(crate) fn js_clone(&self) -> ParsedResult {
        ParsedResult {
            index: self.index,
            text: self.text.clone(),
            start: self.start.js_clone(),
            end: self.end.as_ref().map(ParsedComponents::js_clone),
            reference: self.reference,
        }
    }

    pub fn ref_date(&self) -> Option<DateTime<Utc>> {
        self.reference.instant.to_instant()
    }

    pub fn date(&self) -> Option<DateTime<Utc>> {
        self.start.date()
    }

    pub fn tags(&self) -> BTreeSet<&'static str> {
        let mut tags = self.start.tags().clone();
        if let Some(end) = &self.end {
            tags.extend(end.tags().iter().copied());
        }
        tags
    }

    pub(crate) fn add_tag(&mut self, tag: &'static str) -> &mut Self {
        self.start.add_tag(tag);
        if let Some(end) = &mut self.end {
            end.add_tag(tag);
        }
        self
    }

    pub(crate) fn end_index(&self) -> usize {
        self.index + self.text.len()
    }
}
