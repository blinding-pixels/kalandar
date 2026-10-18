use fancy_regex::{Regex, RegexBuilder};

const JS_SPACE_CLASS_BODY: &str =
    "\\t\\n\\x0B\\x0C\\r \\x{A0}\\x{1680}\\x{2000}-\\x{200A}\\x{2028}\\x{2029}\\x{202F}\\x{205F}\\x{3000}\\x{FEFF}";
const JS_WORD_CLASS_BODY: &str = "A-Za-z0-9_";
const JS_WORD_BOUNDARY: &str =
    "(?:(?<=[A-Za-z0-9_])(?![A-Za-z0-9_])|(?<![A-Za-z0-9_])(?=[A-Za-z0-9_]))";
const JS_NON_WORD_BOUNDARY: &str =
    "(?:(?<=[A-Za-z0-9_])(?=[A-Za-z0-9_])|(?<![A-Za-z0-9_])(?![A-Za-z0-9_]))";

macro_rules! js_re {
    ($pattern:expr) => {{
        static RE: std::sync::LazyLock<$crate::js_regex::JsRegex> =
            std::sync::LazyLock::new(|| $crate::js_regex::JsRegex::new($pattern, false));
        &*RE
    }};
    ($pattern:expr, i) => {{
        static RE: std::sync::LazyLock<$crate::js_regex::JsRegex> =
            std::sync::LazyLock::new(|| $crate::js_regex::JsRegex::new($pattern, true));
        &*RE
    }};
}

pub(crate) use js_re;

pub struct JsRegex {
    regex: Regex,
    prefilter: regex::Regex,
}

pub struct JsMatch<'t> {
    groups: Vec<Option<(usize, usize)>>,
    text: &'t str,
}

impl<'t> JsMatch<'t> {
    pub fn index(&self) -> usize {
        self.groups[0].map_or(0, |g| g.0)
    }

    pub fn full(&self) -> &'t str {
        self.get(0).unwrap_or("")
    }

    pub fn get(&self, group: usize) -> Option<&'t str> {
        self.groups
            .get(group)
            .copied()
            .flatten()
            .map(|(s, e)| &self.text[s..e])
    }

    pub fn non_empty(&self, group: usize) -> Option<&'t str> {
        self.get(group).filter(|s| !s.is_empty())
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }
}

impl JsRegex {
    pub fn new(source: &str, ignore_case: bool) -> JsRegex {
        let pattern = translate(source, ignore_case);
        let regex = RegexBuilder::new(&pattern)
            .backtrack_limit(50_000_000)
            .build()
            .unwrap_or_else(|e| panic!("invalid pattern {pattern}: {e}"));
        let relaxed = without_lookaround(&pattern);
        let prefilter = regex::Regex::new(&relaxed)
            .unwrap_or_else(|e| panic!("invalid prefilter {relaxed}: {e}"));
        JsRegex { regex, prefilter }
    }

    pub fn exec<'t>(&self, text: &'t str) -> Option<JsMatch<'t>> {
        let start = self.prefilter.find(text)?.start();
        let captures = self.regex.captures_from_pos(text, start).ok().flatten()?;
        let groups = (0..captures.len())
            .map(|i| captures.get(i).map(|m| (m.start(), m.end())))
            .collect();
        Some(JsMatch { groups, text })
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.exec(text).is_some()
    }
}

fn push_escaped_literal(out: &mut String, c: char, in_class: bool) {
    let needs_escape = if in_class {
        matches!(c, '\\' | ']' | '[' | '^' | '-' | '&' | '~')
    } else {
        matches!(
            c,
            '\\' | '.'
                | '+'
                | '*'
                | '?'
                | '('
                | ')'
                | '|'
                | '['
                | ']'
                | '{'
                | '}'
                | '^'
                | '$'
                | '#'
                | '-'
        )
    };
    if c.is_ascii() && needs_escape {
        out.push('\\');
        out.push(c);
    } else if c.is_ascii_graphic() || c == ' ' {
        out.push(c);
    } else {
        out.push_str(&format!("\\x{{{:X}}}", c as u32));
    }
}

