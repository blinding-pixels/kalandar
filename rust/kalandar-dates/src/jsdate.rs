use chrono::{DateTime, Utc};

const MS_PER_DAY: f64 = 86_400_000.0;
const MAX_TIME: f64 = 8.64e15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SystemZone {
    #[default]
    Utc,
    Fixed(i32),
    #[cfg(feature = "tz")]
    Iana(chrono_tz::Tz),
}

impl SystemZone {
    #[cfg_attr(not(feature = "tz"), allow(unused_variables))]
    fn offset_ms_at_utc(self, t: f64) -> f64 {
        match self {
            SystemZone::Utc => 0.0,
            SystemZone::Fixed(minutes) => f64::from(minutes) * 60_000.0,
            #[cfg(feature = "tz")]
            SystemZone::Iana(tz) => iana_offset_at_utc(tz, t),
        }
    }

    #[cfg_attr(not(feature = "tz"), allow(unused_variables))]
    fn offset_ms_at_local(self, t: f64) -> f64 {
        match self {
            SystemZone::Utc => 0.0,
            SystemZone::Fixed(minutes) => f64::from(minutes) * 60_000.0,
            #[cfg(feature = "tz")]
            SystemZone::Iana(tz) => iana_offset_at_local(tz, t),
        }
    }
}

#[cfg(feature = "tz")]
fn iana_offset_at_utc(tz: chrono_tz::Tz, t: f64) -> f64 {
    use chrono::{Offset, TimeZone};
    if !t.is_finite() {
        return 0.0;
    }
    match DateTime::<Utc>::from_timestamp_millis(t as i64) {
        Some(instant) => {
            let naive = within_tz_table(instant.naive_utc());
            f64::from(tz.offset_from_utc_datetime(&naive).fix().local_minus_utc()) * 1000.0
        }
        None => 0.0,
    }
}

#[cfg(feature = "tz")]
const LAST_TABLE_YEAR: i32 = 2037;

#[cfg(feature = "tz")]
fn within_tz_table(naive: chrono::NaiveDateTime) -> chrono::NaiveDateTime {
    use chrono::{Datelike, NaiveDate};
    let year = naive.year();
    if year <= LAST_TABLE_YEAR {
        return naive;
    }
    let profile = |y: i32| NaiveDate::from_ymd_opt(y, 1, 1).map(|d| (d.leap_year(), d.weekday()));
    let target = profile(year);
    let equivalent = (LAST_TABLE_YEAR - 27..=LAST_TABLE_YEAR).find(|&y| profile(y) == target);
    match (equivalent, NaiveDate::from_ymd_opt(year, 1, 1)) {
        (Some(equivalent), Some(start)) => match NaiveDate::from_ymd_opt(equivalent, 1, 1) {
            Some(equivalent_start) => naive - (start - equivalent_start),
            None => naive,
        },
        _ => naive,
    }
}

#[cfg(feature = "tz")]
fn iana_offset_at_local(tz: chrono_tz::Tz, t: f64) -> f64 {
    use chrono::{LocalResult, Offset, TimeZone};
    if !t.is_finite() {
        return 0.0;
    }
    let Some(naive) =
        DateTime::<Utc>::from_timestamp_millis(t as i64).map(|d| within_tz_table(d.naive_utc()))
    else {
        return 0.0;
    };
    let seconds = match tz.offset_from_local_datetime(&naive) {
        LocalResult::Single(offset) => offset.fix().local_minus_utc(),
        LocalResult::Ambiguous(earliest, _) => earliest.fix().local_minus_utc(),
        LocalResult::None => {
            let before = naive - chrono::Duration::days(1);
            tz.offset_from_utc_datetime(&before).fix().local_minus_utc()
        }
    };
    f64::from(seconds) * 1000.0
}

#[derive(Clone, Copy, Debug)]
pub struct JsDate {
    time: f64,
    zone: SystemZone,
}

impl PartialEq for JsDate {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl PartialOrd for JsDate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.time.partial_cmp(&other.time)
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !year.is_finite() || !month.is_finite() || !date.is_finite() {
        return f64::NAN;
    }
    let (year, month, date) = (year.trunc(), month.trunc(), date.trunc());
    let ym = year + (month / 12.0).floor();
    if ym.abs() > 400_000.0 {
        return f64::NAN;
    }
    let mn = month.rem_euclid(12.0);
    days_from_civil(ym as i64, mn as i64 + 1, 1) as f64 + date - 1.0
}

