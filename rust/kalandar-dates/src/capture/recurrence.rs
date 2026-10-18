use std::sync::LazyLock;

use chrono::{Datelike, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use regex::Regex;
use serde::Serialize;

use crate::capture::holidays::{Holiday, HolidayScope, YearlyShape};
use crate::capture::lexicon::number_value;
use crate::capture::settings::Settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    pub fn rrule(self) -> &'static str {
        match self {
            Frequency::Minutely => "MINUTELY",
            Frequency::Hourly => "HOURLY",
            Frequency::Daily => "DAILY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Yearly => "YEARLY",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByDay {
    pub position: Option<i8>,
    pub weekday: Weekday,
}

impl ByDay {
    pub fn code(self) -> String {
        let day = weekday_code(self.weekday);
        match self.position {
            Some(p) => format!("{p}{day}"),
            None => day.to_string(),
        }
    }
}

impl Serialize for ByDay {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.code())
    }
}

pub fn weekday_code(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "MO",
        Weekday::Tue => "TU",
        Weekday::Wed => "WE",
        Weekday::Thu => "TH",
        Weekday::Fri => "FR",
        Weekday::Sat => "SA",
        Weekday::Sun => "SU",
    }
}

pub fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecurrenceException {
    pub raw: String,
    pub condition: Option<String>,
    pub action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReminderRecurrence {
    pub frequency: &'static str,
    pub interval: u32,
    pub days: Option<Vec<&'static str>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Rule {
    pub frequency: Frequency,
    pub interval: u32,
    pub weekdays: Vec<ByDay>,
    pub month_days: Vec<i8>,
    pub months: Vec<u32>,
    pub hours: Vec<u32>,
    pub window: Option<(NaiveTime, NaiveTime)>,
    pub count: Option<u32>,
    #[serde(skip)]
    pub span_days: Option<SpanLength>,
    #[serde(skip)]
    pub biweekly: bool,
    #[serde(skip)]
    pub vague: bool,
    #[serde(skip)]
    pub holiday: Option<Holiday>,
    pub time: Option<NaiveTime>,
    #[serde(skip)]
    pub time_is_bare: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanLength {
    Days(i64),
    Months(u32),
}

impl Rule {
    pub(crate) fn new(frequency: Frequency) -> Rule {
        Rule {
            frequency,
            interval: 1,
            weekdays: Vec::new(),
            month_days: Vec::new(),
            months: Vec::new(),
            hours: Vec::new(),
            window: None,
            count: None,
            span_days: None,
            biweekly: false,
            vague: false,
            holiday: None,
            time: None,
            time_is_bare: false,
        }
    }

    pub fn rrule(&self, until: Option<NaiveDate>) -> String {
        let mut parts = vec![
            format!("FREQ={}", self.frequency.rrule()),
            format!("INTERVAL={}", self.interval),
        ];
        if !self.weekdays.is_empty() {
            let days: Vec<String> = self.weekdays.iter().map(|d| d.code()).collect();
            parts.push(format!("BYDAY={}", days.join(",")));
        }
        if !self.month_days.is_empty() {
            let days: Vec<String> = self.month_days.iter().map(|d| d.to_string()).collect();
            parts.push(format!("BYMONTHDAY={}", days.join(",")));
        }
        if !self.months.is_empty() {
            let months: Vec<String> = self.months.iter().map(|d| d.to_string()).collect();
            parts.push(format!("BYMONTH={}", months.join(",")));
        }
        if !self.hours.is_empty() {
            let hours: Vec<String> = self.hours.iter().map(|d| d.to_string()).collect();
            parts.push(format!("BYHOUR={}", hours.join(",")));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        if let Some(until) = until {
            parts.push(format!("UNTIL={}", until.format("%Y%m%d")));
        }
        parts.join(";")
    }

    pub fn reminder_shape(&self) -> Option<ReminderRecurrence> {
        let frequency = match self.frequency {
            Frequency::Daily => "daily",
            Frequency::Weekly => "weekly",
            Frequency::Monthly => "monthly",
            Frequency::Yearly => "yearly",
            Frequency::Hourly | Frequency::Minutely => return None,
        };
        if !self.month_days.is_empty()
            || !self.months.is_empty()
            || self.weekdays.iter().any(|d| d.position.is_some())
        {
            return None;
        }
        let days = (!self.weekdays.is_empty()).then(|| {
            self.weekdays
                .iter()
                .map(|d| weekday_name(d.weekday))
                .collect()
        });
        Some(ReminderRecurrence {
            frequency,
            interval: self.interval,
            days,
        })
    }
}

macro_rules! re_fmt {
    ($($arg:tt)*) => {{
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(&format!($($arg)*)).unwrap_or_else(|e| panic!("pattern: {e}")));
        &*RE
    }};
}

macro_rules! re {
    ($pattern:expr) => {{
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new($pattern).unwrap_or_else(|e| panic!("pattern {}: {e}", $pattern))
        });
        &*RE
    }};
}

const PLURAL_WEEKDAY: &str = r"(?:mondays|tuesdays|wednesdays|thursdays|fridays|saturdays|sundays)";
const WEEKDAY: &str = r"(?:mon(?:day)?|tue(?:s(?:day)?)?|wed(?:s|nesday)?|thu(?:r(?:s(?:day)?)?)?|thursday|fri(?:day)?|sat(?:urday)?|sun(?:day)?)s?";

pub fn parse_weekday(word: &str) -> Option<Weekday> {
    let w = word.to_lowercase();
    let w = w.trim_end_matches('.');
    let prefix = |p: &str| w.starts_with(p);
    if prefix("mon") {
        Some(Weekday::Mon)
    } else if prefix("tue") {
        Some(Weekday::Tue)
    } else if prefix("wed") {
        Some(Weekday::Wed)
    } else if prefix("thu") {
        Some(Weekday::Thu)
    } else if prefix("fri") {
        Some(Weekday::Fri)
    } else if prefix("sat") {
        Some(Weekday::Sat)
    } else if prefix("sun") {
        Some(Weekday::Sun)
    } else {
        None
    }
}

fn month_number(word: &str) -> Option<u32> {
    let w = word.to_lowercase();
    let names = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    names
        .iter()
        .position(|n| w.starts_with(n))
        .map(|p| p as u32 + 1)
}

fn ordinal_position(word: &str) -> Option<i8> {
    match word.to_lowercase().as_str() {
        "first" | "1st" => Some(1),
        "second" | "2nd" => Some(2),
        "third" | "3rd" => Some(3),
        "fourth" | "4th" => Some(4),
        "fifth" | "5th" => Some(5),
        "last" | "final" => Some(-1),
        "second last" | "second-to-last" | "penultimate" => Some(-2),
        _ => None,
    }
}

const MONTH: &str = r"(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sept?(?:ember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\.?";

