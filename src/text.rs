fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

pub(crate) fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

pub(crate) fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(crate) fn skip_utf16(text: &str, units: usize) -> &str {
    let mut count = 0;
    for (i, c) in text.char_indices() {
        if count >= units {
            return &text[i..];
        }
        count += c.len_utf16();
    }
    ""
}

pub(crate) fn parse_int(text: &str) -> i32 {
    let trimmed = text.trim_start_matches(is_js_space);
    let (negative, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let value = digits
        .chars()
        .take_while(char::is_ascii_digit)
        .fold(0i64, |acc, c| {
            acc.saturating_mul(10)
                .saturating_add(i64::from(c.to_digit(10).unwrap_or(0)))
        });
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

pub(crate) fn parse_float(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(is_js_space);
    let mut end = 0;
    let mut seen_dot = false;
    let mut seen_digit = false;
    for (i, c) in trimmed.char_indices() {
        if c.is_ascii_digit() {
            seen_digit = true;
            end = i + 1;
        } else if c == '.' && !seen_dot {
            seen_dot = true;
        } else {
            break;
        }
    }
    if !seen_digit {
        return f64::NAN;
    }
    trimmed[..end].parse::<f64>().unwrap_or(f64::NAN)
}
