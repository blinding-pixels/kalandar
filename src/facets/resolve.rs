use chrono::{
    DateTime, Datelike, Duration, FixedOffset, Months, NaiveDate, NaiveDateTime, NaiveTime,
    Timelike, Weekday,
};
use serde::Serialize;

use crate::facets::context::Context;
use crate::facets::lexicon::{
    line_activity, line_daypart_meridiem, line_mentions_lunch, normalize, scan, task_signal,
    Activity, Daypart, Meridiem, Normalized, Piece, Season, SeasonModifier, Token, WeekdayWord,
};
use crate::facets::recurrence::{
    add_span, exceptions_in, find_recurrence_phrases, first_occurrence, ignoring_interval,
    FacetsRecurrence, PhraseKind, RecurrenceException, Rule,
};
use crate::facets::settings::Clock;
use crate::facets::warning::{Warning, WarningCode};
use crate::{
    Component, ParsedComponents, ParsedResult, ParsingOption, ParsingReference, SystemZone,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureKind {
    CalendarEvent,
    Task,
    Reminder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    EventStart,
    EventEnd,
    Duration,
    DueDate,
    DueTime,
    Trigger,
    LeadTime,
    Alert,
    Recurrence,
    RecurrenceStart,
    RecurrenceEnd,
    RecurrenceException,
    RecurrenceWindow,
}

impl Role {
    pub fn from_label(label: &str) -> Option<Role> {
        Some(match label {
            "start_time" | "event_start" | "date" | "time" => Role::EventStart,
            "end_time" | "event_end" => Role::EventEnd,
            "duration" => Role::Duration,
            "due_date" => Role::DueDate,
            "due_time" => Role::DueTime,
            "trigger_time" | "remind_at" | "trigger" => Role::Trigger,
            "lead_time" => Role::LeadTime,
            "alert" => Role::Alert,
            "recurrence" => Role::Recurrence,
            "recurrence_start" => Role::RecurrenceStart,
            "recurrence_end" => Role::RecurrenceEnd,
            "recurrence_exception" => Role::RecurrenceException,
            "recurrence_window" => Role::RecurrenceWindow,
            _ => return None,
        })
    }

    fn is_moment(self) -> bool {
        matches!(
            self,
            Role::EventStart | Role::DueDate | Role::DueTime | Role::Trigger
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LabelledSpan {
    pub role: Role,
    pub start: usize,
    pub end: usize,
}

impl LabelledSpan {
    pub fn new(role: Role, start: usize, end: usize) -> LabelledSpan {
        LabelledSpan { role, start, end }
    }

    pub fn find(line: &str, role: Role, text: &str) -> Option<LabelledSpan> {
        let lower_line = line.to_lowercase();
        let lower_text = text.trim().to_lowercase();
        if lower_text.is_empty() || lower_line.len() != line.len() {
            return line
                .find(text.trim())
                .map(|s| LabelledSpan::new(role, s, s + text.trim().len()));
        }
        lower_line
            .find(&lower_text)
            .map(|s| LabelledSpan::new(role, s, s + lower_text.len()))
    }

    fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && start < self.end
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Moment {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
    pub local: Option<NaiveDateTime>,
    pub instant: Option<DateTime<FixedOffset>>,
    pub all_day: bool,
    pub implied: Vec<&'static str>,
    pub warnings: Vec<Warning>,
    pub text: String,
    pub alternative: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Alert {
    Relative {
        minutes_before: i64,
        at: Option<Moment>,
    },
    Absolute {
        at: Moment,
    },
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResolvedRecurrence {
    pub rule: Rule,
    pub rrule: String,
    pub facets: Option<FacetsRecurrence>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
    pub exceptions: Vec<RecurrenceException>,
    pub exdates: Vec<NaiveDateTime>,
    pub first: Option<Moment>,
    pub text: String,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LineResolution {
    pub kind: CaptureKind,
    pub start: Option<Moment>,
    pub end: Option<Moment>,
    pub recurrence: Option<ResolvedRecurrence>,
    pub alert: Option<Alert>,
    pub lead_minutes: Option<i64>,
    pub notify_at: Option<NaiveDateTime>,
    pub task_signal: Option<String>,
    pub warnings: Vec<Warning>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WeekdayModifier {
    This,
    Next,
    Last,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateKind {
    Exact,
    Weekday(Weekday, Option<WeekdayModifier>),
    MonthDayNoYear,
    MonthOnly,
    Relative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimeKind {
    Explicit { meridiem_known: bool },
    Daypart,
    Exact,
}

#[derive(Clone, Debug)]
enum StatedZone {
    Fixed(i32, String),
    Iana(chrono_tz::Tz, String),
}

#[derive(Clone, Debug, Default)]
struct Draft {
    zone: Option<StatedZone>,
    date: Option<(NaiveDate, DateKind)>,
    time: Option<(NaiveTime, TimeKind)>,
    end_date: Option<NaiveDate>,
    end_time: Option<(NaiveTime, TimeKind)>,
    approximate: bool,
    range_end_weekday: Option<Weekday>,
    all_day: bool,
    season: bool,
    warnings: Vec<Warning>,
    implied: Vec<&'static str>,
    texts: Vec<String>,
}

impl Draft {
    fn is_empty(&self) -> bool {
        self.date.is_none() && self.time.is_none()
    }
}

fn reference(ctx: &Context) -> ParsingReference {
    ParsingReference::new(ctx.now).with_system_zone(SystemZone::Iana(ctx.zone))
}

fn component_date(c: &ParsedComponents) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(
        c.get(Component::Year)?,
        u32::try_from(c.get(Component::Month)?).ok()?,
        u32::try_from(c.get(Component::Day)?).ok()?,
    )
}

fn has_casual_date(c: &ParsedComponents) -> bool {
    c.tags().iter().any(|t| {
        matches!(
            *t,
            "casualReference/today"
                | "casualReference/tomorrow"
                | "casualReference/yesterday"
                | "casualReference/tonight"
                | "casualReference/now"
                | "result/relativeDate"
                | "parser/ENCasualDateParser"
        )
    })
}

fn leading_zero_clock(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(4).enumerate().any(|(i, w)| {
        let boundary = i == 0 || !bytes[i - 1].is_ascii_digit();
        boundary
            && w[0] == b'0'
            && w[1].is_ascii_digit()
            && (w[2] == b':' || w[2] == b'.')
            && w[3].is_ascii_digit()
    })
}

fn weekday_modifier(text: &str) -> Option<WeekdayModifier> {
    let lower = text.to_lowercase();
    let has = |w: &str| {
        lower
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|t| t == w)
    };
    if has("next") {
        Some(WeekdayModifier::Next)
    } else if has("last") || has("past") {
        Some(WeekdayModifier::Last)
    } else if has("this") || has("coming") {
        Some(WeekdayModifier::This)
    } else {
        None
    }
}

fn components_time(
    c: &ParsedComponents,
    text: &str,
    ctx: &Context,
) -> Option<(NaiveTime, TimeKind)> {
    let tags = c.tags();
    let hour = c.get(Component::Hour)?;
    let minute = c.get(Component::Minute).unwrap_or(0);
    if c.is_certain(Component::Hour) {
        let lower_text = text.to_lowercase();
        if !c.is_certain(Component::Meridiem)
            && (13..=16).contains(&hour)
            && lower_text.contains("night")
            && !lower_text.contains("pm")
            && !lower_text.contains("p.m")
        {
            let time = NaiveTime::from_hms_opt(
                u32::try_from(hour - 12).ok()?,
                u32::try_from(minute).ok()?,
                0,
            )?;
            return Some((
                time,
                TimeKind::Explicit {
                    meridiem_known: true,
                },
            ));
        }
        let relative = tags.contains("result/relativeDateAndTime");
        let named = tags.contains("casualReference/noon")
            || tags.contains("casualReference/midnight")
            || tags.contains("casualReference/now");
        let known = relative
            || named
            || c.is_certain(Component::Meridiem)
            || hour >= 12
            || hour == 0
            || leading_zero_clock(text);
        let time =
            NaiveTime::from_hms_opt(u32::try_from(hour).ok()?, u32::try_from(minute).ok()?, 0)?;
        let kind = if relative {
            TimeKind::Exact
        } else {
            TimeKind::Explicit {
                meridiem_known: known,
            }
        };
        return Some((time, kind));
    }
    let s = &ctx.settings;
    let lower = text.to_lowercase();
    if tags.contains("casualReference/tonight") {
        return Some((s.tonight, TimeKind::Daypart));
    }
    if tags.contains("casualReference/evening") {
        let time = if lower.contains("night") {
            s.tonight
        } else {
            s.evening
        };
        return Some((time, TimeKind::Daypart));
    }
    if tags.contains("casualReference/afternoon") {
        return Some((s.afternoon, TimeKind::Daypart));
    }
    if tags.contains("casualReference/morning") {
        return Some((s.morning, TimeKind::Daypart));
    }
    None
}

fn chrono_draft(result: &ParsedResult, ctx: &Context) -> Draft {
    let mut draft = Draft::default();
    let s = &result.start;
    let certain_date = s.is_certain(Component::Day)
        || s.is_certain(Component::Month)
        || s.is_certain(Component::Year);
    let weekday_only = s.is_certain(Component::Weekday)
        && !s.is_certain(Component::Day)
        && !s.is_certain(Component::Month);
    if let Some(date) = component_date(s) {
        let kind = if weekday_only {
            let weekday = if result.text.to_lowercase().contains("weekend") {
                let modifier = weekday_modifier(&result.text);
                let mut days = ctx.settings.weekend.clone();
                days.sort_by_key(|d| d.num_days_from_monday());
                if modifier == Some(WeekdayModifier::Last) {
                    days.last().copied()
                } else {
                    days.first().copied()
                }
            } else {
                s.get(Component::Weekday)
                    .and_then(|w| u8::try_from(w).ok())
                    .and_then(|w| Weekday::try_from((w + 6) % 7).ok())
            };
            weekday.map(|w| DateKind::Weekday(w, weekday_modifier(&result.text)))
        } else if s.is_certain(Component::Month) && !s.is_certain(Component::Day) {
            Some(DateKind::MonthOnly)
        } else if s.is_certain(Component::Month) && !s.is_certain(Component::Year) {
            Some(DateKind::MonthDayNoYear)
        } else if certain_date || has_casual_date(s) {
            Some(if s.tags().contains("result/relativeDate") {
                DateKind::Relative
            } else {
                DateKind::Exact
            })
        } else {
            None
        };
        if let Some(kind) = kind {
            let shifted = if ctx.before_day_start()
                && s.tags().iter().any(|t| {
                    matches!(
                        *t,
                        "casualReference/today"
                            | "casualReference/tomorrow"
                            | "casualReference/tonight"
                    )
                }) {
                date - Duration::days(1)
            } else {
                date
            };
            draft.date = Some((shifted, kind));
        }
    }
    draft.time = components_time(s, &result.text, ctx);
    let relative = s
        .tags()
        .iter()
        .any(|t| t.starts_with("result/relative") || *t == "casualReference/now");
    if s.is_certain(Component::TimezoneOffset) && !relative {
        if let Some(offset) = s.get(Component::TimezoneOffset) {
            let label = zone_label(&result.text);
            draft.zone = Some(match seasonal_zone(&label) {
                Some(tz) => StatedZone::Iana(tz, label),
                None => StatedZone::Fixed(offset, label),
            });
        }
    }
    if let Some(end) = &result.end {
        let end_certain = end.is_certain(Component::Day) || end.is_certain(Component::Month);
        let start_certain = s.is_certain(Component::Day) || s.is_certain(Component::Month);
        if end_certain && (!start_certain || component_date(end) != component_date(s)) {
            draft.end_date = component_date(end);
        }
        draft.end_time = components_time(end, &result.text, ctx);
    }
    draft.texts.push(result.text.clone());
    draft
}

fn next_week_start(today: NaiveDate, start: Weekday) -> NaiveDate {
    let offset = (7 + today.weekday().num_days_from_monday() - start.num_days_from_monday()) % 7;
    today - Duration::days(i64::from(offset)) + Duration::days(7)
}

fn last_day_of_month(date: NaiveDate) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
    first
        .checked_add_months(Months::new(1))
        .and_then(|d| d.pred_opt())
        .unwrap_or(date)
}

fn next_ordinal_day(today: NaiveDate, day: u32) -> Option<NaiveDate> {
    (0..24).find_map(|m| {
        let base = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?
            .checked_add_months(Months::new(m))?;
        NaiveDate::from_ymd_opt(base.year(), base.month(), day).filter(|d| *d >= today)
    })
}

fn add_weekdays(mut date: NaiveDate, mut n: i64, weekend: &[Weekday]) -> NaiveDate {
    let distinct: std::collections::HashSet<&Weekday> = weekend.iter().collect();
    if distinct.len() >= 7 {
        return date;
    }
    while n > 0 {
        date += Duration::days(1);
        if !weekend.contains(&date.weekday()) {
            n -= 1;
        }
    }
    date
}

fn season_range(
    season: Season,
    modifier: SeasonModifier,
    today: NaiveDate,
) -> Option<(NaiveDate, NaiveDate)> {
    let (start_month, months) = match season {
        Season::Spring => (3, 3),
        Season::Summer => (6, 3),
        Season::Autumn => (9, 3),
        Season::Winter => (12, 3),
    };
    let candidates = [today.year() - 1, today.year(), today.year() + 1];
    let ranges: Vec<(NaiveDate, NaiveDate)> = candidates
        .iter()
        .filter_map(|&y| {
            let start = NaiveDate::from_ymd_opt(y, start_month, 1)?;
            let end = start.checked_add_months(Months::new(months))?.pred_opt()?;
            Some((start, end))
        })
        .collect();
    let current = ranges.iter().position(|(_, end)| *end >= today)?;
    let index = match modifier {
        SeasonModifier::This => current,
        SeasonModifier::Next => {
            if ranges[current].0 <= today {
                current + 1
            } else {
                current
            }
        }
    };
    ranges.get(index).copied()
}

fn end_of_week(today: NaiveDate, ctx: &Context) -> NaiveDate {
    let workdays: Vec<Weekday> = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ]
    .into_iter()
    .filter(|d| !ctx.settings.weekend.contains(d))
    .collect();
    let last_workday = workdays
        .iter()
        .max_by_key(|d| {
            (7 + d.num_days_from_monday() - ctx.settings.week_start.num_days_from_monday()) % 7
        })
        .copied()
        .unwrap_or(Weekday::Fri);
    let week_start = next_week_start(today, ctx.settings.week_start) - Duration::days(7);
    let offset = (7 + last_workday.num_days_from_monday()
        - ctx.settings.week_start.num_days_from_monday())
        % 7;
    let candidate = week_start + Duration::days(i64::from(offset));
    if candidate < today {
        candidate + Duration::days(7)
    } else {
        candidate
    }
}

fn holiday_warnings(holiday: crate::facets::Holiday, foreign: bool) -> Vec<Warning> {
    let mut out = Vec::new();
    let name = holiday.name();
    if foreign {
        out.push(Warning::new(
            WarningCode::HolidayOtherRegion,
            format!(
                "{name} isn't a holiday in your region; used its date in {}",
                holiday.region_code()
            ),
        ));
    }
    if let Some(def) = holiday.definition() {
        if def.computed {
            out.push(Warning::new(
                WarningCode::HolidayComputed,
                format!("{name} is computed from the lunar calendar, not an official list; regional dates can differ"),
            ));
        }
        if def.moon_sighting {
            out.push(Warning::new(
                WarningCode::HolidayMoonSighting,
                format!("{name} depends on the moon sighting; the observed date can move by a day"),
            ));
        }
    }
    out
}

fn apply_piece(draft: &mut Draft, piece: &Piece, line: &str, ctx: &Context) {
    let today = ctx.today();
    let now = ctx.local_now();
    let s = &ctx.settings;
    let exact = |d: NaiveDate| Some((d, DateKind::Exact));
    match &piece.token {
        Token::Holiday(holiday, year, shift, foreign) => {
            let date = match year {
                Some(y) => holiday.date_in(*y),
                None => holiday.next_on_or_after(today - Duration::days(*shift)),
            }
            .map(|d| d + Duration::days(*shift));
            if let Some(date) = date {
                draft.date = exact(date);
                if year.is_none() {
                    draft.implied.push("year");
                }
                for w in holiday_warnings(*holiday, *foreign) {
                    if !draft.warnings.iter().any(|x| x.code == w.code) {
                        draft.warnings.push(w);
                    }
                }
            }
        }
        Token::OrdinalDay(day) => {
            if let Some(date) = next_ordinal_day(today, *day) {
                draft.date = exact(date);
                draft.implied.push("month");
            }
        }
        Token::OrdinalRange(a, b) => {
            if let Some(start) = next_ordinal_day(today, *a) {
                draft.date = exact(start);
                draft.implied.push("month");
                let end = NaiveDate::from_ymd_opt(start.year(), start.month(), *b)
                    .filter(|e| *e >= start)
                    .or_else(|| next_ordinal_day(start, *b));
                draft.end_date = end;
            }
        }
        Token::EndOfWeek => draft.date = exact(end_of_week(today, ctx)),
        Token::EndOfMonth => draft.date = exact(last_day_of_month(today)),
        Token::NextWeek => {
            draft.date = exact(next_week_start(today, s.week_start));
            draft.implied.push("day");
        }
        Token::EarlyNextMonth => {
            if let Some(d) = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .and_then(|d| d.checked_add_months(Months::new(1)))
            {
                draft.date = exact(d);
                draft.implied.push("day");
                draft.warnings.push(Warning::new(
                    WarningCode::DayAssumed,
                    "early next month read as the 1st",
                ));
            }
        }
        Token::EndOfNextMonth => {
            if let Some(d) = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .and_then(|d| d.checked_add_months(Months::new(1)))
            {
                draft.date = exact(last_day_of_month(d));
            }
        }
        Token::MidMonth(next) => {
            let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
            let this_month =
                NaiveDate::from_ymd_opt(today.year(), today.month(), 15).unwrap_or(today);
            let date = if *next || this_month < today {
                first
                    .checked_add_months(Months::new(1))
                    .and_then(|d| d.with_day(15))
            } else {
                Some(this_month)
            };
            if let Some(date) = date {
                draft.date = exact(date);
                draft.implied.push("day");
            }
        }
        Token::OrdinalNextMonth(day) => {
            let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
            if let Some(date) = first
                .checked_add_months(Months::new(1))
                .and_then(|d| d.with_day(*day))
            {
                draft.date = exact(date);
            }
        }
        Token::NthWeekdayInMonth(position, weekday, month) => {
            let found = (today.year()..=today.year() + 1).find_map(|y| {
                let date = if *position > 0 {
                    NaiveDate::from_weekday_of_month_opt(
                        y,
                        *month,
                        *weekday,
                        u8::try_from(*position).ok()?,
                    )
                } else {
                    let first = NaiveDate::from_ymd_opt(y, *month, 1)?;
                    let mut d = last_day_of_month(first);
                    while d.weekday() != *weekday {
                        d = d.pred_opt()?;
                    }
                    Some(d)
                }?;
                (date >= today).then_some(date)
            });
            if let Some(date) = found {
                draft.date = exact(date);
            }
        }
        Token::WeekdayRange(a, b) => {
            draft.date = Some((today, DateKind::Weekday(*a, None)));
            draft.range_end_weekday = Some(*b);
        }
        Token::AllDay => {
            draft.all_day = true;
        }
        Token::Season(season, modifier) => {
            if let Some((start, end)) = season_range(*season, *modifier, today) {
                draft.date = exact(start.max(today));
                draft.end_date = Some(end);
                draft.season = true;
                draft.implied.push("day");
            }
        }
        Token::Daypart(daypart) => {
            let time = match daypart {
                Daypart::Meal(activity, after) => {
                    let base = activity_time(*activity, ctx);
                    if *after {
                        base + Duration::hours(1)
                    } else {
                        base
                    }
                }
                Daypart::FirstThing => s.morning,
                Daypart::Afternoon => s.afternoon,
                Daypart::Lunch => s.lunch,
                Daypart::AfterWork => s.after_work,
                Daypart::EndOfDay => s.end_of_day,
                Daypart::AfterLunch => s.lunch + Duration::hours(1),
                Daypart::BeforeLunch => s.lunch,
            };
            if !matches!(draft.time, Some((_, TimeKind::Explicit { .. }))) {
                draft.time = Some((time, TimeKind::Daypart));
            }
        }
        Token::Clock {
            hour,
            minute,
            meridiem,
            approximate,
        } => {
            let hour = match meridiem {
                Some(Meridiem::Pm) if *hour < 12 => hour + 12,
                Some(Meridiem::Am) if *hour == 12 => 0,
                _ => *hour,
            } % 24;
            if let Some(time) = NaiveTime::from_hms_opt(hour, *minute, 0) {
                let known = meridiem.is_some() || hour >= 12 || hour == 0;
                draft.time = Some((
                    time,
                    TimeKind::Explicit {
                        meridiem_known: known,
                    },
                ));
                draft.approximate |= approximate;
            }
        }
        Token::Zone(tz, label) => {
            draft.zone = Some(StatedZone::Iana(*tz, label.clone()));
        }
        Token::Weekday(weekday, word) => {
            let modifier = word.map(|w| match w {
                WeekdayWord::This => WeekdayModifier::This,
                WeekdayWord::Next => WeekdayModifier::Next,
                WeekdayWord::Last => WeekdayModifier::Last,
            });
            draft.date = Some((today, DateKind::Weekday(*weekday, modifier)));
        }
        Token::ThisWeek => {
            draft.date = exact(today);
            draft.implied.push("day");
        }
        Token::ClockRange { start, end, .. } => {
            let to24 = |h: u32, m: Option<Meridiem>| match m {
                Some(Meridiem::Pm) if h < 12 => h + 12,
                Some(Meridiem::Am) if h == 12 => 0,
                _ => h,
            };
            let (sh, sm, smer) = *start;
            let (eh, em, emer) = *end;
            let (start_hour, start_known) = match (smer, emer) {
                (Some(m), _) => (to24(sh, Some(m)), true),
                (None, Some(Meridiem::Pm)) if sh < 12 && sh + 12 <= to24(eh, emer) => {
                    (sh + 12, true)
                }
                (None, Some(Meridiem::Pm)) => (sh, true),
                (None, Some(Meridiem::Am)) => (to24(sh, Some(Meridiem::Am)), true),
                (None, None) => (sh, sh >= 12 || sh == 0),
            };
            if let Some(t) = NaiveTime::from_hms_opt(start_hour % 24, sm, 0) {
                draft.time = Some((
                    t,
                    TimeKind::Explicit {
                        meridiem_known: start_known,
                    },
                ));
            }
            let end_hour = to24(eh, emer);
            if let Some(t) = NaiveTime::from_hms_opt(end_hour % 24, em, 0) {
                draft.end_time = Some((
                    t,
                    TimeKind::Explicit {
                        meridiem_known: emer.is_some() || end_hour >= 12 || end_hour == 0,
                    },
                ));
            }
        }
        Token::EndClock {
            hour,
            minute,
            meridiem,
        } => {
            let hour = match meridiem {
                Some(Meridiem::Pm) if *hour < 12 => hour + 12,
                Some(Meridiem::Am) if *hour == 12 => 0,
                _ => *hour,
            } % 24;
            if let Some(time) = NaiveTime::from_hms_opt(hour, *minute, 0) {
                let known = meridiem.is_some() || hour >= 12 || hour == 0;
                draft.end_time = Some((
                    time,
                    TimeKind::Explicit {
                        meridiem_known: known,
                    },
                ));
            }
        }
        Token::AddMinutes(minutes) => {
            let moment = now + Duration::minutes(*minutes);
            draft.date = Some((moment.date(), DateKind::Relative));
            draft.time = Some((moment.time(), TimeKind::Exact));
        }
        Token::AddDays(days) => {
            draft.date = Some((today + Duration::days(*days), DateKind::Relative))
        }
        Token::AddWeekdays(n) => {
            draft.date = Some((add_weekdays(today, *n, &s.weekend), DateKind::Relative))
        }
        Token::Duration(_) | Token::LeadMinutes(_) | Token::AlertOff | Token::AlertAt => {}
    }
    draft.texts.push(line[piece.start..piece.end].to_string());
}

fn piece_carries_moment(token: &Token) -> bool {
    !matches!(
        token,
        Token::Duration(_) | Token::LeadMinutes(_) | Token::AlertOff | Token::AlertAt
    )
}

fn bare_hour(time: NaiveTime, line: &str, ctx: &Context) -> (NaiveTime, Option<Warning>, bool) {
    let hour = time.hour();
    if !(1..=11).contains(&hour) {
        return (time, None, false);
    }
    let pm = |t: NaiveTime| t + Duration::hours(12);
    let night = line
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| w == "night" || w == "tonight");
    if night && hour <= 4 {
        return (time, None, true);
    }
    if let Some(m) = line_daypart_meridiem(line) {
        return match m {
            Meridiem::Pm => (pm(time), None, true),
            Meridiem::Am => (time, None, true),
        };
    }
    if line_mentions_lunch(line) && (hour == 11 || (1..=4).contains(&hour)) {
        return if hour == 11 {
            (time, None, true)
        } else {
            (pm(time), None, true)
        };
    }
    if ctx.settings.clock == Clock::TwentyFourHour {
        return (time, None, false);
    }
    let chosen = if hour <= 6 { pm(time) } else { time };
    let warning = Warning::new(
        WarningCode::AmbiguousMeridiem,
        format!(
            "{} has no am or pm; read as {}",
            time.format("%-I:%M"),
            chosen.format("%H:%M")
        ),
    );
    (chosen, Some(warning), true)
}

fn activity_time(activity: Activity, ctx: &Context) -> NaiveTime {
    let s = &ctx.settings;
    match activity {
        Activity::Breakfast => s.breakfast,
        Activity::Brunch => s.brunch,
        Activity::Lunch => s.lunch,
        Activity::AfternoonTea => s.afternoon_tea,
        Activity::Dinner => s.dinner,
        Activity::Drinks => s.drinks,
    }
}

fn kind_default_time(kind: CaptureKind, ctx: &Context) -> Option<(NaiveTime, &'static str)> {
    match kind {
        CaptureKind::CalendarEvent => None,
        CaptureKind::Task => Some((ctx.settings.end_of_day, "end of day")),
        CaptureKind::Reminder => Some((ctx.settings.reminder_default, "the default reminder time")),
    }
}

fn finish(mut draft: Draft, kind: CaptureKind, line: &str, ctx: &Context) -> Option<Moment> {
    if draft.is_empty() {
        return None;
    }
    if draft.all_day && kind == CaptureKind::CalendarEvent && draft.date.is_some() {
        draft.time = None;
    }
    let today = ctx.today();
    let now = ctx.local_now();
    let mut warnings = std::mem::take(&mut draft.warnings);
    let mut implied = std::mem::take(&mut draft.implied);

    let mut time = match draft.time {
        Some((
            t,
            TimeKind::Explicit {
                meridiem_known: false,
            },
        )) => {
            let (t, warning, applied) = bare_hour(t, line, ctx);
            if applied {
                implied.push("meridiem");
            }
            warnings.extend(warning);
            Some(t)
        }
        Some((t, TimeKind::Daypart)) => {
            implied.push("time");
            warnings.push(Warning::new(
                WarningCode::TimeAssumed,
                format!("time assumed {}", t.format("%H:%M")),
            ));
            Some(t)
        }
        Some((t, _)) => Some(t),
        None => None,
    };
    if draft.approximate {
        if let Some(t) = time {
            warnings.push(Warning::new(
                WarningCode::ApproximateTime,
                format!("approximate time, using {}", t.format("%H:%M")),
            ));
        }
    }
    if matches!(draft.time, Some((_, TimeKind::Daypart)))
        && line_activity(line) == Some(Activity::AfternoonTea)
    {
        time = Some(ctx.settings.afternoon_tea);
    }
    if time.is_none() {
        if let Some(activity) = line_activity(line) {
            let t = activity_time(activity, ctx);
            implied.push("time");
            warnings.push(Warning::new(
                WarningCode::TimeAssumed,
                format!("time assumed {} from the activity", t.format("%H:%M")),
            ));
            time = Some(t);
        }
    }
    let mut defaulted_time = false;
    if time.is_none() && draft.date.is_none() {
        if let Some((t, label)) = kind_default_time(kind, ctx) {
            implied.push("time");
            warnings.push(Warning::new(
                WarningCode::TimeAssumed,
                format!("time assumed {} ({label})", t.format("%H:%M")),
            ));
            time = Some(t);
            defaulted_time = true;
        }
    }

    let mut alternative: Option<NaiveDate> = None;
    let passed = |date: NaiveDate, time: Option<NaiveTime>| match time {
        Some(t) => date.and_time(t) < now,
        None => date < today,
    };

    let date = match draft.date {
        None => {
            implied.push("date");
            if passed(today, time) {
                warnings.push(Warning::new(
                    WarningCode::RolledToTomorrow,
                    "moved to tomorrow because the time had passed",
                ));
                today + Duration::days(1)
            } else {
                today
            }
        }
        Some((_, DateKind::Weekday(weekday, modifier))) => {
            let forward =
                (7 + weekday.num_days_from_monday() - today.weekday().num_days_from_monday()) % 7;
            match modifier {
                Some(WeekdayModifier::Next) => {
                    let start = next_week_start(today, ctx.settings.week_start);
                    let offset = (7 + weekday.num_days_from_monday()
                        - ctx.settings.week_start.num_days_from_monday())
                        % 7;
                    let chosen = start + Duration::days(i64::from(offset));
                    let nearer = today + Duration::days(i64::from(forward));
                    if forward > 0 && nearer < chosen {
                        alternative = Some(nearer);
                        warnings.push(Warning::new(
                            WarningCode::AmbiguousNextWeekday,
                            format!(
                                "read as {}; it could also mean {}",
                                chosen.format("%a %b %-d"),
                                nearer.format("%a %b %-d")
                            ),
                        ));
                    }
                    chosen
                }
                Some(WeekdayModifier::Last) => {
                    let back = if forward == 0 { 7 } else { 7 - forward };
                    today - Duration::days(i64::from(back))
                }
                _ => {
                    let candidate = today + Duration::days(i64::from(forward));
                    let before_today = candidate == today
                        && time.is_none()
                        && line
                            .to_lowercase()
                            .split("before ")
                            .skip(1)
                            .filter_map(|rest| rest.split_whitespace().next())
                            .filter_map(crate::facets::recurrence::parse_weekday)
                            .any(|w| w == weekday);
                    if before_today {
                        warnings.push(Warning::new(
                            WarningCode::MovedToNextWeek,
                            "\"before\" today's weekday read as next week's",
                        ));
                        candidate + Duration::days(7)
                    } else if candidate == today && passed(candidate, time) {
                        warnings.push(Warning::new(
                            WarningCode::MovedToNextWeek,
                            "moved to next week because today's time had passed",
                        ));
                        candidate + Duration::days(7)
                    } else {
                        candidate
                    }
                }
            }
        }
        Some((date, DateKind::MonthDayNoYear)) => {
            implied.push("year");
            if date < today {
                date.with_year(date.year() + 1).unwrap_or(date)
            } else if date.year() > today.year() {
                date.with_year(today.year())
                    .filter(|d| *d >= today)
                    .unwrap_or(date)
            } else {
                date
            }
        }
        Some((date, DateKind::MonthOnly)) => {
            implied.push("day");
            warnings.push(Warning::new(
                WarningCode::DayAssumed,
                "only a month was given; used the 1st",
            ));
            let first = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
            let this_year = first.with_year(today.year()).unwrap_or(first);
            if this_year.year() == today.year() && this_year.month() == today.month() {
                today
            } else if this_year < today {
                this_year.with_year(today.year() + 1).unwrap_or(this_year)
            } else {
                this_year
            }
        }
        Some((date, _)) => date,
    };

    if kind == CaptureKind::Reminder && defaulted_time && date == today && passed(date, time) {
        warnings.push(Warning::new(
            WarningCode::TimeAssumed,
            "the default reminder time has already passed today",
        ));
    }

    let mut date = date;
    if let (Some(zone), Some(t)) = (&draft.zone, time) {
        let stated_local = date.and_time(t);
        let converted = match zone {
            StatedZone::Fixed(offset, _) => stated_local
                .checked_sub_signed(Duration::minutes(i64::from(*offset)))
                .map(|utc| utc.and_utc().with_timezone(&ctx.zone).naive_local()),
            StatedZone::Iana(tz, _) => {
                let (instant, _) = crate::facets::context::to_instant(*tz, stated_local);
                Some(instant.with_timezone(&ctx.zone).naive_local())
            }
        };
        if let Some(converted) = converted {
            let label = match zone {
                StatedZone::Fixed(_, label) | StatedZone::Iana(_, label) => label.clone(),
            };
            warnings.push(Warning::new(
                WarningCode::ConvertedTimezone,
                format!(
                    "{} {} is {} in your time zone",
                    stated_local.format("%H:%M"),
                    label,
                    converted.format("%Y-%m-%d %H:%M")
                ),
            ));
            date = converted.date();
            time = Some(converted.time());
            if draft.date.is_none() && converted < now {
                date += Duration::days(1);
            }
        }
    }
    if let (Some(t), Some((_, kind_of_date))) = (time, draft.date) {
        let night = draft
            .texts
            .iter()
            .any(|x| x.to_lowercase().contains("night"));
        if night && t.hour() < 5 && !matches!(kind_of_date, DateKind::Relative) {
            date += Duration::days(1);
        }
    }
    let date_given = draft.date.is_some();
    let all_day = time.is_none();
    if all_day {
        implied.push("time");
        if kind == CaptureKind::CalendarEvent {
            warnings.push(Warning::new(
                WarningCode::AllDayAssumed,
                "no time given; all-day",
            ));
        }
    }
    if date_given
        && kind != CaptureKind::Task
        && (all_day || (defaulted_time && kind == CaptureKind::Reminder))
    {
        warnings.push(Warning::new(
            WarningCode::NoTime,
            "a date was given without a time",
        ));
    }
    let local = time.map(|t| date.and_time(t));
    let instant = local.map(|l| {
        let (instant, warning) = ctx.to_instant(l);
        warnings.extend(warning);
        instant
    });
    implied.sort_unstable();
    implied.dedup();
    Some(Moment {
        date,
        time,
        local,
        instant,
        all_day,
        implied,
        warnings,
        text: draft.texts.join(" "),
        alternative,
    })
}

fn moment_at(
    local: NaiveDateTime,
    text: String,
    warnings: Vec<Warning>,
    implied: Vec<&'static str>,
    ctx: &Context,
) -> Moment {
    let (instant, warning) = ctx.to_instant(local);
    let mut warnings = warnings;
    warnings.extend(warning);
    Moment {
        date: local.date(),
        time: Some(local.time()),
        local: Some(local),
        instant: Some(instant),
        all_day: false,
        implied,
        warnings,
        text,
        alternative: None,
    }
}

struct Analysis {
    scope: crate::facets::HolidayScope,
    norm: Normalized,
    results: Vec<ParsedResult>,
    pieces: Vec<Piece>,
}

fn analyse(line: &str, ctx: &Context) -> Analysis {
    let norm = normalize(line);
    let chrono = if ctx.settings.day_first {
        crate::gb()
    } else {
        crate::casual()
    };
    let mut results = chrono.parse(&norm.text, &reference(ctx), &ParsingOption::default());
    results.retain(|r| keep_result(&norm.text, r));
    let pieces = scan(&norm.text, ctx.holiday_scope());
    results.retain(|r| {
        let (start, end) = (r.index, r.index + r.text.len());
        !pieces.iter().any(|p| {
            matches!(p.token, Token::Holiday(..))
                && p.start < end
                && start < p.end
                && !(start <= p.start && p.end <= end)
        })
    });
    Analysis {
        scope: ctx.holiday_scope(),
        norm,
        results,
        pieces,
    }
}

fn spans_for(spans: &[LabelledSpan], roles: &[Role]) -> Vec<LabelledSpan> {
    spans
        .iter()
        .filter(|s| roles.contains(&s.role))
        .copied()
        .collect()
}

fn overlapping<'a>(
    analysis: &'a Analysis,
    spans: &[LabelledSpan],
) -> (Vec<&'a ParsedResult>, Vec<&'a Piece>) {
    let results = analysis
        .results
        .iter()
        .filter(|r| {
            spans
                .iter()
                .any(|s| s.overlaps(r.index, r.index + r.text.len()))
        })
        .collect();
    let pieces = analysis
        .pieces
        .iter()
        .filter(|p| spans.iter().any(|s| s.overlaps(p.start, p.end)))
        .collect();
    (results, pieces)
}

fn fill(into: &mut Draft, other: Draft) {
    if into.zone.is_none() {
        into.zone = other.zone.clone();
    }
    into.all_day |= other.all_day;
    if into.date.is_none() {
        into.season = other.season;
    }
    if into.range_end_weekday.is_none() {
        into.range_end_weekday = other.range_end_weekday;
    }
    if into.date.is_none() {
        into.date = other.date;
        if into.end_date.is_none() {
            into.end_date = other.end_date;
        }
    }
    let upgrade = matches!(into.time, Some((_, TimeKind::Daypart)))
        && matches!(
            other.time,
            Some((_, TimeKind::Explicit { .. } | TimeKind::Exact))
        );
    if into.time.is_none() || upgrade {
        into.time = other.time;
        into.approximate |= other.approximate;
        if into.end_time.is_none() {
            into.end_time = other.end_time;
        }
    }
    into.warnings.extend(other.warnings);
    into.implied.extend(other.implied);
    into.texts.extend(other.texts);
}

fn overlay(into: &mut Draft, other: Draft) {
    if other.zone.is_some() {
        into.zone = other.zone.clone();
    }
    into.all_day |= other.all_day;
    if other.range_end_weekday.is_some() {
        into.range_end_weekday = other.range_end_weekday;
    }
    if other.date.is_some() {
        into.date = other.date;
        into.end_date = other.end_date.or(into.end_date);
    }
    let keep_explicit = matches!(
        into.time,
        Some((_, TimeKind::Explicit { .. } | TimeKind::Exact))
    ) && matches!(other.time, Some((_, TimeKind::Daypart)));
    if other.time.is_some() && !keep_explicit {
        into.time = other.time;
        into.approximate |= other.approximate;
        if other.end_time.is_some() {
            into.end_time = other.end_time;
        }
    }
    into.warnings.extend(other.warnings);
    into.implied.extend(other.implied);
}

fn part_draft(result: &ParsedResult, span_start: usize, ctx: &Context) -> Draft {
    let lower = result.text.to_lowercase();
    let connector = [" to ", " until ", " till ", " through ", " thru ", "-", "–"]
        .iter()
        .filter_map(|c| lower.get(1..).and_then(|rest| rest.find(c)).map(|p| p + 1))
        .min();
    let in_end_part = connector.is_some_and(|c| span_start >= result.index + c);
    match &result.end {
        Some(end) if in_end_part => {
            let mut copy = result.clone();
            copy.start = end.clone();
            copy.end = None;
            chrono_draft(&copy, ctx)
        }
        _ => chrono_draft(result, ctx),
    }
}

fn draft_from(
    results: &[&ParsedResult],
    pieces: &[&Piece],
    spans: &[LabelledSpan],
    line: &str,
    ctx: &Context,
) -> Draft {
    let span_start = spans.iter().map(|s| s.start).min().unwrap_or(0);
    let blocked = |r: &ParsedResult| {
        let (rs, re) = (r.index, r.index + r.text.len());
        pieces.iter().any(|p| {
            let covers = p.start <= rs && re <= p.end;
            let duration_like = matches!(p.token, Token::Duration(_) | Token::LeadMinutes(_));
            covers && (piece_carries_moment(&p.token) || duration_like)
        })
    };
    let kept: Vec<&&ParsedResult> = results.iter().filter(|r| !blocked(r)).collect();
    let inside_of = |p: &Piece| {
        kept.iter()
            .position(|r| r.index <= p.start && p.end <= r.index + r.text.len())
    };
    let moment_pieces: Vec<&&Piece> = pieces
        .iter()
        .filter(|p| piece_carries_moment(&p.token))
        .filter(|p| {
            matches!(p.token, Token::Holiday(..))
                || !pieces.iter().any(|q| {
                    matches!(q.token, Token::Holiday(..)) && q.start <= p.start && p.end <= q.end
                })
        })
        .filter(|p| {
            !matches!(
                p.token,
                Token::ClockRange {
                    contextual: false,
                    ..
                }
            ) || !results.iter().any(|r| {
                r.start.is_certain(Component::Day)
                    && r.index < p.end
                    && p.start < r.index + r.text.len()
            })
        })
        .collect();
    let mut items: Vec<(usize, Draft)> = Vec::new();
    for (index, r) in kept.iter().enumerate() {
        let mut d = part_draft(r, span_start, ctx);
        for p in moment_pieces.iter().filter(|p| inside_of(p) == Some(index)) {
            let mut single = Draft::default();
            apply_piece(&mut single, p, line, ctx);
            let date_missing = d.date.is_none();
            let time_override = single.time.is_some()
                && (d.time.is_none()
                    || matches!(p.token, Token::Clock { .. } | Token::ClockRange { .. })
                    || matches!(d.time, Some((_, TimeKind::Daypart))));
            if !date_missing
                && !matches!(
                    p.token,
                    Token::Holiday(..)
                        | Token::Zone(..)
                        | Token::OrdinalDay(_)
                        | Token::OrdinalRange(..)
                        | Token::NextWeek
                        | Token::EndOfMonth
                        | Token::EndOfWeek
                        | Token::EarlyNextMonth
                        | Token::ThisWeek
                        | Token::Season(..)
                )
            {
                single.date = None;
                single.end_date = None;
            }
            if !time_override {
                single.time = None;
                single.end_time = None;
            }
            overlay(&mut d, single);
        }
        items.push((r.index, d));
    }
    for p in moment_pieces.iter().filter(|p| inside_of(p).is_none()) {
        let mut single = Draft::default();
        apply_piece(&mut single, p, line, ctx);
        let rank = if matches!(p.token, Token::Holiday(..)) {
            usize::MAX / 2 + p.start
        } else {
            p.start
        };
        items.push((rank, single));
    }
    items.sort_by_key(|(start, _)| *start);
    let mut draft = Draft::default();
    let approximate_pieces: Vec<Draft> = moment_pieces
        .iter()
        .filter(|p| {
            matches!(
                p.token,
                Token::Clock {
                    approximate: true,
                    ..
                }
            )
        })
        .map(|p| {
            let mut single = Draft::default();
            apply_piece(&mut single, p, line, ctx);
            single
        })
        .collect();
    for (_, d) in items {
        fill(&mut draft, d);
    }
    for single in approximate_pieces {
        if draft.time.map(|(t, _)| t) == single.time.map(|(t, _)| t) || draft.time.is_none() {
            overlay(&mut draft, single);
        }
    }
    draft
}

fn moment_roles(kind: CaptureKind) -> &'static [Role] {
    match kind {
        CaptureKind::CalendarEvent => &[
            Role::EventStart,
            Role::DueDate,
            Role::DueTime,
            Role::Trigger,
        ],
        CaptureKind::Task => &[
            Role::DueDate,
            Role::DueTime,
            Role::EventStart,
            Role::Trigger,
        ],
        CaptureKind::Reminder => &[
            Role::Trigger,
            Role::EventStart,
            Role::DueDate,
            Role::DueTime,
        ],
    }
}

fn end_role_time(
    start: &Moment,
    draft_end: Option<(NaiveTime, TimeKind)>,
    end_date: Option<NaiveDate>,
    line: &str,
    ctx: &Context,
) -> Option<Moment> {
    let start_local = start.local;
    match (draft_end, start_local) {
        (Some((t, kind)), Some(start_local)) => {
            let date = end_date.unwrap_or(start.date);
            let candidates: Vec<NaiveTime> = match kind {
                TimeKind::Explicit {
                    meridiem_known: false,
                } if (1..=11).contains(&t.hour()) => vec![t, t + Duration::hours(12)],
                _ => vec![t],
            };
            let mut warnings = Vec::new();
            let chosen = candidates
                .iter()
                .map(|c| date.and_time(*c))
                .find(|c| *c > start_local)
                .unwrap_or_else(|| {
                    let mut end = date.and_time(candidates[0]);
                    while end <= start_local {
                        end += Duration::days(1);
                    }
                    end
                });
            if candidates.len() > 1 && line_daypart_meridiem(line).is_none() {
                warnings.push(Warning::new(
                    WarningCode::AmbiguousMeridiem,
                    format!("end read as {}", chosen.format("%H:%M")),
                ));
            }
            Some(moment_at(chosen, String::new(), warnings, Vec::new(), ctx))
        }
        (_, None) => end_date.map(|d| Moment {
            date: d,
            time: None,
            local: None,
            instant: None,
            all_day: true,
            implied: Vec::new(),
            warnings: Vec::new(),
            text: String::new(),
            alternative: None,
        }),
        (None, Some(_)) => None,
    }
}

fn duration_minutes(pieces: &[&Piece], duration_spans: &[LabelledSpan], line: &str) -> Option<i64> {
    let from_piece = pieces.iter().find_map(|p| match p.token {
        Token::Duration(m) => Some(m),
        _ => None,
    });
    from_piece.or_else(|| {
        duration_spans.iter().find_map(|s| {
            let text = &line[s.start..s.end];
            crate::en::constants::parse_duration(text).map(|d| {
                use crate::types::Timeunit;
                let get = |u: Timeunit| d.get(u).unwrap_or(0.0);
                (get(Timeunit::Week) * 10_080.0
                    + get(Timeunit::Day) * 1440.0
                    + get(Timeunit::Hour) * 60.0
                    + get(Timeunit::Minute))
                .round() as i64
            })
        })
    })
}

pub fn resolve_line(
    line: &str,
    kind: CaptureKind,
    spans: &[LabelledSpan],
    ctx: &Context,
) -> LineResolution {
    let analysis = analyse(line, ctx);
    let spans: Vec<LabelledSpan> = spans
        .iter()
        .filter(|s| s.start <= s.end && s.end <= line.len())
        .map(|s| {
            LabelledSpan::new(
                s.role,
                analysis.norm.to_normalized(floor_boundary(line, s.start)),
                analysis.norm.to_normalized(floor_boundary(line, s.end)),
            )
        })
        .collect();
    let text = analysis.norm.text.clone();
    resolve_with(&text, kind, &spans, &analysis, ctx)
}

fn with_detected_recurrence(
    line: &str,
    kind: CaptureKind,
    spans: &[LabelledSpan],
    scope: crate::facets::HolidayScope,
) -> Vec<LabelledSpan> {
    if spans.iter().any(|s| s.role == Role::Recurrence) {
        return spans.to_vec();
    }
    let moment = spans_for(spans, moment_roles(kind));
    let phrases = find_recurrence_phrases(line, scope);
    let has_rule = phrases.iter().any(|(k, _, _)| *k == PhraseKind::Rule);
    let rules: Vec<(usize, usize)> = phrases
        .iter()
        .filter(|(k, s, e)| {
            phrase_role(*k, kind, has_rule) == Some(Role::Recurrence)
                && moment.iter().any(|m| m.overlaps(*s, *e))
        })
        .map(|(_, s, e)| (*s, *e))
        .collect();
    if rules.is_empty() {
        return spans.to_vec();
    }
    let mut out = spans.to_vec();
    for (s, e) in rules {
        out.push(LabelledSpan::new(Role::Recurrence, s, e));
    }
    for (k, s, e) in phrases {
        if k == PhraseKind::Window && moment.iter().any(|m| m.overlaps(s, e)) {
            out.push(LabelledSpan::new(Role::RecurrenceWindow, s, e));
        }
    }
    out
}

fn resolve_with(
    line: &str,
    kind: CaptureKind,
    spans: &[LabelledSpan],
    analysis: &Analysis,
    ctx: &Context,
) -> LineResolution {
    let spans = &with_detected_recurrence(line, kind, spans, ctx.holiday_scope())[..];
    let mut warnings = Vec::new();
    let mut start_spans = spans_for(spans, moment_roles(kind));
    if let Some(boundary) = multiple_items(analysis, line) {
        warnings.push(Warning::new(
            WarningCode::MultipleItems,
            "the line looks like it holds more than one item; the first was used",
        ));
        start_spans.retain(|s| s.start < boundary);
        for s in &mut start_spans {
            s.end = s.end.min(boundary);
        }
    }
    let (results, pieces) = overlapping(analysis, &start_spans);
    let mut draft = draft_from(&results, &pieces, &start_spans, line, ctx);

    let end_spans = spans_for(spans, &[Role::EventEnd]);
    if kind == CaptureKind::CalendarEvent && !end_spans.is_empty() {
        let (end_results, end_pieces) = overlapping(analysis, &end_spans);
        let shared_end = end_results.iter().find_map(|r| {
            let range = r.end.is_some() && results.iter().any(|s| std::ptr::eq(*s, *r));
            range.then_some(())
        });
        if shared_end.is_none() {
            let end_draft = draft_from(&end_results, &end_pieces, &end_spans, line, ctx);
            if let Some(time) = end_draft.time.or(end_draft.end_time) {
                draft.end_time = Some(time);
            }
            if end_draft.date.is_some() || end_draft.end_date.is_some() {
                draft.end_date = end_draft.end_date.or(end_draft.date.map(|d| d.0));
            }
        }
    }

    let duration_spans = spans_for(spans, &[Role::Duration]);
    let (_, duration_pieces) = overlapping(analysis, &duration_spans);
    let all_pieces: Vec<&Piece> = analysis.pieces.iter().collect();
    let duration = duration_minutes(&duration_pieces, &duration_spans, line).or_else(|| {
        (kind == CaptureKind::CalendarEvent)
            .then(|| duration_minutes(&pieces, &[], line))
            .flatten()
    });

    if kind != CaptureKind::CalendarEvent && draft.time.is_none() {
        draft.time = draft.end_time;
    }
    let end_time = draft.end_time.take();
    let end_date = draft.end_date.take();
    if kind == CaptureKind::CalendarEvent && draft.time.is_none() {
        if let Some((end, TimeKind::Explicit { meridiem_known })) = end_time {
            let mut end = end;
            if !meridiem_known {
                end = bare_hour(end, line, ctx).0;
            }
            let start_time = end - Duration::minutes(i64::from(ctx.settings.default_event_minutes));
            draft.time = Some((
                start_time,
                TimeKind::Explicit {
                    meridiem_known: true,
                },
            ));
            draft.warnings.push(Warning::new(
                WarningCode::TimeAssumed,
                "only an end time was given; start assumed one event length earlier",
            ));
        }
    }

    let recurrence_spans = spans_for(
        spans,
        &[
            Role::Recurrence,
            Role::RecurrenceStart,
            Role::RecurrenceEnd,
            Role::RecurrenceException,
            Role::RecurrenceWindow,
        ],
    );
    let recurrence = if recurrence_spans.iter().any(|s| s.role == Role::Recurrence) {
        resolve_recurrence(line, kind, &recurrence_spans, &draft, analysis, ctx)
    } else {
        None
    };

    let range_end_weekday = draft.range_end_weekday;
    let season = draft.season;
    let mut start = match &recurrence {
        Some(r) => r.first.clone(),
        None => finish(draft.clone(), kind, line, ctx),
    };
    let mut end_date = end_date;
    if let (Some(b), Some(s)) = (range_end_weekday, &start) {
        let forward = (7 + b.num_days_from_monday() - s.date.weekday().num_days_from_monday()) % 7;
        if forward > 0 && recurrence.is_none() {
            end_date = Some(s.date + Duration::days(i64::from(forward)));
        }
    }
    let mut recurrence = recurrence;
    if recurrence.is_none() && !season {
        if let (Some(last), Some(s)) = (end_date, start.clone()) {
            if last > s.date {
                match kind {
                    CaptureKind::Task => {
                        let mut moment = match s.time {
                            Some(t) => moment_at(
                                last.and_time(t),
                                s.text.clone(),
                                s.warnings.clone(),
                                s.implied.clone(),
                                ctx,
                            ),
                            None => Moment {
                                date: last,
                                ..s.clone()
                            },
                        };
                        moment.alternative = s.alternative;
                        start = Some(moment);
                        end_date = None;
                    }
                    CaptureKind::Reminder => {
                        let rule = Rule::new(crate::facets::recurrence::Frequency::Daily);
                        let rrule = rule.rrule(Some(last));
                        recurrence = Some(ResolvedRecurrence {
                            facets: rule.facets_shape(),
                            rrule,
                            rule,
                            start_date: Some(s.date),
                            end_date: Some(last),
                            until: Some(last),
                            exceptions: Vec::new(),
                            exdates: Vec::new(),
                            first: Some(s.clone()),
                            text: s.text.clone(),
                            warnings: Vec::new(),
                        });
                        end_date = None;
                    }
                    CaptureKind::CalendarEvent => {}
                }
            }
        }
    }
    if start.is_none() && !start_spans.is_empty() {
        warnings.push(Warning::new(
            WarningCode::Unparsed,
            "a date or time phrase could not be resolved",
        ));
    }

    let mut end = None;
    if kind == CaptureKind::CalendarEvent {
        if let Some(s) = &start {
            end = end_role_time(s, end_time, end_date, line, ctx);
            if end.is_none() {
                if let (Some(minutes), Some(local)) = (duration, s.local) {
                    end = Some(moment_at(
                        local + Duration::minutes(minutes),
                        String::new(),
                        Vec::new(),
                        Vec::new(),
                        ctx,
                    ));
                } else if let Some(local) = s.local {
                    let minutes = i64::from(ctx.settings.default_event_minutes);
                    end = Some(moment_at(
                        local + Duration::minutes(minutes),
                        String::new(),
                        vec![Warning::new(
                            WarningCode::EndAssumed,
                            format!("no end given; assumed {minutes} minutes"),
                        )],
                        vec!["end"],
                        ctx,
                    ));
                }
            }
        }
        if let (Some(s), Some(e)) = (&start, &end) {
            if s.all_day && e.date < s.date {
                end = None;
            }
        }
    }

    let lead_spans = spans_for(spans, &[Role::LeadTime, Role::Alert]);
    let (_, lead_pieces) = overlapping(analysis, &lead_spans);
    let lead_minutes = lead_pieces
        .iter()
        .chain(all_pieces.iter().filter(|_| lead_spans.is_empty()))
        .find_map(|p| match p.token {
            Token::LeadMinutes(m) => Some(m),
            _ => None,
        })
        .or_else(|| {
            lead_spans.iter().find_map(|s| {
                crate::en::constants::parse_duration(&line[s.start..s.end]).map(|d| {
                    use crate::types::Timeunit;
                    let get = |u: Timeunit| d.get(u).unwrap_or(0.0);
                    (get(Timeunit::Week) * 10_080.0
                        + get(Timeunit::Day) * 1440.0
                        + get(Timeunit::Hour) * 60.0
                        + get(Timeunit::Minute))
                    .round() as i64
                })
            })
        });

    if underspecified(line) {
        let warning = Warning::new(
            WarningCode::UnderspecifiedRecurrence,
            "the line asks for a repeat without saying how often",
        );
        if let Some(rec) = &mut recurrence {
            if !rec
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::UnderspecifiedRecurrence)
            {
                rec.warnings.push(warning.clone());
            }
        }
        warnings.push(warning);
    } else if let Some(rec) = &recurrence {
        if rec.rule.vague {
            warnings.push(Warning::new(
                WarningCode::UnderspecifiedRecurrence,
                "the repeat doesn't say exactly how often; a guess was used",
            ));
        }
    }
    if vague(line) {
        warnings.push(Warning::new(WarningCode::VagueTime, "the time is vague"));
    }
    if unresolved_reference(line, analysis) && !place_trigger(line) {
        warnings.push(Warning::new(
            WarningCode::UnresolvedReference,
            "the timing depends on something without a date",
        ));
    }
    if start.is_none() && recurrence.is_none() {
        match kind {
            CaptureKind::Reminder if lead_minutes.is_none() && !place_trigger(line) => warnings
                .push(Warning::new(
                    WarningCode::MissingTrigger,
                    "the reminder has no time, date or place",
                )),
            CaptureKind::CalendarEvent if !time_like(line, analysis) => warnings.push(
                Warning::new(WarningCode::MissingDate, "the event has no date or time"),
            ),
            _ => {}
        }
    }
    let alert = alert_of(
        line,
        kind,
        &all_pieces,
        lead_minutes,
        start.as_ref(),
        analysis,
        ctx,
    );
    let stated_trigger = start_spans
        .iter()
        .map(|sp| sp.start)
        .min()
        .is_some_and(|first| {
            let lower = line.to_lowercase();
            ["remind me", "ping me", "alert me", "nudge me", "notify me"]
                .iter()
                .filter_map(|p| lower.find(p).map(|i| i + p.len()))
                .any(|end| {
                    end <= first
                        && line[end..first].split_whitespace().all(|w| {
                            matches!(w.to_lowercase().as_str(), "on" | "at" | "this" | "by")
                        })
                })
        });
    if kind == CaptureKind::Reminder && recurrence.is_none() && !stated_trigger {
        if let (Some(minutes), Some(reference)) = (lead_minutes, start.clone()) {
            let defaulted = reference.implied.contains(&"time");
            if defaulted && minutes >= 1440 && reference.all_day {
                let mut warnings_for = reference.warnings.clone();
                warnings_for.push(Warning::new(
                    WarningCode::LeadApplied,
                    format!("{} minutes before {}", minutes, reference.date),
                ));
                start = Some(Moment {
                    date: reference.date - Duration::days(minutes / 1440),
                    warnings: warnings_for,
                    ..reference.clone()
                });
            }
            let trigger = if defaulted && minutes >= 1440 && reference.all_day {
                None
            } else if defaulted && minutes >= 1440 {
                let days = minutes / 1440;
                Some(
                    (reference.date - Duration::days(days)).and_time(ctx.settings.reminder_default),
                )
            } else {
                reference
                    .local
                    .and_then(|l| l.checked_sub_signed(Duration::minutes(minutes)))
            };
            if let Some(trigger) = trigger {
                let mut warnings_for = reference.warnings.clone();
                warnings_for.push(Warning::new(
                    WarningCode::LeadApplied,
                    format!(
                        "{} minutes before {}",
                        minutes,
                        reference.local.map_or(reference.date.to_string(), |l| l
                            .format("%Y-%m-%d %H:%M")
                            .to_string())
                    ),
                ));
                let mut moment = moment_at(
                    trigger,
                    reference.text.clone(),
                    warnings_for,
                    reference.implied.clone(),
                    ctx,
                );
                moment.alternative = reference.alternative;
                start = Some(moment);
            }
        }
    }
    if let Some(warning) = ambiguous_numeric_date(line, &start_spans, ctx) {
        if let Some(s) = &mut start {
            s.warnings.push(warning);
        }
    }
    if let Some(s) = &mut start {
        if s.text.is_empty() {
            s.text = start_spans
                .iter()
                .map(|sp| &line[sp.start..sp.end])
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    let notify_at = match kind {
        CaptureKind::CalendarEvent => None,
        _ => start
            .as_ref()
            .filter(|s| s.all_day)
            .map(|s| s.date.and_time(ctx.settings.default_notification_time)),
    };
    LineResolution {
        kind,
        start,
        end,
        recurrence,
        alert,
        lead_minutes,
        notify_at,
        task_signal: task_signal(line),
        warnings,
    }
}

fn alert_of(
    line: &str,
    kind: CaptureKind,
    pieces: &[&Piece],
    lead_minutes: Option<i64>,
    start: Option<&Moment>,
    analysis: &Analysis,
    ctx: &Context,
) -> Option<Alert> {
    if pieces.iter().any(|p| p.token == Token::AlertOff) {
        return Some(Alert::Off);
    }
    if kind == CaptureKind::CalendarEvent {
        if let Some(at) = pieces.iter().find(|p| p.token == Token::AlertAt) {
            let result = analysis
                .results
                .iter()
                .find(|r| r.index > at.start && r.index <= at.end + 2)?;
            let draft = chrono_draft(result, ctx);
            let mut moment = finish(draft, CaptureKind::Reminder, line, ctx)?;
            if let (Some(s), true) = (start, moment.implied.contains(&"date")) {
                if let Some(t) = moment.time {
                    let local = s.date.and_time(t);
                    moment = moment_at(local, moment.text.clone(), Vec::new(), vec!["date"], ctx);
                }
            }
            return Some(Alert::Absolute { at: moment });
        }
        if let Some(minutes) = lead_minutes {
            let at = start.and_then(|s| s.local).map(|l| {
                moment_at(
                    l - Duration::minutes(minutes),
                    String::new(),
                    Vec::new(),
                    Vec::new(),
                    ctx,
                )
            });
            return Some(Alert::Relative {
                minutes_before: minutes,
                at,
            });
        }
    }
    None
}

fn resolve_recurrence(
    line: &str,
    kind: CaptureKind,
    spans: &[LabelledSpan],
    start_draft: &Draft,
    analysis: &Analysis,
    ctx: &Context,
) -> Option<ResolvedRecurrence> {
    let text_of = |role: Role| -> String {
        spans
            .iter()
            .filter(|s| s.role == role)
            .map(|s| &line[s.start..s.end])
            .collect::<Vec<_>>()
            .join(" ")
    };
    let rule_text = text_of(Role::Recurrence);
    let window_text = text_of(Role::RecurrenceWindow);
    let combined = format!("{rule_text} {window_text}");
    let mut rule = crate::facets::recurrence::parse_rule_scoped(
        &combined,
        &ctx.settings,
        ctx.holiday_scope(),
    )?;
    let mut warnings = Vec::new();

    let explicit_start_time = matches!(start_draft.time, Some((_, TimeKind::Explicit { .. })));
    if rule.time.is_none() || (explicit_start_time && !rule.time_is_bare && !has_clock(&combined)) {
        if let Some((t, kind_of_time)) = start_draft.time {
            rule.time = Some(t);
            rule.time_is_bare = matches!(
                kind_of_time,
                TimeKind::Explicit {
                    meridiem_known: false
                }
            );
        }
    }
    let day_unspecified = rule.weekdays.is_empty() && rule.month_days.is_empty();
    if day_unspecified {
        match (rule.frequency, start_draft.date) {
            (crate::facets::recurrence::Frequency::Weekly, Some((_, DateKind::Weekday(w, _)))) => {
                rule.weekdays = vec![crate::facets::recurrence::ByDay {
                    position: None,
                    weekday: w,
                }];
            }
            (
                crate::facets::recurrence::Frequency::Yearly,
                Some((d, DateKind::MonthDayNoYear | DateKind::Exact)),
            ) if rule.months.is_empty() => {
                rule.months = vec![d.month()];
                rule.month_days = vec![d.day() as i8];
            }
            (crate::facets::recurrence::Frequency::Yearly, Some((d, DateKind::MonthOnly)))
                if rule.months.is_empty() =>
            {
                rule.months = vec![d.month()];
            }
            (crate::facets::recurrence::Frequency::Monthly, Some((d, DateKind::Exact)))
                if start_draft.implied.contains(&"month") =>
            {
                rule.month_days = vec![d.day() as i8];
            }
            _ => {}
        }
    }
    if rule.time.is_none() {
        if let Some(activity) = line_activity(line) {
            let t = activity_time(activity, ctx);
            rule.time = Some(t);
            warnings.push(Warning::new(
                WarningCode::TimeAssumed,
                format!("time assumed {} from the activity", t.format("%H:%M")),
            ));
        }
    }
    if rule.time_is_bare {
        if let Some(t) = rule.time {
            let (t, warning, _) = bare_hour(t, line, ctx);
            rule.time = Some(t);
            warnings.extend(warning);
        }
    }

    let start_spans: Vec<LabelledSpan> = spans
        .iter()
        .filter(|s| s.role == Role::RecurrenceStart)
        .copied()
        .collect();
    let end_spans: Vec<LabelledSpan> = spans
        .iter()
        .filter(|s| s.role == Role::RecurrenceEnd)
        .copied()
        .collect();
    let mut start_date = bound_date(line, &start_spans, analysis, ctx, false, &mut warnings);
    let has_day_spec_early =
        !rule.weekdays.is_empty() || !rule.month_days.is_empty() || !rule.months.is_empty();
    if start_date.is_none()
        && has_day_spec_early
        && matches!(
            start_draft.date,
            Some((_, DateKind::Exact | DateKind::MonthDayNoYear))
        )
        && !start_draft.implied.contains(&"month")
    {
        let mut date_only = start_draft.clone();
        date_only.time = None;
        let candidate = finish(date_only, CaptureKind::CalendarEvent, line, ctx).map(|m| m.date);
        let restates_rule = |d: NaiveDate| {
            rule.frequency == crate::facets::recurrence::Frequency::Yearly
                && (rule.months.is_empty() || rule.months.contains(&d.month()))
        };
        start_date = candidate.filter(|d| rule.span_days.is_none() && !restates_rule(*d));
    }
    let mut end_date = bound_date(line, &end_spans, analysis, ctx, true, &mut warnings);

    let exception_text = text_of(Role::RecurrenceException);
    let mut exceptions = exceptions_in(&exception_text);
    if exceptions.is_empty() && !exception_text.trim().is_empty() {
        exceptions.push(RecurrenceException {
            raw: exception_text.trim().to_string(),
            condition: None,
            action: None,
        });
    }

    let today = ctx.today();
    let has_day_spec = !rule.weekdays.is_empty()
        || !rule.month_days.is_empty()
        || !rule.months.is_empty()
        || rule.holiday.is_some();
    let anchor = start_date
        .or_else(|| {
            if has_day_spec {
                return None;
            }
            match start_draft.date {
                Some((_, DateKind::Weekday(w, _))) => {
                    let forward =
                        (7 + w.num_days_from_monday() - today.weekday().num_days_from_monday()) % 7;
                    Some(today + Duration::days(i64::from(forward)))
                }
                Some((d, DateKind::Exact)) => Some(d),
                Some((d, DateKind::Relative)) if rule.span_days.is_none() => Some(d),
                Some((d, DateKind::MonthDayNoYear)) if d >= today => Some(d),
                Some((d, DateKind::MonthDayNoYear)) => d.with_year(d.year() + 1),
                _ => None,
            }
        })
        .unwrap_or(today);
    let mut exception_warnings: Vec<Warning> = Vec::new();
    if rule.vague {
        warnings.push(Warning::new(
            WarningCode::UnderspecifiedRecurrence,
            "the repeat doesn't say exactly how often; a guess was used",
        ));
    }
    if rule.biweekly {
        warnings.push(Warning::new(
            WarningCode::AmbiguousBiweekly,
            "biweekly read as every two weeks; it can also mean twice a week",
        ));
    }
    if rule.holiday.is_some() {
        warnings.push(Warning::new(
            WarningCode::HolidayDates,
            "this holiday moves each year, so the repeat is listed by date",
        ));
    }
    if rule.count.is_none() && rule.time.is_none() && twice_daily(&combined) {
        return Some(ResolvedRecurrence {
            facets: rule.facets_shape(),
            rrule: rule.rrule(None),
            rule,
            start_date,
            end_date,
            until: None,
            exceptions,
            exdates: Vec::new(),
            first: None,
            text: rule_text,
            warnings,
        });
    }

    let time = rule
        .time
        .or_else(|| kind_default_time(kind, ctx).map(|(t, _)| t));
    let now = ctx.local_now();
    let sub_daily = matches!(
        rule.frequency,
        crate::facets::recurrence::Frequency::Hourly
            | crate::facets::recurrence::Frequency::Minutely
    );
    let until = end_date;
    let mut exdates: Vec<NaiveDateTime> = Vec::new();
    let first: Option<NaiveDateTime> = if sub_daily {
        let anchor_time = rule.time.map(|t| anchor.and_time(t));
        let first = crate::facets::recurrence::first_slot(&rule, now, anchor_time);
        if let (Some(f), Some(span)) = (first, rule.span_days) {
            if let Some(end) = add_span(f.date(), span) {
                let last = end.and_time(NaiveTime::MIN) - Duration::minutes(1);
                let count =
                    crate::facets::recurrence::slot_occurrences(&rule, f, last, 10_000).len();
                rule.count = u32::try_from(count).ok();
                end_date = Some(end);
            }
        }
        first
    } else {
        let free = if has_day_spec && rule.holiday.is_none() {
            ignoring_interval(&rule)
        } else {
            rule.clone()
        };
        let first_date = if let Some(holiday) = rule.holiday {
            let candidate = holiday.next_on_or_after(anchor);
            match (candidate, time) {
                (Some(d), Some(t)) if d.and_time(t) < now => {
                    holiday.next_on_or_after(d + Duration::days(1))
                }
                (c, _) => c,
            }
        } else {
            first_occurrence(&free, anchor, time, now, ctx.settings.week_start, &[])
        };
        let mut weekday_removed = Vec::new();
        for exception in &exceptions {
            let days =
                crate::facets::recurrence::exception_weekdays(std::slice::from_ref(exception));
            if rule.frequency == crate::facets::recurrence::Frequency::Weekly
                && !days.is_empty()
                && days.iter().all(|d| {
                    rule.weekdays
                        .iter()
                        .any(|b| b.weekday == *d && b.position.is_none())
                })
                && rule.weekdays.len() > days.len()
            {
                rule.weekdays.retain(|b| !days.contains(&b.weekday));
                weekday_removed.extend(days);
            }
        }
        let first_date = if weekday_removed.is_empty() {
            first_date
        } else {
            first_occurrence(
                &ignoring_interval(&rule),
                anchor,
                time,
                now,
                ctx.settings.week_start,
                &[],
            )
        };
        first_date.map(|first_date| {
            let horizon = match (rule.count, rule.span_days, until) {
                (_, _, Some(u)) => u,
                (Some(_), _, None) => first_date + Duration::days(3660),
                (None, Some(span), None) => add_span(first_date, span)
                    .and_then(|d| d.pred_opt())
                    .unwrap_or(first_date),
                (None, None, None) => first_date
                    .checked_add_months(Months::new(12))
                    .unwrap_or(first_date),
            };
            let mut dates = crate::facets::recurrence::occurrence_dates(
                &rule,
                first_date,
                ctx.settings.week_start,
                horizon,
            );
            if let Some(count) = rule.count {
                dates.truncate(count as usize);
            }
            let (excluded, unresolved) =
                exception_dates(&exceptions, &weekday_removed, &dates, ctx);
            for raw in unresolved {
                exception_warnings.push(Warning::new(
                    WarningCode::UnresolvedException,
                    format!("\"{raw}\" could not be turned into dates"),
                ));
            }
            let skip_first = exceptions
                .iter()
                .any(|e| e.action.as_deref() == Some("skip_first"));
            let mut excluded = excluded;
            if skip_first {
                if let Some(first) = dates.iter().find(|d| !excluded.contains(d)) {
                    excluded.push(*first);
                }
            }
            excluded.sort_unstable();
            excluded.dedup();
            if rule.count.is_none() {
                if let Some(span) = rule.span_days {
                    let kept = dates.iter().filter(|d| !excluded.contains(d)).count();
                    rule.count = u32::try_from(kept).ok();
                    end_date = add_span(first_date, span);
                }
            }
            let at = time.unwrap_or(NaiveTime::MIN);
            exdates = excluded.iter().map(|d| d.and_time(at)).collect();
            let first = dates
                .iter()
                .find(|d| !excluded.contains(d))
                .copied()
                .unwrap_or(first_date);
            first.and_time(at)
        })
    };

    if let (Some(start), Some(f)) = (start_date, first) {
        if f.date() != start {
            warnings.push(Warning::new(
                WarningCode::StartAdjusted,
                format!(
                    "the start {} does not match the rule; first occurrence is {}",
                    start,
                    f.date()
                ),
            ));
        }
    }

    let first_moment = first.map(|f| {
        if rule.time.is_none() && !sub_daily {
            let warnings = match kind {
                CaptureKind::CalendarEvent => vec![
                    Warning::new(WarningCode::AllDayAssumed, "no time given; all-day"),
                    Warning::new(WarningCode::NoTime, "the repeat has no time"),
                ],
                CaptureKind::Reminder => {
                    vec![Warning::new(WarningCode::NoTime, "the repeat has no time")]
                }
                CaptureKind::Task => Vec::new(),
            };
            Moment {
                date: f.date(),
                time: None,
                local: None,
                instant: None,
                all_day: true,
                implied: vec!["time"],
                warnings,
                text: rule_text.clone(),
                alternative: None,
            }
        } else {
            let mut implied = Vec::new();
            let mut w = Vec::new();
            if rule.time.is_none() && !sub_daily {
                implied.push("time");
                w.push(Warning::new(
                    WarningCode::TimeAssumed,
                    format!("time assumed {}", f.time().format("%H:%M")),
                ));
                if kind == CaptureKind::Reminder {
                    w.push(Warning::new(WarningCode::NoTime, "the repeat has no time"));
                }
            }
            moment_at(f, rule_text.clone(), w, implied, ctx)
        }
    });

    warnings.extend(exception_warnings);
    let facets = rule.facets_shape();
    let rrule = rule.rrule(until);
    Some(ResolvedRecurrence {
        rule,
        rrule,
        facets,
        start_date,
        end_date,
        until,
        exceptions,
        exdates,
        first: first_moment,
        text: rule_text,
        warnings,
    })
}

fn exception_dates(
    exceptions: &[RecurrenceException],
    weekday_removed: &[Weekday],
    occurrences: &[NaiveDate],
    ctx: &Context,
) -> (Vec<NaiveDate>, Vec<String>) {
    let mut out = Vec::new();
    let mut unresolved = Vec::new();
    let today = ctx.today();
    for exception in exceptions {
        let raw = exception.raw.as_str();
        let lower = raw.to_lowercase();
        let mut candidates: Vec<Box<dyn Fn(NaiveDate) -> bool>> = Vec::new();
        let days = crate::facets::recurrence::exception_weekdays(std::slice::from_ref(exception));
        let days: Vec<Weekday> = days
            .into_iter()
            .filter(|d| !weekday_removed.contains(d))
            .collect();
        if !days.is_empty() {
            candidates.push(Box::new(move |d: NaiveDate| days.contains(&d.weekday())));
        }
        let scope = ctx.holiday_scope();
        if lower.contains("holiday") && !lower.contains("school") && scope.region().is_some() {
            candidates.push(Box::new(move |d: NaiveDate| {
                crate::facets::holidays::is_day_off(scope, d)
            }));
        }
        let holidays: Vec<crate::facets::Holiday> =
            crate::facets::holidays::find_holidays(raw, scope)
                .into_iter()
                .map(|h| h.holiday)
                .collect();
        let week_of = lower.contains("week of") || (lower.contains("week") && !holidays.is_empty());
        let analysis = analyse(raw, ctx);
        let mut months: Vec<u32> = Vec::new();
        for result in &analysis.results {
            let c = &result.start;
            if c.is_certain(Component::Month) && !c.is_certain(Component::Day) {
                months.extend(c.get(Component::Month).and_then(|m| u32::try_from(m).ok()));
            }
        }
        for piece in &analysis.pieces {
            if let Token::Season(season, _) = piece.token {
                let range: &[u32] = match season {
                    Season::Spring => &[3, 4, 5],
                    Season::Summer => &[6, 7, 8],
                    Season::Autumn => &[9, 10, 11],
                    Season::Winter => &[12, 1, 2],
                };
                months.extend_from_slice(range);
            }
        }
        if !months.is_empty() && !week_of {
            let months = months.clone();
            candidates.push(Box::new(move |d: NaiveDate| months.contains(&d.month())));
        }
        let mut month_days: Vec<(Option<u32>, u32)> = Vec::new();
        for piece in &analysis.pieces {
            if let Token::OrdinalDay(day) = piece.token {
                month_days.push((None, day));
            }
        }
        for result in &analysis.results {
            let c = &result.start;
            if c.is_certain(Component::Day) && c.is_certain(Component::Month) {
                if let (Some(m), Some(d)) = (c.get(Component::Month), c.get(Component::Day)) {
                    month_days.push((u32::try_from(m).ok(), u32::try_from(d).unwrap_or(0)));
                }
            }
        }
        if week_of {
            let mut anchors: Vec<NaiveDate> = Vec::new();
            for (m, d) in &month_days {
                let found = match m {
                    Some(m) => (today.year()..=today.year() + 1)
                        .filter_map(|y| NaiveDate::from_ymd_opt(y, *m, *d))
                        .find(|x| *x >= today),
                    None => next_ordinal_day(today, *d),
                };
                anchors.extend(found);
            }
            for h in &holidays {
                anchors.extend(h.next_on_or_after(today));
            }
            let start = ctx.settings.week_start;
            let ranges: Vec<(NaiveDate, NaiveDate)> = anchors
                .into_iter()
                .map(|a| {
                    let begin = next_week_start(a, start) - Duration::days(7);
                    (begin, begin + Duration::days(6))
                })
                .collect();
            candidates.push(Box::new(move |d: NaiveDate| {
                ranges.iter().any(|(a, b)| d >= *a && d <= *b)
            }));
        } else {
            if !holidays.is_empty() {
                candidates.push(Box::new(move |d: NaiveDate| {
                    holidays.iter().any(|h| h.date_in(d.year()) == Some(d))
                }));
            }
            if !month_days.is_empty() {
                candidates.push(Box::new(move |d: NaiveDate| {
                    month_days
                        .iter()
                        .any(|(m, day)| d.day() == *day && m.is_none_or(|m| m == d.month()))
                }));
            }
        }
        if candidates.is_empty()
            && exception.action.as_deref() != Some("skip_first")
            && !exception_removed_weekday(exception, weekday_removed)
        {
            unresolved.push(exception.raw.clone());
        }
        out.extend(
            occurrences
                .iter()
                .filter(|d| candidates.iter().any(|c| c(**d)))
                .copied(),
        );
    }
    (out, unresolved)
}

fn exception_removed_weekday(exception: &RecurrenceException, removed: &[Weekday]) -> bool {
    let days = crate::facets::recurrence::exception_weekdays(std::slice::from_ref(exception));
    !days.is_empty() && days.iter().all(|d| removed.contains(d))
}

fn bound_date(
    line: &str,
    spans: &[LabelledSpan],
    analysis: &Analysis,
    ctx: &Context,
    is_end: bool,
    warnings: &mut Vec<Warning>,
) -> Option<NaiveDate> {
    if spans.is_empty() {
        return None;
    }
    let text: String = spans
        .iter()
        .map(|s| &line[s.start..s.end])
        .collect::<Vec<_>>()
        .join(" ");
    let lower = text.to_lowercase();
    let today = ctx.today();
    if is_end {
        let month = crate::facets::recurrence::month_only(&lower);
        if let Some(month) = month {
            let mut year = today.year();
            if month < today.month() || (month == today.month()) {
                year += 1;
            }
            let first = NaiveDate::from_ymd_opt(year, month, 1)?;
            let through =
                lower.contains("through") || lower.contains("thru") || lower.contains("end of");
            return if through {
                Some(last_day_of_month(first))
            } else {
                warnings.push(Warning::new(
                    WarningCode::EndAssumed,
                    format!(
                        "until {} read as ending {}",
                        first.format("%B"),
                        first.pred_opt()?
                    ),
                ));
                first.pred_opt()
            };
        }
    }
    let _ = analysis;
    let sub = analyse(&text, ctx);
    let whole = [LabelledSpan::new(Role::EventStart, 0, sub.norm.text.len())];
    let (results, pieces) = overlapping(&sub, &whole);
    let draft = draft_from(&results, &pieces, &whole, &sub.norm.text, ctx);
    let moment = finish(draft, CaptureKind::CalendarEvent, &sub.norm.text, ctx)?;
    Some(moment.date)
}

pub fn resolve_phrase(
    phrase: &str,
    role: Role,
    kind: CaptureKind,
    ctx: &Context,
) -> LineResolution {
    let spans = [LabelledSpan::new(role, 0, phrase.len())];
    resolve_line(phrase, kind, &spans, ctx)
}

pub fn detect_spans(line: &str, kind: CaptureKind, ctx: &Context) -> Vec<LabelledSpan> {
    let analysis = analyse(line, ctx);
    detect_with(&analysis.norm.text, kind, &analysis)
        .into_iter()
        .map(|s| {
            LabelledSpan::new(
                s.role,
                analysis.norm.to_original(s.start),
                analysis.norm.to_original(s.end),
            )
        })
        .collect()
}

fn detect_with(line: &str, kind: CaptureKind, analysis: &Analysis) -> Vec<LabelledSpan> {
    let scope = analysis.scope;
    let moment_role = moment_roles(kind)[0];
    let mut spans = Vec::new();
    let phrases = find_recurrence_phrases(line, scope);
    let has_rule = phrases.iter().any(|(k, _, _)| *k == PhraseKind::Rule);
    let has_any_rule = has_rule
        || phrases.iter().any(|(k, _, _)| {
            phrase_role(*k, kind, has_rule).is_some_and(|r| r == Role::Recurrence)
        });
    for (phrase, start, end) in phrases {
        let Some(role) = phrase_role(phrase, kind, has_rule) else {
            continue;
        };
        if !has_any_rule && role != Role::Recurrence {
            continue;
        }
        spans.push(LabelledSpan::new(role, start, end));
    }
    let covered = |start: usize, end: usize, spans: &[LabelledSpan]| {
        spans.iter().any(|s| s.start <= start && end <= s.end)
    };
    for piece in &analysis.pieces {
        let role = match piece.token {
            Token::Duration(_) => Role::Duration,
            Token::LeadMinutes(_) => Role::LeadTime,
            Token::AlertOff | Token::AlertAt => continue,
            Token::ClockRange {
                contextual: false, ..
            } => continue,
            _ => moment_role,
        };
        if !covered(piece.start, piece.end, &spans) {
            spans.push(LabelledSpan::new(role, piece.start, piece.end));
        }
    }
    let alert_at: Vec<(usize, usize)> = analysis
        .pieces
        .iter()
        .filter(|p| p.token == Token::AlertAt)
        .map(|p| (p.start, p.end))
        .collect();
    for result in &analysis.results {
        let (start, end) = (result.index, result.index + result.text.len());
        if covered(start, end, &spans)
            || (kind == CaptureKind::CalendarEvent
                && alert_at.iter().any(|(a, b)| start > *a && start <= *b + 2))
        {
            continue;
        }
        if spans
            .iter()
            .any(|s| matches!(s.role, Role::Duration | Role::LeadTime) && s.overlaps(start, end))
        {
            continue;
        }
        let role = if kind == CaptureKind::CalendarEvent
            && result.text.to_lowercase().trim_start().starts_with("to ")
        {
            Role::EventEnd
        } else {
            moment_role
        };
        spans.push(LabelledSpan::new(role, start, end));
    }
    spans
}

pub fn resolve_unlabelled(line: &str, kind: CaptureKind, ctx: &Context) -> LineResolution {
    let analysis = analyse(line, ctx);
    let text = analysis.norm.text.clone();
    let spans = detect_with(&text, kind, &analysis);
    resolve_with(&text, kind, &spans, &analysis, ctx)
}

fn has_clock(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && (text.contains(':')
            || text.to_lowercase().contains("at ")
            || text.to_lowercase().contains("am")
            || text.to_lowercase().contains("pm"))
}

fn twice_daily(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "twice a day",
        "two times a day",
        "three times a day",
        "twice daily",
    ]
    .iter()
    .any(|p| lower.contains(p))
}

fn is_time_role(role: Role) -> bool {
    !matches!(role, Role::Alert)
}

fn regions_of(line: &str, spans: &[LabelledSpan]) -> Vec<(usize, usize)> {
    let mut sorted: Vec<(usize, usize)> = spans
        .iter()
        .filter(|s| is_time_role(s.role))
        .map(|s| (s.start, s.end))
        .collect();
    sorted.sort_unstable();
    let mut regions: Vec<(usize, usize)> = Vec::new();
    for (start, end) in sorted {
        if let Some(last) = regions.last_mut() {
            let gap = line
                .get(last.1.min(start)..start)
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let joinable = gap.is_empty()
                || gap.chars().all(|c| !c.is_alphanumeric())
                || matches!(
                    gap.as_str(),
                    "and" | "of" | "at" | "on" | "from" | "to" | "the" | "every"
                );
            if start <= last.1 || joinable {
                last.1 = last.1.max(end);
                continue;
            }
        }
        regions.push((start, end));
    }
    regions
}

pub fn resolve_capture(
    line: &str,
    kind: CaptureKind,
    spans: &[LabelledSpan],
    ctx: &Context,
) -> LineResolution {
    let analysis = analyse(line, ctx);
    let text = analysis.norm.text.clone();
    let model: Vec<LabelledSpan> = spans
        .iter()
        .filter(|s| s.start <= s.end && s.end <= line.len())
        .map(|s| {
            LabelledSpan::new(
                s.role,
                analysis.norm.to_normalized(floor_boundary(line, s.start)),
                analysis.norm.to_normalized(floor_boundary(line, s.end)),
            )
        })
        .collect();
    let detected = detect_with(&text, kind, &analysis);
    if !model.iter().any(|s| is_time_role(s.role)) {
        let mut resolution = resolve_with(&text, kind, &detected, &analysis, ctx);
        if !detected.is_empty() {
            resolution.warnings.push(Warning::new(
                WarningCode::DateAssumed,
                "no date or time phrase was marked; found one in the text",
            ));
        }
        return resolution;
    }
    let regions = regions_of(&text, &model);
    let inside = |s: &LabelledSpan| regions.iter().any(|(a, b)| s.start < *b && *a < s.end);
    let mut chosen: Vec<LabelledSpan> = detected
        .iter()
        .filter(|s| inside(s))
        .map(|s| {
            let cover = model.iter().find(|m| {
                m.start <= s.start
                    && s.end <= m.end
                    && matches!(m.role, Role::EventEnd | Role::Duration | Role::LeadTime)
            });
            match (cover, s.role.is_moment()) {
                (Some(m), true) => LabelledSpan::new(m.role, s.start, s.end),
                _ => *s,
            }
        })
        .collect();
    if chosen.is_empty() {
        chosen = model;
    }
    resolve_with(&text, kind, &chosen, &analysis, ctx)
}

fn floor_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn zone_label(text: &str) -> String {
    text.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ','))
        .filter(|t| !t.is_empty())
        .rev()
        .find(|t| {
            let upper = t.to_uppercase();
            crate::default_timezone_abbr(&upper).is_some()
                || upper.starts_with("UTC")
                || upper.starts_with("GMT")
                || upper == "Z"
                || t.starts_with(['+', '-'])
        })
        .unwrap_or("")
        .to_string()
}

fn seasonal_zone(label: &str) -> Option<chrono_tz::Tz> {
    match label.to_uppercase().as_str() {
        "PT" => Some(chrono_tz::America::Los_Angeles),
        "ET" => Some(chrono_tz::America::New_York),
        "CT" => Some(chrono_tz::America::Chicago),
        "MT" => Some(chrono_tz::America::Denver),
        "CET" => Some(chrono_tz::Europe::Paris),
        _ => None,
    }
}

fn ambiguous_numeric_date(line: &str, spans: &[LabelledSpan], ctx: &Context) -> Option<Warning> {
    static PATTERN: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"(?:^|[^\d/.:-])(\d{1,2})[/.-](\d{1,2})(?:[/.-](\d{2,4}))?(?:$|[^\d/.:-])",
        )
        .ok()
    });
    let pattern = PATTERN.as_ref()?;
    for c in pattern.captures_iter(line) {
        let (Some(a), Some(b)) = (c.get(1), c.get(2)) else {
            continue;
        };
        if !spans.iter().any(|s| s.overlaps(a.start(), b.end())) {
            continue;
        }
        let (Ok(x), Ok(y)) = (a.as_str().parse::<u32>(), b.as_str().parse::<u32>()) else {
            continue;
        };
        if (1..=12).contains(&x)
            && (1..=12).contains(&y)
            && x != y
            && (c.get(3).is_some()
                || line[b.end()..]
                    .trim_start()
                    .chars()
                    .next()
                    .is_none_or(|ch| !ch.is_ascii_digit()))
        {
            let (month, day, other_month, other_day) = if ctx.settings.day_first {
                (y, x, x, y)
            } else {
                (x, y, y, x)
            };
            return Some(Warning::new(
                WarningCode::AmbiguousDateFormat,
                format!(
                    "{}/{} read as month {} day {}; it could also be month {} day {}",
                    x, y, month, day, other_month, other_day
                ),
            ));
        }
    }
    None
}