fn weekday_list(text: &str) -> Vec<Weekday> {
    let order = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];
    let mut days = Vec::new();
    let range = re_fmt!(r"(?i)\b({WEEKDAY})\s*(?:thru|through|to|until|till|-|–)\s*({WEEKDAY})\b");
    for c in range.captures_iter(text) {
        let (Some(a), Some(b)) = (
            c.get(1).and_then(|m| parse_weekday(m.as_str())),
            c.get(2).and_then(|m| parse_weekday(m.as_str())),
        ) else {
            continue;
        };
        let start = a.num_days_from_monday() as usize;
        let end = b.num_days_from_monday() as usize;
        let mut i = start;
        loop {
            if !days.contains(&order[i]) {
                days.push(order[i]);
            }
            if i == end {
                break;
            }
            i = (i + 1) % 7;
        }
    }
    let regex = re_fmt!(r"(?i)\b({WEEKDAY})\b");
    for m in regex.find_iter(text) {
        if let Some(d) = parse_weekday(m.as_str()) {
            if !days.contains(&d) {
                days.push(d);
            }
        }
    }
    days.sort_by_key(|d| d.num_days_from_monday());
    days
}

fn workdays(settings: &Settings) -> Vec<Weekday> {
    [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
    .into_iter()
    .filter(|d| !settings.weekend.contains(d))
    .collect()
}

fn byday(days: &[Weekday]) -> Vec<ByDay> {
    days.iter()
        .map(|&weekday| ByDay {
            position: None,
            weekday,
        })
        .collect()
}

pub fn parse_rule(text: &str, settings: &Settings) -> Option<Rule> {
    parse_rule_scoped(text, settings, HolidayScope::from_settings(settings, None))
}

pub fn parse_rule_scoped(text: &str, settings: &Settings, scope: HolidayScope) -> Option<Rule> {
    let t = text.to_lowercase();
    let t = t.as_str();
    let mut rule = rule_core(t, settings, scope)?;
    if rule.frequency == Frequency::Yearly && rule.months.is_empty() {
        let in_month = re_fmt!(r"(?i)\b(?:in|during|each|every)\s+({MONTH})(?:\s|$|,)");
        if let Some(m) = in_month
            .captures(t)
            .and_then(|c| c.get(1))
            .and_then(|m| month_number(m.as_str()))
        {
            rule.months = vec![m];
        }
    }
    if re!(r"(?i)\bbi-?weekly\b").is_match(t) {
        rule.biweekly = true;
    }
    if let Some(c) = re!(r"(?i)\b(?:for\s+)?(?:the\s+next\s+)?(\d+|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|fifteen|twenty)\s+(?:more\s+)?(?:times|sessions|classes|lessons|occurrences|appointments|meetings|visits|rounds|doses|weeks?\s+running)\b")
        .captures(t)
    {
        rule.count = c
            .get(1)
            .and_then(|m| number_value(m.as_str()))
            .map(|v| v.clamp(1.0, 10_000.0) as u32);
    }
    if let Some(c) = re!(r"(?i)\bfor\s+(?:the\s+next\s+)?(\d+|an?|one|two|three|four|five|six|seven|eight|nine|ten|twelve)\s+(days?|weeks?|months?|years?)\b").captures(t) {
        let n = c.get(1).and_then(|m| number_value(m.as_str())).unwrap_or(1.0).clamp(1.0, 1200.0) as i64;
        let unit = c.get(2).map(|m| m.as_str()).unwrap_or("");
        rule.span_days = Some(if unit.starts_with("day") {
            SpanLength::Days(n)
        } else if unit.starts_with("week") {
            SpanLength::Days(n * 7)
        } else if unit.starts_with("month") {
            SpanLength::Months(n as u32)
        } else {
            SpanLength::Months(n as u32 * 12)
        });
    }
    if matches!(rule.frequency, Frequency::Hourly | Frequency::Minutely) && rule.weekdays.is_empty()
    {
        let days = if re!(r"(?i)\b(?:week\s*days|work\s*days|business\s+days|mon(?:day)?\s*(?:-|–|to|thru|through)\s*fri(?:day)?)\b").is_match(t) {
            workdays(settings)
        } else if re!(r"(?i)\bweekends?\b").is_match(t) {
            let mut days = settings.weekend.clone();
            days.sort_by_key(|d| d.num_days_from_monday());
            days
        } else {
            weekday_list(t)
        };
        rule.weekdays = byday(&days);
    }
    if matches!(rule.frequency, Frequency::Hourly | Frequency::Minutely) {
        if let Some((start, end)) = rule_window(t) {
            rule.window = Some((start, end));
            let (sh, eh) = (start.hour(), end.hour());
            rule.hours = if rule.frequency == Frequency::Hourly {
                (sh..=eh).collect()
            } else if end.minute() == 0 {
                (sh..eh).collect()
            } else {
                (sh..=eh).collect()
            };
            return Some(rule);
        }
    }
    if let Some((time, bare)) = rule_time(t, settings) {
        rule.time = Some(time);
        rule.time_is_bare = bare;
    }
    Some(rule)
}

fn window_hour(hour: u32, minute: u32, ampm: Option<&str>) -> Option<NaiveTime> {
    let hour = match ampm.map(|m| m.to_lowercase()) {
        Some(m) if m.starts_with('p') && hour < 12 => hour + 12,
        Some(m) if m.starts_with('a') && hour == 12 => 0,
        _ => hour,
    };
    NaiveTime::from_hms_opt(hour % 24, minute, 0)
}

fn rule_window(t: &str) -> Option<(NaiveTime, NaiveTime)> {
    if re!(r"(?i)\b(?:during\s+)?(?:work(?:ing)?|business|office)\s+hours\b").is_match(t) {
        return Some((
            NaiveTime::from_hms_opt(9, 0, 0)?,
            NaiveTime::from_hms_opt(17, 0, 0)?,
        ));
    }
    let c = re!(r"(?i)\b(?:between|from)?\s*(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?\s*(?:and|to|until|till|-|–)\s*(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?").captures(t)?;
    let sh: u32 = c.get(1)?.as_str().parse().ok()?;
    let sm: u32 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
    let eh: u32 = c.get(4)?.as_str().parse().ok()?;
    let em: u32 = c.get(5).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
    if sh > 24 || eh > 24 || sm > 59 || em > 59 {
        return None;
    }
    let s_ampm = c.get(3).map(|m| m.as_str());
    let e_ampm = c.get(6).map(|m| m.as_str());
    let mut start = window_hour(sh, sm, s_ampm)?;
    if s_ampm.is_none() && (1..=6).contains(&sh) && e_ampm.is_none() {
        start = window_hour(sh + 12, sm, None)?;
    }
    let mut end = window_hour(eh, em, e_ampm)?;
    if e_ampm.is_none() && end <= start && eh < 12 {
        end = window_hour(eh + 12, em, None)?;
    }
    (end > start).then_some((start, end))
}

fn rule_time(t: &str, settings: &Settings) -> Option<(NaiveTime, bool)> {
    let clock = re!(
        r"(?i)\b(?:at|@|by|from|around)\s+(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?|\b(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)|\b(\d{1,2}):(\d{2})\b"
    );
    if let Some(c) = clock.captures(t) {
        let (hour, minute, ampm) = if let Some(h) = c.get(1) {
            (h, c.get(2), c.get(3))
        } else if let Some(h) = c.get(4) {
            (h, c.get(5), c.get(6))
        } else {
            (c.get(7)?, c.get(8), None)
        };
        let mut hour: u32 = hour.as_str().parse().ok()?;
        let minute: u32 = minute.and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let leading_zero = c.get(0).is_some_and(|m| re!(r"\b0\d").is_match(m.as_str()));
        let meridiem = ampm.map(|m| m.as_str().starts_with('p'));
        let mut bare = false;
        match meridiem {
            Some(true) if hour < 12 => hour += 12,
            Some(false) if hour == 12 => hour = 0,
            Some(_) => {}
            None => bare = (1..=11).contains(&hour) && !leading_zero,
        }
        return NaiveTime::from_hms_opt(hour, minute, 0).map(|time| (time, bare));
    }
    let dayparts: [(&Regex, NaiveTime); 6] = [
        (
            re!(r"(?i)\bnoon|midday\b"),
            NaiveTime::from_hms_opt(12, 0, 0)?,
        ),
        (re!(r"(?i)\bmornings?\b"), settings.morning),
        (re!(r"(?i)\bafternoons?\b"), settings.afternoon),
        (re!(r"(?i)\bevenings?\b"), settings.evening),
        (
            re!(r"(?i)\b(?:nights?|nightly|weeknights?|school\s+nights?|bedtime)\b"),
            settings.tonight,
        ),
        (re!(r"(?i)\bafter\s+work\b"), settings.after_work),
    ];
    dayparts
        .into_iter()
        .find(|(r, _)| r.is_match(t))
        .map(|(_, time)| (time, false))
}

fn holiday_rule(t: &str, scope: HolidayScope) -> Option<Rule> {
    let after_every = re!(r"(?i)\b(?:every|each)\s+(?:year\s+(?:on|at)\s+)?(.+)").captures(t)?;
    let rest = after_every.get(1)?.as_str();
    let found = crate::capture::holidays::find_holidays(rest, scope)
        .into_iter()
        .find(|h| h.start == 0)?;
    let mut rule = Rule::new(Frequency::Yearly);
    match found.holiday.yearly_shape() {
        Some(YearlyShape::Fixed { month, day }) => {
            rule.months = vec![month];
            rule.month_days = vec![i8::try_from(day).unwrap_or(1)];
        }
        Some(YearlyShape::Nth {
            month,
            position,
            weekday,
        }) => {
            rule.months = vec![month];
            rule.weekdays = vec![ByDay {
                position: Some(position),
                weekday,
            }];
        }
        None => rule.holiday = Some(found.holiday),
    }
    Some(rule)
}

fn rule_core(t: &str, settings: &Settings, scope: HolidayScope) -> Option<Rule> {
    if let Some(rule) = holiday_rule(t, scope) {
        return Some(rule);
    }
    if re!(r"(?i)\b(?:hourly|half[\s-]hourly|every\s+(?:(?:other|second|\d+|two|three|four|five|six|ten|fifteen|twenty|thirty|forty-five|half|few|couple\s+of)\s+)?(?:an?\s+)?(?:minutes?|mins?|hours?|hrs?)|on\s+the\s+hour)\b").is_match(t) {
        let sub_daily = sub_daily_core(t);
        if sub_daily.is_some() {
            return sub_daily;
        }
    }
    let day_list = re!(
        r"(?i)\b(?:every|each|on\s+the|the)\s+((?:\d{1,2}(?:st|nd|rd|th))(?:\s*(?:,|and|&)\s*(?:the\s+)?\d{1,2}(?:st|nd|rd|th))+)"
    );
    if let Some(c) = day_list.captures(t) {
        let days: Vec<i8> = re!(r"\d{1,2}")
            .find_iter(c.get(1)?.as_str())
            .filter_map(|m| m.as_str().parse::<i8>().ok())
            .filter(|d| (1..=31).contains(d))
            .collect();
        if days.len() >= 2 {
            let mut rule = Rule::new(Frequency::Monthly);
            rule.month_days = days;
            return Some(rule);
        }
    }
    if re!(r"\bschool\s*nights?\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        rule.weekdays = byday(&[
            Weekday::Mon,
            Weekday::Tue,
            Weekday::Wed,
            Weekday::Thu,
            Weekday::Sun,
        ]);
        return Some(rule);
    }
    if re!(r"\b(?:every|each)\s+(?:week\s*day|work\s*day|business\s+day|working\s+day)s?\b|\b(?:weekdays|workdays|business\s+days|weeknights)\b|\bevery\s+weeknight\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        rule.weekdays = byday(&workdays(settings));
        return Some(rule);
    }
    if re!(r"\b(?:every|each)\s+weekends?\b|\bweekends\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        let mut days = settings.weekend.clone();
        days.sort_by_key(|d| d.num_days_from_monday());
        rule.weekdays = byday(&days);
        return Some(rule);
    }
    if re!(r"\b(?:last|final)\s+day\s+of\s+(?:every|each|the)\s+month\b|\bend\s+of\s+(?:every|each)\s+month\b|\bevery\s+(?:last|final)\s+day\s+of\s+the\s+month\b|\bmonthly\s+on\s+the\s+last\s+day\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Monthly);
        rule.month_days = vec![-1];
        return Some(rule);
    }
    if re!(r"\bfirst\s+day\s+of\s+(?:every|each|the)\s+month\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Monthly);
        rule.month_days = vec![1];
        return Some(rule);
    }

    let multi = re_fmt!(
        r"(?i)(?:\b(?:every|each|on\s+the|the)\s+|^|\s)((?:first|1st|second|2nd|third|3rd|fourth|4th|fifth|5th|last)(?:\s*(?:,|and|&)\s*(?:the\s+)?(?:first|1st|second|2nd|third|3rd|fourth|4th|fifth|5th|last))+)\s+({WEEKDAY})"
    );
    if let Some(c) = multi.captures(t) {
        let weekday = c.get(2).and_then(|m| parse_weekday(m.as_str()))?;
        let list = c.get(1).map(|m| m.as_str()).unwrap_or("");
        let positions: Vec<i8> =
            re!(r"(?i)first|1st|second|2nd|third|3rd|fourth|4th|fifth|5th|last")
                .find_iter(list)
                .filter_map(|m| ordinal_position(m.as_str()))
                .collect();
        let mut rule = Rule::new(Frequency::Monthly);
        rule.weekdays = positions
            .into_iter()
            .map(|position| ByDay {
                position: Some(position),
                weekday,
            })
            .collect();
        return Some(rule);
    }
    let of_month = re_fmt!(
        r"(?i)\b(first|1st|second|2nd|third|3rd|fourth|4th|fifth|5th|last|final)\s+({WEEKDAY})\s+(?:of|in)\s+(?:the|each|every)\s+month"
    );
    if let Some(c) = of_month.captures(t) {
        let weekday = c.get(2).and_then(|m| parse_weekday(m.as_str()))?;
        let position = ordinal_position(c.get(1).map(|m| m.as_str()).unwrap_or(""))?;
        let mut rule = Rule::new(Frequency::Monthly);
        rule.weekdays = vec![ByDay {
            position: Some(position),
            weekday,
        }];
        return Some(rule);
    }
    let positional = re_fmt!(
        r"(?i)(?:\b(every|each)\s+|\b(?:on\s+the|the|monthly\s+on\s+the)\s+|^|\s)(first|1st|second|2nd|third|3rd|fourth|4th|fifth|5th|last|final)\s+({WEEKDAY})(\s+of\s+(?:the|each|every)\s+month)?"
    );
    if let Some(c) = positional.captures(t) {
        let word = c.get(2).map(|m| m.as_str()).unwrap_or("");
        let weekday = c.get(3).and_then(|m| parse_weekday(m.as_str()))?;
        let position = ordinal_position(word)?;
        let of_month = c.get(4).is_some();
        let every = c.get(1).is_some();
        if position == 2
            && !of_month
            && every
            && !word.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        {
            let mut rule = Rule::new(Frequency::Weekly);
            rule.interval = 2;
            rule.weekdays = byday(&[weekday]);
            return Some(rule);
        }
        let mut rule = Rule::new(Frequency::Monthly);
        rule.weekdays = vec![ByDay {
            position: Some(position),
            weekday,
        }];
        return Some(rule);
    }

    let yearly = re_fmt!(
        r"(?i)\b(?:every|each|annually\s+on|yearly\s+on|every\s+year\s+on)\s+({MONTH})\s+(\d{{1,2}})(?:st|nd|rd|th)?\b|\b(?:every|each)\s+(\d{{1,2}})(?:st|nd|rd|th)?\s+(?:of\s+)?({MONTH})"
    );
    if let Some(c) = yearly.captures(t) {
        let (month, day) = if let (Some(m), Some(d)) = (c.get(1), c.get(2)) {
            (month_number(m.as_str())?, d.as_str().parse::<i8>().ok()?)
        } else {
            (
                month_number(c.get(4)?.as_str())?,
                c.get(3)?.as_str().parse::<i8>().ok()?,
            )
        };
        let mut rule = Rule::new(Frequency::Yearly);
        rule.months = vec![month];
        rule.month_days = vec![day];
        return Some(rule);
    }

    let month_day = re!(
        r"(?i)\b(?:every|each)\s+(?:month\s+on\s+the\s+)?(\d{1,2})(?:st|nd|rd|th)\b|\bthe\s+(\d{1,2})(?:st|nd|rd|th)\s+of\s+(?:every|each|the)\s+month\b|\bmonthly\s+on\s+the\s+(\d{1,2})(?:st|nd|rd|th)?\b|\bevery\s+month\s+on\s+the\s+(\d{1,2})(?:st|nd|rd|th)?\b|\bon\s+the\s+(\d{1,2})(?:st|nd|rd|th)\s+(?:of\s+)?(?:every|each)\s+month\b"
    );
    if let Some(c) = month_day.captures(t) {
        let day = (1..=5)
            .find_map(|i| c.get(i))
            .and_then(|m| m.as_str().parse::<i8>().ok())?;
        let mut rule = Rule::new(Frequency::Monthly);
        rule.month_days = vec![day];
        return Some(rule);
    }

    let fixed_intervals: [(&Regex, Frequency, u32, bool); 5] = [
        (
            re!(r"(?i)\bevery\s+half\s+(?:an\s+)?hour\b|\bhalf[\s-]hourly\b"),
            Frequency::Minutely,
            30,
            false,
        ),
        (
            re!(
                r"(?i)\bevery\s+(?:an?\s+)?hour\s+and\s+a\s+half\b|\bevery\s+one\s+and\s+a\s+half\s+hours\b"
            ),
            Frequency::Minutely,
            90,
            false,
        ),
        (
            re!(r"(?i)\bevery\s+couple\s+(?:of\s+)?hours\b"),
            Frequency::Hourly,
            2,
            false,
        ),
        (
            re!(r"(?i)\bevery\s+few\s+hours\b"),
            Frequency::Hourly,
            3,
            true,
        ),
        (
            re!(r"(?i)\bevery\s+few\s+(?:days)\b"),
            Frequency::Daily,
            3,
            true,
        ),
    ];
    for (regex, frequency, interval, vague) in fixed_intervals {
        if regex.is_match(t) {
            let mut rule = Rule::new(frequency);
            rule.interval = interval;
            rule.vague = vague;
            return Some(rule);
        }
    }
    if re!(r"(?i)\bevery\s+few\s+weeks\b").is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        rule.interval = 3;
        rule.vague = true;
        return Some(rule);
    }
    let interval = re!(
        r"(?i)\bevery\s+(other|second|\d+|two|three|four|five|six|seven|eight|nine|ten|twelve|fifteen|twenty|thirty|forty-five)\s+(minutes?|mins?|hours?|hrs?|days?|weeks?|months?|years?|quarters?)\b"
    );
    if let Some(c) = interval.captures(t) {
        let n = match c.get(1).map(|m| m.as_str()).unwrap_or("") {
            "other" | "second" => 2,
            word => number_value(word)
                .map(|v| v.clamp(1.0, 1000.0) as u32)
                .unwrap_or(1),
        };
        let unit = c.get(2).map(|m| m.as_str()).unwrap_or("");
        let (frequency, factor) = unit_frequency(unit);
        let mut rule = Rule::new(frequency);
        rule.interval = n.max(1) * factor;
        if frequency == Frequency::Weekly {
            rule.weekdays = byday(&weekday_list(t));
        }
        return Some(rule);
    }

    let other_weekday = re_fmt!(r"(?i)\bevery\s+(?:other|second|alternate)\s+({WEEKDAY})");
    if let Some(c) = other_weekday.captures(t) {
        let weekday = c.get(1).and_then(|m| parse_weekday(m.as_str()))?;
        let mut rule = Rule::new(Frequency::Weekly);
        rule.interval = 2;
        rule.weekdays = byday(&[weekday]);
        return Some(rule);
    }

    let days = weekday_list(t);
    let range_list = re_fmt!(r"(?i)\b{WEEKDAY}\s*(?:thru|through|to|-|–)\s*{WEEKDAY}\b");
    if days.len() >= 2 && range_list.is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        rule.weekdays = byday(&days);
        return Some(rule);
    }
    let weekday_parts = re_fmt!(r"(?i)\b{WEEKDAY}\s+(?:mornings|afternoons|evenings|nights)\b");
    if !days.is_empty() && weekday_parts.is_match(t) {
        let mut rule = Rule::new(Frequency::Weekly);
        rule.weekdays = byday(&days);
        return Some(rule);
    }
    let weekly_words = re_fmt!(
        r"(?i)\b(?:every|each)\s+{WEEKDAY}|\b{PLURAL_WEEKDAY}\b|\bweekly\b|\bevery\s+week\b"
    );
    let bare_list = days.len() >= 2 && re!(r"(?i)^\s*(?:(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?\s*(?:,|/|&|\band\b|\s)\s*)+(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?\b").is_match(t);
    if !days.is_empty() && (weekly_words.is_match(t) || bare_list) {
        let mut rule = Rule::new(Frequency::Weekly);
        if re!(r"(?i)\b(?:bi-?weekly|fortnightly)\b").is_match(t) {
            rule.interval = 2;
        }
        rule.weekdays = byday(&days);
        return Some(rule);
    }

    let simple: [(&Regex, Frequency, u32); 13] = [
        (
            re!(r"(?i)\b(?:every|each)\s+hour\b|\bhourly\b|\bon\s+the\s+hour\b"),
            Frequency::Hourly,
            1,
        ),
        (
            re!(r"(?i)\b(?:bi-?weekly|fortnightly|every\s+fortnight)\b"),
            Frequency::Weekly,
            2,
        ),
        (
            re!(r"(?i)\b(?:every|each)\s+quarter\b|\bquarterly\b"),
            Frequency::Monthly,
            3,
        ),
        (
            re!(r"(?i)\btwice\s+a\s+year\b|\bsemi-?annually\b"),
            Frequency::Monthly,
            6,
        ),
        (
            re!(r"(?i)\b(?:every|each)\s+day\b|\bdaily\b|\bevery\s*day\b"),
            Frequency::Daily,
            1,
        ),
        (
            re!(
                r"(?i)\b(?:every|each)\s+(?:morning|afternoon|evening|night)\b|\bnightly\b|\b(?:mornings|evenings|nights)\b"
            ),
            Frequency::Daily,
            1,
        ),
        (
            re!(r"(?i)\b(?:every|each)\s+week\b|\bweekly\b|\bonce\s+a\s+week\b"),
            Frequency::Weekly,
            1,
        ),
        (
            re!(r"(?i)\b(?:every|each)\s+month\b|\bmonthly\b|\bonce\s+a\s+month\b"),
            Frequency::Monthly,
            1,
        ),
        (
            re!(r"(?i)\b(?:every|each)\s+year\b|\byearly\b|\bannually\b|\bonce\s+a\s+year\b"),
            Frequency::Yearly,
            1,
        ),
        (re!(r"(?i)\bevery\s+birthday\b"), Frequency::Yearly, 1),
        (
            re!(r"(?i)\bonce\s+a\s+day\b|\btwice\s+a\s+day\b"),
            Frequency::Daily,
            1,
        ),
        (re!(r"(?i)\bon\s+weekdays\b"), Frequency::Weekly, 1),
        (re!(r"(?i)\bevery\s+single\s+day\b"), Frequency::Daily, 1),
    ];
    simple
        .into_iter()
        .find(|(r, _, _)| r.is_match(t))
        .map(|(_, frequency, interval)| {
            let mut rule = Rule::new(frequency);
            rule.interval = interval;
            rule
        })
}

