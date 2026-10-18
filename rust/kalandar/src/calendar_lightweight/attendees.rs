use super::ParseContext;
use crate::js::{cached, match_all};
use fancy_regex::Regex;
use std::sync::OnceLock;

fn prefixed_attendee_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(
        &REGEX,
        r"\b(?:with|invite|w\/)\s+([^,;]+?)(?=(?:\s+\b(?:every|at|in|on|from|to|for|starting|start|remind|alert|until|ending|except)\b|[,.;]|$))",
        "gi",
    )
}

fn email_pattern() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(&REGEX, r"\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", "gi")
}

fn extract_emails(input: &str) -> Vec<(usize, String)> {
    match_all(email_pattern(), input)
        .into_iter()
        .filter_map(|captures| captures.get(0))
        .map(|email| (email.start(), email.as_str().to_string()))
        .collect()
}

fn push_attendee(ctx: &mut ParseContext, email: &str) {
    let normalized = email.to_lowercase();
    let current = ctx.attendees.get_or_insert_with(Vec::new);
    if !current.contains(&normalized) {
        current.push(normalized);
    }
}

pub(super) fn attendees_pass(ctx: &mut ParseContext) {
    let prefixed_matches: Vec<(usize, String)> =
        match_all(prefixed_attendee_regex(), &ctx.masked_input)
            .into_iter()
            .filter_map(|captures| captures.get(0))
            .map(|full| (full.start(), full.as_str().to_string()))
            .collect();

    for (start, full_text) in prefixed_matches {
        let end = start + full_text.len();
        if !ctx.is_span_available(start, end) {
            continue;
        }
        let emails = extract_emails(&full_text);
        if emails.is_empty() {
            continue;
        }
        for (_, email_text) in &emails {
            push_attendee(ctx, email_text);
        }
        ctx.mask_span(start, end);
    }

    for (start, email_text) in extract_emails(&ctx.masked_input) {
        let end = start + email_text.len();
        if !ctx.is_span_available(start, end) {
            continue;
        }
        push_attendee(ctx, &email_text);
        ctx.mask_span(start, end);
    }
}