fn phrase_role(phrase: PhraseKind, kind: CaptureKind, has_rule: bool) -> Option<Role> {
    Some(match phrase {
        PhraseKind::Rule => Role::Recurrence,
        PhraseKind::OrdinalRule if kind != CaptureKind::Task => Role::Recurrence,
        PhraseKind::RangeRule if kind == CaptureKind::Reminder || has_rule => Role::Recurrence,
        PhraseKind::OrdinalRule | PhraseKind::RangeRule => return None,
        PhraseKind::Window => Role::RecurrenceWindow,
        PhraseKind::Exception => Role::RecurrenceException,
        PhraseKind::Start => Role::RecurrenceStart,
        PhraseKind::End => Role::RecurrenceEnd,
    })
}

fn keep_result(text: &str, result: &ParsedResult) -> bool {
    let raw = result.text.as_str();
    let lower = raw.to_lowercase();
    let stripped = lower.strip_prefix("on ").unwrap_or(&lower);
    let offset = raw.len() - stripped.len();
    let word = stripped.trim_end_matches('.');
    let start = result.index + offset;
    let end = result.index + raw.len();
    if matches!(word, "sat" | "sun") {
        return crate::facets::lexicon::lookalike_is_day(text, start, end);
    }
    let months = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
        "jan",
        "feb",
        "mar",
        "apr",
        "jun",
        "jul",
        "aug",
        "sep",
        "sept",
        "oct",
        "nov",
        "dec",
    ];
    if months.contains(&word) && crate::facets::lexicon::month_is_person(text, start, end) {
        return false;
    }
    true
}