fn sub_daily_core(t: &str) -> Option<Rule> {
    let fixed: [(&Regex, Frequency, u32, bool); 4] = [
        (
            re!(r"(?i)\bevery\s+half\s+(?:an\s+)?hour\b|\bhalf[\s-]hourly\b"),
            Frequency::Minutely,
            30,
            false,
        ),
        (
            re!(r"(?i)\bevery\s+(?:an?\s+)?hour\s+and\s+a\s+half\b"),
            Frequency::Minutely,
            90,
            false,
        ),
        (
            re!(r"(?i)\bevery\s+couple\s+(?:of\s+)?hours\b"),
            Frequency::Hourly,
            2,
            false,
        ),
        (
            re!(r"(?i)\bevery\s+few\s+hours\b"),
            Frequency::Hourly,
            3,
            true,
        ),
    ];
    for (regex, frequency, interval, vague) in fixed {
        if regex.is_match(t) {
            let mut rule = Rule::new(frequency);
            rule.interval = interval;
            rule.vague = vague;
            return Some(rule);
        }
    }
    if let Some(c) = re!(r"(?i)\bevery\s+(other|second|\d+|two|three|four|five|six|ten|fifteen|twenty|thirty|forty-five)\s+(minutes?|mins?|hours?|hrs?)\b").captures(t) {
        let n = match c.get(1).map(|m| m.as_str()).unwrap_or("") {
            "other" | "second" => 2,
            word => number_value(word).map(|v| v.clamp(1.0, 1000.0) as u32).unwrap_or(1),
        };
        let (frequency, _) = unit_frequency(c.get(2).map(|m| m.as_str()).unwrap_or(""));
        let mut rule = Rule::new(frequency);
        rule.interval = n.max(1);
        return Some(rule);
    }
    if re!(r"(?i)\b(?:every|each)\s+hour\b|\bhourly\b|\bon\s+the\s+hour\b").is_match(t) {
        return Some(Rule::new(Frequency::Hourly));
    }
    None
}

