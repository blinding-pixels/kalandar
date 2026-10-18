mod context;
mod holidays;
mod lexicon;
mod preview;
mod recurrence;
mod resolve;
mod settings;
mod warning;

pub use context::{to_instant, Context};
pub use holidays::{
    data as holiday_data, festival_data_info, find_holidays, holiday_data_info, is_day_off,
    Holiday, HolidayDataInfo, HolidayKind, HolidayMatch, HolidayReach, HolidayScope, HolidaySource,
    YearlyShape,
};
pub use lexicon::task_signal;
pub use preview::{
    capture_schedule, CaptureSchedule, PreviewMoment, PreviewRecurrence, PreviewSchedule,
    PreviewWarning, PreviewWeekday, ScheduleSpan, ScheduleTexts,
};
pub use recurrence::{ByDay, Frequency, RecurrenceException, ReminderRecurrence, Rule};
pub use resolve::{
    detect_spans, resolve_capture, resolve_line, resolve_phrase, resolve_unlabelled,
    schedule_parts, Alert, CaptureKind, LabelledSpan, LineResolution, Moment, ResolvedRecurrence,
    Role, ScheduleParts,
};
pub use settings::{Clock, HolidaySetting, Settings};
pub use warning::{Warning, WarningCode};

pub fn parse_recurrence(text: &str, settings: &Settings) -> Option<Rule> {
    recurrence::parse_rule(text, settings)
}
