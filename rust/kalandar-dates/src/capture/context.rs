use chrono::{
    DateTime, Duration, FixedOffset, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc,
};
use chrono_tz::Tz;

use crate::capture::settings::Settings;
use crate::capture::warning::{Warning, WarningCode};

#[derive(Clone, Debug)]
pub struct Context {
    pub now: DateTime<Utc>,
    pub zone: Tz,
    pub settings: Settings,
}

impl Context {
    pub fn new(now: DateTime<Utc>, zone: Tz) -> Context {
        Context {
            now,
            zone,
            settings: Settings::default(),
        }
    }

    pub fn with_settings(mut self, settings: Settings) -> Context {
        self.settings = settings;
        self
    }

    pub fn holiday_scope(&self) -> crate::capture::holidays::HolidayScope {
        crate::capture::holidays::HolidayScope::from_settings(
            &self.settings,
            Some(self.zone.name()),
        )
    }

    pub fn local_now(&self) -> NaiveDateTime {
        self.now.with_timezone(&self.zone).naive_local()
    }

    pub fn today(&self) -> NaiveDate {
        let local = self.local_now();
        if local.time() < self.settings.day_start {
            local.date() - Duration::days(1)
        } else {
            local.date()
        }
    }

    pub fn before_day_start(&self) -> bool {
        self.local_now().time() < self.settings.day_start
    }

    pub fn to_instant(&self, local: NaiveDateTime) -> (DateTime<FixedOffset>, Option<Warning>) {
        to_instant(self.zone, local)
    }
}

pub fn to_instant(zone: Tz, local: NaiveDateTime) -> (DateTime<FixedOffset>, Option<Warning>) {
    match zone.from_local_datetime(&local) {
        LocalResult::Single(dt) => (dt.fixed_offset(), None),
        LocalResult::Ambiguous(earliest, _) => (
            earliest.fixed_offset(),
            Some(Warning::new(
                WarningCode::DstOverlapEarlier,
                format!(
                    "{} happens twice because clocks go back; used the first one",
                    local.format("%H:%M")
                ),
            )),
        ),
        LocalResult::None => {
            let before = zone
                .offset_from_utc_datetime(&(local - Duration::days(1)))
                .fix();
            let instant =
                (local - Duration::seconds(i64::from(before.local_minus_utc()))).and_utc();
            let shifted = instant.with_timezone(&zone).fixed_offset();
            (
                shifted,
                Some(Warning::new(
                    WarningCode::DstGapShifted,
                    format!(
                        "{} does not exist because clocks go forward; moved to {}",
                        local.format("%H:%M"),
                        shifted.format("%H:%M")
                    ),
                )),
            )
        }
    }
}