fn underspecified(line: &str) -> bool {
    static PATTERN: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b(?:every\s+so\s+often|regularly|from\s+time\s+to\s+time|(?:every\s+)?now\s+and\s+(?:then|again)|every\s+once\s+in\s+a\s+while|once\s+in\s+a\s+while|occasionally|periodically|sometimes|often|frequently|routinely|on\s+occasion|every\s+few\s+\w+)\b").ok()
    });
    PATTERN.as_ref().is_some_and(|r| r.is_match(line))
}

fn vague(line: &str) -> bool {
    static VAGUE: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b(?:some\s*time|soonish|soon|at\s+some\s+point|in\s+a\s+bit|in\s+a\s+while|asap|as\s+soon\s+as\s+possible|one\s+of\s+these\s+days|whenever|eventually)\b").ok()
    });
    static LATER: std::sync::LazyLock<Option<regex::Regex>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\blater\b").ok());
    if VAGUE.as_ref().is_some_and(|r| r.is_match(line)) {
        return true;
    }
    let Some(later) = LATER.as_ref() else {
        return false;
    };
    later.find_iter(line).any(|m| {
        let before = line[..m.start()].trim_end().to_lowercase();
        let after = line[m.end()..].trim_start().to_lowercase();
        let after_unit = [
            "minute", "minutes", "min", "mins", "hour", "hours", "hr", "hrs", "day", "days",
            "week", "weeks", "month", "months", "year", "years",
        ]
        .iter()
        .any(|u| before.ends_with(u));
        let qualified = [
            "today",
            "tonight",
            "this",
            "on today",
            "that day",
            "in the day",
            "in the week",
            "than",
        ]
        .iter()
        .any(|q| after.starts_with(q));
        !after_unit && !qualified && !before.ends_with("no")
    })
}

