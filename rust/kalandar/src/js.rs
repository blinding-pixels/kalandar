use fancy_regex::{Captures, Regex};
use std::sync::OnceLock;

const SPACE_SET: &str = r"\t\n\x{0B}\x{0C}\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";
const WORD_SET: &str = "A-Za-z0-9_";
const DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

pub(crate) fn is_js_space(char: char) -> bool {
    matches!(
        char,
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

pub(crate) fn trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

pub(crate) fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_js_space).filter(|token| !token.is_empty())
}

pub(crate) fn normalize_whitespace(text: &str) -> String {
    words(text).collect::<Vec<_>>().join(" ")
}

pub(crate) fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.len_utf16() == 1 => first.to_uppercase().chain(chars).collect(),
        _ => text.to_string(),
    }
}

pub(crate) fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub(crate) fn has_non_space(text: &str) -> bool {
    text.chars().any(|char| !is_js_space(char))
}

pub(crate) fn slice(text: &str, start: usize, end: usize) -> &str {
    let end = end.min(text.len());
    text.get(start.min(end)..end).unwrap_or("")
}

fn translate(source: &str) -> String {
    let mut output = String::with_capacity(source.len() * 2);
    let mut chars = source.chars().peekable();
    let mut in_class = false;
    while let Some(char) = chars.next() {
        match char {
            '\\' => match chars.next() {
                Some(escaped) => output.push_str(&translate_escape(escaped, in_class)),
                None => output.push('\\'),
            },
            '[' if in_class => output.push_str(r"\["),
            '[' => {
                in_class = true;
                output.push('[');
                if chars.peek() == Some(&'^') {
                    chars.next();
                    output.push('^');
                }
            }
            ']' if in_class => {
                in_class = false;
                output.push(']');
            }
            '&' | '~' if in_class => {
                output.push('\\');
                output.push(char);
            }
            '.' if !in_class => output.push_str(DOT),
            _ => output.push(char),
        }
    }
    output
}

fn translate_escape(escaped: char, in_class: bool) -> String {
    match (escaped, in_class) {
        ('b', false) => {
            format!("(?-i:(?:(?<=[{WORD_SET}])(?![{WORD_SET}])|(?<![{WORD_SET}])(?=[{WORD_SET}])))")
        }
        ('B', false) => {
            format!("(?-i:(?:(?<=[{WORD_SET}])(?=[{WORD_SET}])|(?<![{WORD_SET}])(?![{WORD_SET}])))")
        }
        ('b', true) => r"\x08".to_string(),
        ('d', false) => "[0-9]".to_string(),
        ('d', true) => "0-9".to_string(),
        ('D', _) => "[^0-9]".to_string(),
        ('w', false) => format!("(?-i:[{WORD_SET}])"),
        ('w', true) => WORD_SET.to_string(),
        ('W', _) => format!("(?-i:[^{WORD_SET}])"),
        ('s', false) => format!("[{SPACE_SET}]"),
        ('s', true) => SPACE_SET.to_string(),
        ('S', _) => format!("[^{SPACE_SET}]"),
        _ => format!("\\{escaped}"),
    }
}

pub(crate) fn compile(source: &str, flags: &str) -> Regex {
    let pattern = if flags.contains('i') {
        format!("(?i){}", translate(source))
    } else {
        translate(source)
    };
    Regex::new(&pattern)
        .unwrap_or_else(|error| panic!("rule regex should compile: {source}: {error}"))
}

pub(crate) fn regex(source: &str) -> Regex {
    compile(source, "")
}

pub(crate) fn cached<'a>(cell: &'a OnceLock<Regex>, source: &str, flags: &str) -> &'a Regex {
    cell.get_or_init(|| compile(source, flags))
}

pub(crate) fn test(regex: &Regex, text: &str) -> bool {
    regex.is_match(text).unwrap_or(false)
}

pub(crate) fn exec<'t>(regex: &Regex, text: &'t str) -> Option<Captures<'t>> {
    regex.captures(text).ok().flatten()
}

pub(crate) fn match_all<'t>(regex: &Regex, text: &'t str) -> Vec<Captures<'t>> {
    regex.captures_iter(text).filter_map(Result::ok).collect()
}

pub(crate) fn replace_first(regex: &Regex, text: &str, replacement: &str) -> String {
    regex.replace(text, replacement).into_owned()
}

pub(crate) fn replace_all(regex: &Regex, text: &str, replacement: &str) -> String {
    regex.replace_all(text, replacement).into_owned()
}

pub(crate) fn group<'t>(captures: &Captures<'t>, index: usize) -> &'t str {
    captures.get(index).map_or("", |group| group.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_boundaries_are_ascii_like_javascript() {
        let regex = compile(r"\bam\b", "i");
        assert!(test(&regex, "at 9 AM"));
        assert!(test(&regex, "éam"));
        assert!(!test(&regex, "gamma"));
    }

    #[test]
    fn brackets_inside_classes_are_literal() {
        let regex = compile(r"[()[\]]", "g");
        assert_eq!(regex.replace_all("a[b](c)", " "), "a b  c ");
    }
}
