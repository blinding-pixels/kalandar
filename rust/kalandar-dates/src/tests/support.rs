use std::sync::LazyLock;

use chrono::{DateTime, Utc};

use crate::chrono::Chrono;
use crate::en;
use crate::results::ParsedResult;
use crate::types::{ParsingOption, ParsingReference, TimezoneAbbr};

static STRICT_WITH_CASUAL_RELATIVE: LazyLock<Chrono> =
    LazyLock::new(en::create_strict_configuration_with_casual_relative_without_abbreviations);
static CASUAL_WITHOUT_TIME_EXPRESSION: LazyLock<Chrono> =
    LazyLock::new(en::create_casual_configuration_without_time_expression);

pub(crate) fn now() -> ParsingReference {
    ParsingReference::new(Utc::now())
}

pub(crate) fn at(millis: i64) -> ParsingReference {
    ParsingReference::new(DateTime::<Utc>::from_timestamp_millis(millis).expect("valid instant"))
}

pub(crate) struct OptionBuilder(ParsingOption);

pub(crate) fn options() -> OptionBuilder {
    OptionBuilder(ParsingOption::default())
}

impl OptionBuilder {
    pub(crate) fn forward(mut self) -> OptionBuilder {
        self.0.forward_date = true;
        self
    }

    pub(crate) fn timezone(mut self, name: &str, abbr: TimezoneAbbr) -> OptionBuilder {
        self.0.timezones.insert(name.to_string(), abbr);
        self
    }
}

fn chrono_for(config: &str) -> &'static Chrono {
    match config {
        "casual" => crate::casual(),
        "strict" => crate::strict(),
        "gb" => crate::gb(),
        "strict+casualRelativeNoAbbr" => &STRICT_WITH_CASUAL_RELATIVE,
        "casual-timeExpression" => &CASUAL_WITHOUT_TIME_EXPRESSION,
        other => panic!("unknown configuration {other}"),
    }
}

pub(crate) fn run(
    config: &str,
    text: &str,
    reference: ParsingReference,
    option: OptionBuilder,
) -> Vec<ParsedResult> {
    chrono_for(config).parse(text, &reference, &option.0)
}

pub(crate) fn index16(text: &str, result: &ParsedResult) -> usize {
    text[..result.index].encode_utf16().count()
}

pub(crate) fn millis(date: Option<DateTime<Utc>>) -> Option<i64> {
    date.map(|d| d.timestamp_millis())
}