fn place_trigger(line: &str) -> bool {
    static PLACE: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b(?:when|once|as\s+soon\s+as|after|next\s+time|whenever)\s+(?:i|we)(?:'m|’m|\s+am|\s+are|'re)?\s+(?:get|gets|arrive|arrives|leave|leaves|reach|land|walk|go|pass|am|at|in|near|back|home|out|by)\b|\b(?:when|once|whenever)\s+(?:i|we)\s+(?:get|arrive|leave|reach|land)\b|\b(?:arriving|leaving|getting)\s+(?:home|to\s+work|work|the\s+office)\b|\bat\s+(?:the\s+)?(?:office|gym|store|shop|supermarket|grocery\s+store|grocery|pharmacy|chemist|airport|station|bank|post\s+office|library|school|work|home|mall|market)\b|\bnext\s+time\s+i'?m\b|\b(?:when|once)\s+i(?:'m|’m|\s+am)\s+(?:at|in|near|by|back|home|out)\b").ok()
    });
    PLACE.as_ref().is_some_and(|r| r.is_match(line))
}

fn time_like(line: &str, analysis: &Analysis) -> bool {
    static TIMEISH: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\b(?:\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?)|\d{1,2}:\d{2}|at\s+\d{1,2}|noon|midnight|morning|afternoon|evening|tonight|tonite|night|today|tomorrow|tmrw|yesterday|weekend|week|month|year|in\s+\d+\s+\w+|monday|tuesday|wednesday|thursday|friday|saturday|sunday)\b").ok()
    });
    !analysis.results.is_empty()
        || analysis
            .pieces
            .iter()
            .any(|p| piece_carries_moment(&p.token))
        || TIMEISH.as_ref().is_some_and(|r| r.is_match(line))
}