fn make_time(hour: f64, minute: f64, second: f64, ms: f64) -> f64 {
    if !hour.is_finite() || !minute.is_finite() || !second.is_finite() || !ms.is_finite() {
        return f64::NAN;
    }
    hour.trunc() * 3_600_000.0 + minute.trunc() * 60_000.0 + second.trunc() * 1000.0 + ms.trunc()
}

fn make_date(day: f64, time: f64) -> f64 {
    if !day.is_finite() || !time.is_finite() {
        return f64::NAN;
    }
    day * MS_PER_DAY + time
}

fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > MAX_TIME {
        return f64::NAN;
    }
    time.trunc() + 0.0
}

fn day_of(t: f64) -> f64 {
    (t / MS_PER_DAY).floor()
}

fn time_within_day(t: f64) -> f64 {
    t.rem_euclid(MS_PER_DAY)
}

fn year_month_date(t: f64) -> (f64, f64, f64) {
    let (y, m, d) = civil_from_days(day_of(t) as i64);
    (y as f64, (m - 1) as f64, d as f64)
}

fn full_year_rule(year: f64) -> f64 {
    let truncated = year.trunc();
    if (0.0..=99.0).contains(&truncated) {
        1900.0 + truncated
    } else {
        year
    }
}

fn to_component(value: f64) -> i32 {
    value as i32
}

impl JsDate {
    pub fn from_time(time: f64, zone: SystemZone) -> JsDate {
        JsDate {
            time: time_clip(time),
            zone,
        }
    }

