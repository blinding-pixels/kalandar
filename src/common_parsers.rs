use crate::calculation::{find_most_likely_ad_year, find_year_closest_to_ref};
use crate::chrono::{js_substring, suffix_from, Extracted, Parser, ParserMatch, ParsingContext};
use crate::js_regex::{js_re, JsMatch, JsRegex};
use crate::results::{ParsedComponents, ParsedResult};
use crate::text::parse_int;
use crate::types::{Component, Meridiem};

pub(crate) fn word_boundary_pattern(inner: &str, ignore_case: bool) -> JsRegex {
    JsRegex::new(&format!("(\\W|^){inner}"), ignore_case)
}

pub(crate) struct IsoFormatParser {
    pattern: JsRegex,
}

impl IsoFormatParser {
    pub fn new() -> IsoFormatParser {
        let inner = concat!(
            "([0-9]{4})\\-([0-9]{1,2})\\-([0-9]{1,2})",
            "(?:T",
            "([0-9]{1,2}):([0-9]{1,2})",
            "(?:",
            ":([0-9]{1,2})(?:\\.(\\d{1,4}))?",
            ")?",
            "(",
            "Z|([+-]\\d{2}):?(\\d{2})?",
            ")?",
            ")?",
            "(?=\\W|$)"
        );
        IsoFormatParser {
            pattern: word_boundary_pattern(inner, true),
        }
    }
}

impl Parser for IsoFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        m.strip_left_boundary();
        let mut components = context.components_with(&[
            (Component::Year, parse_int(m.group(1))),
            (Component::Month, parse_int(m.group(2))),
            (Component::Day, parse_int(m.group(3))),
        ]);
        if let Some(hour) = m.get(4) {
            components.assign(Component::Hour, parse_int(hour));
            components.assign(Component::Minute, parse_int(m.group(5)));
            if let Some(second) = m.get(6) {
                components.assign(Component::Second, parse_int(second));
            }
            if let Some(ms) = m.get(7) {
                components.assign(Component::Millisecond, parse_int(ms));
            }
            if m.get(8).is_some() {
                let mut offset = 0;
                if let Some(hour_offset) = m.non_empty(9) {
                    let hour_offset = parse_int(hour_offset);
                    let minute_offset = m.get(10).map_or(0, parse_int);
                    offset = hour_offset * 60;
                    if offset < 0 {
                        offset -= minute_offset;
                    } else {
                        offset += minute_offset;
                    }
                }
                components.assign(Component::TimezoneOffset, offset);
            }
        }
        components.add_tag("parser/ISOFormatParser");
        Some(Extracted::Components(components))
    }
}

pub(crate) struct SlashDateFormatParser {
    pattern: JsRegex,
    group_month: usize,
    group_day: usize,
}

impl SlashDateFormatParser {
    pub fn new(little_endian: bool) -> SlashDateFormatParser {
        SlashDateFormatParser {
            pattern: JsRegex::new(
                "([^\\d]|^)([0-3]{0,1}[0-9]{1})[\\/\\.\\-]([0-3]{0,1}[0-9]{1})(?:[\\/\\.\\-]([0-9]{4}|[0-9]{2}))?(\\W|$)",
                true,
            ),
            group_month: if little_endian { 3 } else { 2 },
            group_day: if little_endian { 2 } else { 3 },
        }
    }
}

impl Parser for SlashDateFormatParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.pattern
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        let index = m.index + m.group(1).len();
        let index_end = m.index + m.text().len() - m.group(5).len();
        if index > 0 && js_re!("\\d/?$").is_match(&context.text[..index]) {
            return None;
        }
        if index_end < context.text.len()
            && js_re!("^/?\\d").is_match(suffix_from(context.text, index_end))
        {
            return None;
        }

        let text = js_substring(context.text, index, index_end);
        if js_re!("^\\d\\.\\d$").is_match(text)
            || js_re!("^\\d\\.\\d{1,2}\\.\\d{1,2}\\s*$").is_match(text)
        {
            return None;
        }
        if m.non_empty(4).is_none() && !text.contains('/') {
            return None;
        }

        let mut result = context.result(index, text);
        let mut month = parse_int(m.group(self.group_month));
        let mut day = parse_int(m.group(self.group_day));
        if month > 12 {
            if (1..=12).contains(&day) && month <= 31 {
                std::mem::swap(&mut day, &mut month);
            } else {
                return None;
            }
        }
        if !(1..=31).contains(&day) {
            return None;
        }

        result.start.assign(Component::Day, day);
        result.start.assign(Component::Month, month);
        if let Some(year) = m.non_empty(4) {
            result
                .start
                .assign(Component::Year, find_most_likely_ad_year(parse_int(year)));
        } else {
            let year = find_year_closest_to_ref(context.reference.instant, day, month);
            result.start.imply(Component::Year, year);
        }
        result.add_tag("parser/SlashDateFormatParser");
        Some(Extracted::Result(Box::new(result)))
    }
}