fn unit_frequency(unit: &str) -> (Frequency, u32) {
    if unit.starts_with("min") {
        (Frequency::Minutely, 1)
    } else if unit.starts_with("hour") || unit.starts_with("hr") {
        (Frequency::Hourly, 1)
    } else if unit.starts_with("day") {
        (Frequency::Daily, 1)
    } else if unit.starts_with("week") {
        (Frequency::Weekly, 1)
    } else if unit.starts_with("quarter") {
        (Frequency::Monthly, 3)
    } else if unit.starts_with("month") {
        (Frequency::Monthly, 1)
    } else {
        (Frequency::Yearly, 1)
    }
}

pub fn exceptions_in(text: &str) -> Vec<RecurrenceException> {
    let pattern = re!(
        r"(?i)\b(?:except(?:\s+for|\s+on|\s+in|\s+during)?|unless|but\s+(?:skip|not)|skip(?:ping)?|excluding|not\s+(?:on|in|during|the\s+week)|no\s+\w+\s+in|apart\s+from|other\s+than)\b[^,;.]*"
    );
    pattern
        .find_iter(text)
        .map(|m| {
            let raw = m.as_str().trim().to_string();
            let lower = raw.to_lowercase();
            let condition = if lower.contains("holiday") {
                Some("holidays".to_string())
            } else {
                let days = weekday_list(&lower);
                (!days.is_empty()).then(|| {
                    days.iter()
                        .map(|d| format!("weekday:{}", weekday_name(*d)))
                        .collect::<Vec<_>>()
                        .join(",")
                })
            };
            let action = if lower.contains("first one") {
                Some("skip_first".to_string())
            } else {
                Some("skip".to_string())
            };
            RecurrenceException {
                raw,
                condition,
                action,
            }
        })
        .collect()
}

