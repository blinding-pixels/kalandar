use super::{
    postprocess::finalize_task_title, schedule::extract_schedule, CapturePreviewRequest,
    CapturePreviewTaskDraft, CapturePreviewWarning, PreviewSchedule,
};
use kalandar_dates::capture::CaptureKind;
use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub struct TaskParseOutput {
    pub draft: CapturePreviewTaskDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub confidence: f64,
    pub schedule: Option<PreviewSchedule>,
}

pub fn parse_task_preview(text: &str, request: &CapturePreviewRequest) -> TaskParseOutput {
    let mut working = normalize_whitespace(text);

    let priority = detect_priority(&mut working);
    let status = detect_status(&mut working);
    let tags = extract_tags(&mut working);
    let project = extract_project(&mut working);
    let assignees = extract_assignees(&mut working, request.known_assignees.as_deref());
    let schedule = extract_schedule(text, &working, CaptureKind::Task, request);
    let title = build_task_title(&schedule.remaining);

    let mut warnings = Vec::new();
    if schedule.texts.moment.is_none() && schedule.texts.recurrence.is_none() {
        warnings.push(warning(
            "no_due_date",
            "info",
            "No due date or recurrence extracted.",
        ));
    }

    if let Some(known_assignees) = &request.known_assignees {
        let known = known_assignee_set(known_assignees);
        let unknown = assignees
            .iter()
            .filter(|assignee| {
                !assignee.starts_with('@') && !known.contains(&assignee.trim().to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>();
        if !unknown.is_empty() {
            warnings.push(warning(
                "unknown_assignee",
                "warning",
                &format!("Unknown assignee: {}", unknown.join(", ")),
            ));
        }
    }
    warnings.extend(schedule.warnings.iter().cloned());

    let confidence = if title == "Task" {
        0.3
    } else if title.split_whitespace().count() >= 5 {
        0.98
    } else if title.split_whitespace().count() >= 3 {
        0.9
    } else if title.split_whitespace().count() >= 2 {
        0.75
    } else {
        0.55
    };

    let recurrence_exceptions = schedule.recurrence_exceptions();
    TaskParseOutput {
        draft: CapturePreviewTaskDraft {
            title,
            assignees,
            project_hint: project,
            tags,
            priority,
            status,
            due_date_text: schedule.texts.date.clone(),
            due_time_text: schedule.texts.time.clone(),
            duration_text: schedule.texts.duration.clone(),
            recurrence_text: schedule.texts.recurrence.clone(),
            recurrence_start_text: schedule.texts.recurrence_start.clone(),
            recurrence_end_text: schedule.texts.recurrence_end.clone(),
            recurrence_exceptions,
        },
        warnings,
        confidence,
        schedule: schedule.schedule,
    }
}

fn warning(code: &str, severity: &str, message: &str) -> CapturePreviewWarning {
    CapturePreviewWarning {
        code: code.to_string(),
        severity: severity.to_string(),
        message: message.to_string(),
    }
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_segment(working: &mut String, start: usize, end: usize) {
    *working = normalize_whitespace(&format!("{} {}", &working[..start], &working[end..]));
}

fn capture(text: &str, regex: &Regex) -> Option<(usize, usize, String)> {
    regex.find(text).map(|matched| {
        (
            matched.start(),
            matched.end(),
            matched.as_str().trim().to_string(),
        )
    })
}

fn detect_priority(working: &mut String) -> Option<String> {
    for (patterns, value) in [
        (high_priority_patterns(), "high"),
        (medium_priority_patterns(), "medium"),
        (low_priority_patterns(), "low"),
    ] {
        let mut matched = false;
        for pattern in patterns {
            if pattern.is_match(working) {
                matched = true;
            }
            *working = normalize_whitespace(&pattern.replace_all(working, " "));
        }
        if matched {
            return Some(value.to_string());
        }
    }
    None
}

fn detect_status(working: &mut String) -> Option<String> {
    for (pattern, value) in [
        (r"(?i)\bin\s+progress\b", "in progress"),
        (r"(?i)\bnot\s+started\b", "not started"),
        (r"(?i)\bto\s*do\b", "todo"),
        (r"(?i)\btodo\b", "todo"),
        (r"(?i)\bblocked\b", "blocked"),
        (r"(?i)\bwaiting\b", "waiting"),
        (r"(?i)\bdone\b", "done"),
        (r"(?i)\bcompleted?\b", "done"),
    ] {
        let regex = Regex::new(pattern).expect("status regex should compile");
        if let Some((start, end, _)) = capture(working, &regex) {
            remove_segment(working, start, end);
            return Some(value.to_string());
        }
    }
    None
}

fn extract_tags(working: &mut String) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = BTreeSet::new();

    let trailing_clause_regex =
        Regex::new(r"(?i)\b(?:with\s+)?tags?\s*:?\s*([^.;]+)$").expect("tags regex should compile");
    if let Some(captures) = trailing_clause_regex.captures(working.as_str()) {
        if let Some(full) = captures.get(0) {
            if let Some(group) = captures.get(1) {
                for tag in parse_tag_list(group.as_str()) {
                    if seen.insert(tag.to_lowercase()) {
                        tags.push(tag);
                    }
                }
            }
            remove_segment(working, full.start(), full.end());
        }
    }

    let hashtag_regex =
        Regex::new(r"(?i)#[a-z0-9][a-z0-9_-]*").expect("hashtag regex should compile");
    let matches = hashtag_regex
        .find_iter(working.as_str())
        .map(|matched| matched.as_str().to_string())
        .collect::<Vec<_>>();
    if !matches.is_empty() {
        for tag in matches {
            let cleaned = cleanup_tag_token(&tag);
            if let Some(cleaned) = cleaned {
                if seen.insert(cleaned.to_lowercase()) {
                    tags.push(cleaned);
                }
            }
        }
        *working = normalize_whitespace(
            &hashtag_regex
                .replace_all(working, " ")
                .replace("tags", " ")
                .replace("tag", " "),
        );
    }

    tags
}

fn cleanup_tag_token(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|char: char| ",;:!?".contains(char));
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        let normalized = rest
            .chars()
            .filter(|char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-')
            .collect::<String>();
        return (!normalized.is_empty()).then_some(normalized);
    }
    let normalized = trimmed
        .chars()
        .filter(|char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-')
        .collect::<String>();
    (!normalized.is_empty()).then_some(normalized)
}

fn parse_tag_list(value: &str) -> Vec<String> {
    let hashtag_regex =
        Regex::new(r"(?i)#[a-z0-9][a-z0-9_-]*").expect("hashtag regex should compile");
    let raw = if hashtag_regex.is_match(value) {
        hashtag_regex
            .find_iter(value)
            .map(|matched| matched.as_str().to_string())
            .collect::<Vec<_>>()
    } else {
        Regex::new(r"(?i)\s*(?:,|and)\s*")
            .expect("tag list regex should compile")
            .split(value)
            .map(str::to_string)
            .collect::<Vec<_>>()
    };

    let mut seen = BTreeSet::new();
    raw.into_iter()
        .filter_map(|part| cleanup_tag_token(&part))
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .collect()
}

fn extract_project(working: &mut String) -> Option<String> {
    let patterns = [
        (
            Regex::new(r"(?i)\b(?:in|under)\s+project\s+(.+?)(?:\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b|\d+\s*(?:minutes?|mins?|hours?|hrs?)\b)|\s*[,.;]|$)")
                .expect("project explicit regex should compile"),
            true,
            false,
        ),
        (
            Regex::new(r"(?i)\bfor\s+project\s+(.+?)(?:\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b|\d+\s*(?:minutes?|mins?|hours?|hrs?)\b)|\s*[,.;]|$)")
                .expect("project for-project regex should compile"),
            true,
            false,
        ),
        (
            Regex::new(r"(?i)\bfor\s+([A-Z][A-Za-z0-9-]*(?:\s+[A-Z][A-Za-z0-9-]*){1,3})(?:\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b|\d+\s*(?:minutes?|mins?|hours?|hrs?)\b)|\s*[,.;]|$)")
                .expect("project titlecase regex should compile"),
            false,
            true,
        ),
    ];

    for (regex, project_prefix, title_case_only) in patterns {
        if let Some(captures) = regex.captures(working.as_str()) {
            let full = captures.get(0).expect("project full match should exist");
            let group = captures.get(1).expect("project capture group should exist");
            let candidate = clean_named_value(group.as_str());
            let valid = if title_case_only {
                is_likely_project(&candidate)
            } else {
                !candidate.is_empty()
            };
            if valid {
                remove_segment(working, full.start(), group.end());
                return Some(if project_prefix {
                    format!("Project {}", candidate)
                } else {
                    candidate
                });
            }
        }
    }

    None
}

fn clean_named_value(value: &str) -> String {
    normalize_whitespace(
        value
            .trim()
            .trim_matches(|char: char| ",;:!?".contains(char)),
    )
}

fn split_entity_list(value: &str) -> Vec<String> {
    Regex::new(r"(?i)\s*(?:,|and|&)\s*")
        .expect("entity split regex should compile")
        .split(value)
        .map(clean_named_value)
        .filter(|value| !value.is_empty())
        .collect()
}

fn known_assignee_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn is_likely_person(value: &str, known_assignees: &BTreeSet<String>) -> bool {
    if value.is_empty() {
        return false;
    }
    if Regex::new(r"(?i)^@[a-z0-9_.+-]+$")
        .expect("mention regex should compile")
        .is_match(value)
    {
        return true;
    }
    if Regex::new(r"(?i)^[a-z0-9.+-]+@[a-z0-9.-]+\.[a-z]{2,}$")
        .expect("email regex should compile")
        .is_match(value)
    {
        return true;
    }
    let normalized = value.to_lowercase();
    if known_assignees.contains(&normalized) {
        return true;
    }
    if Regex::new(
        r"(?i)^(?:project|tag|tags|today|tomorrow|tonight|next|this|every|daily|weekly|monthly)$",
    )
    .expect("reserved value regex should compile")
    .is_match(value)
    {
        return false;
    }
    if value.chars().any(|char| char.is_ascii_digit()) {
        return false;
    }
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() > 2 {
        return false;
    }
    if tokens.len() == 1
        && Regex::new(r"^[a-z][a-z'-]+$")
            .expect("single lowercase person token regex should compile")
            .is_match(tokens[0])
    {
        return !matches!(
            tokens[0],
            "the" | "a" | "an" | "conference" | "report" | "invoice" | "passport" | "bus"
        );
    }
    tokens.iter().all(|token| {
        Regex::new(r"^[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?$")
            .expect("person token regex should compile")
            .is_match(token)
    })
}

fn title_case_name(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            if part.starts_with('@') || part.contains('@') {
                part.to_string()
            } else {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => {
                        format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase())
                    }
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_assignee_value(value: &str, known_assignees: &BTreeSet<String>) -> Option<String> {
    let cleaned = clean_named_value(value);
    if !is_likely_person(&cleaned, known_assignees) {
        return None;
    }
    if cleaned.starts_with('@') || cleaned.contains('@') {
        return Some(cleaned);
    }
    Some(title_case_name(&cleaned))
}

fn extract_assignees(
    working: &mut String,
    known_assignees_input: Option<&[String]>,
) -> Vec<String> {
    let known = known_assignees_input
        .map(known_assignee_set)
        .unwrap_or_default();
    let mut assignees = Vec::new();
    let mut seen = BTreeSet::new();

    let mention_regex = Regex::new(r"(?i)@[a-z0-9_.+-]+").expect("mention regex should compile");
    let mentions = mention_regex
        .find_iter(working.as_str())
        .filter(|matched| {
            if matched.start() == 0 {
                return true;
            }
            working[..matched.start()]
                .chars()
                .last()
                .map(|char| !char.is_ascii_alphanumeric() && !matches!(char, '_' | '.' | '+' | '-'))
                .unwrap_or(true)
        })
        .map(|matched| matched.as_str().to_string())
        .collect::<Vec<_>>();
    if !mentions.is_empty() {
        for mention in mentions {
            if seen.insert(mention.to_lowercase()) {
                assignees.push(mention);
            }
        }
        *working = normalize_whitespace(&mention_regex.replace_all(working, " "));
    }

    let contextual_with_lead =
        Regex::new(r"(?i)^\s*(?:meeting|meet(?:ing)?|sync|chat|coffee|lunch|dinner|breakfast|call|review)\s+with\b")
            .expect("contextual with regex should compile");
    loop {
        let mut changed = false;
        for (prefix, skip_contextual) in [("to ", false), ("for ", false), ("with ", true)] {
            if skip_contextual && contextual_with_lead.is_match(working) {
                continue;
            }
            if let Some((start, end, clause)) = extract_prefixed_clause(working, prefix) {
                let values = split_entity_list(&clause)
                    .into_iter()
                    .filter_map(|part| normalize_assignee_value(&part, &known))
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    continue;
                }
                for value in values {
                    if seen.insert(value.to_lowercase()) {
                        assignees.push(value);
                    }
                }
                remove_segment(working, start, end);
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }

    assignees
}

fn is_likely_project(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if Regex::new(r"(?i)^(?:with|to|by|before|every|starting|until|till|high|medium|low|urgent)$")
        .expect("project stopword regex should compile")
        .is_match(value)
    {
        return false;
    }
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.len() >= 2 {
        return true;
    }
    Regex::new(r"^(?:[A-Z][a-z]+[A-Z][A-Za-z]*|[A-Z]{2,}|[A-Z][a-z]+\d+)$")
        .expect("project token regex should compile")
        .is_match(value)
}

fn extract_prefixed_clause(working: &str, prefix: &str) -> Option<(usize, usize, String)> {
    let lower = working.to_lowercase();
    let prefix_lower = prefix.to_lowercase();
    let needle = format!(" {}", prefix_lower);

    let mut candidates = Vec::new();
    if lower.starts_with(&prefix_lower) {
        candidates.push(0usize);
    }
    candidates.extend(lower.match_indices(&needle).map(|(index, _)| index + 1));

    for start in candidates {
        let clause_start = start + prefix.len();
        let end = find_clause_end(working, clause_start);
        let clause = clean_named_value(&working[clause_start..end]);
        if !clause.is_empty() {
            return Some((start, end, clause));
        }
    }

    None
}

fn find_clause_end(working: &str, search_start: usize) -> usize {
    let boundaries = [
        r"(?i)\s+(?:with|to|every|daily|weekly|monthly|yearly)\b",
        r"(?i)\s+(?:but\s+(?:skip|not)\b|unless\b|except(?:\s+for)?\b)",
        r"(?i)\s+(?:starting|beginning|from)\b",
        r"(?i)\s+(?:until|till|through|ending(?:\s+on)?)\b",
        r"(?i)\s+(?:by|at|before)\b",
        r"(?i)\s+for\s+\d+\s*(?:minutes?|mins?|hours?|hrs?)\b",
        r"(?i)\s+(?:high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b)",
        r"(?i)\s*[,.;:!?]+(?:\s|$)",
    ];

    let mut end = working.len();
    for boundary in boundaries {
        let regex = Regex::new(boundary).expect("boundary regex should compile");
        if let Some(found) = regex.find(&working[search_start..]) {
            end = end.min(search_start + found.start());
        }
    }
    end
}

fn build_task_title(input: &str) -> String {
    finalize_task_title(input)
}

fn high_priority_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\burgent\b").unwrap(),
            Regex::new(r"(?i)\burgnt\b").unwrap(),
            Regex::new(r"(?i)\basap\b").unwrap(),
            Regex::new(r"(?i)\bcritical\b").unwrap(),
            Regex::new(r"(?i)\bcritcal\b").unwrap(),
            Regex::new(r"(?i)\bblocker\b").unwrap(),
            Regex::new(r"(?i)\bp1\b").unwrap(),
            Regex::new(r"(?i)\bimportant\b").unwrap(),
            Regex::new(r"(?i)\bimportnt\b").unwrap(),
            Regex::new(r"(?i)\bhigh\s+priority\b").unwrap(),
            Regex::new(r"(?i)\bhigh\s*priority\b").unwrap(),
            Regex::new(r"(?i)\bhighpri\w*\b").unwrap(),
            Regex::new(r"(?i)\bhigh\s+pri(?:o)?\b").unwrap(),
            Regex::new(r"(?i)!{3,}\s*urgent!*").unwrap(),
            Regex::new("🚨").unwrap(),
            Regex::new(r"\bNOW\b").unwrap(),
        ]
    })
}

fn medium_priority_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\bnormal\s+priority\b").unwrap(),
            Regex::new(r"(?i)\bmedium\s+prio\b").unwrap(),
            Regex::new(r"(?i)\bmedium\b").unwrap(),
            Regex::new(r"(?i)\bnormal\b").unwrap(),
        ]
    })
}

fn low_priority_patterns() -> &'static [Regex] {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    REGEXES.get_or_init(|| {
        vec![
            Regex::new(r"(?i)\blow\s+priority\b").unwrap(),
            Regex::new(r"(?i)\blow\s+pri(?:o)?\b").unwrap(),
            Regex::new(r"(?i)\bno\s+rush\b").unwrap(),
            Regex::new(r"(?i)\bnice\s+to\s+have\b").unwrap(),
            Regex::new(r"(?i)\bwhen\s+you\s+can\b").unwrap(),
            Regex::new(r"(?i)\bwhen\s+you\s+get\s+a\s+chance\b").unwrap(),
            Regex::new(r"(?i)\bwhenever(?:\s+you\s+can)?\b").unwrap(),
            Regex::new(r"(?i)\bthis\s+can\s+wait\b").unwrap(),
            Regex::new(r"(?i)\bnot\s+urgent\b").unwrap(),
        ]
    })
}