const HOUR_GROUP: usize = 2;
const MINUTE_GROUP: usize = 3;
const SECOND_GROUP: usize = 4;
const MILLI_SECOND_GROUP: usize = 5;
const AM_PM_HOUR_GROUP: usize = 6;

pub(crate) struct TimeExpressionParser {
    strict_mode: bool,
    primary: JsRegex,
    following: JsRegex,
}

fn group_of<'t>(m: &JsMatch<'t>, group: usize) -> Option<&'t str> {
    m.get(group)
}

impl TimeExpressionParser {
    pub fn new(strict_mode: bool) -> TimeExpressionParser {
        let primary_prefix = "(?:(?:at|from)\\s*)??";
        let primary_suffix =
            "(?:\\s*(?:o\\W*clock|at\\s*night|in\\s*the\\s*(?:morning|afternoon)))?(?!/)(?=\\W|$)";
        let following_phase = "\\s*(?:\\-|\\–|\\~|\\〜|to|until|through|till|\\?)\\s*";
        let following_suffix = "(?!/)(?=\\W|$)";
        let primary = format!(
            "(^|\\s|T|\\b){primary_prefix}(\\d{{1,4}})(?:(?:\\.|:|：)(\\d{{1,2}})(?:(?::|：)(\\d{{2}})(?:\\.(\\d{{1,6}}))?)?)?(?:\\s*(a\\.m\\.|p\\.m\\.|am?|pm?))?{primary_suffix}"
        );
        let following = format!(
            "^({following_phase})(\\d{{1,4}})(?:(?:\\.|\\:|\\：)(\\d{{1,2}})(?:(?:\\.|\\:|\\：)(\\d{{1,2}})(?:\\.(\\d{{1,6}}))?)?)?(?:\\s*(a\\.m\\.|p\\.m\\.|am?|pm?))?{following_suffix}"
        );
        TimeExpressionParser {
            strict_mode,
            primary: JsRegex::new(&primary, true),
            following: JsRegex::new(&following, true),
        }
    }

    fn extract_primary_time_components(
        &self,
        context: &ParsingContext,
        m: &ParserMatch,
    ) -> Option<ParsedComponents> {
        let mut components = self.base_primary_time_components(context, m)?;
        let full = m.text();
        if full.ends_with("night") {
            let hour = components.value(Component::Hour);
            if (6..12).contains(&hour) {
                components.assign(Component::Hour, hour + 12);
                components.assign(Component::Meridiem, Meridiem::Pm as i32);
            } else if hour < 6 {
                components.assign(Component::Meridiem, Meridiem::Am as i32);
            }
        }
        if full.ends_with("afternoon") {
            components.assign(Component::Meridiem, Meridiem::Pm as i32);
            let hour = components.value(Component::Hour);
            if (0..=6).contains(&hour) {
                components.assign(Component::Hour, hour + 12);
            }
        }
        if full.ends_with("morning") {
            components.assign(Component::Meridiem, Meridiem::Am as i32);
            let hour = components.value(Component::Hour);
            if hour < 12 {
                components.assign(Component::Hour, hour);
            }
        }
        components.add_tag("parser/ENTimeExpressionParser");
        Some(components)
    }

