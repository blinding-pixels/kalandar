use std::sync::LazyLock;

use crate::calculation::find_most_likely_ad_year;
use crate::js_regex::JsRegex;
use crate::text::{js_trim, parse_float, parse_int, skip_utf16};
use crate::types::{Duration, Timeunit};

pub(crate) const WEEKDAY_DICTIONARY: &[(&str, i32)] = &[
    ("sunday", 0),
    ("sun", 0),
    ("sun.", 0),
    ("monday", 1),
    ("mon", 1),
    ("mon.", 1),
    ("tuesday", 2),
    ("tue", 2),
    ("tue.", 2),
    ("wednesday", 3),
    ("wed", 3),
    ("wed.", 3),
    ("thursday", 4),
    ("thurs", 4),
    ("thurs.", 4),
    ("thur", 4),
    ("thur.", 4),
    ("thu", 4),
    ("thu.", 4),
    ("friday", 5),
    ("fri", 5),
    ("fri.", 5),
    ("saturday", 6),
    ("sat", 6),
    ("sat.", 6),
];

pub(crate) const FULL_MONTH_NAME_DICTIONARY: &[(&str, i32)] = &[
    ("january", 1),
    ("february", 2),
    ("march", 3),
    ("april", 4),
    ("may", 5),
    ("june", 6),
    ("july", 7),
    ("august", 8),
    ("september", 9),
    ("october", 10),
    ("november", 11),
    ("december", 12),
];

pub(crate) const MONTH_DICTIONARY: &[(&str, i32)] = &[
    ("january", 1),
    ("february", 2),
    ("march", 3),
    ("april", 4),
    ("may", 5),
    ("june", 6),
    ("july", 7),
    ("august", 8),
    ("september", 9),
    ("october", 10),
    ("november", 11),
    ("december", 12),
    ("jan", 1),
    ("jan.", 1),
    ("feb", 2),
    ("feb.", 2),
    ("mar", 3),
    ("mar.", 3),
    ("apr", 4),
    ("apr.", 4),
    ("jun", 6),
    ("jun.", 6),
    ("jul", 7),
    ("jul.", 7),
    ("aug", 8),
    ("aug.", 8),
    ("sep", 9),
    ("sep.", 9),
    ("sept", 9),
    ("sept.", 9),
    ("oct", 10),
    ("oct.", 10),
    ("nov", 11),
    ("nov.", 11),
    ("dec", 12),
    ("dec.", 12),
];

pub(crate) const INTEGER_WORD_DICTIONARY: &[(&str, i32)] = &[
    ("one", 1),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
];

pub(crate) const ORDINAL_WORD_DICTIONARY: &[(&str, i32)] = &[
    ("first", 1),
    ("second", 2),
    ("third", 3),
    ("fourth", 4),
    ("fifth", 5),
    ("sixth", 6),
    ("seventh", 7),
    ("eighth", 8),
    ("ninth", 9),
    ("tenth", 10),
    ("eleventh", 11),
    ("twelfth", 12),
    ("thirteenth", 13),
    ("fourteenth", 14),
    ("fifteenth", 15),
    ("sixteenth", 16),
    ("seventeenth", 17),
    ("eighteenth", 18),
    ("nineteenth", 19),
    ("twentieth", 20),
    ("twenty first", 21),
    ("twenty-first", 21),
    ("twenty second", 22),
    ("twenty-second", 22),
    ("twenty third", 23),
    ("twenty-third", 23),
    ("twenty fourth", 24),
    ("twenty-fourth", 24),
    ("twenty fifth", 25),
    ("twenty-fifth", 25),
    ("twenty sixth", 26),
    ("twenty-sixth", 26),
    ("twenty seventh", 27),
    ("twenty-seventh", 27),
    ("twenty eighth", 28),
    ("twenty-eighth", 28),
    ("twenty ninth", 29),
    ("twenty-ninth", 29),
    ("thirtieth", 30),
    ("thirty first", 31),
    ("thirty-first", 31),
];

