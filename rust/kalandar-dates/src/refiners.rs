use std::cmp::Ordering;

use crate::calculation::{add_days, add_years, days_forward_to_weekday, merge_date_time_result};
use crate::chrono::{
    filter_results, js_substring, merge_consecutive, suffix_from, Filter, MergingRefiner,
    ParsingContext, Refiner,
};
use crate::en::constants::{parse_duration, parse_year, YEAR_PATTERN};
use crate::js_regex::{js_re, JsRegex};
use crate::results::{ParsedComponents, ParsedResult, ReferenceWithTimezone};
use crate::text::{js_len, js_trim, parse_int};
use crate::timezone::to_timezone_offset;
use crate::types::{Component, Duration};

pub(crate) struct OverlapRemovalRefiner;

impl Refiner for OverlapRemovalRefiner {
    fn refine(&self, _context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        if results.len() < 2 {
            return results;
        }
        let mut filtered = Vec::with_capacity(results.len());
        let mut iter = results.into_iter();
        let Some(mut previous) = iter.next() else {
            return filtered;
        };
        for result in iter {
            if result.index >= previous.end_index() {
                filtered.push(std::mem::replace(&mut previous, result));
                continue;
            }
            if js_len(&result.text) > js_len(&previous.text) {
                previous = result;
            }
        }
        filtered.push(previous);
        filtered
    }
}

pub(crate) struct ExtractTimezoneOffsetRefiner;

impl Refiner for ExtractTimezoneOffsetRefiner {
    fn refine(
        &self,
        context: &ParsingContext,
        mut results: Vec<ParsedResult>,
    ) -> Vec<ParsedResult> {
        let pattern = js_re!(
            "^\\s*(?:\\(?(?:GMT|UTC)\\s?)?([+-])(\\d{1,2})(?::?(\\d{2}))?\\)?",
            i
        );
        for result in &mut results {
            if result.start.is_certain(Component::TimezoneOffset) {
                continue;
            }
            let suffix = suffix_from(context.text, result.end_index());
            let Some(m) = pattern.exec(suffix) else {
                continue;
            };
            let hour_offset = parse_int(m.get(2).unwrap_or(""));
            let minute_offset = parse_int(m.non_empty(3).unwrap_or("0"));
            let mut offset = hour_offset * 60 + minute_offset;
            if offset > 14 * 60 {
                continue;
            }
            if m.get(1) == Some("-") {
                offset = -offset;
            }
            if let Some(end) = &mut result.end {
                end.assign(Component::TimezoneOffset, offset);
            }
            result.start.assign(Component::TimezoneOffset, offset);
            result.text.push_str(m.full());
        }
        results
    }
}

pub(crate) struct ExtractTimezoneAbbrRefiner;

impl Refiner for ExtractTimezoneAbbrRefiner {
    fn refine(
        &self,
        context: &ParsingContext,
        mut results: Vec<ParsedResult>,
    ) -> Vec<ParsedResult> {
        let pattern = js_re!("^\\s*,?\\s*\\(?([A-Z]{2,4})\\)?(?=\\W|$)", i);
        for result in &mut results {
            let suffix = suffix_from(context.text, result.end_index());
            let Some(m) = pattern.exec(suffix) else {
                continue;
            };
            let raw = m.get(1).unwrap_or("");
            let abbr = raw.to_uppercase();
            let ref_date = result.start.js_date();
            let Some(extracted) =
                to_timezone_offset(&abbr, Some(ref_date), &context.option.timezones)
            else {
                continue;
            };
            if let Some(current) = result.start.get(Component::TimezoneOffset) {
                if extracted != current {
                    if result.start.is_certain(Component::TimezoneOffset) {
                        continue;
                    }
                    if abbr != raw {
                        continue;
                    }
                }
            }
            if result.start.is_only_date() && abbr != raw {
                continue;
            }
            result.text.push_str(m.full());
            if !result.start.is_certain(Component::TimezoneOffset) {
                result.start.assign(Component::TimezoneOffset, extracted);
            }
            if let Some(end) = &mut result.end {
                if !end.is_certain(Component::TimezoneOffset) {
                    end.assign(Component::TimezoneOffset, extracted);
                }
            }
        }
        results
    }
}

pub(crate) struct MergeWeekdayComponentRefiner;