    fn base_primary_time_components(
        &self,
        context: &ParsingContext,
        m: &ParserMatch,
    ) -> Option<ParsedComponents> {
        let mut components = context.components();
        let mut minute = 0;
        let mut meridiem = None;

        let hour_text = m.group(HOUR_GROUP);
        let mut hour = parse_int(hour_text);
        if hour > 100 {
            if hour_text.len() == 4
                && m.get(MINUTE_GROUP).is_none()
                && m.non_empty(AM_PM_HOUR_GROUP).is_none()
            {
                return None;
            }
            if self.strict_mode || m.get(MINUTE_GROUP).is_some() {
                return None;
            }
            minute = hour % 100;
            hour /= 100;
        }
        if hour > 24 {
            return None;
        }

        if let Some(minute_text) = m.get(MINUTE_GROUP) {
            if minute_text.len() == 1 && m.non_empty(AM_PM_HOUR_GROUP).is_none() {
                return None;
            }
            minute = parse_int(minute_text);
        }
        if minute >= 60 {
            return None;
        }
        if hour > 12 {
            meridiem = Some(Meridiem::Pm);
        }

        if let Some(ampm) = m.get(AM_PM_HOUR_GROUP) {
            if hour > 12 {
                return None;
            }
            match ampm.chars().next().map(|c| c.to_ascii_lowercase()) {
                Some('a') => {
                    meridiem = Some(Meridiem::Am);
                    if hour == 12 {
                        hour = 0;
                    }
                }
                Some('p') => {
                    meridiem = Some(Meridiem::Pm);
                    if hour != 12 {
                        hour += 12;
                    }
                }
                _ => {}
            }
        }

        components.assign(Component::Hour, hour);
        components.assign(Component::Minute, minute);
        match meridiem {
            Some(meridiem) => {
                components.assign(Component::Meridiem, meridiem as i32);
            }
            None => {
                let implied = if hour < 12 {
                    Meridiem::Am
                } else {
                    Meridiem::Pm
                };
                components.imply(Component::Meridiem, implied as i32);
            }
        }

        if let Some(ms) = m.get(MILLI_SECOND_GROUP) {
            let ms = parse_int(&ms.chars().take(3).collect::<String>());
            if ms >= 1000 {
                return None;
            }
            components.assign(Component::Millisecond, ms);
        }
        if let Some(second) = m.get(SECOND_GROUP) {
            let second = parse_int(second);
            if second >= 60 {
                return None;
            }
            components.assign(Component::Second, second);
        }
        Some(components)
    }

    fn extract_following_time_components(
        &self,
        context: &ParsingContext,
        m: &JsMatch,
        result: &mut ParsedResult,
    ) -> Option<ParsedComponents> {
        let mut components = context.components();

        if let Some(ms) = group_of(m, MILLI_SECOND_GROUP) {
            let ms = parse_int(&ms.chars().take(3).collect::<String>());
            if ms >= 1000 {
                return None;
            }
            components.assign(Component::Millisecond, ms);
        }
        if let Some(second) = group_of(m, SECOND_GROUP) {
            let second = parse_int(second);
            if second >= 60 {
                return None;
            }
            components.assign(Component::Second, second);
        }

        let mut hour = parse_int(group_of(m, HOUR_GROUP).unwrap_or(""));
        let mut minute = 0;
        let mut meridiem: i32 = -1;

        if let Some(minute_text) = group_of(m, MINUTE_GROUP) {
            minute = parse_int(minute_text);
        } else if hour > 100 {
            minute = hour % 100;
            hour /= 100;
        }
        if minute >= 60 || hour > 24 {
            return None;
        }
        if hour >= 12 {
            meridiem = Meridiem::Pm as i32;
        }

        if let Some(ampm) = group_of(m, AM_PM_HOUR_GROUP) {
            if hour > 12 {
                return None;
            }
            match ampm.chars().next().map(|c| c.to_ascii_lowercase()) {
                Some('a') => {
                    meridiem = Meridiem::Am as i32;
                    if hour == 12 {
                        hour = 0;
                        if !components.is_certain(Component::Day) {
                            let day = components.value(Component::Day) + 1;
                            components.imply(Component::Day, day);
                        }
                    }
                }
                Some('p') => {
                    meridiem = Meridiem::Pm as i32;
                    if hour != 12 {
                        hour += 12;
                    }
                }
                _ => {}
            }

            if !result.start.is_certain(Component::Meridiem) {
                if meridiem == Meridiem::Am as i32 {
                    result.start.imply(Component::Meridiem, Meridiem::Am as i32);
                    if result.start.get(Component::Hour) == Some(12) {
                        result.start.assign(Component::Hour, 0);
                    }
                } else {
                    result.start.imply(Component::Meridiem, Meridiem::Pm as i32);
                    if result.start.get(Component::Hour) != Some(12) {
                        let start_hour = result.start.value(Component::Hour) + 12;
                        result.start.assign(Component::Hour, start_hour);
                    }
                }
            }
        }

        components.assign(Component::Hour, hour);
        components.assign(Component::Minute, minute);

        if meridiem >= 0 {
            components.assign(Component::Meridiem, meridiem);
        } else {
            let start_hour = result.start.value(Component::Hour);
            let start_at_pm = result.start.is_certain(Component::Meridiem) && start_hour > 12;
            if start_at_pm {
                if start_hour - 12 > hour {
                    components.imply(Component::Meridiem, Meridiem::Am as i32);
                } else if hour <= 12 {
                    components.assign(Component::Hour, hour + 12);
                    components.assign(Component::Meridiem, Meridiem::Pm as i32);
                }
            } else if hour > 12 {
                components.imply(Component::Meridiem, Meridiem::Pm as i32);
            } else if hour <= 12 {
                components.imply(Component::Meridiem, Meridiem::Am as i32);
            }
        }

        if components.js_date().time() < result.start.js_date().time() {
            let day = components.value(Component::Day) + 1;
            components.imply(Component::Day, day);
        }

        components.add_tag("parser/ENTimeExpressionParser");
        Some(components)
    }

