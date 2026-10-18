use crate::calculation::{
    create_parsing_components_at_weekday, find_year_closest_to_ref, WeekdayModifier,
};
use crate::casual_references as references;
use crate::chrono::{Extracted, Parser, ParserMatch, ParsingContext};
use crate::common_parsers::word_boundary_pattern;
use crate::en::constants::{
    lookup, match_any_pattern, parse_duration, parse_ordinal_number_pattern, parse_year,
    FULL_MONTH_NAME_DICTIONARY, MONTH_DICTIONARY, ORDINAL_NUMBER_PATTERN,
    TIME_UNITS_NO_ABBR_PATTERN, TIME_UNITS_PATTERN, TIME_UNIT_DICTIONARY, WEEKDAY_DICTIONARY,
    YEAR_PATTERN,
};
use crate::js_regex::{js_re, JsRegex};
use crate::results::ParsedComponents;
use crate::text::parse_int;
use crate::types::{Component, Duration, Weekday};

fn relative(context: &ParsingContext, duration: &Duration) -> Option<Extracted> {
    Some(Extracted::Components(
        ParsedComponents::create_relative_from_reference(context.reference, duration),
    ))
}

pub(crate) struct CasualDateParser {
    pattern: JsRegex,
}

impl CasualDateParser {
    pub fn new() -> CasualDateParser {
        CasualDateParser {
            pattern: word_boundary_pattern(
                "(now|today|tonight|tomorrow|overmorrow|tmr|tmrw|yesterday|last\\s*night)(?=\\W|$)",
                true,
            ),
        }
    }
}

impl Parser for CasualDateParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let lower = m.text().to_lowercase();
        let reference = context.reference;
        let mut component = match lower.as_str() {
            "now" => references::now(reference),
            "today" => references::today(reference),
            "yesterday" => references::yesterday(reference),
            "tomorrow" | "tmr" | "tmrw" => references::tomorrow(reference),
            "tonight" => references::tonight(reference),
            "overmorrow" => references::the_day_after(reference, 2),
            _ => {
                let mut component = context.components();
                if js_re!("last\\s*night").is_match(&lower) {
                    let mut target = reference.instant;
                    if target.get_hours() > 6 {
                        target.set_date(f64::from(target.get_date()) - 1.0);
                    }
                    component.assign_similar_date(&target);
                    component.imply(Component::Hour, 0);
                }
                component
            }
        };
        component.add_tag("parser/ENCasualDateParser");
        Some(Extracted::Components(component))
    }
}

pub(crate) struct CasualTimeParser {
    pattern: JsRegex,
}

impl CasualTimeParser {
    pub fn new() -> CasualTimeParser {
        CasualTimeParser {
            pattern: word_boundary_pattern(
                "(?:this)?\\s{0,3}(morning|afternoon|evening|night|midnight|midday|noon)(?=\\W|$)",
                true,
            ),
        }
    }
}

impl Parser for CasualTimeParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let reference = context.reference;
        let mut component = match m.group(1).to_lowercase().as_str() {
            "afternoon" => references::afternoon(reference),
            "evening" | "night" => references::evening(reference),
            "midnight" => references::midnight(reference),
            "morning" => references::morning(reference),
            "noon" | "midday" => references::noon(reference),
            _ => return None,
        };
        component.add_tag("parser/ENCasualTimeParser");
        Some(Extracted::Components(component))
    }
}

pub(crate) struct MonthNameLittleEndianParser {
    pattern: JsRegex,
}