pub fn exception_weekdays(exceptions: &[RecurrenceException]) -> Vec<Weekday> {
    exceptions
        .iter()
        .filter_map(|e| e.condition.as_deref())
        .flat_map(|c| c.split(','))
        .filter_map(|c| c.strip_prefix("weekday:"))
        .filter_map(parse_weekday)
        .collect()
}

fn days_in_month(date: NaiveDate) -> u32 {
    let first_next = if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
    };
    first_next
        .and_then(|d| d.pred_opt())
        .map_or(31, |d| d.day())
}

fn month_day_matches(date: NaiveDate, days: &[i8]) -> bool {
    let dim = days_in_month(date) as i32;
    days.iter().any(|&d| {
        let d = i32::from(d);
        let target = if d < 0 { dim + 1 + d } else { d.min(dim) };
        target == date.day() as i32
    })
}

fn position_matches(date: NaiveDate, by: &ByDay) -> bool {
    if date.weekday() != by.weekday {
        return false;
    }
    match by.position {
        None => true,
        Some(p) if p > 0 => ((date.day() - 1) / 7 + 1) as i8 == p,
        Some(p) => {
            let from_end = ((days_in_month(date) - date.day()) / 7 + 1) as i8;
            from_end == -p
        }
    }
}

fn week_start(date: NaiveDate, start: Weekday) -> NaiveDate {
    let offset = (7 + date.weekday().num_days_from_monday() - start.num_days_from_monday()) % 7;
    date - Duration::days(i64::from(offset))
}