pub(crate) const TIME_UNIT_DICTIONARY_NO_ABBR: &[(&str, Timeunit)] = &[
    ("second", Timeunit::Second),
    ("seconds", Timeunit::Second),
    ("minute", Timeunit::Minute),
    ("minutes", Timeunit::Minute),
    ("hour", Timeunit::Hour),
    ("hours", Timeunit::Hour),
    ("day", Timeunit::Day),
    ("days", Timeunit::Day),
    ("week", Timeunit::Week),
    ("weeks", Timeunit::Week),
    ("month", Timeunit::Month),
    ("months", Timeunit::Month),
    ("quarter", Timeunit::Quarter),
    ("quarters", Timeunit::Quarter),
    ("year", Timeunit::Year),
    ("years", Timeunit::Year),
];

pub(crate) const TIME_UNIT_DICTIONARY: &[(&str, Timeunit)] = &[
    ("s", Timeunit::Second),
    ("sec", Timeunit::Second),
    ("second", Timeunit::Second),
    ("seconds", Timeunit::Second),
    ("m", Timeunit::Minute),
    ("min", Timeunit::Minute),
    ("mins", Timeunit::Minute),
    ("minute", Timeunit::Minute),
    ("minutes", Timeunit::Minute),
    ("h", Timeunit::Hour),
    ("hr", Timeunit::Hour),
    ("hrs", Timeunit::Hour),
    ("hour", Timeunit::Hour),
    ("hours", Timeunit::Hour),
    ("d", Timeunit::Day),
    ("day", Timeunit::Day),
    ("days", Timeunit::Day),
    ("w", Timeunit::Week),
    ("week", Timeunit::Week),
    ("weeks", Timeunit::Week),
    ("mo", Timeunit::Month),
    ("mon", Timeunit::Month),
    ("mos", Timeunit::Month),
    ("month", Timeunit::Month),
    ("months", Timeunit::Month),
    ("qtr", Timeunit::Quarter),
    ("quarter", Timeunit::Quarter),
    ("quarters", Timeunit::Quarter),
    ("y", Timeunit::Year),
    ("yr", Timeunit::Year),
    ("year", Timeunit::Year),
    ("years", Timeunit::Year),
];

pub(crate) fn lookup<T: Copy>(dictionary: &[(&str, T)], key: &str) -> Option<T> {
    dictionary
        .iter()
        .find(|(word, _)| *word == key)
        .map(|&(_, value)| value)
}

pub(crate) fn match_any_pattern<T>(dictionary: &[(&str, T)]) -> String {
    let mut terms: Vec<&str> = dictionary.iter().map(|(word, _)| *word).collect();
    terms.sort_by_key(|t| std::cmp::Reverse(t.encode_utf16().count()));
    format!("(?:{})", terms.join("|").replace('.', "\\."))
}

pub(crate) fn repeated_timeunit_pattern(prefix: &str, single: &str, connector: &str) -> String {
    let no_capture = remove_capturing_groups(single);
    format!("{prefix}{no_capture}(?:{connector}{no_capture}){{0,10}}")
}

fn remove_capturing_groups(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len() + 16);
    for (i, &c) in chars.iter().enumerate() {
        out.push(c);
        if c == '(' && chars.get(i + 1) != Some(&'?') {
            out.push_str("?:");
        }
    }
    out
}

pub(crate) static NUMBER_PATTERN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "(?:{}|[0-9]+|[0-9]+\\.[0-9]+|half(?:\\s{{0,2}}an?)?|an?\\b(?:\\s{{0,2}}few)?|few|several|the|a?\\s{{0,2}}couple\\s{{0,2}}(?:of)?)",
        match_any_pattern(INTEGER_WORD_DICTIONARY)
    )
});

pub(crate) fn parse_number_pattern(text: &str) -> f64 {
    let num = text.to_lowercase();
    if let Some(value) = lookup(INTEGER_WORD_DICTIONARY, &num) {
        return f64::from(value);
    }
    if num == "a" || num == "an" || num == "the" {
        return 1.0;
    }
    if num.contains("few") {
        return 3.0;
    }
    if num.contains("half") {
        return 0.5;
    }
    if num.contains("couple") {
        return 2.0;
    }
    if num.contains("several") {
        return 7.0;
    }
    parse_float(&num)
}