fn translate(source: &str, ignore_case: bool) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len() * 2);
    let mut i = 0;
    let mut in_class = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            let e = chars[i + 1];
            i += 2;
            match (e, in_class) {
                ('d', false) => out.push_str("[0-9]"),
                ('D', false) => out.push_str("[^0-9]"),
                ('w', false) => out.push_str(&format!("[{JS_WORD_CLASS_BODY}]")),
                ('W', false) => out.push_str(&format!("[^{JS_WORD_CLASS_BODY}]")),
                ('s', false) => out.push_str(&format!("[{JS_SPACE_CLASS_BODY}]")),
                ('S', false) => out.push_str(&format!("[^{JS_SPACE_CLASS_BODY}]")),
                ('b', false) => out.push_str(JS_WORD_BOUNDARY),
                ('B', false) => out.push_str(JS_NON_WORD_BOUNDARY),
                ('d', true) => out.push_str("0-9"),
                ('w', true) => out.push_str(JS_WORD_CLASS_BODY),
                ('s', true) => out.push_str(JS_SPACE_CLASS_BODY),
                ('D' | 'W' | 'S', true) => {
                    panic!("unsupported negated class escape in class: {source}")
                }
                ('t', _) => out.push_str("\\t"),
                ('n', _) => out.push_str("\\n"),
                ('r', _) => out.push_str("\\r"),
                ('f', _) => out.push_str("\\x0C"),
                ('v', _) => out.push_str("\\x0B"),
                _ if e.is_ascii_digit() => {
                    out.push('\\');
                    out.push(e);
                }
                _ if ignore_case && e.is_ascii_alphabetic() && !in_class => {
                    out.push('[');
                    push_both_cases(&mut out, e);
                    out.push(']');
                }
                _ if ignore_case && e.is_ascii_alphabetic() => push_both_cases(&mut out, e),
                _ => push_escaped_literal(&mut out, e, in_class),
            }
            continue;
        }
        if in_class {
            match c {
                ']' => {
                    in_class = false;
                    out.push(']');
                }
                '-' => out.push('-'),
                _ if ignore_case && c.is_ascii_alphabetic() => {
                    let range_end = match (chars.get(i + 1), chars.get(i + 2)) {
                        (Some('-'), Some(&end)) if end.is_ascii_alphabetic() => Some(end),
                        _ => None,
                    };
                    match range_end {
                        Some(end) => {
                            push_case_range(&mut out, c, end);
                            i += 2;
                        }
                        None => push_both_cases(&mut out, c),
                    }
                }
                _ => push_escaped_literal(&mut out, c, true),
            }
        } else {
            match c {
                '[' => {
                    in_class = true;
                    out.push('[');
                    if chars.get(i + 1) == Some(&'^') {
                        out.push('^');
                        i += 1;
                    }
                }
                '.' => out.push_str("[^\\n\\r\\x{2028}\\x{2029}]"),
                _ if ignore_case && c.is_ascii_alphabetic() => {
                    out.push('[');
                    push_both_cases(&mut out, c);
                    out.push(']');
                }
                '(' | ')' | '|' | '*' | '+' | '?' | '{' | '}' | '^' | '$' | ':' | '=' | '!'
                | '<' | ',' => out.push(c),
                _ => push_escaped_literal(&mut out, c, false),
            }
        }
        i += 1;
    }
    out
}

fn push_both_cases(out: &mut String, c: char) {
    out.push(c.to_ascii_lowercase());
    out.push(c.to_ascii_uppercase());
}

fn push_case_range(out: &mut String, start: char, end: char) {
    for (a, b) in [
        (start.to_ascii_lowercase(), end.to_ascii_lowercase()),
        (start.to_ascii_uppercase(), end.to_ascii_uppercase()),
    ] {
        out.push(a);
        out.push('-');
        out.push(b);
    }
}

fn without_lookaround(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0;
    let mut in_class = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            out.push(c);
            if let Some(&next) = chars.get(i + 1) {
                out.push(next);
            }
            i += 2;
            continue;
        }
        if in_class {
            in_class = c != ']';
            out.push(c);
            i += 1;
            continue;
        }
        let rest: String = chars[i..chars.len().min(i + 4)].iter().collect();
        if rest.starts_with("(?=")
            || rest.starts_with("(?!")
            || rest.starts_with("(?<=")
            || rest.starts_with("(?<!")
        {
            i = skip_group(&chars, i);
            continue;
        }
        if c == '[' {
            in_class = true;
            out.push(c);
            if chars.get(i + 1) == Some(&'^') {
                out.push('^');
                i += 1;
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn skip_group(chars: &[char], open: usize) -> usize {
    let mut depth = 0;
    let mut i = open;
    let mut in_class = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            i += 2;
            continue;
        }
        if in_class {
            in_class = c != ']';
        } else if c == '[' {
            in_class = true;
            if chars.get(i + 1) == Some(&'^') {
                i += 1;
            }
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    chars.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_classes_and_boundaries() {
        let re = JsRegex::new("(\\W|^)(\\d{1,2})\\s*(am|pm)(?=\\W|$)", true);
        let m = re.exec("at 5 PM.").unwrap();
        assert_eq!(m.get(2), Some("5"));
        assert_eq!(m.get(3), Some("PM"));
        let re = JsRegex::new("\\b(in)$", true);
        assert!(re.is_match("go in"));
        assert!(!re.is_match("goin"));
        let re = JsRegex::new("[^\\s\\w]", false);
        assert!(re.is_match("a,b"));
        assert!(!re.is_match("a b"));
        let re = JsRegex::new("\\–|\\〜|\\：", false);
        assert!(re.is_match("〜"));
        let re = JsRegex::new("^(sunday|[A-Z]{2})$", true);
        assert!(re.is_match("SUNDAY"));
        assert!(re.is_match("pt"));
        assert!(!re.is_match("\u{17F}unday"));
        assert!(!re.is_match("\u{212A}"));
    }
}