pub fn matches(rule: &Rule, anchor: NaiveDate, date: NaiveDate, week_start_day: Weekday) -> bool {
    if date < anchor {
        return false;
    }
    let interval = i64::from(rule.interval.max(1));
    match rule.frequency {
        Frequency::Minutely | Frequency::Hourly | Frequency::Daily => {
            (date - anchor).num_days() % interval == 0
        }
        Frequency::Weekly => {
            let weeks = (week_start(date, week_start_day) - week_start(anchor, week_start_day))
                .num_days()
                / 7;
            let day_ok = if rule.weekdays.is_empty() {
                date.weekday() == anchor.weekday()
            } else {
                rule.weekdays.iter().any(|d| d.weekday == date.weekday())
            };
            weeks % interval == 0 && day_ok
        }
        Frequency::Monthly => {
            let months = i64::from(date.year() - anchor.year()) * 12 + i64::from(date.month())
                - i64::from(anchor.month());
            if months % interval != 0 {
                return false;
            }
            day_rule_matches(rule, anchor, date)
        }
        Frequency::Yearly => {
            if i64::from(date.year() - anchor.year()) % interval != 0 {
                return false;
            }
            let month_ok = if rule.months.is_empty() {
                date.month() == anchor.month()
            } else {
                rule.months.contains(&date.month())
            };
            month_ok && day_rule_matches(rule, anchor, date)
        }
    }
}

fn day_rule_matches(rule: &Rule, anchor: NaiveDate, date: NaiveDate) -> bool {
    if rule.frequency == Frequency::Yearly
        && !rule.months.is_empty()
        && rule.month_days.is_empty()
        && rule.weekdays.is_empty()
    {
        return date.day() == 1;
    }
    if !rule.month_days.is_empty() {
        month_day_matches(date, &rule.month_days)
    } else if !rule.weekdays.is_empty() {
        rule.weekdays.iter().any(|d| position_matches(date, d))
    } else {
        date.day() == anchor.day().min(days_in_month(date))
    }
}

pub fn first_occurrence(
    rule: &Rule,
    anchor: NaiveDate,
    time: Option<NaiveTime>,
    now: NaiveDateTime,
    week_start_day: Weekday,
    skip_weekdays: &[Weekday],
) -> Option<NaiveDate> {
    let mut date = anchor;
    for _ in 0..(366 * 9) {
        if matches(rule, anchor, date, week_start_day) && !skip_weekdays.contains(&date.weekday()) {
            let moment = date.and_time(time.unwrap_or(NaiveTime::MIN));
            let in_future = match time {
                Some(_) => moment >= now,
                None => date >= now.date(),
            };
            if in_future {
                return Some(date);
            }
        }
        date = date.succ_opt()?;
    }
    None
}

pub fn ignoring_interval(rule: &Rule) -> Rule {
    let mut free = rule.clone();
    free.interval = 1;
    free
}

pub fn add_span(date: NaiveDate, span: SpanLength) -> Option<NaiveDate> {
    match span {
        SpanLength::Days(days) => date.checked_add_signed(Duration::days(days)),
        SpanLength::Months(months) => date.checked_add_months(Months::new(months)),
    }
}

