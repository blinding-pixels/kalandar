use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    TimeAssumed,
    DateAssumed,
    DayAssumed,
    AmbiguousMeridiem,
    RolledToTomorrow,
    MovedToNextWeek,
    MovedToNextYear,
    AllDayAssumed,
    EndAssumed,
    DstGapShifted,
    DstOverlapEarlier,
    ApproximateTime,
    StartAdjusted,
    Unparsed,
    NoTime,
    AmbiguousDateFormat,
    AmbiguousBiweekly,
    AmbiguousNextWeekday,
    ConvertedTimezone,
    HolidayDates,
    LeadApplied,
    MultipleItems,
    UnresolvedReference,
    UnresolvedException,
    UnderspecifiedRecurrence,
    MissingTrigger,
    MissingDate,
    VagueTime,
    HolidayOtherRegion,
    HolidayComputed,
    HolidayMoonSighting,
}

impl WarningCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::TimeAssumed => "time_assumed",
            WarningCode::DateAssumed => "date_assumed",
            WarningCode::DayAssumed => "day_assumed",
            WarningCode::AmbiguousMeridiem => "ambiguous_meridiem",
            WarningCode::RolledToTomorrow => "rolled_to_tomorrow",
            WarningCode::MovedToNextWeek => "moved_to_next_week",
            WarningCode::MovedToNextYear => "moved_to_next_year",
            WarningCode::AllDayAssumed => "all_day_assumed",
            WarningCode::EndAssumed => "end_assumed",
            WarningCode::DstGapShifted => "dst_gap_shifted",
            WarningCode::DstOverlapEarlier => "dst_overlap_earlier",
            WarningCode::ApproximateTime => "approximate_time",
            WarningCode::StartAdjusted => "start_adjusted",
            WarningCode::Unparsed => "unparsed",
            WarningCode::NoTime => "no_time",
            WarningCode::AmbiguousDateFormat => "ambiguous_date_format",
            WarningCode::AmbiguousBiweekly => "ambiguous_biweekly",
            WarningCode::AmbiguousNextWeekday => "ambiguous_next_weekday",
            WarningCode::ConvertedTimezone => "converted_timezone",
            WarningCode::HolidayDates => "holiday_dates",
            WarningCode::LeadApplied => "lead_applied",
            WarningCode::MultipleItems => "multiple_items",
            WarningCode::UnresolvedReference => "unresolved_reference",
            WarningCode::UnresolvedException => "unresolved_exception",
            WarningCode::UnderspecifiedRecurrence => "underspecified_recurrence",
            WarningCode::MissingTrigger => "missing_trigger",
            WarningCode::MissingDate => "missing_date",
            WarningCode::VagueTime => "vague_time",
            WarningCode::HolidayOtherRegion => "holiday_other_region",
            WarningCode::HolidayComputed => "holiday_computed",
            WarningCode::HolidayMoonSighting => "holiday_moon_sighting",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub code: WarningCode,
    pub message: String,
}

impl Warning {
    pub fn new(code: WarningCode, message: impl Into<String>) -> Warning {
        Warning {
            code,
            message: message.into(),
        }
    }
}