impl MergingRefiner for MergeWeekdayComponentRefiner {
    fn should_merge(
        &self,
        between: &str,
        current: &ParsedResult,
        next: &ParsedResult,
        _: &ParsingContext,
    ) -> bool {
        let weekday_then_normal_date = current.start.is_only_weekday_component()
            && !current.start.is_certain(Component::Hour)
            && next.start.is_certain(Component::Day);
        weekday_then_normal_date && js_re!("^,?\\s*$").is_match(between)
    }

    fn merge(
        &self,
        between: &str,
        current: ParsedResult,
        next: ParsedResult,
        _: &ParsingContext,
    ) -> ParsedResult {
        let mut merged = next.js_clone();
        merged.index = current.index;
        merged.text = format!("{}{}{}", current.text, between, merged.text);
        let weekday = current.start.value(Component::Weekday);
        merged.start.assign(Component::Weekday, weekday);
        if let Some(end) = &mut merged.end {
            end.assign(Component::Weekday, weekday);
        }
        merged
    }
}

impl Refiner for MergeWeekdayComponentRefiner {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        merge_consecutive(self, context, results)
    }
}

pub(crate) struct ForwardDateRefiner;

fn later(a: &crate::jsdate::JsDate, b: &crate::jsdate::JsDate) -> bool {
    a.partial_cmp(b) == Some(Ordering::Greater)
}

impl Refiner for ForwardDateRefiner {
    fn refine(
        &self,
        context: &ParsingContext,
        mut results: Vec<ParsedResult>,
    ) -> Vec<ParsedResult> {
        if !context.option.forward_date {
            return results;
        }
        for result in &mut results {
            let ref_date = context.reference.date_with_adjusted_timezone();

            if result.start.is_only_time()
                && later(&context.reference.instant, &result.start.js_date())
            {
                let mut following_day = context.reference.date_with_adjusted_timezone();
                following_day.set_date(f64::from(following_day.get_date()) + 1.0);
                result.start.imply_similar_date(&following_day);
                if let Some(end) = &mut result.end {
                    if end.is_only_time() {
                        end.imply_similar_date(&following_day);
                        if later(&result.start.js_date(), &end.js_date()) {
                            following_day.set_date(f64::from(following_day.get_date()) + 1.0);
                            end.imply_similar_date(&following_day);
                        }
                    }
                }
            }

            if result.start.is_only_weekday_component() && later(&ref_date, &result.start.js_date())
            {
                let weekday = result.start.value(Component::Weekday);
                let days = match days_forward_to_weekday(&ref_date, weekday) {
                    0 => 7,
                    days => days,
                };
                let forwarded = add_days(ref_date, f64::from(days));
                result.start.imply_similar_date(&forwarded);
                let start_date = result.start.js_date();
                if let Some(end) = &mut result.end {
                    if later(&start_date, &end.js_date()) {
                        end.imply_similar_date(&forwarded);
                    }
                }
            }

            if result.start.is_date_with_unknown_year() && later(&ref_date, &result.start.js_date())
            {
                let mut i = 0;
                while i < 3 && later(&ref_date, &result.start.js_date()) {
                    let year = result.start.value(Component::Year) + 1;
                    result.start.imply(Component::Year, year);
                    if let Some(end) = &mut result.end {
                        if !end.is_certain(Component::Year) {
                            let end_year = end.value(Component::Year) + 1;
                            end.imply(Component::Year, end_year);
                        }
                    }
                    i += 1;
                }
            }
        }
        results
    }
}

pub(crate) struct UnlikelyFormatFilter {
    pub strict_mode: bool,
}

impl Filter for UnlikelyFormatFilter {
    fn is_valid(&self, _context: &ParsingContext, result: &ParsedResult) -> bool {
        let without_first_space = result.text.replacen(' ', "", 1);
        if js_re!("^\\d*(\\.\\d*)?$").is_match(&without_first_space) {
            return false;
        }
        if !result.start.is_valid_date() {
            return false;
        }
        if let Some(end) = &result.end {
            if !end.is_valid_date() {
                return false;
            }
        }
        if self.strict_mode {
            return !result.start.is_only_weekday_component();
        }
        true
    }
}

impl Refiner for UnlikelyFormatFilter {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        filter_results(self, context, results)
    }
}

pub(crate) struct MergeDateTimeRefiner;

