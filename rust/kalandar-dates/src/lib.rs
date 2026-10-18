#![forbid(unsafe_code)]

mod calculation;
mod casual_references;
mod chrono;
mod common_parsers;
mod en;
#[cfg(feature = "capture")]
pub mod capture;
mod js_regex;
mod jsdate;
mod refiners;
mod results;
mod text;
mod timezone;
mod types;

use std::sync::LazyLock;

pub use crate::chrono::Chrono;
pub use crate::jsdate::SystemZone;
pub use crate::results::{ParsedComponents, ParsedResult};
pub use crate::timezone::default_timezone_abbr;
pub use crate::types::{
    Component, DstRule, Meridiem, ParsingOption, ParsingReference, TimezoneAbbr, TimezoneInput,
    Weekday,
};
#[cfg(feature = "tz")]
pub use chrono_tz;

static CASUAL: LazyLock<Chrono> = LazyLock::new(|| en::create_casual_configuration(false));
static STRICT: LazyLock<Chrono> = LazyLock::new(|| en::create_configuration(true, false));
static GB: LazyLock<Chrono> = LazyLock::new(|| en::create_casual_configuration(true));

pub fn casual() -> &'static Chrono {
    &CASUAL
}

pub fn strict() -> &'static Chrono {
    &STRICT
}

pub fn gb() -> &'static Chrono {
    &GB
}

pub fn parse(
    text: &str,
    reference: &ParsingReference,
    option: &ParsingOption,
) -> Vec<ParsedResult> {
    casual().parse(text, reference, option)
}

pub fn parse_date(
    text: &str,
    reference: &ParsingReference,
    option: &ParsingOption,
) -> Option<::chrono::DateTime<::chrono::Utc>> {
    casual().parse_date(text, reference, option)
}

#[cfg(test)]
mod tests;