pub(crate) static ORDINAL_NUMBER_PATTERN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "(?:{}|[0-9]{{1,2}}(?:st|nd|rd|th)?)",
        match_any_pattern(ORDINAL_WORD_DICTIONARY)
    )
});

pub(crate) fn parse_ordinal_number_pattern(text: &str) -> i32 {
    let num = text.to_lowercase();
    if let Some(value) = lookup(ORDINAL_WORD_DICTIONARY, &num) {
        return value;
    }
    let stripped = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|suffix| num.strip_suffix(suffix))
        .unwrap_or(&num);
    parse_int(stripped)
}

pub(crate) const YEAR_PATTERN: &str = "(?:[1-9][0-9]{0,3}\\s{0,2}(?:BE|AD|BC|BCE|CE)|[1-9][0-9]{3}|[0-9]{2}(?!\\w|:\\d|\\s+(?:am|pm|o\\s*clock|jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)))";

fn replace_first_ignore_case(text: &str, needles: &[&str]) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let (position, needle) = needles
        .iter()
        .filter_map(|n| lower.find(n).map(|p| (p, *n)))
        .min_by_key(|(p, n)| (*p, std::cmp::Reverse(n.len())))?;
    Some(format!(
        "{}{}",
        &text[..position],
        &text[position + needle.len()..]
    ))
}

pub(crate) fn parse_year(text: &str) -> i32 {
    if let Some(rest) = replace_first_ignore_case(text, &["be"]) {
        return parse_int(&rest) - 543;
    }
    if let Some(rest) = replace_first_ignore_case(text, &["bce", "bc"]) {
        return -parse_int(&rest);
    }
    if let Some(rest) = replace_first_ignore_case(text, &["ad", "ce"]) {
        return parse_int(&rest);
    }
    find_most_likely_ad_year(parse_int(text))
}

pub(crate) static SINGLE_TIME_UNIT_PATTERN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "({})\\s{{0,3}}({})",
        *NUMBER_PATTERN,
        match_any_pattern(TIME_UNIT_DICTIONARY)
    )
});

static SINGLE_TIME_UNIT_REGEX: LazyLock<JsRegex> =
    LazyLock::new(|| JsRegex::new(&SINGLE_TIME_UNIT_PATTERN, true));

static SINGLE_TIME_UNIT_NO_ABBR_PATTERN: LazyLock<String> = LazyLock::new(|| {
    format!(
        "({})\\s{{0,3}}({})",
        *NUMBER_PATTERN,
        match_any_pattern(TIME_UNIT_DICTIONARY_NO_ABBR)
    )
});

const TIME_UNIT_CONNECTOR_PATTERN: &str = "\\s{0,5},?(?:\\s*and)?\\s{0,5}";

pub(crate) static TIME_UNITS_PATTERN: LazyLock<String> = LazyLock::new(|| {
    repeated_timeunit_pattern(
        "(?:(?:about|around)\\s{0,3})?",
        &SINGLE_TIME_UNIT_PATTERN,
        TIME_UNIT_CONNECTOR_PATTERN,
    )
});

pub(crate) static TIME_UNITS_NO_ABBR_PATTERN: LazyLock<String> = LazyLock::new(|| {
    repeated_timeunit_pattern(
        "(?:(?:about|around)\\s{0,3})?",
        &SINGLE_TIME_UNIT_NO_ABBR_PATTERN,
        TIME_UNIT_CONNECTOR_PATTERN,
    )
});

pub(crate) fn parse_duration(text: &str) -> Option<Duration> {
    let mut fragments = Duration::new();
    let mut remaining = text.to_string();
    while let Some(m) = SINGLE_TIME_UNIT_REGEX.exec(&remaining) {
        let full = m.full();
        if !full.chars().all(|c| c.is_ascii_alphabetic()) || full.is_empty() {
            let num = parse_number_pattern(m.get(1).unwrap_or(""));
            if let Some(unit) = lookup(TIME_UNIT_DICTIONARY, &m.get(2).unwrap_or("").to_lowercase())
            {
                fragments.set(unit, num);
            }
        }
        let units = full.encode_utf16().count();
        remaining = js_trim(skip_utf16(&remaining, units)).to_string();
    }
    if fragments.is_empty() {
        None
    } else {
        Some(fragments)
    }
}
