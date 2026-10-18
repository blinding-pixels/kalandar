use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::jsdate::SystemZone;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Component {
    Year,
    Month,
    Day,
    Weekday,
    Hour,
    Minute,
    Second,
    Millisecond,
    Meridiem,
    TimezoneOffset,
}

impl Component {
    pub const ALL: [Component; 10] = [
        Component::Year,
        Component::Month,
        Component::Day,
        Component::Weekday,
        Component::Hour,
        Component::Minute,
        Component::Second,
        Component::Millisecond,
        Component::Meridiem,
        Component::TimezoneOffset,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Component::Year => "year",
            Component::Month => "month",
            Component::Day => "day",
            Component::Weekday => "weekday",
            Component::Hour => "hour",
            Component::Minute => "minute",
            Component::Second => "second",
            Component::Millisecond => "millisecond",
            Component::Meridiem => "meridiem",
            Component::TimezoneOffset => "timezoneOffset",
        }
    }

    pub fn from_name(name: &str) -> Option<Component> {
        Component::ALL.into_iter().find(|c| c.name() == name)
    }

    pub(crate) fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meridiem {
    Am = 0,
    Pm = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Timeunit {
    Year,
    Quarter,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond,
}

impl Timeunit {
    fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Duration {
    values: [Option<f64>; 9],
}

impl Duration {
    pub fn new() -> Duration {
        Duration::default()
    }

    pub fn with(mut self, unit: Timeunit, value: f64) -> Duration {
        self.set(unit, value);
        self
    }

    pub fn get(&self, unit: Timeunit) -> Option<f64> {
        self.values[unit.slot()]
    }

    pub fn has(&self, unit: Timeunit) -> bool {
        self.values[unit.slot()].is_some()
    }

    pub fn set(&mut self, unit: Timeunit, value: f64) {
        self.values[unit.slot()] = Some(value);
    }

    pub fn is_empty(&self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    pub fn reversed(&self) -> Duration {
        Duration {
            values: self.values.map(|v| v.map(|x| -x)),
        }
    }

    pub(crate) fn has_time_unit(&self) -> bool {
        self.has(Timeunit::Hour)
            || self.has(Timeunit::Minute)
            || self.has(Timeunit::Second)
            || self.has(Timeunit::Millisecond)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DstRule {
    NthWeekdayOfMonth {
        month: u32,
        weekday: Weekday,
        n: u32,
        hour: u32,
    },
    LastWeekdayOfMonth {
        month: u32,
        weekday: Weekday,
        hour: u32,
    },
    Custom(fn(i32) -> NaiveDateTime),
}

#[derive(Clone, Copy, Debug)]
pub enum TimezoneAbbr {
    Fixed(i32),
    Ambiguous {
        offset_during_dst: i32,
        offset_non_dst: i32,
        dst_start: DstRule,
        dst_end: DstRule,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimezoneInput {
    Offset(i32),
    Abbreviation(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsingReference {
    pub instant: DateTime<Utc>,
    pub timezone: Option<TimezoneInput>,
    pub system_zone: SystemZone,
}

impl ParsingReference {
    pub fn new(instant: DateTime<Utc>) -> ParsingReference {
        ParsingReference {
            instant,
            timezone: None,
            system_zone: SystemZone::Utc,
        }
    }

    pub fn with_timezone_offset(mut self, minutes: i32) -> ParsingReference {
        self.timezone = Some(TimezoneInput::Offset(minutes));
        self
    }

    pub fn with_timezone_abbreviation(mut self, abbreviation: &str) -> ParsingReference {
        self.timezone = Some(TimezoneInput::Abbreviation(abbreviation.to_string()));
        self
    }

    pub fn with_system_zone(mut self, zone: SystemZone) -> ParsingReference {
        self.system_zone = zone;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct ParsingOption {
    pub forward_date: bool,
    pub timezones: HashMap<String, TimezoneAbbr>,
}

impl ParsingOption {
    pub fn forward() -> ParsingOption {
        ParsingOption {
            forward_date: true,
            ..ParsingOption::default()
        }
    }
}
