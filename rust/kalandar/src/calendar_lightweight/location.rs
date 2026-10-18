use super::js::{cached, compile, exec, group, is_js_space, match_all, test, trim, utf16_len};
use super::ParseContext;
use fancy_regex::Regex;
use std::sync::OnceLock;

fn prefix_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(&REGEX, r"(?:^|\s)(at|in|on)\b|@", "gi")
}

fn stop_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(
        &REGEX,
        r"\s+\b(?:with|every|remind|alert|on|from|to|starting|start|for|until|ending|except|invite|w\/|at|in)\b|[,.;()]",
        "i",
    )
}

fn virtual_location_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(
        &REGEX,
        r"^(?:zoom|google\s+meet|meet|teams|webex|slack|discord)\b",
        "i",
    )
}

fn weekday_word_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    cached(
        &REGEX,
        r"\b(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
        "i",
    )
}

fn time_like_patterns() -> &'static [Regex; 13] {
    static REGEXES: OnceLock<[Regex; 13]> = OnceLock::new();
    REGEXES.get_or_init(|| {
        [
            r"^(?:noon|midnight|morning|afternoon|evening|night|today|tomorrow|tonight|next|this)\b",
            r"^(?:in\s+)?\d+\s+(?:day|days|week|weeks|month|months|year|years)\b",
            r"^(?:a|an|one|two|three|four|five|six|seven|eight|nine|ten)\s+(?:day|days|week|weeks|month|months|year|years)\b",
            r"^\d{1,2}(?::\d{2})?\s*(?:am|pm)?$",
            r"^\d{1,2}:\d{2}\b",
            r"^\d{1,2}\s+[a-z]",
            r"^\d+\s+(?:main|room|suite|floor|st|street|ave|avenue|rd|road|blvd|boulevard|park|hall)\b",
            r"^\d{1,2}\s+\d",
            r"^\d{1,2}(?::\d{2})?\s*(?:am|pm)\b",
            r"^\d{1,2}\s*(?:-|–|to)\s*\d{1,2}(?::\d{2})?",
            r"^\d{4}-\d{2}-\d{2}$",
            r"^\d+\s+(?:day|days|week|weeks|month|months|year|years)\b",
            r"^(?:in|after)\s+\d+\s+(?:day|days|week|weeks|month|months|year|years)\b",
        ]
        .map(|source| compile(source, ""))
    })
}

fn is_time_like(value: &str) -> bool {
    let candidate = trim(value).to_lowercase();
    if candidate.is_empty() {
        return true;
    }
    let [noon, relative_count, word_count, clock, colon, number_word, street, number_number, meridiem, range, iso, count, after_count] =
        time_like_patterns();
    let matches = |regex: &Regex| test(regex, &candidate);
    matches(noon)
        || matches(relative_count)
        || matches(word_count)
        || matches(clock)
        || matches(colon)
        || (matches(number_word) && !matches(street))
        || matches(number_number)
        || matches(meridiem)
        || matches(range)
        || matches(iso)
        || matches(count)
        || matches(after_count)
}

fn cleanup_location(value: &str) -> String {
    static QUOTES: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    static TRAILING: OnceLock<Regex> = OnceLock::new();
    let unquoted = cached(&QUOTES, r#"^["']+|["']+$"#, "g").replace_all(trim(value), "");
    let collapsed = cached(&SPACES, r"\s+", "g").replace_all(&unquoted, " ");
    let stripped = cached(&TRAILING, r"[\s-]+$", "").replace(&collapsed, "");
    trim(&stripped).to_string()
}

fn has_strong_location_signal(value: &str) -> bool {
    value.chars().any(|char| char.is_ascii_uppercase())
        || value.chars().any(|char| char.is_ascii_digit())
        || value.contains('@')
        || test(virtual_location_regex(), value)
}

pub(super) fn location_pass(ctx: &mut ParseContext) {
    static THE: OnceLock<Regex> = OnceLock::new();
    let prefix_matches: Vec<(usize, usize, String)> = match_all(prefix_regex(), &ctx.masked_input)
        .into_iter()
        .filter_map(|captures| {
            let full = captures.get(0)?;
            Some((
                full.start(),
                full.end() - full.start(),
                group(&captures, 1).to_string(),
            ))
        })
        .collect();

    for (match_index, match_len, word_prefix) in prefix_matches {
        let prefix_text = if word_prefix.is_empty() {
            "@"
        } else {
            word_prefix.as_str()
        };
        let prefix_start = if word_prefix.is_empty() {
            match_index
        } else {
            match_index + match_len - word_prefix.len()
        };
        let value_start = prefix_start + prefix_text.len();

        let following = ctx
            .masked_input
            .get(value_start..)
            .and_then(|rest| rest.chars().next());
        if !following.is_some_and(is_js_space) {
            continue;
        }

        let after_value = ctx.masked_input.get(value_start..).unwrap_or("");
        let remainder = after_value.trim_start_matches(is_js_space);
        let normalized_value_start = value_start + after_value.len() - remainder.len();
        let raw_candidate = match exec(stop_regex(), remainder).and_then(|found| found.get(0)) {
            Some(stop) => &remainder[..stop.start()],
            None => remainder,
        };
        let cleaned = cleanup_location(raw_candidate);

        if cleaned.is_empty() || utf16_len(&cleaned) > 80 || is_time_like(&cleaned) {
            continue;
        }
        let prefix_lower = prefix_text.to_lowercase();
        let is_virtual = test(virtual_location_regex(), &cleaned);
        if prefix_lower == "on" && !is_virtual {
            continue;
        }
        if prefix_lower == "in" && !has_strong_location_signal(&cleaned) {
            continue;
        }
        if prefix_lower == "in"
            && (test(cached(&THE, r"^the\s+", "i"), &cleaned)
                || test(weekday_word_regex(), &cleaned))
        {
            continue;
        }

        let span_start = prefix_start;
        let span_end = normalized_value_start + raw_candidate.len();
        let quoted = raw_candidate.contains(['"', '\'']);
        let confidence = if quoted {
            0.82
        } else if is_virtual {
            0.78
        } else {
            0.68
        };

        if ctx.location.is_some() && confidence < ctx.location_confidence {
            continue;
        }

        ctx.location = Some(cleaned);
        ctx.location_confidence = ctx.location_confidence.max(confidence);
        ctx.mask_span(span_start, span_end);
    }
}