fn unresolved_reference(line: &str, analysis: &Analysis) -> bool {
    static REFERENCE: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"\b([Aa]fter|[Bb]efore|[Oo]nce|[Ww]hen|[Uu]ntil|[Tt]ill|[Aa]s\s+soon\s+as)\s+((?:the|my|our|his|her|their|this|that|these|those|it|he|she|they|we|you|[A-Z][a-z]+(?:['’]s)?|[a-z]+['’]s)\b(?:\s+[a-zA-Z'’]+){0,4})").ok()
    });
    let Some(regex) = REFERENCE.as_ref() else {
        return false;
    };
    for c in regex.captures_iter(line) {
        let (Some(whole), Some(rest)) = (c.get(0), c.get(2)) else {
            continue;
        };
        let lower = rest.as_str().to_lowercase();
        let first = lower.split_whitespace().next().unwrap_or("");
        if first.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let date_words = [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
            "christmas",
            "thanksgiving",
            "easter",
            "halloween",
            "today",
            "tomorrow",
            "tonight",
        ];
        if date_words.contains(&first) {
            continue;
        }
        if lower.starts_with("the end")
            || lower.starts_with("the weekend")
            || lower.starts_with("the day")
            || lower.starts_with("the week")
            || lower.starts_with("the month")
        {
            continue;
        }
        let resolved = analysis.results.iter().any(|r| {
            let (rs, re) = (r.index, r.index + r.text.len());
            rs < rest.end() && rest.start() < re
        }) || analysis.pieces.iter().any(|p| {
            piece_carries_moment(&p.token)
                && p.start < rest.end()
                && rest.start() < p.end
                && !matches!(p.token, Token::Weekday(..) if false)
        }) || analysis
            .results
            .iter()
            .any(|r| r.index <= whole.start() && r.index + r.text.len() >= rest.end());
        if resolved {
            continue;
        }
        return true;
    }
    false
}