    pub fn from_instant(instant: DateTime<Utc>, zone: SystemZone) -> JsDate {
        JsDate::from_time(instant.timestamp_millis() as f64, zone)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn local(
        zone: SystemZone,
        year: i32,
        month: i32,
        date: i32,
        hour: i32,
        minute: i32,
        second: i32,
        ms: i32,
    ) -> JsDate {
        let local = make_date(
            make_day(
                full_year_rule(f64::from(year)),
                f64::from(month),
                f64::from(date),
            ),
            make_time(
                f64::from(hour),
                f64::from(minute),
                f64::from(second),
                f64::from(ms),
            ),
        );
        let mut d = JsDate {
            time: f64::NAN,
            zone,
        };
        d.time = time_clip(d.utc_from_local(local));
        d
    }

    #[allow(clippy::too_many_arguments)]
    pub fn utc(
        zone: SystemZone,
        year: i32,
        month: i32,
        date: i32,
        hour: i32,
        minute: i32,
        second: i32,
        ms: i32,
    ) -> JsDate {
        let t = make_date(
            make_day(
                full_year_rule(f64::from(year)),
                f64::from(month),
                f64::from(date),
            ),
            make_time(
                f64::from(hour),
                f64::from(minute),
                f64::from(second),
                f64::from(ms),
            ),
        );
        JsDate {
            time: time_clip(t),
            zone,
        }
    }

    pub fn zone(&self) -> SystemZone {
        self.zone
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn to_instant(self) -> Option<DateTime<Utc>> {
        if self.time.is_nan() {
            return None;
        }
        DateTime::<Utc>::from_timestamp_millis(self.time as i64)
    }

    pub fn plus_ms(&self, delta: f64) -> JsDate {
        JsDate::from_time(self.time + delta, self.zone)
    }

    fn local_time(&self) -> f64 {
        self.time + self.zone.offset_ms_at_utc(self.time)
    }

    fn utc_from_local(&self, local: f64) -> f64 {
        if !local.is_finite() {
            return f64::NAN;
        }
        local - self.zone.offset_ms_at_local(local)
    }

    pub fn timezone_offset_minutes(&self) -> i32 {
        if self.time.is_nan() {
            return 0;
        }
        to_component(-self.zone.offset_ms_at_utc(self.time) / 60_000.0)
    }

    fn local_fields(&self) -> Option<(f64, f64, f64, f64)> {
        if self.time.is_nan() {
            return None;
        }
        let t = self.local_time();
        let (y, m, d) = year_month_date(t);
        Some((y, m, d, time_within_day(t)))
    }

    fn utc_fields(&self) -> Option<(f64, f64, f64, f64)> {
        if self.time.is_nan() {
            return None;
        }
        let (y, m, d) = year_month_date(self.time);
        Some((y, m, d, time_within_day(self.time)))
    }

    pub fn get_full_year(&self) -> i32 {
        self.local_fields().map_or(0, |f| to_component(f.0))
    }

    pub fn get_month(&self) -> i32 {
        self.local_fields().map_or(0, |f| to_component(f.1))
    }

    pub fn get_date(&self) -> i32 {
        self.local_fields().map_or(0, |f| to_component(f.2))
    }

    pub fn get_day(&self) -> i32 {
        if self.time.is_nan() {
            return 0;
        }
        to_component((day_of(self.local_time()) + 4.0).rem_euclid(7.0))
    }

    pub fn get_hours(&self) -> i32 {
        self.local_fields()
            .map_or(0, |f| to_component((f.3 / 3_600_000.0).floor()))
    }

    pub fn get_minutes(&self) -> i32 {
        self.local_fields()
            .map_or(0, |f| to_component((f.3 / 60_000.0).floor() % 60.0))
    }

    pub fn get_seconds(&self) -> i32 {
        self.local_fields()
            .map_or(0, |f| to_component((f.3 / 1000.0).floor() % 60.0))
    }

    pub fn get_milliseconds(&self) -> i32 {
        self.local_fields()
            .map_or(0, |f| to_component(f.3 % 1000.0))
    }

    pub fn get_utc_full_year(&self) -> Option<i32> {
        self.utc_fields().map(|f| to_component(f.0))
    }

    pub fn get_utc_month(&self) -> Option<i32> {
        self.utc_fields().map(|f| to_component(f.1))
    }

    pub fn get_utc_date(&self) -> Option<i32> {
        self.utc_fields().map(|f| to_component(f.2))
    }

    pub fn get_utc_hours(&self) -> Option<i32> {
        self.utc_fields()
            .map(|f| to_component((f.3 / 3_600_000.0).floor()))
    }

    pub fn get_utc_minutes(&self) -> Option<i32> {
        self.utc_fields()
            .map(|f| to_component((f.3 / 60_000.0).floor() % 60.0))
    }

    fn set_local_parts(&mut self, day: f64, time: f64) {
        let local = make_date(day, time);
        self.time = time_clip(self.utc_from_local(local));
    }

    pub fn set_full_year(&mut self, year: f64) {
        let t = if self.time.is_nan() {
            0.0
        } else {
            self.local_time()
        };
        let (_, m, d) = year_month_date(t);
        self.set_local_parts(make_day(year, m, d), time_within_day(t));
    }

    pub fn set_utc_full_year(&mut self, year: f64) {
        let t = if self.time.is_nan() { 0.0 } else { self.time };
        let (_, m, d) = year_month_date(t);
        self.time = time_clip(make_date(make_day(year, m, d), time_within_day(t)));
    }

    pub fn set_month(&mut self, month: f64) {
        if let Some((y, _, d, tod)) = self.local_fields() {
            self.set_local_parts(make_day(y, month, d), tod);
        }
    }

    pub fn set_date(&mut self, date: f64) {
        if let Some((y, m, _, tod)) = self.local_fields() {
            self.set_local_parts(make_day(y, m, date), tod);
        }
    }

    fn set_time_of_day(&mut self, update: impl FnOnce(f64, f64, f64, f64) -> f64) {
        if let Some((y, m, d, tod)) = self.local_fields() {
            let hour = (tod / 3_600_000.0).floor();
            let minute = (tod / 60_000.0).floor() % 60.0;
            let second = (tod / 1000.0).floor() % 60.0;
            let ms = tod % 1000.0;
            let time = update(hour, minute, second, ms);
            self.set_local_parts(make_day(y, m, d), time);
        }
    }

    pub fn set_hours(&mut self, hour: f64) {
        self.set_time_of_day(|_, mi, s, ms| make_time(hour, mi, s, ms));
    }

    pub fn set_minutes(&mut self, minute: f64) {
        self.set_time_of_day(|h, _, s, ms| make_time(h, minute, s, ms));
    }

    pub fn set_seconds(&mut self, second: f64) {
        self.set_time_of_day(|h, mi, _, ms| make_time(h, mi, second, ms));
    }

    pub fn set_milliseconds(&mut self, ms: f64) {
        self.set_time_of_day(|h, mi, s, _| make_time(h, mi, s, ms));
    }
}
