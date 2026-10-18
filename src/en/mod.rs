pub(crate) mod constants;
pub(crate) mod parsers;

use crate::chrono::{Chrono, Parser, Refiner};
use crate::common_parsers::{IsoFormatParser, SlashDateFormatParser, TimeExpressionParser};
use crate::refiners::{
    EnUnlikelyFormatFilter, ExtractTimezoneAbbrRefiner, ExtractTimezoneOffsetRefiner,
    ExtractYearSuffixRefiner, ForwardDateRefiner, MergeDateRangeRefiner, MergeDateTimeRefiner,
    MergeRelativeAfterDateRefiner, MergeRelativeFollowByDateRefiner, MergeWeekdayComponentRefiner,
    OverlapRemovalRefiner, UnlikelyFormatFilter,
};
use parsers::{
    CasualDateParser, CasualTimeParser, MonthNameLittleEndianParser, MonthNameMiddleEndianParser,
    MonthNameParser, RelativeDateFormatParser, SlashMonthFormatParser, TimeUnitAgoFormatParser,
    TimeUnitCasualRelativeFormatParser, TimeUnitLaterFormatParser, TimeUnitWithinFormatParser,
    WeekdayParser, YearMonthDayParser, YearMonthNameParser,
};

struct Configuration {
    parsers: Vec<Box<dyn Parser>>,
    refiners: Vec<Box<dyn Refiner>>,
}

impl Configuration {
    fn base(strict_mode: bool, little_endian: bool, time_expressions: bool) -> Configuration {
        let mut parsers: Vec<Box<dyn Parser>> = vec![
            Box::new(YearMonthDayParser::new(strict_mode)),
            Box::new(IsoFormatParser::new()),
            Box::new(SlashDateFormatParser::new(little_endian)),
            Box::new(TimeUnitWithinFormatParser::new(strict_mode)),
            Box::new(MonthNameLittleEndianParser::new()),
            Box::new(MonthNameMiddleEndianParser::new(little_endian)),
            Box::new(WeekdayParser::new()),
            Box::new(SlashMonthFormatParser::new()),
        ];
        if time_expressions {
            parsers.push(Box::new(TimeExpressionParser::new(strict_mode)));
        }
        parsers.push(Box::new(TimeUnitAgoFormatParser::new(strict_mode)));
        parsers.push(Box::new(TimeUnitLaterFormatParser::new(strict_mode)));
        parsers.push(Box::new(YearMonthNameParser::new()));

        let refiners: Vec<Box<dyn Refiner>> = vec![
            Box::new(OverlapRemovalRefiner),
            Box::new(MergeRelativeAfterDateRefiner),
            Box::new(MergeRelativeFollowByDateRefiner),
            Box::new(OverlapRemovalRefiner),
            Box::new(ExtractTimezoneOffsetRefiner),
            Box::new(MergeWeekdayComponentRefiner),
            Box::new(MergeDateTimeRefiner),
            Box::new(ExtractTimezoneAbbrRefiner),
            Box::new(OverlapRemovalRefiner),
            Box::new(ForwardDateRefiner),
            Box::new(UnlikelyFormatFilter { strict_mode }),
            Box::new(MergeDateTimeRefiner),
            Box::new(ExtractYearSuffixRefiner::new()),
            Box::new(MergeDateRangeRefiner),
        ];
        Configuration { parsers, refiners }
    }

    fn casual(little_endian: bool, time_expressions: bool) -> Configuration {
        let mut configuration = Configuration::base(false, little_endian, time_expressions);
        configuration
            .parsers
            .push(Box::new(CasualDateParser::new()));
        configuration
            .parsers
            .push(Box::new(CasualTimeParser::new()));
        configuration.parsers.push(Box::new(MonthNameParser::new()));
        configuration
            .parsers
            .push(Box::new(RelativeDateFormatParser::new()));
        configuration
            .parsers
            .push(Box::new(TimeUnitCasualRelativeFormatParser::new(true)));
        configuration
            .refiners
            .push(Box::new(EnUnlikelyFormatFilter));
        configuration
    }

    fn build(self) -> Chrono {
        Chrono::from_configuration(self.parsers, self.refiners)
    }
}

pub(crate) fn create_configuration(strict_mode: bool, little_endian: bool) -> Chrono {
    Configuration::base(strict_mode, little_endian, true).build()
}

pub(crate) fn create_casual_configuration(little_endian: bool) -> Chrono {
    Configuration::casual(little_endian, true).build()
}

#[cfg(test)]
pub(crate) fn create_strict_configuration_with_casual_relative_without_abbreviations() -> Chrono {
    let mut configuration = Configuration::base(true, false, true);
    configuration
        .parsers
        .push(Box::new(TimeUnitCasualRelativeFormatParser::new(false)));
    configuration.build()
}

#[cfg(test)]
pub(crate) fn create_casual_configuration_without_time_expression() -> Chrono {
    Configuration::casual(false, false).build()
}