impl MergingRefiner for MergeDateTimeRefiner {
    fn should_merge(
        &self,
        between: &str,
        current: &ParsedResult,
        next: &ParsedResult,
        _: &ParsingContext,
    ) -> bool {
        ((current.start.is_only_date() && next.start.is_only_time())
            || (next.start.is_only_date() && current.start.is_only_time()))
            && js_re!("^\\s*(T|at|after|before|on|of|,|-|\\.|∙|:)?\\s*$").is_match(between)
    }

    fn merge(
        &self,
        between: &str,
        current: ParsedResult,
        next: ParsedResult,
        _: &ParsingContext,
    ) -> ParsedResult {
        let mut merged = if current.start.is_only_date() {
            merge_date_time_result(&current, &next)
        } else {
            merge_date_time_result(&next, &current)
        };
        merged.index = current.index;
        merged.text = format!("{}{}{}", current.text, between, next.text);
        merged
    }
}

impl Refiner for MergeDateTimeRefiner {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        merge_consecutive(self, context, results)
    }
}

pub(crate) struct MergeDateRangeRefiner;

fn imply_date(components: &mut ParsedComponents, date: &crate::jsdate::JsDate) {
    components.imply(Component::Day, date.get_date());
    components.imply(Component::Month, date.get_month() + 1);
    components.imply(Component::Year, date.get_full_year());
}

impl MergingRefiner for MergeDateRangeRefiner {
    fn should_merge(
        &self,
        between: &str,
        current: &ParsedResult,
        next: &ParsedResult,
        _: &ParsingContext,
    ) -> bool {
        current.end.is_none()
            && next.end.is_none()
            && js_re!("^\\s*(to|-|–|until|through|till)\\s*$", i).is_match(between)
    }

    fn merge(
        &self,
        between: &str,
        current: ParsedResult,
        next: ParsedResult,
        _: &ParsingContext,
    ) -> ParsedResult {
        let mut from = current;
        let mut to = next;
        if !from.start.is_only_weekday_component() && !to.start.is_only_weekday_component() {
            for key in to.start.certain_components() {
                if !from.start.is_certain(key) {
                    from.start.imply(key, to.start.value(key));
                }
            }
            for key in from.start.certain_components() {
                if !to.start.is_certain(key) {
                    to.start.imply(key, from.start.value(key));
                }
            }
        }
        if later(&from.start.js_date(), &to.start.js_date()) {
            let mut from_date = from.start.js_date();
            let mut to_date = to.start.js_date();
            if to.start.is_only_weekday_component() && later(&add_days(to_date, 7.0), &from_date) {
                to_date = add_days(to_date, 7.0);
                imply_date(&mut to.start, &to_date);
            } else if from.start.is_only_weekday_component()
                && later(&to_date, &add_days(from_date, -7.0))
            {
                from_date = add_days(from_date, -7.0);
                imply_date(&mut from.start, &from_date);
            } else if to.start.is_date_with_unknown_year()
                && later(&add_years(to_date, 1.0), &from_date)
            {
                to_date = add_years(to_date, 1.0);
                to.start.imply(Component::Year, to_date.get_full_year());
            } else if from.start.is_date_with_unknown_year()
                && later(&to_date, &add_years(from_date, -1.0))
            {
                from_date = add_years(from_date, -1.0);
                from.start.imply(Component::Year, from_date.get_full_year());
            } else {
                std::mem::swap(&mut from, &mut to);
            }
        }
        let mut result = from.js_clone();
        result.index = from.index.min(to.index);
        result.text = if from.index < to.index {
            format!("{}{}{}", from.text, between, to.text)
        } else {
            format!("{}{}{}", to.text, between, from.text)
        };
        result.start = from.start;
        result.end = Some(to.start);
        result
    }
}

impl Refiner for MergeDateRangeRefiner {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        merge_consecutive(self, context, results)
    }
}

pub(crate) struct ExtractYearSuffixRefiner {
    pattern: JsRegex,
}

impl ExtractYearSuffixRefiner {
    pub fn new() -> ExtractYearSuffixRefiner {
        ExtractYearSuffixRefiner {
            pattern: JsRegex::new(&format!("^\\s*({YEAR_PATTERN})"), true),
        }
    }
}

impl Refiner for ExtractYearSuffixRefiner {
    fn refine(
        &self,
        context: &ParsingContext,
        mut results: Vec<ParsedResult>,
    ) -> Vec<ParsedResult> {
        for result in &mut results {
            if !result.start.is_date_with_unknown_year() {
                continue;
            }
            let suffix = suffix_from(context.text, result.end_index());
            let Some(m) = self.pattern.exec(suffix) else {
                continue;
            };
            if js_len(js_trim(m.full())) <= 3 {
                continue;
            }
            let year = parse_year(m.get(1).unwrap_or(""));
            if let Some(end) = &mut result.end {
                end.assign(Component::Year, year);
            }
            result.start.assign(Component::Year, year);
            result.text.push_str(m.full());
        }
        results
    }
}

