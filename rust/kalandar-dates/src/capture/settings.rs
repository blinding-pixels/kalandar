use chrono::{NaiveTime, Weekday};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Clock {
    TwelveHour,
    TwentyFourHour,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HolidaySetting {
    Off,
    #[default]
    Auto,
    Region {
        country: String,
        region: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub clock: Clock,
    pub week_start: Weekday,
    pub weekend: Vec<Weekday>,
    pub day_start: NaiveTime,
    pub morning: NaiveTime,
    pub afternoon: NaiveTime,
    pub evening: NaiveTime,
    pub tonight: NaiveTime,
    pub after_work: NaiveTime,
    pub end_of_day: NaiveTime,
    pub reminder_default: NaiveTime,
    pub breakfast: NaiveTime,
    pub brunch: NaiveTime,
    pub lunch: NaiveTime,
    pub afternoon_tea: NaiveTime,
    pub dinner: NaiveTime,
    pub drinks: NaiveTime,
    pub default_event_minutes: u32,
    pub day_first: bool,
    pub default_notification_time: NaiveTime,
    pub holidays: HolidaySetting,
    pub locale: Option<String>,
}

pub(crate) fn hm(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN)
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            clock: Clock::TwelveHour,
            week_start: Weekday::Mon,
            weekend: vec![Weekday::Sat, Weekday::Sun],
            day_start: hm(0, 0),
            morning: hm(9, 0),
            afternoon: hm(14, 0),
            evening: hm(19, 0),
            tonight: hm(20, 0),
            after_work: hm(18, 0),
            end_of_day: hm(17, 0),
            reminder_default: hm(9, 0),
            breakfast: hm(8, 0),
            brunch: hm(11, 0),
            lunch: hm(12, 0),
            afternoon_tea: hm(15, 0),
            dinner: hm(18, 0),
            drinks: hm(21, 0),
            default_event_minutes: 60,
            day_first: false,
            default_notification_time: hm(9, 0),
            holidays: HolidaySetting::Auto,
            locale: None,
        }
    }
}
