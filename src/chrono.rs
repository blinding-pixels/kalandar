use crate::js_regex::{JsMatch, JsRegex};
use crate::results::{ParsedComponents, ParsedResult, ReferenceWithTimezone};
use crate::types::{Component, ParsingOption, ParsingReference};

pub(crate) struct ParsingContext<'a> {
    pub text: &'a str,
    pub option: &'a ParsingOption,
    pub reference: ReferenceWithTimezone,
}

impl<'a> ParsingContext<'a> {
    pub fn new(
        text: &'a str,
        reference: &ParsingReference,
        option: &'a ParsingOption,
    ) -> ParsingContext<'a> {
        ParsingContext {
            text,
            option,
            reference: ReferenceWithTimezone::from_input(reference, option),
        }
    }

    pub fn components(&self) -> ParsedComponents {
        ParsedComponents::new(self.reference)
    }

    pub fn components_with(&self, known: &[(Component, i32)]) -> ParsedComponents {
        ParsedComponents::with_known(self.reference, known)
    }

    pub fn result(&self, index: usize, text: &str) -> ParsedResult {
        ParsedResult::new(self.reference, index, text, None, None)
    }

    pub fn result_between(&self, index: usize, end: usize) -> ParsedResult {
        self.result(index, js_substring(self.text, index, end))
    }
}

pub(crate) fn js_substring(text: &str, start: usize, end: usize) -> &str {
    let (a, b) = if start > end {
        (end, start)
    } else {
        (start, end)
    };
    let a = a.min(text.len());
    let b = b.min(text.len());
    &text[a..b]
}

pub(crate) fn suffix_from(text: &str, index: usize) -> &str {
    &text[index.min(text.len())..]
}

pub(crate) struct ParserMatch<'t> {
    pub index: usize,
    groups: Vec<Option<&'t str>>,
}

impl<'t> ParserMatch<'t> {
    fn from_js(m: &JsMatch<'t>, offset: usize) -> ParserMatch<'t> {
        ParserMatch {
            index: m.index() + offset,
            groups: (0..m.len()).map(|i| m.get(i)).collect(),
        }
    }

    pub fn text(&self) -> &'t str {
        self.groups[0].unwrap_or("")
    }

    pub fn get(&self, group: usize) -> Option<&'t str> {
        self.groups.get(group).copied().flatten()
    }

    pub fn non_empty(&self, group: usize) -> Option<&'t str> {
        self.get(group).filter(|s| !s.is_empty())
    }

    pub fn group(&self, group: usize) -> &'t str {
        self.get(group).unwrap_or("")
    }

    pub fn strip_left_boundary(&mut self) {
        let header = self.get(1).unwrap_or("");
        self.index += header.len();
        self.groups[0] = Some(&self.text()[header.len()..]);
        self.groups.remove(1);
    }
}

pub(crate) enum Extracted {
    Components(ParsedComponents),
    Result(Box<ParsedResult>),
}

pub(crate) trait Parser: Send + Sync {
    fn pattern(&self, context: &ParsingContext) -> &JsRegex;
    fn extract(&self, context: &ParsingContext, m: &mut ParserMatch) -> Option<Extracted>;
}

pub(crate) trait Refiner: Send + Sync {
    fn refine(&self, context: &ParsingContext, results: Vec<ParsedResult>) -> Vec<ParsedResult>;
}

pub struct Chrono {
    parsers: Vec<Box<dyn Parser>>,
    refiners: Vec<Box<dyn Refiner>>,
}

impl Chrono {
    pub(crate) fn from_configuration(
        parsers: Vec<Box<dyn Parser>>,
        refiners: Vec<Box<dyn Refiner>>,
    ) -> Chrono {
        Chrono { parsers, refiners }
    }

    pub fn parse(
        &self,
        text: &str,
        reference: &ParsingReference,
        option: &ParsingOption,
    ) -> Vec<ParsedResult> {
        let context = ParsingContext::new(text, reference, option);
        let mut results = Vec::new();
        for parser in &self.parsers {
            results.extend(execute_parser(&context, parser.as_ref()));
        }
        results.sort_by_key(|r| r.index);
        for refiner in &self.refiners {
            results = refiner.refine(&context, results);
        }
        results
    }

    pub fn parse_date(
        &self,
        text: &str,
        reference: &ParsingReference,
        option: &ParsingOption,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        self.parse(text, reference, option)
            .first()
            .and_then(ParsedResult::date)
    }
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    match text.get(index..).and_then(|rest| rest.chars().next()) {
        Some(c) => index + c.len_utf8(),
        None => index + 1,
    }
}

fn execute_parser(context: &ParsingContext, parser: &dyn Parser) -> Vec<ParsedResult> {
    let mut results = Vec::new();
    let pattern = parser.pattern(context);
    let text = context.text;
    let mut remaining_start = 0;
    while remaining_start <= text.len() {
        let Some(js_match) = pattern.exec(&text[remaining_start..]) else {
            break;
        };
        let mut m = ParserMatch::from_js(&js_match, remaining_start);
        match parser.extract(context, &mut m) {
            None => {
                remaining_start = next_char_boundary(text, m.index);
            }
            Some(extracted) => {
                let result = match extracted {
                    Extracted::Result(result) => *result,
                    Extracted::Components(components) => ParsedResult::new(
                        context.reference,
                        m.index,
                        m.text(),
                        Some(components),
                        None,
                    ),
                };
                remaining_start = result.end_index();
                results.push(result);
            }
        }
    }
    results
}

pub(crate) trait MergingRefiner: Send + Sync {
    fn should_merge(
        &self,
        text_between: &str,
        current: &ParsedResult,
        next: &ParsedResult,
        context: &ParsingContext,
    ) -> bool;

    fn merge(
        &self,
        text_between: &str,
        current: ParsedResult,
        next: ParsedResult,
        context: &ParsingContext,
    ) -> ParsedResult;
}

pub(crate) fn merge_consecutive<M: MergingRefiner + ?Sized>(
    refiner: &M,
    context: &ParsingContext,
    results: Vec<ParsedResult>,
) -> Vec<ParsedResult> {
    if results.len() < 2 {
        return results;
    }
    let mut merged = Vec::with_capacity(results.len());
    let mut iter = results.into_iter();
    let Some(mut current) = iter.next() else {
        return merged;
    };
    for next in iter {
        let between = js_substring(context.text, current.end_index(), next.index).to_string();
        if refiner.should_merge(&between, &current, &next, context) {
            current = refiner.merge(&between, current, next, context);
        } else {
            merged.push(std::mem::replace(&mut current, next));
        }
    }
    merged.push(current);
    merged
}

pub(crate) trait Filter: Send + Sync {
    fn is_valid(&self, context: &ParsingContext, result: &ParsedResult) -> bool;
}

pub(crate) fn filter_results<F: Filter + ?Sized>(
    filter: &F,
    context: &ParsingContext,
    results: Vec<ParsedResult>,
) -> Vec<ParsedResult> {
    results
        .into_iter()
        .filter(|r| filter.is_valid(context, r))
        .collect()
}
