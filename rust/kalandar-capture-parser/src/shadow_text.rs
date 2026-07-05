use regex::Regex;

pub fn normalize_shadow_text(input: &str) -> String {
    let mut output = input.to_string();

    for (pattern, replacement) in direct_replacements() {
        output = pattern.replace_all(&output, *replacement).into_owned();
    }

    for (pattern, replacement) in time_of_day_shorthand() {
        output = pattern.replace_all(&output, *replacement).into_owned();
    }

    for (pattern, replacement) in weekday_shorthand() {
        output = pattern.replace_all(&output, *replacement).into_owned();
    }

    whitespace_regex()
        .replace_all(
            &numeric_range_regex().replace_all(&output.replace(['–', '—'], "-"), "$1 to $2"),
            " ",
        )
        .trim()
        .to_string()
}

fn direct_replacements() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: std::sync::OnceLock<Vec<(Regex, &'static str)>> =
        std::sync::OnceLock::new();
    REPLACEMENTS.get_or_init(|| {
        vec![
            (word_regex("tmrw"), "tomorrow"),
            (word_regex("tmr"), "tomorrow"),
            (word_regex("tomorow"), "tomorrow"),
            (word_regex("tommorow"), "tomorrow"),
            (word_regex("tmorow"), "tomorrow"),
            (word_regex("tonite"), "tonight"),
            (
                Regex::new(r"(?i)\b2\s*hrs?\b")
                    .expect("shadow text replacement regex should compile"),
                "2 hours",
            ),
            (
                Regex::new(r"(?i)\b(\d+)\s*hrs?\b")
                    .expect("shadow text replacement regex should compile"),
                "$1 hours",
            ),
            (
                Regex::new(r"(?i)\b(\d+)\s*mins?\b")
                    .expect("shadow text replacement regex should compile"),
                "$1 minutes",
            ),
            (
                Regex::new(r"(?i)\b(\d+)\s*min\b")
                    .expect("shadow text replacement regex should compile"),
                "$1 minutes",
            ),
            (
                Regex::new(r"(?i)\b(\d+)\s*secs?\b")
                    .expect("shadow text replacement regex should compile"),
                "$1 seconds",
            ),
        ]
    })
}

fn weekday_shorthand() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: std::sync::OnceLock<Vec<(Regex, &'static str)>> =
        std::sync::OnceLock::new();
    REPLACEMENTS.get_or_init(|| {
        vec![
            (word_regex("mon"), "monday"),
            (word_regex("tue"), "tuesday"),
            (word_regex("tues"), "tuesday"),
            (word_regex("wed"), "wednesday"),
            (word_regex("thu"), "thursday"),
            (word_regex("thur"), "thursday"),
            (word_regex("thurs"), "thursday"),
            (word_regex("fri"), "friday"),
            (word_regex("sat"), "saturday"),
            (word_regex("sun"), "sunday"),
        ]
    })
}

fn time_of_day_shorthand() -> &'static [(Regex, &'static str)] {
    static REPLACEMENTS: std::sync::OnceLock<Vec<(Regex, &'static str)>> =
        std::sync::OnceLock::new();
    REPLACEMENTS.get_or_init(|| {
        vec![
            (
                Regex::new(r"(?i)\btom(?:orrow)?\s+night\b")
                    .expect("shadow text replacement regex should compile"),
                "tomorrow night",
            ),
            (
                Regex::new(r"(?i)\btom(?:orrow)?\s+morn(?:ing)?\b")
                    .expect("shadow text replacement regex should compile"),
                "tomorrow morning",
            ),
            (
                Regex::new(r"(?i)\bfri\s+eve\b")
                    .expect("shadow text replacement regex should compile"),
                "friday evening",
            ),
            (
                Regex::new(r"(?i)\bfri\s+morn\b")
                    .expect("shadow text replacement regex should compile"),
                "friday morning",
            ),
        ]
    })
}

fn numeric_range_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(\d{1,2})\s*-\s*(\d{1,2})\b")
            .expect("shadow text numeric range regex should compile")
    })
}

fn whitespace_regex() -> &'static Regex {
    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"\s+").expect("shadow text whitespace regex should compile"))
}

fn word_regex(word: &str) -> Regex {
    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(word)))
        .expect("shadow text word regex should compile")
}
