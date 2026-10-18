use std::sync::LazyLock;

use regex::Regex;

use crate::facets::holidays::{find_holidays, Holiday, HolidayReach, HolidayScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Daypart {
    Meal(Activity, bool),
    Afternoon,
    Lunch,
    AfterWork,
    EndOfDay,
    FirstThing,
    AfterLunch,
    BeforeLunch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Breakfast,
    Brunch,
    Lunch,
    AfternoonTea,
    Dinner,
    Drinks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeasonModifier {
    This,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meridiem {
    Am,
    Pm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeekdayWord {
    This,
    Next,
    Last,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Weekday(chrono::Weekday, Option<WeekdayWord>),
    ThisWeek,
    ClockRange {
        start: (u32, u32, Option<Meridiem>),
        end: (u32, u32, Option<Meridiem>),
        contextual: bool,
    },
    Holiday(Holiday, Option<i32>, i64, HolidayReach),
    Zone(chrono_tz::Tz, String),
    OrdinalDay(u32),
    OrdinalRange(u32, u32),
    EndOfWeek,
    EndOfMonth,
    NextWeek,
    EarlyNextMonth,
    EndOfNextMonth,
    MidMonth(bool),
    OrdinalNextMonth(u32),
    NthWeekdayInMonth(i8, chrono::Weekday, u32),
    WeekdayRange(chrono::Weekday, chrono::Weekday),
    AllDay,
    Season(Season, SeasonModifier),
    Daypart(Daypart),
    Clock {
        hour: u32,
        minute: u32,
        meridiem: Option<Meridiem>,
        approximate: bool,
    },
    AddMinutes(i64),
    AddDays(i64),
    AddWeekdays(i64),
    Duration(i64),
    EndClock {
        hour: u32,
        minute: u32,
        meridiem: Option<Meridiem>,
    },
    LeadMinutes(i64),
    AlertOff,
    AlertAt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub start: usize,
    pub end: usize,
    pub token: Token,
}

pub const NUMBER_WORDS: &[(&str, i64)] = &[
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("fifteen", 15),
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("forty-five", 45),
    ("ninety", 90),
];

const HOUR_WORD: &str = "one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve";
const COUNT: &str = r"\d+(?:\.\d+)?|an?|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|fifteen|twenty|thirty|forty-five|forty|ninety|a\s+couple\s+of|a\s+few|half\s+an?|a\s+half";

const MAX_MINUTES: f64 = 5_259_600.0;

fn bounded_minutes(value: f64) -> i64 {
    value.clamp(-MAX_MINUTES, MAX_MINUTES).round() as i64
}

pub fn number_value(text: &str) -> Option<f64> {
    let lower = text.trim().to_lowercase();
    let lower = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Ok(v) = lower.parse::<f64>() {
        return Some(v);
    }
    match lower.as_str() {
        "a" | "an" => Some(1.0),
        "half a" | "half an" | "a half" => Some(0.5),
        "a couple of" => Some(2.0),
        "a few" => Some(3.0),
        _ => NUMBER_WORDS
            .iter()
            .find(|(w, _)| *w == lower)
            .map(|(_, v)| *v as f64),
    }
}

pub fn unit_minutes(unit: &str) -> Option<f64> {
    let u = unit.to_lowercase();
    if u.starts_with("min") || u == "m" {
        Some(1.0)
    } else if u.starts_with('h') {
        Some(60.0)
    } else if u.starts_with("day") || u == "d" {
        Some(1440.0)
    } else if u.starts_with("week") || u == "w" {
        Some(10_080.0)
    } else if u.starts_with("fortnight") {
        Some(20_160.0)
    } else {
        None
    }
}

fn hour_value(text: &str) -> Option<u32> {
    text.parse::<u32>()
        .ok()
        .or_else(|| number_value(text).map(|v| v as u32))
}

fn meridiem_of(text: Option<&str>) -> Option<Meridiem> {
    let t = text?.to_lowercase();
    if t.starts_with('a') {
        Some(Meridiem::Am)
    } else if t.starts_with('p') {
        Some(Meridiem::Pm)
    } else {
        None
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

const MONTH_WORDS: &str = r"jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec|january|february|march|april|june|july|august|september|october|november|december";

fn ordinal_followed_by_month(text: &str, end: usize) -> bool {
    let rest = text[end..].to_lowercase();
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("of").map(str::trim_start).unwrap_or(rest);
    MONTH_WORDS.split('|').any(|m| {
        rest.starts_with(m)
            && !rest[m.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
    })
}

pub fn scan(text: &str, scope: HolidayScope) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut push =
        |start: usize, end: usize, token: Token| pieces.push(Piece { start, end, token });

    for h in find_holidays(text, scope) {
        let before = &text[..h.start];
        let offset = re!(r"(?i)\b(?:the\s+)?day\s+(after|before)\s*$").captures(before);
        match offset {
            Some(c) => {
                let shift = if c
                    .get(1)
                    .is_some_and(|m| m.as_str().eq_ignore_ascii_case("after"))
                {
                    1
                } else {
                    -1
                };
                let start = c.get(0).map_or(h.start, |m| m.start());
                push(
                    start,
                    h.end,
                    Token::Holiday(h.holiday, h.year, shift, h.reach),
                );
            }
            None => push(
                h.start,
                h.end,
                Token::Holiday(h.holiday, h.year, 0, h.reach),
            ),
        }
    }
    for c in
        re!(r"(?i)\b(?:the\s+)?day\s+(after|before)\s+(tomorrow|yesterday)\b").captures_iter(text)
    {
        let (Some(whole), Some(rel), Some(base)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        let base = if base.as_str().eq_ignore_ascii_case("tomorrow") {
            1
        } else {
            -1
        };
        let shift = if rel.as_str().eq_ignore_ascii_case("after") {
            1
        } else {
            -1
        };
        push(whole.start(), whole.end(), Token::AddDays(base + shift));
    }

    let zones: [(&str, chrono_tz::Tz); 20] = [
        ("london", chrono_tz::Europe::London),
        ("uk", chrono_tz::Europe::London),
        ("new york", chrono_tz::America::New_York),
        ("nyc", chrono_tz::America::New_York),
        ("paris", chrono_tz::Europe::Paris),
        ("berlin", chrono_tz::Europe::Berlin),
        ("tokyo", chrono_tz::Asia::Tokyo),
        ("japan", chrono_tz::Asia::Tokyo),
        ("sydney", chrono_tz::Australia::Sydney),
        ("chicago", chrono_tz::America::Chicago),
        ("la", chrono_tz::America::Los_Angeles),
        ("los angeles", chrono_tz::America::Los_Angeles),
        ("denver", chrono_tz::America::Denver),
        ("india", chrono_tz::Asia::Kolkata),
        ("singapore", chrono_tz::Asia::Singapore),
        ("dubai", chrono_tz::Asia::Dubai),
        ("pacific", chrono_tz::America::Los_Angeles),
        ("eastern", chrono_tz::America::New_York),
        ("central", chrono_tz::America::Chicago),
        ("mountain", chrono_tz::America::Denver),
    ];
    let zone_words = re!(
        r"(?i)\b(?:in\s+)?(london|uk|new\s+york|nyc|paris|berlin|tokyo|japan|sydney|chicago|la|los\s+angeles|denver|india|singapore|dubai|pacific|eastern|central|mountain)(\s+(?:standard\s+|daylight\s+)?time)?\b"
    );
    for c in zone_words.captures_iter(text) {
        let (Some(whole), Some(name)) = (c.get(0), c.get(1)) else {
            continue;
        };
        let lower = name.as_str().to_lowercase();
        let lower = lower.split_whitespace().collect::<Vec<_>>().join(" ");
        let regional = matches!(
            lower.as_str(),
            "pacific" | "eastern" | "central" | "mountain"
        );
        let after_clock = re!(r"(?i)(?:\d|am|pm|a\.m\.|p\.m\.|noon|midnight)\s*$")
            .is_match(&text[..whole.start()]);
        if c.get(2).is_none() && !(regional && after_clock) {
            continue;
        }
        if let Some((_, tz)) = zones.iter().find(|(n, _)| *n == lower) {
            push(
                whole.start(),
                whole.end(),
                Token::Zone(*tz, whole.as_str().trim().to_string()),
            );
        }
    }

    let four_digit = re!(
        r"(?i)\b(?:(at|by|from|until|till|before|after|around|@)\s+)?([01]\d|2[0-3])([0-5]\d)(\s*(?:h\b|hrs\b|hours\b))?"
    );
    for c in four_digit.captures_iter(text) {
        let (Some(whole), Some(h), Some(m)) = (c.get(0), c.get(2), c.get(3)) else {
            continue;
        };
        if c.get(1).is_none() && c.get(4).is_none() {
            continue;
        }
        let after = &text[whole.end()..];
        if re!(r"^\s*(?:-|–|to\b|until\b)\s*\d").is_match(after) || re!(r"^[\d/]").is_match(after)
        {
            continue;
        }
        let (Ok(hour), Ok(minute)) = (h.as_str().parse::<u32>(), m.as_str().parse::<u32>()) else {
            continue;
        };
        let meridiem = Some(if hour < 12 {
            Meridiem::Am
        } else {
            Meridiem::Pm
        });
        push(
            whole.start(),
            whole.end(),
            Token::Clock {
                hour,
                minute,
                meridiem,
                approximate: false,
            },
        );
    }
    let h_suffix =
        re!(r"(?i)\b(?:(at|by|from|until|till|before|after|à)\s+)?([01]?\d|2[0-3])h([0-5]\d)?\b");
    for c in h_suffix.captures_iter(text) {
        let (Some(whole), Some(h)) = (c.get(0), c.get(2)) else {
            continue;
        };
        let Ok(hour) = h.as_str().parse::<u32>() else {
            continue;
        };
        if c.get(1).is_none() && c.get(3).is_none() && hour < 13 {
            continue;
        }
        let minute = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let meridiem = Some(if hour < 12 {
            Meridiem::Am
        } else {
            Meridiem::Pm
        });
        push(
            whole.start(),
            whole.end(),
            Token::Clock {
                hour,
                minute,
                meridiem,
                approximate: false,
            },
        );
    }
    let four_range = re!(
        r"(?i)\b(?:(from|between)\s+)?([01]\d|2[0-3])([0-5]\d)\s*(?:h|hrs)?\s*(?:-|–|to|until|till|and)\s*([01]\d|2[0-3])([0-5]\d)\s*(?:h\b|hrs\b)?"
    );
    for c in four_range.captures_iter(text) {
        let (Some(whole), Some(sh), Some(sm), Some(eh), Some(em)) =
            (c.get(0), c.get(2), c.get(3), c.get(4), c.get(5))
        else {
            continue;
        };
        let parse = |m: regex::Match| m.as_str().parse::<u32>().unwrap_or(0);
        let (sh, sm, eh, em) = (parse(sh), parse(sm), parse(eh), parse(em));
        let mer = |h: u32| Some(if h < 12 { Meridiem::Am } else { Meridiem::Pm });
        push(
            whole.start(),
            whole.end(),
            Token::ClockRange {
                start: (sh, sm, mer(sh)),
                end: (eh, em, mer(eh)),
                contextual: true,
            },
        );
    }

    let range = re!(
        r"(?i)\b(?:from\s+)?the\s+(\d{1,2})(?:st|nd|rd|th)\s*(?:to|until|till|through|-|–)\s*(?:the\s+)?(\d{1,2})(?:st|nd|rd|th)\b"
    );
    let mut range_spans = Vec::new();
    for c in range.captures_iter(text) {
        let (Some(whole), Some(a), Some(b)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        if ordinal_followed_by_month(text, whole.end()) {
            continue;
        }
        if let (Ok(a), Ok(b)) = (a.as_str().parse(), b.as_str().parse()) {
            range_spans.push((whole.start(), whole.end()));
            push(whole.start(), whole.end(), Token::OrdinalRange(a, b));
        }
    }

    let ordinal =
        re!(r"(?i)\b(?:(?:on|by|before|until|till|for)\s+)?the\s+(\d{1,2})(?:st|nd|rd|th)\b");
    for c in ordinal.captures_iter(text) {
        let (Some(whole), Some(day)) = (c.get(0), c.get(1)) else {
            continue;
        };
        if ordinal_followed_by_month(text, whole.end())
            || range_spans
                .iter()
                .any(|(s, e)| whole.start() < *e && *s < whole.end())
        {
            continue;
        }
        if let Ok(day) = day.as_str().parse() {
            push(whole.start(), whole.end(), Token::OrdinalDay(day));
        }
    }

    for m in
        re!(r"(?i)\b(?:(?:by|before|at|until)\s+)?(?:the\s+)?(?:end\s+of\s+(?:the\s+)?week|eow)\b")
            .find_iter(text)
    {
        push(m.start(), m.end(), Token::EndOfWeek);
    }
    for m in re!(r"(?i)\b(?:(?:by|before|at|until|on|sometime)\s+)?(?:the\s+)?(?:end\s+of\s+(?:the\s+|this\s+)?month|eom|last\s+day\s+of\s+(?:the|this)\s+month|this\s+month)\b").find_iter(text) {
        push(m.start(), m.end(), Token::EndOfMonth);
    }
    for m in re!(r"(?i)\b(?:early|beginning\s+of|start\s+of)\s+next\s+month\b").find_iter(text) {
        push(m.start(), m.end(), Token::EarlyNextMonth);
    }
    for m in
        re!(r"(?i)\b(?:(?:by|before|during|sometime|some\s+time)\s+)?next\s+week\b").find_iter(text)
    {
        if !text[m.end()..]
            .trim_start()
            .to_lowercase()
            .starts_with("end")
        {
            push(m.start(), m.end(), Token::NextWeek);
        }
    }

    let seasons = re!(
        r"(?i)\b(?:(this|next|in\s+the|in|before|by|until|after|during|over\s+the|over)\s+)?(spring|summer|autumn|fall|winter)\b"
    );
    for c in seasons.captures_iter(text) {
        let (Some(whole), Some(name)) = (c.get(0), c.get(2)) else {
            continue;
        };
        let modifier = match c.get(1).map(|m| m.as_str().to_lowercase()) {
            Some(m) if m == "next" => SeasonModifier::Next,
            Some(_) => SeasonModifier::This,
            None => continue,
        };
        let season = match name.as_str().to_lowercase().as_str() {
            "spring" => Season::Spring,
            "summer" => Season::Summer,
            "winter" => Season::Winter,
            _ => Season::Autumn,
        };
        push(whole.start(), whole.end(), Token::Season(season, modifier));
    }

    let dayparts: [(&Regex, Daypart); 7] = [
        (re!(r"(?i)\bafter\s+work\b"), Daypart::AfterWork),
        (
            re!(
                r"(?i)\b(?:(?:by|before|at)\s+)?(?:the\s+)?(?:end\s+of\s+(?:the\s+)?(?:work\s*)?day|eod|cob|close\s+of\s+business)\b"
            ),
            Daypart::EndOfDay,
        ),
        (
            re!(r"(?i)\bfirst\s+thing(?:\s+in\s+the\s+morning)?\b"),
            Daypart::FirstThing,
        ),
        (re!(r"(?i)\bafter\s+lunch(?:time)?\b"), Daypart::AfterLunch),
        (
            re!(r"(?i)\bbefore\s+lunch(?:time)?\b"),
            Daypart::BeforeLunch,
        ),
        (
            re!(r"(?i)\b(?:mid-?\s?afternoon|arvo)\b"),
            Daypart::Afternoon,
        ),
        (
            re!(r"(?i)\b(?:at\s+)?lunch\s*time\b|\bat\s+lunch\b"),
            Daypart::Lunch,
        ),
    ];
    for (regex, daypart) in dayparts {
        for m in regex.find_iter(text) {
            push(m.start(), m.end(), Token::Daypart(daypart));
        }
    }

    let quarter = re!(
        r"(?i)\b(?:at\s+)?(quarter|half)\s+(past|after|to|til|till)\s+(\d{1,2}|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)(?:\s*(a\.?m\.?|p\.?m\.?))?\b"
    );
    for c in quarter.captures_iter(text) {
        let (Some(whole), Some(kind), Some(rel), Some(hour)) =
            (c.get(0), c.get(1), c.get(2), c.get(3))
        else {
            continue;
        };
        let Some(hour) = hour_value(hour.as_str()).filter(|h| (1..=24).contains(h)) else {
            continue;
        };
        let forward = matches!(rel.as_str().to_lowercase().as_str(), "past" | "after");
        let half = kind.as_str().eq_ignore_ascii_case("half");
        if half && !forward {
            continue;
        }
        let (hour, minute) = match (half, forward) {
            (true, _) => (hour, 30),
            (false, true) => (hour, 15),
            (false, false) => (if hour == 1 { 12 } else { hour - 1 }, 45),
        };
        push(
            whole.start(),
            whole.end(),
            Token::Clock {
                hour,
                minute,
                meridiem: meridiem_of(c.get(4).map(|m| m.as_str())),
                approximate: false,
            },
        );
    }

    let approx_before = re!(
        r"(?i)\b(?:around|about|approximately|approx\.?|roughly|circa|~)\s*(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?(?:\s*-?\s*ish)?\b"
    );
    let approx_after =
        re!(r"(?i)\b(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?\s*-?\s*ish\b");
    for regex in [approx_before, approx_after] {
        for c in regex.captures_iter(text) {
            let (Some(whole), Some(hour)) = (c.get(0), c.get(1)) else {
                continue;
            };
            let after = &text[whole.end()..];
            if re!(r"(?i)^\s*(?:minutes?|mins?|hours?|hrs?|days?|weeks?|months?|years?|people|%)\b")
                .is_match(after)
            {
                continue;
            }
            let Ok(hour) = hour.as_str().parse::<u32>() else {
                continue;
            };
            let minute = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            if hour > 24 || minute > 59 {
                continue;
            }
            push(
                whole.start(),
                whole.end(),
                Token::Clock {
                    hour,
                    minute,
                    meridiem: meridiem_of(c.get(3).map(|m| m.as_str())),
                    approximate: true,
                },
            );
        }
    }

    let hour_words = re_fmt!(
        r"(?i)\b(?:at|by|before|until)\s+({HOUR_WORD})(?:\s*(?:o['’ʼ]?\s?clock))?(?:\s*(a\.?m\.?|p\.?m\.?))?\b"
    );
    for c in hour_words.captures_iter(text) {
        let (Some(whole), Some(hour)) = (c.get(0), c.get(1)) else {
            continue;
        };
        if let Some(hour) = hour_value(hour.as_str()) {
            push(
                whole.start(),
                whole.end(),
                Token::Clock {
                    hour,
                    minute: 0,
                    meridiem: meridiem_of(c.get(2).map(|m| m.as_str())),
                    approximate: false,
                },
            );
        }
    }

    for m in re!(r"(?i)\bin\s+(?:a\s+half|half\s+an?|half)\s+hour\b").find_iter(text) {
        push(m.start(), m.end(), Token::AddMinutes(30));
    }
    for c in re!(r"(?i)\bin\s+(an?|one|two|three|\d+)\s+hours?\s+and\s+a\s+half\b|\bin\s+(\d+)\s+and\s+a\s+half\s+hours\b").captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        let hours = c.get(1).or(c.get(2)).and_then(|m| number_value(m.as_str())).unwrap_or(1.0);
        push(whole.start(), whole.end(), Token::AddMinutes(bounded_minutes(hours * 60.0 + 30.0)));
    }

    let weekday = re!(
        r"(?i)\b(?:(this\s+coming|this|next|last|past|coming|on)\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues?|weds?|thur?s?|fri|sat|sun)\b\.?"
    );
    for c in weekday.captures_iter(text) {
        let (Some(whole), Some(day)) = (c.get(0), c.get(2)) else {
            continue;
        };
        let word = day.as_str();
        let after = &text[whole.end()..];
        if after.starts_with(['s', 'S']) || after.starts_with('\'') {
            continue;
        }
        let lower_word = word.to_lowercase();
        if (lower_word == "sat" || lower_word == "sun")
            && !lookalike_is_day(text, day.start(), whole.end())
        {
            continue;
        }
        let Some(weekday) = crate::facets::recurrence::parse_weekday(word) else {
            continue;
        };
        let modifier = match c.get(1).map(|m| m.as_str().to_lowercase()) {
            Some(m) if m == "next" => Some(WeekdayWord::Next),
            Some(m) if m == "last" || m == "past" => Some(WeekdayWord::Last),
            Some(m) if m.starts_with("this") || m == "coming" => Some(WeekdayWord::This),
            _ => None,
        };
        push(
            whole.start(),
            whole.end(),
            Token::Weekday(weekday, modifier),
        );
    }
    for m in re!(r"(?i)\b(?:sometime\s+)?this\s+week\b").find_iter(text) {
        push(m.start(), m.end(), Token::ThisWeek);
    }

    let clock_range = re!(
        r"(?i)\b(from|between|at|@)?\s*(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?\s*(?:-|–|to|until|till|and)\s*(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?(?:\b|$)"
    );
    for c in clock_range.captures_iter(text) {
        let (Some(whole), Some(sh), Some(eh)) = (c.get(0), c.get(2), c.get(5)) else {
            continue;
        };
        let after = &text[whole.end()..];
        if re!(r"^\s*(?:[-/.]\d|\d|%|st\b|nd\b|rd\b|th\b|minutes?|mins?|hours?|hrs?|days?|weeks?|months?|years?|people|guests|pm\b|am\b|x\b|of\b)").is_match(after) {
            continue;
        }
        let before = &text[..sh.start()];
        if re!(r"[-/.]\s*$|\d\s*$").is_match(before) {
            continue;
        }
        let (Ok(sh_v), Ok(eh_v)) = (sh.as_str().parse::<u32>(), eh.as_str().parse::<u32>()) else {
            continue;
        };
        let sm = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        let em = c.get(6).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        if sh_v > 24 || eh_v > 24 || sm > 59 || em > 59 {
            continue;
        }
        let smer = meridiem_of(c.get(4).map(|m| m.as_str()));
        let emer = meridiem_of(c.get(7).map(|m| m.as_str()));
        let has_minutes = c.get(3).is_some() || c.get(6).is_some();
        let joiner_is_and = whole.as_str().to_lowercase().contains(" and ")
            && c.get(1).map(|m| m.as_str().to_lowercase()) != Some("between".to_string());
        if joiner_is_and {
            continue;
        }
        let lead_context = re!(r"(?i)(?:\b(?:mon|tue|wed|thu|fri|sat|sun)[a-z]*|today|tomorrow|tonight|morning|afternoon|evening|lunch|dinner|\d(?:st|nd|rd|th))\.?,?\s*$").is_match(before);
        let contextual =
            smer.is_some() || emer.is_some() || c.get(1).is_some() || has_minutes || lead_context;
        let start = c.get(1).map_or(sh.start(), |m| m.start());
        push(
            start,
            whole.end(),
            Token::ClockRange {
                start: (sh_v, sm, smer),
                end: (eh_v, em, emer),
                contextual,
            },
        );
    }

    let meal = re!(
        r"(?i)\b(before|after|at|by|for|over)\s+(breakfast|brunch|lunch|dinner|supper|drinks)\b"
    );
    for c in meal.captures_iter(text) {
        let (Some(whole), Some(word), Some(food)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        let activity = match food.as_str().to_lowercase().as_str() {
            "breakfast" => Activity::Breakfast,
            "brunch" => Activity::Brunch,
            "lunch" => Activity::Lunch,
            "drinks" => Activity::Drinks,
            _ => Activity::Dinner,
        };
        let after = word.as_str().eq_ignore_ascii_case("after");
        push(
            whole.start(),
            whole.end(),
            Token::Daypart(Daypart::Meal(activity, after)),
        );
    }

    let ordinal_words = re!(
        r"(?i)\b(?:on\s+|by\s+|before\s+)?the\s+(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|fifteenth|twentieth|thirtieth|last)\s+of\s+(?:the|this|next)\s+month\b"
    );
    for c in ordinal_words.captures_iter(text) {
        let (Some(whole), Some(word)) = (c.get(0), c.get(1)) else {
            continue;
        };
        let day = match word.as_str().to_lowercase().as_str() {
            "first" => 1,
            "second" => 2,
            "third" => 3,
            "fourth" => 4,
            "fifth" => 5,
            "sixth" => 6,
            "seventh" => 7,
            "eighth" => 8,
            "ninth" => 9,
            "tenth" => 10,
            "fifteenth" => 15,
            "twentieth" => 20,
            "thirtieth" => 30,
            _ => {
                push(whole.start(), whole.end(), Token::EndOfMonth);
                continue;
            }
        };
        push(whole.start(), whole.end(), Token::OrdinalDay(day));
    }

    let by_hour = re!(
        r"(?i)\b(by|before|until|till|after|to|ending\s+at|ends\s+at)\s+(\d{1,2})(?:[:.](\d{2}))?\s*(a\.?m\.?|p\.?m\.?)?(?:\b|$)"
    );
    for c in by_hour.captures_iter(text) {
        let (Some(whole), Some(word), Some(hour)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        let after = &text[whole.end()..];
        if re!(r"(?i)^\s*(?:[-/.:]\d|\d|%|st\b|nd\b|rd\b|th\b|minutes?|mins?|hours?|hrs?|days?|weeks?|months?|years?|people|of\b|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec|-|–|to\b)").is_match(after) {
            continue;
        }
        let Ok(hour) = hour.as_str().parse::<u32>() else {
            continue;
        };
        let minute = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        if hour > 24 || minute > 59 {
            continue;
        }
        let meridiem = meridiem_of(c.get(4).map(|m| m.as_str()));
        let ends = matches!(
            word.as_str().to_lowercase().split_whitespace().next(),
            Some("to" | "until" | "till" | "ending" | "ends")
        );
        let token = if ends {
            Token::EndClock {
                hour,
                minute,
                meridiem,
            }
        } else {
            Token::Clock {
                hour,
                minute,
                meridiem,
                approximate: false,
            }
        };
        push(whole.start(), whole.end(), token);
    }

    let fortnight = re_fmt!(
        r"(?i)\b(?:in\s+({COUNT})\s+fortnights?|({COUNT})\s+fortnights?\s+(?:from\s+(?:now|today)|later))\b"
    );
    for c in fortnight.captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        let count = c
            .get(1)
            .or(c.get(2))
            .and_then(|m| number_value(m.as_str()))
            .unwrap_or(1.0);
        push(
            whole.start(),
            whole.end(),
            Token::AddDays((count * 14.0) as i64),
        );
    }
    let weekdays = re_fmt!(
        r"(?i)\bin\s+({COUNT})\s+(?:week\s*days?|business\s+days?|working\s+days?|work\s*days?)\b"
    );
    for c in weekdays.captures_iter(text) {
        let (Some(whole), Some(n)) = (c.get(0), c.get(1)) else {
            continue;
        };
        if let Some(n) = number_value(n.as_str()) {
            push(
                whole.start(),
                whole.end(),
                Token::AddWeekdays(bounded_minutes(n * 1440.0) / 1440),
            );
        }
    }

    let duration = re_fmt!(
        r"(?i)\bfor\s+(?:about\s+|around\s+|roughly\s+)?({COUNT})\s*(minutes?|mins?|hours?|hrs?|h)\b(?:\s+and\s+(?:a\s+)?(half|\d+\s*(?:minutes?|mins?)))?"
    );
    for c in duration.captures_iter(text) {
        let (Some(whole), Some(n), Some(unit)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        let (Some(n), Some(per)) = (number_value(n.as_str()), unit_minutes(unit.as_str())) else {
            continue;
        };
        let extra = match c.get(3).map(|m| m.as_str().to_lowercase()) {
            Some(h) if h == "half" => per / 2.0,
            Some(m) => m
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0),
            None => 0.0,
        };
        push(
            whole.start(),
            whole.end(),
            Token::Duration((n * per + extra).round() as i64),
        );
    }

    let lead = re_fmt!(
        r"(?i)\b(?:({COUNT})\s*(minutes?|mins?|hours?|hrs?|h|days?|weeks?)|(the)\s+(day|night|week)|(half\s+an?|a\s+half)\s+(hour))\s+(?:before(?:hand)?|ahead|prior|earlier|early|in\s+advance)\b"
    );
    for c in lead.captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        let minutes = if let (Some(n), Some(unit)) = (c.get(1), c.get(2)) {
            number_value(n.as_str())
                .zip(unit_minutes(unit.as_str()))
                .map(|(n, u)| n * u)
        } else if let Some(unit) = c.get(4) {
            match unit.as_str().to_lowercase().as_str() {
                "week" => Some(10_080.0),
                _ => Some(1440.0),
            }
        } else if c.get(5).is_some() {
            Some(30.0)
        } else {
            None
        };
        if let Some(minutes) = minutes {
            push(
                whole.start(),
                whole.end(),
                Token::LeadMinutes(minutes.round() as i64),
            );
        }
    }

    for m in re!(r"(?i)\b(?:no\s+(?:alerts?|reminders?|notifications?)|alerts?\s+off|(?:turn|switch)\s+off\s+(?:the\s+)?alerts?|without\s+(?:an?\s+)?(?:alert|reminder|notification)s?|don['’]?t\s+(?:alert|notify)\s+me|silent)\b").find_iter(text) {
        push(m.start(), m.end(), Token::AlertOff);
    }
    for m in re!(r"(?i)\b(?:alert|notify|ping|remind)\s+(?:me\s+)?at\b").find_iter(text) {
        push(m.start(), m.end(), Token::AlertAt);
    }

    for m in re!(r"(?i)\b(?:(?:by|before|until|at|on)\s+)?(?:the\s+)?(?:end\s+of\s+next\s+month|late\s+next\s+month)\b").find_iter(text) {
        push(m.start(), m.end(), Token::EndOfNextMonth);
    }
    for c in re!(r"(?i)\b(?:mid-?\s?month|(?:the\s+)?middle\s+of\s+(?:the|this)\s+month|mid-?\s?(next)\s+month|(?:the\s+)?middle\s+of\s+(next)\s+month)\b").captures_iter(text) {
        let Some(whole) = c.get(0) else { continue };
        let next = c.get(1).is_some() || c.get(2).is_some();
        push(whole.start(), whole.end(), Token::MidMonth(next));
    }
    for c in re!(r"(?i)\b(?:on\s+|by\s+)?the\s+(\d{1,2})(?:st|nd|rd|th)\s+of\s+next\s+month\b")
        .captures_iter(text)
    {
        let (Some(whole), Some(day)) = (c.get(0), c.get(1)) else {
            continue;
        };
        if let Ok(day) = day.as_str().parse() {
            push(whole.start(), whole.end(), Token::OrdinalNextMonth(day));
        }
    }
    let ordinal_word = re!(
        r"(?i)\b(?:(?:on|by|before|until|till|for)\s+)?the\s+(twenty[\s-]first|twenty[\s-]second|twenty[\s-]third|twenty[\s-]fourth|twenty[\s-]fifth|twenty[\s-]sixth|twenty[\s-]seventh|twenty[\s-]eighth|twenty[\s-]ninth|thirty[\s-]first|first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth|thirteenth|fourteenth|fifteenth|sixteenth|seventeenth|eighteenth|nineteenth|twentieth|thirtieth)\b"
    );
    for c in ordinal_word.captures_iter(text) {
        let (Some(whole), Some(word)) = (c.get(0), c.get(1)) else {
            continue;
        };
        let after = &text[whole.end()..];
        if re!(r"(?i)^\s*(?:of\s+)?(?:the\s+)?(?:month|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec|mon|tue|wed|thu|fri|sat|sun|week|time|one|item|day|hour|quarter|half|floor|place|draft|round|step|page|chapter|of\s+(?:every|each|next))").is_match(after) {
            continue;
        }
        if let Some(day) = ordinal_word_value(word.as_str()) {
            push(whole.start(), whole.end(), Token::OrdinalDay(day));
        }
    }
    let nth_in_month = re!(
        r"(?i)\b(?:the\s+|on\s+the\s+)?(1st|2nd|3rd|4th|5th|first|second|third|fourth|fifth|last)\s+(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues?|wed|thu|thurs|fri|sat|sun)\s+(?:in|of)\s+(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sept?(?:ember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\b"
    );
    for c in nth_in_month.captures_iter(text) {
        let (Some(whole), Some(n), Some(w), Some(m)) = (c.get(0), c.get(1), c.get(2), c.get(3))
        else {
            continue;
        };
        let position = match n.as_str().to_lowercase().as_str() {
            "1st" | "first" => 1,
            "2nd" | "second" => 2,
            "3rd" | "third" => 3,
            "4th" | "fourth" => 4,
            "5th" | "fifth" => 5,
            _ => -1,
        };
        let months = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
        ];
        let lower_m = m.as_str().to_lowercase();
        let (Some(weekday), Some(month)) = (
            crate::facets::recurrence::parse_weekday(w.as_str()),
            months.iter().position(|x| lower_m.starts_with(x)),
        ) else {
            continue;
        };
        push(
            whole.start(),
            whole.end(),
            Token::NthWeekdayInMonth(position, weekday, month as u32 + 1),
        );
    }
    let weekday_range = re!(
        r"(?i)\b(?:from\s+)?(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues?|weds?|thur?s?|fri|sat|sun)\.?\s*(?:to|thru|through|till|until|-|–)\s*(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon|tues?|weds?|thur?s?|fri|sat|sun)\b"
    );
    for c in weekday_range.captures_iter(text) {
        let (Some(whole), Some(a), Some(b)) = (c.get(0), c.get(1), c.get(2)) else {
            continue;
        };
        if let (Some(a), Some(b)) = (
            crate::facets::recurrence::parse_weekday(a.as_str()),
            crate::facets::recurrence::parse_weekday(b.as_str()),
        ) {
            if a != b {
                push(whole.start(), whole.end(), Token::WeekdayRange(a, b));
            }
        }
    }
    for m in re!(r"(?i)\b(?:all[\s-]day|for\s+(?:a|the)\s+(?:full|whole)\s+day)\b").find_iter(text)
    {
        push(m.start(), m.end(), Token::AllDay);
    }
    for m in re!(r"(?i)\bfor\s+half\s+a\s+day\b|\bhalf[\s-]day\b").find_iter(text) {
        push(m.start(), m.end(), Token::Duration(240));
    }
    for c in re!(r"(?i)(?:(\d|am|pm|a\.m\.|p\.m\.)\s+|\()(\d+(?:\.\d+)?)\s*(h|hrs?|hours?|mins?|minutes?)\b\)?(?:\s+long)?").captures_iter(text) {
        let (Some(whole), Some(n), Some(unit)) = (c.get(0), c.get(2), c.get(3)) else { continue };
        let lead_len = c.get(1).map_or(1, |m| m.end() - whole.start());
        let after = &text[whole.end()..];
        if re!(r"^\s*(?:before|ahead|prior|earlier|early|ago|later|from)").is_match(after) {
            continue;
        }
        let (Ok(n), Some(per)) = (n.as_str().parse::<f64>(), unit_minutes(unit.as_str())) else { continue };
        push(whole.start() + lead_len, whole.end(), Token::Duration(bounded_minutes(n * per)));
    }

    let shifted: Vec<(usize, usize)> = pieces
        .iter()
        .filter(|p| matches!(p.token, Token::Holiday(_, _, shift, _) if shift != 0))
        .map(|p| (p.start, p.end))
        .collect();
    pieces.retain(|p| {
        !(matches!(p.token, Token::LeadMinutes(_))
            && shifted.iter().any(|(a, b)| p.start < *b && *a < p.end))
    });
    pieces
}

pub fn line_daypart_meridiem(line: &str) -> Option<Meridiem> {
    let pm =
        re!(r"(?i)\b(?:afternoon|evening|tonight|tonite|night|dinner|supper|drinks|p\.?m\.?)\b");
    let am = re!(r"(?i)\b(?:morning|breakfast|first\s+thing|a\.?m\.?)\b");
    if pm.is_match(line) {
        Some(Meridiem::Pm)
    } else if am.is_match(line) {
        Some(Meridiem::Am)
    } else {
        None
    }
}

pub fn line_mentions_lunch(line: &str) -> bool {
    re!(r"(?i)\b(?:lunch|brunch|lunchtime)\b").is_match(line)
}

pub fn line_activity(line: &str) -> Option<Activity> {
    let entries: [(&Regex, Activity); 6] = [
        (re!(r"(?i)\bafternoon\s+tea\b"), Activity::AfternoonTea),
        (re!(r"(?i)\bbrunch\b"), Activity::Brunch),
        (re!(r"(?i)\bbreakfast\b"), Activity::Breakfast),
        (re!(r"(?i)\blunch\b"), Activity::Lunch),
        (re!(r"(?i)\b(?:dinner|supper)\b"), Activity::Dinner),
        (re!(r"(?i)\bdrinks\b"), Activity::Drinks),
    ];
    entries
        .into_iter()
        .find(|(regex, _)| regex.is_match(line))
        .map(|(_, activity)| activity)
}

pub fn task_signal(line: &str) -> Option<String> {
    let signal = re!(
        r"(?i)^\s*(todo\b|to-do\b|to\s+do\b|task\b|remember\s+to\b|remind\s+me\s+to\b|✓|✔|☑|\[\s?\]|\[x\])"
    );
    signal
        .captures(line)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
}

const TYPOS: &[(&str, &str)] = &[
    (
        r"wedn?s?e?sday|wednsday|wedensday|wensday|wednesay|wedsday|wenesday",
        "wednesday",
    ),
    (
        r"tomm?orr?ow|tomorow|tommorow|tommorrow|tomorrw|tomm?oro|tomorro|2moro|2morrow|2morow|tmorrow|tomrrow",
        "tomorrow",
    ),
    (r"saturady|satuday|saterday|satruday|saturdy", "saturday"),
    (r"thrusday|thurday|thursady|thursdy", "thursday"),
    (r"tuesay|tusday|teusday|tuseday", "tuesday"),
    (r"firday|fridya|fridy|frday", "friday"),
    (r"mondey|mondy|monady", "monday"),
    (r"sundya|sundy|sunady", "sunday"),
    (
        r"afternon|afternooon|aftrnoon|afternnoon|aftenoon",
        "afternoon",
    ),
    (r"evning|evenin|eveing", "evening"),
    (r"mornin|morining|moring|mornig", "morning"),
    (r"tonite|2nite|2night|tonigt|tonihgt|toniht", "tonight"),
    (r"tomoro|tomorow|tmrow|tmorow|2mrw|2mro", "tomorrow"),
    (r"thru", "through"),
    (r"weds", "wed"),
    (r"tues", "tue"),
    (r"2day|tday", "today"),
    (r"rn|rite\s+now", "now"),
    (r"nxt", "next"),
    (r"wknd|wkend|weeknd|wkd", "weekend"),
    (r"teh", "the"),
    (r"evry|evey|everyy|evrey", "every"),
];

#[derive(Clone, Debug)]
pub struct Normalized {
    pub text: String,
    map: Vec<usize>,
}

impl Normalized {
    pub fn to_original(&self, normalized: usize) -> usize {
        self.map
            .get(normalized)
            .copied()
            .unwrap_or_else(|| self.map.last().copied().unwrap_or(0))
    }

    pub fn to_normalized(&self, original: usize) -> usize {
        self.map
            .iter()
            .position(|&o| o >= original)
            .unwrap_or(self.text.len())
    }
}

fn replace_all(
    current: &Normalized,
    regex: &Regex,
    replacement: &dyn Fn(&regex::Captures) -> String,
) -> Normalized {
    let mut text = String::with_capacity(current.text.len());
    let mut map = Vec::with_capacity(current.map.len());
    let mut last = 0;
    for c in regex.captures_iter(&current.text) {
        let Some(whole) = c.get(0) else { continue };
        text.push_str(&current.text[last..whole.start()]);
        map.extend_from_slice(&current.map[last..whole.start()]);
        let new = replacement(&c);
        let origin = current.map[whole.start()];
        map.extend(std::iter::repeat_n(origin, new.len()));
        text.push_str(&new);
        last = whole.end();
    }
    text.push_str(&current.text[last..]);
    map.extend_from_slice(&current.map[last..]);
    Normalized { text, map }
}

pub fn normalize(line: &str) -> Normalized {
    let mut current = Normalized {
        text: line.to_string(),
        map: (0..=line.len()).collect(),
    };
    static COMPILED_TYPOS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
        TYPOS
            .iter()
            .map(|(pattern, word)| {
                let regex = Regex::new(&format!(r"(?i)\b(?:{pattern})\b"))
                    .unwrap_or_else(|e| panic!("typo: {e}"));
                (regex, *word)
            })
            .collect()
    });
    for (regex, word) in COMPILED_TYPOS.iter() {
        current = replace_all(&current, regex, &|_| (*word).to_string());
    }
    current = replace_all(&current, re!(r"@\s*(\d)"), &|c| format!("at {}", &c[1]));
    current = replace_all(
        &current,
        re!(r"(?i)\b(\d+|a|one|two|three|four|five|six|seven|eight|nine|ten|twelve)\s+wks?\b"),
        &|c| format!("{} weeks", &c[1]),
    );
    current = replace_all(
        &current,
        re!(r"(?i)\b(next|this|last|per|a|each|every|the|of\s+the|end\s+of\s+the|end\s+of)\s+wk\b"),
        &|c| format!("{} week", &c[1]),
    );
    current = replace_all(
        &current,
        re!(r"(?i)\b(\d{1,2}(?::\d{2})?)\s?([ap])\b"),
        &|c| format!("{}{}m", &c[1], &c[2]),
    );
    current = replace_all(
        &current,
        re!(r"(?i)\b(at|by|from|around)\s+half\s+(\d{1,2})\b"),
        &|c| format!("{} half past {}", &c[1], &c[2]),
    );
    current
}

fn ordinal_word_value(word: &str) -> Option<u32> {
    let w = word.to_lowercase().replace('-', " ");
    let w = w.split_whitespace().collect::<Vec<_>>().join(" ");
    let simple = [
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
        "twentieth",
    ];
    if let Some(i) = simple.iter().position(|x| *x == w) {
        return Some(i as u32 + 1);
    }
    if w == "thirtieth" {
        return Some(30);
    }
    let (tens, rest) = w.split_once(' ')?;
    let base = match tens {
        "twenty" => 20,
        "thirty" => 30,
        _ => return None,
    };
    let unit = simple.iter().take(9).position(|x| *x == rest)? as u32 + 1;
    Some(base + unit)
}

fn last_word(text: &str) -> String {
    text.trim_end()
        .rsplit(|c: char| c.is_whitespace())
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

pub fn lookalike_is_day(text: &str, start: usize, end: usize) -> bool {
    let before = &text[..start];
    let previous = last_word(before);
    if matches!(
        previous.as_str(),
        "on" | "this"
            | "next"
            | "last"
            | "every"
            | "each"
            | "by"
            | "until"
            | "till"
            | "from"
            | "and"
            | "or"
    ) {
        return true;
    }
    let after = &text[end..];
    if re!(r"(?i)^\.?\s*(?:\d|at\b|@|morning|afternoon|evening|night|noon|,\s*\d|(?:jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\b)").is_match(after) {
        return true;
    }
    let ends_phrase = after.trim().is_empty() || re!(r"^\.?\s*[,;:!?)\-–—]").is_match(after);
    ends_phrase && !before.trim().is_empty() || before.trim().is_empty() && after.trim().is_empty()
}

const PERSON_VERBS: &[&str] = &[
    "with",
    "call",
    "tell",
    "ask",
    "text",
    "email",
    "message",
    "meet",
    "see",
    "visit",
    "ping",
    "invite",
    "thank",
    "remind",
    "phone",
    "dm",
    "and",
    "pick",
    "drop",
    "help",
    "congratulate",
    "for",
];

pub fn month_is_person(text: &str, start: usize, end: usize) -> bool {
    let previous = last_word(&text[..start]);
    let after = &text[end..];
    if after.starts_with("'s") || after.starts_with("’s") {
        return true;
    }
    if re!(r"^\s*\d").is_match(after) {
        return false;
    }
    PERSON_VERBS.contains(&previous.as_str())
}