impl MonthNameLittleEndianParser {
    pub fn new() -> MonthNameLittleEndianParser {
        let ordinal = &*ORDINAL_NUMBER_PATTERN;
        let months = match_any_pattern(MONTH_DICTIONARY);
        let inner = format!(
            "(?:on\\s{{0,3}})?({ordinal})(?:\\s{{0,3}}(?:to|\\-|\\–|until|through|till)\\s{{0,3}}({ordinal}))?(?:-|/|\\s{{0,3}}(?:of)?\\s{{0,3}})({months})(?:(?:-|/|,?\\s{{0,3}})({YEAR_PATTERN}(?!\\w)))?(?=\\W|$)"
        );
        MonthNameLittleEndianParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for MonthNameLittleEndianParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let mut result = context.result(m.index, m.text());
        let month = lookup(MONTH_DICTIONARY, &m.group(3).to_lowercase()).unwrap_or(0);
        let day = parse_ordinal_number_pattern(m.group(1));
        if day > 31 {
            m.index += m.group(1).len();
            return None;
        }
        result.start.assign(Component::Month, month);
        result.start.assign(Component::Day, day);
        if let Some(year) = m.non_empty(4) {
            result.start.assign(Component::Year, parse_year(year));
        } else {
            let year = find_year_closest_to_ref(context.reference.instant, day, month);
            result.start.imply(Component::Year, year);
        }
        if let Some(to) = m.non_empty(2) {
            let end_date = parse_ordinal_number_pattern(to);
            let mut end = result.start.js_clone();
            end.assign(Component::Day, end_date);
            result.end = Some(end);
        }
        Some(Extracted::Result(Box::new(result)))
    }
}

pub(crate) struct MonthNameMiddleEndianParser {
    pattern: JsRegex,
    should_skip_year_like_date: bool,
}

impl MonthNameMiddleEndianParser {
    pub fn new(should_skip_year_like_date: bool) -> MonthNameMiddleEndianParser {
        let ordinal = &*ORDINAL_NUMBER_PATTERN;
        let months = match_any_pattern(MONTH_DICTIONARY);
        let inner = format!(
            "({months})(?:-|/|\\s*,?\\s*)({ordinal})(?!\\s*(?:am|pm))\\s*(?:(?:to|\\-)\\s*({ordinal})\\s*)?(?:(?:-|/|\\s*,\\s*|\\s+)({YEAR_PATTERN}))?(?=\\W|$)(?!\\:\\d)"
        );
        MonthNameMiddleEndianParser {
            pattern: word_boundary_pattern(&inner, true),
            should_skip_year_like_date,
        }
    }
}

impl Parser for MonthNameMiddleEndianParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let month = lookup(MONTH_DICTIONARY, &m.group(1).to_lowercase()).unwrap_or(0);
        let day = parse_ordinal_number_pattern(m.group(2));
        if day > 31 {
            return None;
        }
        if self.should_skip_year_like_date
            && m.non_empty(3).is_none()
            && m.non_empty(4).is_none()
            && js_re!("^\\d{2}$").is_match(m.group(2))
        {
            return None;
        }
        let mut components =
            context.components_with(&[(Component::Day, day), (Component::Month, month)]);
        components.add_tag("parser/ENMonthNameMiddleEndianParser");
        if let Some(year) = m.non_empty(4) {
            components.assign(Component::Year, parse_year(year));
        } else {
            let year = find_year_closest_to_ref(context.reference.instant, day, month);
            components.imply(Component::Year, year);
        }
        let Some(to) = m.non_empty(3) else {
            return Some(Extracted::Components(components));
        };
        let end_date = parse_ordinal_number_pattern(to);
        let mut result = context.result(m.index, m.text());
        let mut end = components.js_clone();
        end.assign(Component::Day, end_date);
        result.start = components;
        result.end = Some(end);
        Some(Extracted::Result(Box::new(result)))
    }
}

pub(crate) struct MonthNameParser {
    pattern: JsRegex,
}