    fn check_without_following_pattern(&self, result: ParsedResult) -> Option<ParsedResult> {
        let text = result.text.as_str();
        if js_re!("^\\d$").is_match(text) {
            return None;
        }
        if js_re!("^\\d\\d\\d+$").is_match(text) {
            return None;
        }
        if js_re!("\\d[apAP]$").is_match(text) {
            return None;
        }
        if let Some(ending) = js_re!("[^\\d:.](\\d[\\d.]+)$").exec(text) {
            let ending_numbers = ending.get(1).unwrap_or("");
            if self.strict_mode {
                return None;
            }
            if ending_numbers.contains('.') && !js_re!("\\d(\\.\\d{2})+$").is_match(ending_numbers)
            {
                return None;
            }
            if parse_int(ending_numbers) > 24 {
                return None;
            }
        }
        Some(result)
    }

    fn check_with_following_pattern(&self, result: ParsedResult) -> Option<ParsedResult> {
        let text = result.text.as_str();
        if js_re!("^\\d+-\\d+$").is_match(text) {
            return None;
        }
        if let Some(ending) = js_re!("[^\\d:.](\\d[\\d.]+)\\s*-\\s*(\\d[\\d.]+)$").exec(text) {
            if self.strict_mode {
                return None;
            }
            let starting_numbers = ending.get(1).unwrap_or("");
            let ending_numbers = ending.get(2).unwrap_or("");
            if ending_numbers.contains('.') && !js_re!("\\d(\\.\\d{2})+$").is_match(ending_numbers)
            {
                return None;
            }
            if parse_int(ending_numbers) > 24 || parse_int(starting_numbers) > 24 {
                return None;
            }
        }
        Some(result)
    }
}

impl Parser for TimeExpressionParser {
    fn pattern(&self, _context: &ParsingContext) -> &JsRegex {
        &self.primary
    }

    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted> {
        let full = m.text();
        let Some(start_components) = self.extract_primary_time_components(context, m) else {
            if js_re!("^\\d{4}").is_match(full) {
                m.index += 4;
                return None;
            }
            m.index += full.len();
            return None;
        };

        let boundary = m.group(1);
        let index = m.index + boundary.len();
        let text = &full[boundary.len()..];
        let mut result = context.result(index, text);
        result.start = start_components;
        m.index += full.len();

        let remaining = suffix_from(context.text, m.index);
        let following = self.following.exec(remaining);

        if let Some(following) = &following {
            if js_re!("^\\d{3,4}").is_match(text) {
                if js_re!("^\\s*([+-])\\s*\\d{2,4}$").is_match(following.full()) {
                    return None;
                }
                if js_re!("^\\s*([+-])\\s*\\d{2}\\W\\d{2}").is_match(following.full()) {
                    return None;
                }
            }
        }

        let following = match following {
            Some(f) if !js_re!("^\\s*([+-])\\s*\\d{3,4}$").is_match(f.full()) => f,
            _ => {
                return self
                    .check_without_following_pattern(result)
                    .map(|r| Extracted::Result(Box::new(r)))
            }
        };

        result.end = self.extract_following_time_components(context, &following, &mut result);
        if result.end.is_some() {
            result.text.push_str(following.full());
        }
        self.check_with_following_pattern(result)
            .map(|r| Extracted::Result(Box::new(r)))
    }
}
