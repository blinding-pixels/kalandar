mod context;
mod holidays;
mod lexicon;
mod recurrence;
mod resolve;
mod settings;
mod warning;

pub use context::{to_instant, Context};
pub use holidays::{
    data as holiday_data, festival_data_info, find_holidays, holiday_data_info, is_day_off,
    Holiday, HolidayDataInfo, HolidayKind, HolidayMatch, HolidayScope, HolidaySource, YearlyShape,
};
pub use lexicon::task_signal;
pub use recurrence::{ByDay, FacetsRecurrence, Frequency, RecurrenceException, Rule};
pub use resolve::{
    detect_spans, resolve_capture, resolve_line, resolve_phrase, resolve_unlabelled, Alert,
    CaptureKind, LabelledSpan, LineResolution, Moment, ResolvedRecurrence, Role,
};
pub use settings::{Clock, HolidaySetting, Settings};
pub use warning::{Warning, WarningCode};

pub fn parse_recurrence(text: &str, settings: &Settings) -> Option<Rule> {
    recurrence::parse_rule(text, settings)
}