impl MonthNameParser {
    pub fn new() -> MonthNameParser {
        let months = match_any_pattern(MONTH_DICTIONARY);
        let inner = format!(
            "((?:in)\\s*)?({months})\\s*(?:(?:,|-|of)?\\s*({YEAR_PATTERN})?)?(?=[^\\s\\w]|\\s+[^0-9]|\\s+$|$)"
        );
        MonthNameParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for MonthNameParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let month_name = m.group(2).to_lowercase();
        if m.text().encode_utf16().count() <= 3
            && lookup(FULL_MONTH_NAME_DICTIONARY, &month_name).is_none()
        {
            return None;
        }
        let prefix_len = m.group(1).len();
        let mut result = context.result_between(m.index + prefix_len, m.index + m.text().len());
        result.start.imply(Component::Day, 1);
        result.start.add_tag("parser/ENMonthNameParser");
        let month = lookup(MONTH_DICTIONARY, &month_name).unwrap_or(0);
        result.start.assign(Component::Month, month);
        if let Some(year) = m.non_empty(3) {
            result.start.assign(Component::Year, parse_year(year));
        } else {
            let year = find_year_closest_to_ref(context.reference.instant, 1, month);
            result.start.imply(Component::Year, year);
        }
        Some(Extracted::Result(Box::new(result)))
    }
}

pub(crate) struct RelativeDateFormatParser {
    pattern: JsRegex,
}

impl RelativeDateFormatParser {
    pub fn new() -> RelativeDateFormatParser {
        let units = match_any_pattern(TIME_UNIT_DICTIONARY);
        let inner = format!("(this|last|past|next|after\\s*this)\\s*({units})(?=\\s*)(?=\\W|$)");
        RelativeDateFormatParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for RelativeDateFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let modifier = m.group(1).to_lowercase();
        let unit_word = m.group(2).to_lowercase();
        let timeunit = lookup(TIME_UNIT_DICTIONARY, &unit_word)?;

        if modifier == "next" || modifier.starts_with("after") {
            return relative(context, &Duration::new().with(timeunit, 1.0));
        }
        if modifier == "last" || modifier == "past" {
            return relative(context, &Duration::new().with(timeunit, -1.0));
        }

        let mut components = context.components();
        let mut date = context.reference.instant;
        if unit_word.contains("week") {
            date.set_date(f64::from(date.get_date() - date.get_day()));
            components.imply(Component::Day, date.get_date());
            components.imply(Component::Month, date.get_month() + 1);
            components.imply(Component::Year, date.get_full_year());
        } else if unit_word.contains("month") {
            date.set_date(1.0);
            components.imply(Component::Day, date.get_date());
            components.assign(Component::Year, date.get_full_year());
            components.assign(Component::Month, date.get_month() + 1);
        } else if unit_word.contains("year") {
            date.set_date(1.0);
            date.set_month(0.0);
            components.imply(Component::Day, date.get_date());
            components.imply(Component::Month, date.get_month() + 1);
            components.assign(Component::Year, date.get_full_year());
        }
        Some(Extracted::Components(components))
    }
}

pub(crate) struct SlashMonthFormatParser {
    pattern: JsRegex,
}

impl SlashMonthFormatParser {
    pub fn new() -> SlashMonthFormatParser {
        SlashMonthFormatParser {
            pattern: word_boundary_pattern("([0-9]|0[1-9]|1[012])/([0-9]{4})", true),
        }
    }
}

impl Parser for SlashMonthFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let year = parse_int(m.group(2));
        let month = parse_int(m.group(1));
        let mut components = context.components();
        components
            .imply(Component::Day, 1)
            .assign(Component::Month, month)
            .assign(Component::Year, year);
        Some(Extracted::Components(components))
    }
}

pub(crate) struct TimeUnitAgoFormatParser {
    pattern: JsRegex,
}

impl TimeUnitAgoFormatParser {
    pub fn new(strict_mode: bool) -> TimeUnitAgoFormatParser {
        let units = if strict_mode {
            &*TIME_UNITS_NO_ABBR_PATTERN
        } else {
            &*TIME_UNITS_PATTERN
        };
        let inner = format!("({units})\\s{{0,5}}(?:ago|before|earlier)(?=\\W|$)");
        TimeUnitAgoFormatParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for TimeUnitAgoFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let duration = parse_duration(m.group(1))?;
        relative(context, &duration.reversed())
    }
}

pub(crate) struct TimeUnitCasualRelativeFormatParser {
    pattern: JsRegex,
}