pub(crate) struct MergeRelativeAfterDateRefiner;

fn is_positive_following_reference(result: &ParsedResult) -> bool {
    js_re!("^[+-]", i).is_match(&result.text)
}

fn is_negative_following_reference(result: &ParsedResult) -> bool {
    js_re!("^-", i).is_match(&result.text)
}

impl MergingRefiner for MergeRelativeAfterDateRefiner {
    fn should_merge(
        &self,
        between: &str,
        _current: &ParsedResult,
        next: &ParsedResult,
        _: &ParsingContext,
    ) -> bool {
        if !js_re!("^\\s*$", i).is_match(between) {
            return false;
        }
        is_positive_following_reference(next) || is_negative_following_reference(next)
    }

    fn merge(
        &self,
        between: &str,
        current: ParsedResult,
        next: ParsedResult,
        _: &ParsingContext,
    ) -> ParsedResult {
        let mut duration = parse_duration(&next.text).unwrap_or_default();
        if is_negative_following_reference(&next) {
            duration = duration.reversed();
        }
        let components = ParsedComponents::create_relative_from_reference(
            ReferenceWithTimezone::from_date(current.start.js_date()),
            &duration,
        );
        ParsedResult::new(
            current.reference,
            current.index,
            format!("{}{}{}", current.text, between, next.text),
            Some(components),
            None,
        )
    }
}

impl Refiner for MergeRelativeAfterDateRefiner {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        merge_consecutive(self, context, results)
    }
}

pub(crate) struct MergeRelativeFollowByDateRefiner;

fn has_implied_earlier_reference_date(result: &ParsedResult) -> bool {
    js_re!("\\s+(before|from)$", i).is_match(&result.text)
}

fn has_implied_later_reference_date(result: &ParsedResult) -> bool {
    js_re!("\\s+(after|since)$", i).is_match(&result.text)
}

fn truthy(value: Option<i32>) -> bool {
    value.is_some_and(|v| v != 0)
}

impl MergingRefiner for MergeRelativeFollowByDateRefiner {
    fn should_merge(
        &self,
        between: &str,
        current: &ParsedResult,
        next: &ParsedResult,
        _: &ParsingContext,
    ) -> bool {
        if !js_re!("^\\s*$", i).is_match(between) {
            return false;
        }
        if !has_implied_earlier_reference_date(current)
            && !has_implied_later_reference_date(current)
        {
            return false;
        }
        truthy(next.start.get(Component::Day))
            && truthy(next.start.get(Component::Month))
            && truthy(next.start.get(Component::Year))
    }

    fn merge(
        &self,
        between: &str,
        current: ParsedResult,
        next: ParsedResult,
        _: &ParsingContext,
    ) -> ParsedResult {
        let mut duration: Duration = parse_duration(&current.text).unwrap_or_default();
        if has_implied_earlier_reference_date(&current) {
            duration = duration.reversed();
        }
        let components = ParsedComponents::create_relative_from_reference(
            ReferenceWithTimezone::from_date(next.start.js_date()),
            &duration,
        );
        ParsedResult::new(
            next.reference,
            current.index,
            format!("{}{}{}", current.text, between, next.text),
            Some(components),
            None,
        )
    }
}

impl Refiner for MergeRelativeFollowByDateRefiner {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        merge_consecutive(self, context, results)
    }
}

pub(crate) struct EnUnlikelyFormatFilter;

impl Filter for EnUnlikelyFormatFilter {
    fn is_valid(&self, context: &ParsingContext, result: &ParsedResult) -> bool {
        let text = js_trim(&result.text);
        if text == js_trim(context.text) {
            return true;
        }
        let lower = text.to_lowercase();
        if lower == "may" {
            let before = js_trim(js_substring(context.text, 0, result.index));
            if !js_re!("\\b(in)$", i).is_match(before) {
                return false;
            }
        }
        if lower.ends_with("the second") {
            return false;
        }
        true
    }
}

impl Refiner for EnUnlikelyFormatFilter {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult> {
        filter_results(self, context, results)
    }
}