fn multiple_items(analysis: &Analysis, line: &str) -> Option<usize> {
    static SEPARATOR: std::sync::LazyLock<Option<regex::Regex>> = std::sync::LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)\s+(?:and|then|also|plus)\s+|\s*;\s*|,\s+(?:and\s+|then\s+|also\s+|plus\s+)?",
        )
        .ok()
    });
    let separator = SEPARATOR.as_ref()?;
    let carriers: Vec<(usize, usize)> = analysis
        .results
        .iter()
        .map(|r| (r.index, r.index + r.text.len()))
        .chain(
            analysis
                .pieces
                .iter()
                .filter(|p| piece_carries_moment(&p.token) && !matches!(p.token, Token::Zone(..)))
                .map(|p| (p.start, p.end)),
        )
        .collect();
    let mut cuts: Vec<(usize, usize)> = separator
        .find_iter(line)
        .map(|m| (m.start(), m.end()))
        .collect();
    cuts.retain(|(a, b)| !carriers.iter().any(|(s, e)| *s < *a && *b < *e));
    let has_content = |segment: &str| {
        let filler = [
            "at",
            "on",
            "by",
            "in",
            "the",
            "a",
            "an",
            "and",
            "then",
            "also",
            "plus",
            "every",
            "next",
            "this",
            "last",
            "am",
            "pm",
            "noon",
            "midnight",
            "morning",
            "afternoon",
            "evening",
            "night",
            "tonight",
            "today",
            "tomorrow",
            "week",
            "weekend",
            "month",
            "of",
            "from",
            "to",
            "until",
            "till",
            "before",
            "after",
            "around",
            "about",
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
            "mon",
            "tue",
            "tues",
            "wed",
            "thu",
            "thurs",
            "fri",
            "sat",
            "sun",
            "o'clock",
            "oclock",
            "or",
        ];
        segment
            .split(|c: char| !c.is_alphabetic() && c != '\'')
            .filter(|w| w.len() > 1)
            .any(|w| {
                let lower = w.to_lowercase();
                !filler.contains(&lower.as_str())
                    && crate::facets::recurrence::parse_weekday(&lower)
                        .is_none_or(|_| lower.len() > 9)
                    && ![
                        "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct",
                        "nov", "dec", "st", "nd", "rd", "th",
                    ]
                    .contains(&lower.as_str())
            })
    };
    let has_carrier = |a: usize, b: usize| carriers.iter().any(|(s, e)| *s < b && a < *e);
    let content_count = |segment: &str| {
        segment
            .split(|c: char| !c.is_alphabetic() && c != '\'')
            .filter(|w| {
                w.len() > 1
                    && !matches!(
                        w.to_lowercase().as_str(),
                        "the"
                            | "a"
                            | "an"
                            | "to"
                            | "my"
                            | "our"
                            | "up"
                            | "at"
                            | "on"
                            | "by"
                            | "in"
                            | "for"
                            | "with"
                    )
            })
            .count()
    };
    let mut bounds = vec![0];
    bounds.extend(cuts.iter().map(|(a, _)| *a));
    bounds.push(line.len());
    for (i, (cut_start, cut_end)) in cuts.iter().enumerate() {
        let left = (bounds[i], *cut_start);
        let right_end = cuts.get(i + 1).map_or(line.len(), |(a, _)| *a);
        let right = (*cut_end, right_end);
        if right.0 >= right.1 {
            continue;
        }
        let first_carrier = carriers
            .iter()
            .filter(|(s, e)| *s < right.1 && right.0 < *e)
            .map(|(s, _)| (*s).max(right.0))
            .min();
        let Some(first_carrier) = first_carrier else {
            continue;
        };
        let lead_in = &line[right.0..first_carrier];
        let lead_words = content_count(lead_in);
        if has_carrier(left.0, left.1)
            && has_content(&line[left.0..left.1])
            && has_content(lead_in)
            && lead_words <= 3
        {
            return Some(*cut_start);
        }
    }
    None
}