impl TimeUnitCasualRelativeFormatParser {
    pub fn new(allow_abbreviations: bool) -> TimeUnitCasualRelativeFormatParser {
        let units = if allow_abbreviations {
            &*TIME_UNITS_PATTERN
        } else {
            &*TIME_UNITS_NO_ABBR_PATTERN
        };
        let inner = format!("(this|last|past|next|after|\\+|-)\\s*({units})(?=\\W|$)");
        TimeUnitCasualRelativeFormatParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for TimeUnitCasualRelativeFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let prefix = m.group(1).to_lowercase();
        let mut duration = parse_duration(m.group(2))?;
        if matches!(prefix.as_str(), "last" | "past" | "-") {
            duration = duration.reversed();
        }
        relative(context, &duration)
    }
}

pub(crate) struct TimeUnitLaterFormatParser {
    pattern: JsRegex,
}

impl TimeUnitLaterFormatParser {
    pub fn new(strict_mode: bool) -> TimeUnitLaterFormatParser {
        let inner = if strict_mode {
            format!(
                "({})\\s{{0,5}}(later|after|from now)(?=\\W|$)",
                *TIME_UNITS_NO_ABBR_PATTERN
            )
        } else {
            format!(
                "({})\\s{{0,5}}(?:later|after|from now|henceforth|forward|out)(?=(?:\\W|$))",
                *TIME_UNITS_PATTERN
            )
        };
        TimeUnitLaterFormatParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for TimeUnitLaterFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let duration = parse_duration(m.group(1))?;
        relative(context, &duration)
    }
}

pub(crate) struct TimeUnitWithinFormatParser {
    strict_mode: bool,
    optional_prefix: JsRegex,
    with_prefix: JsRegex,
    with_prefix_strict: JsRegex,
}

impl TimeUnitWithinFormatParser {
    pub fn new(strict_mode: bool) -> TimeUnitWithinFormatParser {
        let approx = "(?:(?:about|around|roughly|approximately|just)\\s*(?:~\\s*)?)?";
        let units = &*TIME_UNITS_PATTERN;
        let units_strict = &*TIME_UNITS_NO_ABBR_PATTERN;
        TimeUnitWithinFormatParser {
            strict_mode,
            optional_prefix: word_boundary_pattern(
                &format!("(?:(?:within|in|for)\\s*)?{approx}({units})(?=\\W|$)"),
                true,
            ),
            with_prefix: word_boundary_pattern(
                &format!("(?:within|in|for)\\s*{approx}({units})(?=\\W|$)"),
                true,
            ),
            with_prefix_strict: word_boundary_pattern(
                &format!("(?:within|in|for)\\s*{approx}({units_strict})(?=\\W|$)"),
                true,
            ),
        }
    }
}

impl Parser for TimeUnitWithinFormatParser {
    fn pattern(&self, context: &ParsingContext) -> &JsRegex {
        if self.strict_mode {
            &self.with_prefix_strict
        } else if context.option.forward_date {
            &self.optional_prefix
        } else {
            &self.with_prefix
        }
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        if js_re!("^for\\s*the\\s*\\w+").is_match(m.text()) {
            return None;
        }
        let duration = parse_duration(m.group(1))?;
        relative(context, &duration)
    }
}

pub(crate) struct WeekdayParser {
    pattern: JsRegex,
}