pub fn month_only(text: &str) -> Option<u32> {
    let pattern = re_fmt!(
        r"(?i)^\s*(?:until|till|til|through|thru|to|ending(?:\s+in)?|ends(?:\s+in)?|up\s+to|by)?\s*(?:the\s+end\s+of\s+)?({MONTH})\s*$"
    );
    pattern
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| month_number(m.as_str()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhraseKind {
    Rule,
    OrdinalRule,
    RangeRule,
    Start,
    End,
    Exception,
    Window,
}

pub fn find_recurrence_phrases(line: &str, scope: HolidayScope) -> Vec<(PhraseKind, usize, usize)> {
    let rule = re_fmt!(
        r"(?i)\b(?:every|each|on\s+the)\s+\d{{1,2}}(?:st|nd|rd|th)(?:\s*(?:,|and|&)\s*(?:the\s+)?\d{{1,2}}(?:st|nd|rd|th))+(?:\s+of\s+(?:every|each|the)\s+month)?|\b(?:every|each)\s+(?:{WEEKDAY})(?:\s*(?:thru|through|to|-|–)\s*{WEEKDAY})\b|\b(?:every|each)\s+(?:half\s+(?:an\s+)?hour|(?:an?\s+)?hour\s+and\s+a\s+half|couple\s+(?:of\s+)?hours|few\s+(?:hours|days|weeks))\b|\bhalf[\s-]hourly\b|\b(?:every|each)\s+(?:(?:other|second|alternate|single|first|third|fourth|fifth|last|final|\d+(?:st|nd|rd|th)?|two|three|four|five|six|ten|fifteen|twenty|thirty|forty-five)\s+)?(?:minutes?|mins?|hrs?)\b|\b(?:every|each)\s+(?:(?:other|second|alternate|single|first|third|fourth|fifth|last|final|\d+(?:st|nd|rd|th)?|two|three|four|five|six)\s+)?(?:{WEEKDAY}(?:\s*(?:,|and|&|/)\s*{WEEKDAY})*|{MONTH}\s+\d{{1,2}}(?:st|nd|rd|th)?|\d{{1,2}}(?:st|nd|rd|th)?\s+(?:of\s+)?{MONTH}|week\s*days?|weeknights?|weekends?|work\s*days?|business\s+days?|working\s+days?|school\s+nights?|days?|weeks?|months?|years?|quarters?|hours?|mornings?|afternoons?|evenings?|nights?|fortnight|birthday)\b(?:\s+of\s+(?:the|each|every)\s+month)?(?:\s+(?:at|from)\s+\d{{1,2}}(?:[:.]\d{{2}})?\s*(?:a\.?m\.?|p\.?m\.?)?)?|\b(?:daily|weekly|monthly|yearly|annually|hourly|nightly|quarterly|bi-?weekly|fortnightly|twice\s+a\s+(?:day|week|month|year)|once\s+a\s+(?:day|week|month|year))\b|\b(?:on\s+)?{PLURAL_WEEKDAY}(?:\s*(?:,|and|&|/)\s*{WEEKDAY})*\b|\b{WEEKDAY}\s+(?:mornings|afternoons|evenings|nights)\b|\b(?:last|final|first)\s+day\s+of\s+(?:every|each)\s+month\b|\b(?:first|1st|second|2nd|third|3rd|fourth|4th|last)\s+{WEEKDAY}\s+(?:of|in)\s+(?:every|each|the)\s+month\b|\bend\s+of\s+(?:every|each)\s+month\b|\b(?:every|each)\s+\d{{1,2}}(?:st|nd|rd|th)\b|\bthe\s+\d{{1,2}}(?:st|nd|rd|th)\s+of\s+(?:every|each)\s+month\b|\b(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?(?:\s*(?:,|/|&|\s)\s*(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*\.?){{2,}}\b"
    );
    let adverb = re!(r"(?i)^(?:daily|weekly|monthly|yearly|annually|hourly|nightly|quarterly)$");
    let allowed_after = re!(
        r"(?i)^\s*(?:$|[,;.!]|on\b|at\b|from\b|until\b|till\b|starting\b|beginning\b|for\b|in\b|by\b|before\b|after\b|except\b|and\b|between\b|during\b|every\b|-)"
    );
    let mut out: Vec<(PhraseKind, usize, usize)> = rule
        .find_iter(line)
        .filter(|m| !adverb.is_match(m.as_str()) || allowed_after.is_match(&line[m.end()..]))
        .map(|m| (PhraseKind::Rule, m.start(), m.end()))
        .collect();
    let ordinal = re_fmt!(
        r"(?i)(?:\b(every|each)\s+)?(?:\b(?:the|on\s+the|monthly\s+on\s+the)\s+)?\b(?:1st|2nd|3rd|4th|5th|first|second|third|fourth|fifth|last)(?:\s*(?:,|and|&)\s*(?:the\s+)?(?:1st|2nd|3rd|4th|5th|first|second|third|fourth|fifth|last))*\s+(?:{WEEKDAY})\b(?:\s+of\s+(?:the|each|every)\s+month)?"
    );
    for c in ordinal.captures_iter(line) {
        let Some(whole) = c.get(0) else { continue };
        if c.get(1).is_some() {
            continue;
        }
        let after = &line[whole.end()..];
        if re_fmt!(r"(?i)^\s+(?:in|of)\s+(?:{MONTH}|next\s+month|this\s+month)").is_match(after) {
            continue;
        }
        let text = whole.as_str().to_lowercase();
        let bare_last = text.trim_start().starts_with("last") && !text.contains("month");
        if bare_last {
            continue;
        }
        out.push((PhraseKind::OrdinalRule, whole.start(), whole.end()));
    }
    let range = re_fmt!(
        r"(?i)\b(?:from\s+)?{WEEKDAY}\.?\s*(?:to|thru|through|till|until|-|–)\s*{WEEKDAY}\b"
    );
    for m in range.find_iter(line) {
        out.push((PhraseKind::RangeRule, m.start(), m.end()));
    }
    for m in re!(r"(?i)\b(?:every|each)\s+(?:year\s+(?:on|at)\s+)?").find_iter(line) {
        if let Some(h) = crate::capture::holidays::find_holidays(&line[m.end()..], scope)
            .into_iter()
            .find(|h| h.start == 0)
        {
            out.push((PhraseKind::Rule, m.start(), m.end() + h.end));
        }
    }
    if out.is_empty() {
        return out;
    }
    for m in re!(r"(?i)\bskip(?:ping)?\s+").find_iter(line) {
        if let Some(h) = crate::capture::holidays::find_holidays(&line[m.end()..], scope)
            .into_iter()
            .find(|h| h.start == 0)
        {
            let tail = &line[m.end() + h.end..];
            let stop = tail.find([',', ';', '.']).unwrap_or(tail.len());
            out.push((PhraseKind::Exception, m.start(), m.end() + h.end + stop));
        }
    }
    let delimiter = re!(
        r"(?i)[,;]|\s(?:except|unless|starting|beginning|from|every|but|until|till|through|thru|and\s+(?:then|also)|at\s+\d)\b"
    );
    let keyword = |pattern: &Regex, kind: PhraseKind, out: &mut Vec<(PhraseKind, usize, usize)>| {
        for m in pattern.find_iter(line) {
            let rest = &line[m.end()..];
            let stop = delimiter
                .find(rest)
                .map_or(line.len(), |d| m.end() + d.start());
            if line[m.end()..stop].trim().is_empty() {
                continue;
            }
            let clock_window = re!(
                r"(?i)^\s*\d{1,2}(?:[:.]\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)?\s*(?:to|until|till|and|-|–)\s*\d"
            );
            if kind == PhraseKind::Start && clock_window.is_match(rest) {
                continue;
            }
            let before = &line[..m.start()];
            let weekday_before = re_fmt!(r"(?i)\b{WEEKDAY}\.?\s*$").is_match(before);
            let weekday_after = re_fmt!(r"(?i)^\s*{WEEKDAY}\b").is_match(rest);
            let clock_before = re!(r"\d\s*(?:a\.?m\.?|p\.?m\.?)?\s*$").is_match(before)
                && re!(r"^\s*\d").is_match(rest);
            if kind == PhraseKind::End && ((weekday_before && weekday_after) || clock_before) {
                continue;
            }
            out.push((kind, m.start(), stop));
        }
    };
    keyword(
        re!(r"(?i)\b(?:starting|beginning|commencing|as\s+of|effective|start(?:ing)?\s+on|from)\b"),
        PhraseKind::Start,
        &mut out,
    );
    keyword(
        re!(r"(?i)\b(?:until|till|til|through|thru|ending|ends)\b"),
        PhraseKind::End,
        &mut out,
    );
    let exception = re_fmt!(
        r"(?i)\b(?:except(?:\s+for|\s+on)?|unless|but\s+(?:skip|not)|skip(?:ping)?\s+(?:the\s+)?(?:week|\d|\w+\s+(?:week|\d)|holidays|first)|excluding|not\s+(?:on|in|during|the\s+week)|no\s+\w+\s+in|apart\s+from|other\s+than)\b[^,;.]*"
    );
    let mut position = 0;
    while let Some(m) = exception.find_at(line, position) {
        let end = exception_end(line, m.start(), m.end());
        out.push((PhraseKind::Exception, m.start(), end));
        position = end.max(m.start() + 1);
        while position < line.len() && !line.is_char_boundary(position) {
            position += 1;
        }
        if position >= line.len() {
            break;
        }
    }
    let sub_daily = out.iter().any(|(k, s, e)| {
        *k == PhraseKind::Rule
            && re!(r"(?i)minute|mins?\b|hour|hrs?\b|hourly").is_match(&line[*s..*e])
    });
    if sub_daily {
        for m in re_fmt!(r"(?i)\b(?:on\s+)?(?:week\s*days|weekends|work\s*days|{PLURAL_WEEKDAY}|mon(?:day)?\s*(?:-|–|to|thru|through)\s*fri(?:day)?)\b").find_iter(line) {
            out.push((PhraseKind::Window, m.start(), m.end()));
        }
        for m in re!(r"(?i)\b\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)\s*(?:-|–|to|until|till)\s*\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)|\b(?:[01]\d|2[0-3]):?[0-5]\d\s*(?:-|–|to|until|till)\s*(?:[01]\d|2[0-3]):?[0-5]\d\b").find_iter(line) {
            out.push((PhraseKind::Window, m.start(), m.end()));
        }
    }
    for m in re!(r"(?i)\bfor\s+(?:the\s+next\s+)?(?:\d+|an?|one|two|three|four|five|six|seven|eight|nine|ten|twelve)\s+(?:days?|weeks?|months?|years?)\b|\b(?:for\s+)?(?:the\s+next\s+)?(?:\d+|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|fifteen|twenty)\s+(?:more\s+)?(?:times|sessions|classes|lessons|occurrences|appointments|meetings|visits|rounds|doses)\b|\b(?:between|from)\s+\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)?\s*(?:and|to|-|–)\s*\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)?|\b(?:during\s+)?(?:work(?:ing)?|business|office)\s+hours\b").find_iter(line) {
        out.push((PhraseKind::Window, m.start(), m.end()));
    }
    out
}

fn exception_end(line: &str, start: usize, end: usize) -> usize {
    let text = &line[start..end];
    let mut cut = text.len();
    if let Some(m) = re!(r"(?i)\s(?:until|till|through|thru|starting|beginning)\s").find(text) {
        cut = cut.min(m.start());
    }
    if let Some(m) = re!(r"(?i)\s(?:but|unless)\s").find(&text[1.min(text.len())..]) {
        cut = cut.min(m.start() + 1);
    }
    for m in re!(r"(?i)\s(?:and|then\s+also)\s+([a-z']+)").captures_iter(text) {
        let (Some(whole), Some(word)) = (m.get(0), m.get(1)) else {
            continue;
        };
        let w = word.as_str().to_lowercase();
        let datey = w.chars().next().is_some_and(|c| c.is_ascii_digit())
            || parse_weekday(&w).is_some()
            || matches!(
                w.as_str(),
                "the"
                    | "on"
                    | "every"
                    | "weekends"
                    | "holidays"
                    | "christmas"
                    | "thanksgiving"
                    | "easter"
                    | "new"
            )
            || re_fmt!(r"(?i)^(?:{MONTH})$").is_match(&w);
        if !datey {
            cut = cut.min(whole.start());
            break;
        }
    }
    start + text[..cut].trim_end().len()
}

pub fn occurrence_dates(
    rule: &Rule,
    dtstart: NaiveDate,
    week_start_day: Weekday,
    last: NaiveDate,
) -> Vec<NaiveDate> {
    if let Some(holiday) = rule.holiday {
        return (dtstart.year()..=last.year())
            .filter_map(|y| holiday.date_in(y))
            .filter(|d| *d >= dtstart && *d <= last)
            .collect();
    }
    let mut out = Vec::new();
    let mut date = dtstart;
    while date <= last && out.len() < 5000 {
        if matches(rule, dtstart, date, week_start_day) {
            out.push(date);
        }
        let Some(next) = date.succ_opt() else { break };
        date = next;
    }
    out
}

pub fn step_minutes(rule: &Rule) -> i64 {
    let unit = if rule.frequency == Frequency::Minutely {
        1
    } else {
        60
    };
    i64::from(rule.interval.max(1)) * unit
}

fn allowed_day(rule: &Rule, day: NaiveDate) -> bool {
    rule.weekdays.is_empty()
        || rule
            .weekdays
            .iter()
            .any(|d| d.position.is_none() && d.weekday == day.weekday())
}

fn day_slots(rule: &Rule, day: NaiveDate) -> Vec<NaiveDateTime> {
    if !allowed_day(rule, day) {
        return Vec::new();
    }
    let (start, end) = match rule.window {
        Some(w) => w,
        None => (
            NaiveTime::MIN,
            NaiveTime::from_hms_opt(23, 59, 0).unwrap_or(NaiveTime::MIN),
        ),
    };
    let step = step_minutes(rule);
    let mut out = Vec::new();
    let mut moment = day.and_time(start);
    let last = day.and_time(end);
    while moment <= last && out.len() < 1500 {
        out.push(moment);
        moment += Duration::minutes(step);
    }
    out
}

pub fn first_slot(
    rule: &Rule,
    now: NaiveDateTime,
    anchor: Option<NaiveDateTime>,
) -> Option<NaiveDateTime> {
    let step = Duration::minutes(step_minutes(rule));
    let target = now.checked_add_signed(step)?;
    let target =
        target
            .date()
            .and_time(NaiveTime::from_hms_opt(target.hour(), target.minute(), 0)?);
    if rule.window.is_some() || !rule.weekdays.is_empty() {
        let mut day = target.date();
        for _ in 0..9 {
            if let Some(slot) = day_slots(rule, day).into_iter().find(|s| *s >= target) {
                return Some(slot);
            }
            day = day.succ_opt()?;
        }
        return None;
    }
    match anchor {
        Some(base) if base >= now => Some(base),
        Some(base) => {
            let steps = (now - base).num_minutes() / step.num_minutes().max(1) + 1;
            base.checked_add_signed(step * i32::try_from(steps).unwrap_or(i32::MAX))
        }
        None => Some(target),
    }
}

pub fn slot_occurrences(
    rule: &Rule,
    first: NaiveDateTime,
    last: NaiveDateTime,
    limit: usize,
) -> Vec<NaiveDateTime> {
    let mut out = Vec::new();
    if rule.window.is_some() || !rule.weekdays.is_empty() {
        let mut day = first.date();
        while day <= last.date() && out.len() < limit {
            out.extend(
                day_slots(rule, day)
                    .into_iter()
                    .filter(|s| *s >= first && *s <= last),
            );
            let Some(next) = day.succ_opt() else { break };
            day = next;
        }
    } else {
        let step = Duration::minutes(step_minutes(rule));
        let mut moment = first;
        while moment <= last && out.len() < limit {
            out.push(moment);
            moment += step;
        }
    }
    out.truncate(limit);
    out
}