impl WeekdayParser {
    pub fn new() -> WeekdayParser {
        let weekdays = match_any_pattern(WEEKDAY_DICTIONARY);
        let inner = format!(
            "(?:(?:\\,|\\(|\\（)\\s*)?(?:on\\s*?)?(?:(this|last|past|next)\\s*)?({weekdays}|weekend|weekday)(?:\\s*(?:\\,|\\)|\\）))?(?:\\s*(?:of\\s*)?(this|last|past|next)\\s*week)?(?=\\W|$)"
        );
        WeekdayParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for WeekdayParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let modifier_word = m
            .non_empty(1)
            .or_else(|| m.non_empty(3))
            .unwrap_or("")
            .to_lowercase();
        let modifier = match modifier_word.as_str() {
            "last" | "past" => Some(WeekdayModifier::Last),
            "next" => Some(WeekdayModifier::Next),
            "this" => Some(WeekdayModifier::This),
            _ => None,
        };

        let weekday_word = m.group(2).to_lowercase();
        let weekday = if let Some(weekday) = lookup(WEEKDAY_DICTIONARY, &weekday_word) {
            weekday
        } else if weekday_word == "weekend" {
            if modifier == Some(WeekdayModifier::Last) {
                Weekday::Sunday as i32
            } else {
                Weekday::Saturday as i32
            }
        } else if weekday_word == "weekday" {
            let ref_weekday = context.reference.date_with_adjusted_timezone().get_day();
            if ref_weekday == Weekday::Sunday as i32 || ref_weekday == Weekday::Saturday as i32 {
                if modifier == Some(WeekdayModifier::Last) {
                    Weekday::Friday as i32
                } else {
                    Weekday::Monday as i32
                }
            } else {
                let weekday = ref_weekday - 1;
                let weekday = if modifier == Some(WeekdayModifier::Last) {
                    weekday - 1
                } else {
                    weekday + 1
                };
                (weekday % 5) + 1
            }
        } else {
            return None;
        };

        Some(Extracted::Components(create_parsing_components_at_weekday(
            context.reference,
            weekday,
            modifier,
        )))
    }
}

pub(crate) struct YearMonthDayParser {
    pattern: JsRegex,
    strict_month_date_order: bool,
}

impl YearMonthDayParser {
    pub fn new(strict_month_date_order: bool) -> YearMonthDayParser {
        let months = match_any_pattern(MONTH_DICTIONARY);
        let inner = format!("([0-9]{{4}})[-\\.\\/\\s](?:({months})|([0-9]{{1,2}}))[-\\.\\/\\s]([0-9]{{1,2}})(?=\\W|$)");
        YearMonthDayParser {
            pattern: word_boundary_pattern(&inner, true),
            strict_month_date_order,
        }
    }
}

impl Parser for YearMonthDayParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let year = parse_int(m.group(1));
        let mut day = parse_int(m.group(4));
        let mut month = match m.non_empty(3) {
            Some(number) => parse_int(number),
            None => lookup(MONTH_DICTIONARY, &m.group(2).to_lowercase()).unwrap_or(0),
        };
        if !(1..=12).contains(&month) {
            if self.strict_month_date_order {
                return None;
            }
            if (1..=12).contains(&day) {
                std::mem::swap(&mut month, &mut day);
            }
        }
        if !(1..=31).contains(&day) {
            return None;
        }
        Some(Extracted::Components(context.components_with(&[
            (Component::Day, day),
            (Component::Month, month),
            (Component::Year, year),
        ])))
    }
}

pub(crate) struct YearMonthNameParser {
    pattern: JsRegex,
}

impl YearMonthNameParser {
    pub fn new() -> YearMonthNameParser {
        let year = "(?:[1-9][0-9]{0,3}\\s{0,2}(?:BE|AD|BC|BCE|CE)|[1-9][0-9]{3})";
        let months = match_any_pattern(MONTH_DICTIONARY);
        let inner = format!(
            "({year})(?:\\s*[-.\\/,]?\\s*|\\s+of\\s+)({months})(?=[^\\s\\w]|\\s+[^0-9]|\\s+$|$)"
        );
        YearMonthNameParser {
            pattern: word_boundary_pattern(&inner, true),
        }
    }
}

impl Parser for YearMonthNameParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let year = parse_year(m.group(1));
        let month = lookup(MONTH_DICTIONARY, &m.group(2).to_lowercase()).unwrap_or(0);
        let mut result = context.result(m.index, m.text());
        result.start.imply(Component::Day, 1);
        result.start.assign(Component::Month, month);
        result.start.assign(Component::Year, year);
        result.start.add_tag("parser/ENYearMonthNameParser");
        Some(Extracted::Result(Box::new(result)))
    }
}
