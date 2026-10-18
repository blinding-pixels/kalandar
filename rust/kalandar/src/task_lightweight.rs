use super::{
    postprocess::{finalize_task_title, js_regex, normalize_whitespace, replace_all},
    schedule::context_for,
    CapturePreviewRecurrenceException, CapturePreviewRequest, CapturePreviewTaskDraft,
    CapturePreviewWarning, PreviewSchedule,
};
use chrono::{Datelike, NaiveDate};
use fancy_regex::{Captures, Regex};
use kalandar_dates::capture::{capture_schedule, CaptureKind, Context, ScheduleSpan};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub struct TaskParseOutput {
    pub draft: CapturePreviewTaskDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub confidence: f64,
    pub schedule: Option<PreviewSchedule>,
}

const ASCII_WORD_BOUNDARY: &str =
    "(?:(?<=[0-9A-Za-z_])(?![0-9A-Za-z_])|(?<![0-9A-Za-z_])(?=[0-9A-Za-z_]))";

const DEFAULT_KNOWN_ASSIGNEES: [&str; 17] = [
    "alex", "diego", "jake", "james", "jamie", "jen", "jester", "john", "lee", "mark", "mike",
    "maria", "nate", "priya", "rachel", "sam", "sarah",
];

const DURATION_TOKEN: &str = r"\d+\s*(?:minutes?|mins?|hours?|hrs?)";
const PERSON_TOKEN: &str = r"(?:@[a-z0-9_.+-]+|[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?(?:\s+[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?)?|[a-z]+)";
const PROJECT_CHAIN_RESERVED: &str = r"with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|on|in\s+progress|blocked|waiting|done|completed|not\s+started|todo";

const STATUS_PATTERNS: [(&str, &str); 8] = [
    (r"(?i)\bin\s+progress\b", "in progress"),
    (r"(?i)\bnot\s+started\b", "not started"),
    (r"(?i)\bto\s*do\b", "todo"),
    (r"(?i)\btodo\b", "todo"),
    (r"(?i)\bblocked\b", "blocked"),
    (r"(?i)\bwaiting\b", "waiting"),
    (r"(?i)\bdone\b", "done"),
    (r"(?i)\bcompleted?\b", "done"),
];

const HIGH_PRIORITY_PATTERNS: [&str; 17] = [
    r"(?i)\burgent\b",
    r"(?i)\burgnt\b",
    r"(?i)\basap\b",
    r"(?i)\bcritical\b",
    r"(?i)\bcritcal\b",
    r"(?i)\bblocker\b",
    r"(?i)\bp1\b",
    r"(?i)\bimportant\b",
    r"(?i)\bimportnt\b",
    r"(?i)\bhigh\s+priority\b",
    r"(?i)\bhigh\s*priority\b",
    r"(?i)\bhighpri\w*\b",
    r"(?i)\bhigh\s+pri(?:o)?\b",
    r"(?i)!{3,}\s*urgent!*",
    r"!{2,}",
    "\u{1F6A8}",
    r"\bNOW\b",
];

const MEDIUM_PRIORITY_PATTERNS: [&str; 4] = [
    r"(?i)\bnormal\s+priority\b",
    r"(?i)\bmedium\s+prio\b",
    r"(?i)\bmedium\b",
    r"(?i)\bnormal\b",
];

const LOW_PRIORITY_PATTERNS: [&str; 9] = [
    r"(?i)\blow\s+priority\b",
    r"(?i)\blow\s+pri(?:o)?\b",
    r"(?i)\bno\s+rush\b",
    r"(?i)\bnice\s+to\s+have\b",
    r"(?i)\bwhen\s+you\s+can\b",
    r"(?i)\bwhen\s+you\s+get\s+a\s+chance\b",
    r"(?i)\bwhenever(?:\s+you\s+can)?\b",
    r"(?i)\bthis\s+can\s+wait\b",
    r"(?i)\bnot\s+urgent\b",
];

const WEEKDAY_TOKEN: &str = r"(monday|tuesday|wednesday|thursday|friday|saturday|sunday|mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)";
const MONTH_TOKEN: &str = r"(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sep|sept|october|oct|november|nov|december|dec)";

type ValueFn<'a, T> = &'a dyn Fn(&Captures) -> Option<T>;

struct RegexExtraction<T> {
    value: T,
    working: String,
}

struct StructuredRecurrenceException {
    raw: String,
    condition: Option<&'static str>,
    action: Option<&'static str>,
}

#[derive(Default)]
struct TaskFields {
    title: String,
    assignee: Vec<String>,
    project: Option<String>,
    tags: Vec<String>,
    priority: Option<&'static str>,
    status: Option<&'static str>,
    duration: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    recurrence: Option<String>,
    recurrence_start: Option<String>,
    recurrence_end: Option<String>,
    recurrence_exception: Vec<StructuredRecurrenceException>,
}

struct TaskParseContext<'a> {
    working: String,
    known_assignees: Option<&'a [String]>,
    context: Context,
    fields: TaskFields,
    title_confidence: f64,
    warnings: Vec<CapturePreviewWarning>,
    schedule: Option<PreviewSchedule>,
}

pub(crate) fn ascii_regex(pattern: &str) -> Regex {
    let mut converted = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            converted.push(current);
            continue;
        }
        match chars.next() {
            Some('b') => converted.push_str(ASCII_WORD_BOUNDARY),
            Some('w') => converted.push_str("[0-9A-Za-z_]"),
            Some('d') => converted.push_str("[0-9]"),
            Some(other) => {
                converted.push('\\');
                converted.push(other);
            }
            None => converted.push('\\'),
        }
    }
    js_regex(&converted)
}

fn compile_all(patterns: &[&str]) -> Vec<Regex> {
    patterns.iter().map(|pattern| ascii_regex(pattern)).collect()
}

fn temporal_boundary() -> String {
    format!(
        r"(?=\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b|{DURATION_TOKEN}\b)|\s*[,.;]|$)"
    )
}

fn person_list() -> String {
    format!(r"{PERSON_TOKEN}(?:\s*(?:,|and|&)\s*{PERSON_TOKEN})*")
}

fn is_match(regex: &Regex, text: &str) -> bool {
    regex.is_match(text).unwrap_or(false)
}

fn group(captures: &Captures, index: usize) -> String {
    captures
        .get(index)
        .map(|matched| matched.as_str().to_string())
        .unwrap_or_default()
}

fn whitespace_tokens(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

pub fn parse_task_preview(text: &str, request: &CapturePreviewRequest) -> TaskParseOutput {
    let mut ctx = create_task_parse_context(text, request);
    priority_pass(&mut ctx);
    status_pass(&mut ctx);
    schedule_pass(&mut ctx);
    tags_pass(&mut ctx);
    project_pass(&mut ctx);
    assignee_pass(&mut ctx);
    title_pass(&mut ctx);
    finalize_task_parse_context(ctx)
}

fn create_task_parse_context<'a>(
    input: &str,
    request: &'a CapturePreviewRequest,
) -> TaskParseContext<'a> {
    TaskParseContext {
        working: normalize_whitespace(input),
        known_assignees: request.known_assignees.as_deref(),
        context: context_for(request),
        fields: TaskFields::default(),
        title_confidence: 0.0,
        warnings: Vec::new(),
        schedule: None,
    }
}

fn priority_pass(ctx: &mut TaskParseContext) {
    let (priority, working) = detect_priority(&ctx.working);
    ctx.working = working;
    if priority.is_some() {
        ctx.fields.priority = priority;
    }
}

fn status_pass(ctx: &mut TaskParseContext) {
    let (status, working) = detect_status(&ctx.working);
    ctx.working = working;
    if status.is_some() {
        ctx.fields.status = status;
    }
}

fn schedule_pass(ctx: &mut TaskParseContext) {
    let found = capture_schedule(&ctx.working, CaptureKind::Task, None, &ctx.context);
    let texts = found.texts;
    ctx.fields.due_date = texts.date;
    ctx.fields.due_time = texts.time;
    ctx.fields.duration = texts.duration;
    ctx.fields.recurrence = texts.recurrence;
    ctx.fields.recurrence_start = texts.recurrence_start;
    ctx.fields.recurrence_end = texts.recurrence_end;
    for raw in &texts.recurrence_exceptions {
        merge_structured_recurrence_exceptions(
            &mut ctx.fields.recurrence_exception,
            raw,
            &ctx.context,
        );
    }
    ctx.schedule = found.schedule;
    ctx.warnings
        .extend(found.warnings.iter().map(|warning| CapturePreviewWarning {
            code: warning.code.to_string(),
            severity: "info".to_string(),
            message: warning.message.clone(),
        }));
    ctx.working = remove_schedule_spans(&ctx.working, &found.spans);
}

fn tags_pass(ctx: &mut TaskParseContext) {
    let (tags, working) = extract_tags(&ctx.working);
    ctx.working = working;
    if !tags.is_empty() {
        ctx.fields.tags = tags;
    }
}

fn project_pass(ctx: &mut TaskParseContext) {
    let (project, working) = extract_project(&ctx.working);
    ctx.working = working;
    if project.is_some() {
        ctx.fields.project = project;
    }
}

fn assignee_pass(ctx: &mut TaskParseContext) {
    let (assignees, working) = extract_assignees(&ctx.working, ctx.known_assignees);
    ctx.working = working;
    if !assignees.is_empty() {
        ctx.fields.assignee = assignees;
    }
}

fn title_pass(ctx: &mut TaskParseContext) {
    let (title, confidence) = build_task_title(&ctx.working);
    ctx.fields.title = title;
    ctx.title_confidence = confidence;
}

fn remove_schedule_spans(text: &str, spans: &[ScheduleSpan]) -> String {
    let mut ordered = spans.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|span| (span.start, span.end));
    let mut output = String::new();
    let mut cursor = 0;
    for span in ordered {
        if span.start >= cursor {
            output.push_str(text.get(cursor..span.start).unwrap_or_default());
            output.push(' ');
        }
        cursor = cursor.max(span.end);
    }
    output.push_str(text.get(cursor..).unwrap_or_default());
    normalize_whitespace(&output)
}

fn build_task_warnings(ctx: &TaskParseContext) -> Vec<CapturePreviewWarning> {
    let mut warnings = Vec::new();
    if let Some(known) = ctx.known_assignees.filter(|known| !known.is_empty()) {
        let known_set = get_known_assignee_set(Some(known));
        let unknown_assignees = ctx
            .fields
            .assignee
            .iter()
            .filter(|hint| {
                !hint.starts_with('@')
                    && !hint.contains('@')
                    && !known_set.contains(&hint.trim().to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>();
        if !unknown_assignees.is_empty() {
            warnings.push(CapturePreviewWarning {
                code: "unknown_assignee".to_string(),
                severity: "warning".to_string(),
                message: format!("Unknown assignee: {}", unknown_assignees.join(", ")),
            });
        }
    }
    let is_empty = |value: &Option<String>| value.as_deref().is_none_or(str::is_empty);
    if is_empty(&ctx.fields.due_date)
        && is_empty(&ctx.fields.due_time)
        && is_empty(&ctx.fields.recurrence)
    {
        warnings.push(CapturePreviewWarning {
            code: "no_due_date".to_string(),
            severity: "info".to_string(),
            message: "No due date or recurrence extracted.".to_string(),
        });
    }
    warnings
}

fn finalize_task_parse_context(ctx: TaskParseContext) -> TaskParseOutput {
    let task_warnings = build_task_warnings(&ctx);
    let mut warnings = ctx.warnings;
    warnings.extend(task_warnings);
    let recurrence_exceptions =
        to_recurrence_exceptions(&ctx.fields.recurrence_exception, ctx.schedule.as_ref());
    let fields = ctx.fields;
    TaskParseOutput {
        draft: CapturePreviewTaskDraft {
            title: fields.title,
            assignees: fields.assignee,
            project_hint: fields.project,
            tags: fields.tags,
            priority: fields.priority.map(str::to_string),
            status: fields.status.map(str::to_string),
            due_date_text: fields.due_date,
            due_time_text: fields.due_time,
            duration_text: fields.duration,
            recurrence_text: fields.recurrence,
            recurrence_start_text: fields.recurrence_start,
            recurrence_end_text: fields.recurrence_end,
            recurrence_exceptions,
        },
        warnings,
        confidence: ctx.title_confidence,
        schedule: ctx.schedule,
    }
}

fn to_recurrence_exceptions(
    entries: &[StructuredRecurrenceException],
    schedule: Option<&PreviewSchedule>,
) -> Vec<CapturePreviewRecurrenceException> {
    let resolved = schedule
        .and_then(|schedule| schedule.recurrence.as_ref())
        .map(|recurrence| recurrence.exceptions.as_slice())
        .unwrap_or_default();
    let same_count = resolved.len() == entries.len();
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            if resolved.is_empty() {
                return CapturePreviewRecurrenceException {
                    raw: entry.raw.clone(),
                    condition: entry.condition.map(str::to_string),
                    action: entry.action.map(str::to_string),
                };
            }
            let matched = if same_count {
                resolved.get(index)
            } else {
                resolved.iter().find(|exception| {
                    exception.raw.contains(entry.raw.as_str())
                        || entry.raw.contains(exception.raw.as_str())
                })
            };
            CapturePreviewRecurrenceException {
                raw: entry.raw.clone(),
                condition: matched.and_then(|exception| exception.condition.clone()),
                action: matched.and_then(|exception| exception.action.clone()),
            }
        })
        .collect()
}

fn remove_segment(working: &str, start: usize, end: usize) -> String {
    normalize_whitespace(&format!("{} {}", &working[..start], &working[end..]))
}

fn find_best_regex_extraction<T>(
    working: &str,
    definitions: &[(&Regex, ValueFn<'_, T>)],
) -> Option<RegexExtraction<T>> {
    let mut best: Option<(usize, usize, T)> = None;
    for (regex, value) in definitions {
        let Some(captures) = regex.captures(working).ok().flatten() else {
            continue;
        };
        let Some(matched) = captures.get(0) else {
            continue;
        };
        let Some(value) = value(&captures) else {
            continue;
        };
        let better = best.as_ref().is_none_or(|(start, end, _)| {
            matched.start() < *start
                || (matched.start() == *start && matched.end() - matched.start() > end - start)
        });
        if better {
            best = Some((matched.start(), matched.end(), value));
        }
    }
    best.map(|(start, end, value)| RegexExtraction {
        value,
        working: remove_segment(working, start, end),
    })
}

fn detect_priority(input: &str) -> (Option<&'static str>, String) {
    static NEGATED: OnceLock<Regex> = OnceLock::new();
    static HIGH: OnceLock<Vec<Regex>> = OnceLock::new();
    static MEDIUM: OnceLock<Vec<Regex>> = OnceLock::new();
    static LOW: OnceLock<Vec<Regex>> = OnceLock::new();
    let negated = NEGATED
        .get_or_init(|| ascii_regex(r"(?i)\b(?:not|non)[\s-]*(?:urgent|important|critical)\b"));
    let high = HIGH.get_or_init(|| compile_all(&HIGH_PRIORITY_PATTERNS));
    let medium = MEDIUM.get_or_init(|| compile_all(&MEDIUM_PRIORITY_PATTERNS));
    let low = LOW.get_or_init(|| compile_all(&LOW_PRIORITY_PATTERNS));

    let mut working = input.to_string();
    let mut priority = None;

    let apply_patterns = |working: &mut String,
                          priority: &mut Option<&'static str>,
                          patterns: &[Regex],
                          next_priority: &'static str| {
        let mut matched = false;
        for pattern in patterns {
            if is_match(pattern, working) {
                matched = true;
            }
            *working = replace_all(pattern, working, " ");
        }
        if matched && priority.is_none() {
            *priority = Some(next_priority);
        }
    };
    let strip_patterns = |working: &mut String, patterns: &[Regex]| {
        for pattern in patterns {
            *working = replace_all(pattern, working, " ");
        }
    };

    if is_match(negated, &working) {
        working = replace_all(negated, &working, " ");
        priority = Some("low");
    }
    apply_patterns(&mut working, &mut priority, high, "high");
    if priority.is_none() {
        apply_patterns(&mut working, &mut priority, medium, "medium");
    } else {
        strip_patterns(&mut working, medium);
    }
    if priority.is_none() {
        apply_patterns(&mut working, &mut priority, low, "low");
    } else {
        strip_patterns(&mut working, low);
    }

    (priority, normalize_whitespace(&working))
}

fn detect_status(input: &str) -> (Option<&'static str>, String) {
    static REGEXES: OnceLock<Vec<Regex>> = OnceLock::new();
    let regexes = REGEXES.get_or_init(|| {
        STATUS_PATTERNS
            .iter()
            .map(|(pattern, _)| ascii_regex(pattern))
            .collect()
    });
    let values = STATUS_PATTERNS
        .iter()
        .map(|(_, value)| move |_: &Captures| Some(*value))
        .collect::<Vec<_>>();
    let definitions = regexes
        .iter()
        .zip(values.iter())
        .map(|(regex, value)| (regex, value as ValueFn<'_, &'static str>))
        .collect::<Vec<_>>();
    match find_best_regex_extraction(input, &definitions) {
        Some(extraction) => (Some(extraction.value), extraction.working),
        None => (None, input.to_string()),
    }
}

fn cleanup_tag_token(value: &str) -> Option<String> {
    static EDGES: OnceLock<Regex> = OnceLock::new();
    let edges = EDGES.get_or_init(|| ascii_regex(r"^[,;:]+|[,;:!?]+$"));
    let trimmed = replace_all(edges, value.trim(), "");
    if trimmed.is_empty() {
        return None;
    }
    let keep = |char: &char| char.is_ascii_alphanumeric() || *char == '_' || *char == '-';
    if let Some(rest) = trimmed.strip_prefix('#') {
        let normalized = format!("#{}", rest.chars().filter(keep).collect::<String>());
        return (normalized.len() > 1).then_some(normalized);
    }
    let normalized = trimmed.chars().filter(keep).collect::<String>();
    (!normalized.is_empty()).then_some(normalized)
}

fn hashtag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| ascii_regex(r"#[A-Za-z0-9][A-Za-z0-9_-]*"))
}

fn parse_tag_list(value: &str) -> Vec<String> {
    static SPLIT: OnceLock<Regex> = OnceLock::new();
    let hashtag_matches = hashtag_regex()
        .find_iter(value)
        .filter_map(Result::ok)
        .map(|matched| matched.as_str())
        .collect::<Vec<_>>();
    let raw_parts = if hashtag_matches.is_empty() {
        SPLIT
            .get_or_init(|| ascii_regex(r"(?i)\s*(?:,|and)\s*"))
            .split(value)
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
    } else {
        hashtag_matches
    };
    let mut tags = Vec::new();
    let mut seen = BTreeSet::new();
    for part in raw_parts {
        let Some(cleaned) = cleanup_tag_token(part) else {
            continue;
        };
        if seen.insert(cleaned.to_lowercase()) {
            tags.push(cleaned);
        }
    }
    tags
}

fn extract_tags(input: &str) -> (Vec<String>, String) {
    static CLAUSE: OnceLock<Regex> = OnceLock::new();
    static TAG_WORD: OnceLock<Regex> = OnceLock::new();
    let mut working = input.to_string();
    let mut tags = Vec::new();
    let mut seen = BTreeSet::new();
    let mut append_tags = |values: Vec<String>, tags: &mut Vec<String>| {
        for value in values {
            if seen.insert(value.to_lowercase()) {
                tags.push(value);
            }
        }
    };

    let clause = CLAUSE.get_or_init(|| {
        ascii_regex(r"(?i)\b(?:with\s+)?(?:tags?\b\s*:?\s*|tagged\s+)([^.;]+)$")
    });
    let clause_value = |captures: &Captures| Some(group(captures, 1));
    if let Some(extraction) = find_best_regex_extraction(&working, &[(clause, &clause_value)]) {
        append_tags(parse_tag_list(&extraction.value), &mut tags);
        working = extraction.working;
    }

    let hashtag = hashtag_regex();
    let hashtag_spans = hashtag
        .find_iter(&working)
        .filter_map(Result::ok)
        .map(|matched| matched.as_str().to_string())
        .collect::<Vec<_>>();
    if !hashtag_spans.is_empty() {
        append_tags(
            hashtag_spans
                .iter()
                .filter_map(|span| cleanup_tag_token(span))
                .collect(),
            &mut tags,
        );
        let tag_word = TAG_WORD.get_or_init(|| ascii_regex(r"(?i)\b(?:with\s+)?tags?\b[:]?"));
        working = normalize_whitespace(&replace_all(
            tag_word,
            &replace_all(hashtag, &working, " "),
            " ",
        ));
    }

    (tags, working)
}

fn title_case_name(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            if part.contains('@') {
                return part.to_string();
            }
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_named_value(value: &str) -> String {
    static EDGES: OnceLock<Regex> = OnceLock::new();
    let edges = EDGES.get_or_init(|| ascii_regex(r"^[,;:]+|[,;:!?]+$"));
    normalize_whitespace(&replace_all(edges, value, ""))
}

fn split_entity_list(value: &str) -> Vec<String> {
    static SPLIT: OnceLock<Regex> = OnceLock::new();
    SPLIT
        .get_or_init(|| ascii_regex(r"(?i)\s*(?:,|and|&)\s*"))
        .split(value)
        .filter_map(Result::ok)
        .map(clean_named_value)
        .filter(|part| !part.is_empty())
        .collect()
}

fn is_weekday_or_month(token: &str) -> bool {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    is_match(
        REGEX.get_or_init(|| {
            ascii_regex(r"(?i)^(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?|january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)$")
        }),
        token,
    )
}

fn is_likely_person(value: &str, known_assignees: &BTreeSet<String>) -> bool {
    static REGEXES: OnceLock<[Regex; 4]> = OnceLock::new();
    let [mention, email, reserved, name_token] = REGEXES.get_or_init(|| {
        [
            ascii_regex(r"(?i)^@[a-z0-9_.+-]+$"),
            ascii_regex(r"(?i)^[a-z0-9.+-]+@[a-z0-9.-]+\.[a-z]{2,}$"),
            ascii_regex(
                r"(?i)^(?:project|tag|tags|today|tomorrow|tonight|next|this|every|daily|weekly|monthly)$",
            ),
            ascii_regex(r"^[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?$"),
        ]
    });
    let cleaned = clean_named_value(value);
    if cleaned.is_empty() {
        return false;
    }
    if is_match(mention, &cleaned) || is_match(email, &cleaned) {
        return true;
    }
    if known_assignees.contains(&cleaned.to_lowercase()) {
        return true;
    }
    if is_match(reserved, &cleaned) {
        return false;
    }
    let tokens = whitespace_tokens(&cleaned);
    if tokens.iter().any(|token| is_weekday_or_month(token)) {
        return false;
    }
    if cleaned.chars().any(|char| char.is_ascii_digit()) {
        return false;
    }
    if tokens.len() > 2 {
        return false;
    }
    tokens.iter().all(|token| is_match(name_token, token))
}

fn normalize_assignee_value(value: &str, known_assignees: &BTreeSet<String>) -> Option<String> {
    let cleaned = clean_named_value(value);
    if !is_likely_person(&cleaned, known_assignees) {
        return None;
    }
    if cleaned.contains('@') {
        return Some(cleaned);
    }
    Some(title_case_name(&cleaned))
}

fn is_likely_project(value: &str) -> bool {
    static REGEXES: OnceLock<[Regex; 2]> = OnceLock::new();
    let [reserved, single] = REGEXES.get_or_init(|| {
        [
            ascii_regex(
                r"(?i)^(?:with|to|by|before|every|starting|until|till|high|medium|low|urgent)$",
            ),
            ascii_regex(r"^(?:[A-Z][a-z]+[A-Z][A-Za-z]*|[A-Z]{2,}|[A-Z][a-z]+\d+|[A-Z][a-z]+)$"),
        ]
    });
    let cleaned = clean_named_value(value);
    if cleaned.is_empty() || is_match(reserved, &cleaned) {
        return false;
    }
    let tokens = whitespace_tokens(&cleaned);
    if tokens.iter().any(|token| is_weekday_or_month(token)) {
        return false;
    }
    if tokens.len() >= 2 {
        return true;
    }
    is_match(single, &cleaned)
}

fn extract_project(input: &str) -> (Option<String>, String) {
    static REGEXES: OnceLock<[Regex; 3]> = OnceLock::new();
    let [explicit, for_project, titlecase] = REGEXES.get_or_init(|| {
        let boundary = temporal_boundary();
        [
            ascii_regex(&format!(r"(?i)\b(?:in|under)\s+project\s+(.+?){boundary}")),
            ascii_regex(&format!(r"(?i)\bfor\s+project\s+(.+?){boundary}")),
            ascii_regex(&format!(
                r"\b[Ff][Oo][Rr]\s+([A-Z][a-z0-9-]*(?:\s+(?!(?:{PROJECT_CHAIN_RESERVED})\b)[A-Za-z][A-Za-z0-9-]*){{0,3}}){boundary}"
            )),
        ]
    });
    let keyword_value = |captures: &Captures| {
        let candidate = clean_named_value(&group(captures, 1));
        (!candidate.is_empty()).then(|| format!("Project {candidate}"))
    };
    let titlecase_value = |captures: &Captures| {
        let candidate = clean_named_value(&group(captures, 1));
        is_likely_project(&candidate).then_some(candidate)
    };
    match find_best_regex_extraction(
        input,
        &[
            (explicit, &keyword_value),
            (for_project, &keyword_value),
            (titlecase, &titlecase_value),
        ],
    ) {
        Some(extraction) => (Some(extraction.value), extraction.working),
        None => (None, input.to_string()),
    }
}

fn get_known_assignee_set(values: Option<&[String]>) -> BTreeSet<String> {
    let seed = match values {
        Some(values) => values.iter().map(String::as_str).collect::<Vec<_>>(),
        None => DEFAULT_KNOWN_ASSIGNEES.to_vec(),
    };
    seed.into_iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn extract_assignees(
    input: &str,
    known_assignees_input: Option<&[String]>,
) -> (Vec<String>, String) {
    static MENTION: OnceLock<Regex> = OnceLock::new();
    static CONTEXTUAL_WITH_LEAD: OnceLock<Regex> = OnceLock::new();
    static CLAUSES: OnceLock<[Regex; 3]> = OnceLock::new();
    let mut working = input.to_string();
    let known_assignees = get_known_assignee_set(known_assignees_input);
    let mut assignees = Vec::new();
    let mut seen = BTreeSet::new();
    let mut add_assignee = |value: String, assignees: &mut Vec<String>| {
        if seen.insert(value.to_lowercase()) {
            assignees.push(value);
        }
    };

    let mention = MENTION.get_or_init(|| ascii_regex(r"(?<![A-Za-z0-9_.+-])@[A-Za-z0-9_.+-]+"));
    let mentions = mention
        .find_iter(&working)
        .filter_map(Result::ok)
        .map(|matched| matched.as_str().to_string())
        .collect::<Vec<_>>();
    if !mentions.is_empty() {
        for value in mentions {
            add_assignee(value, &mut assignees);
        }
        working = normalize_whitespace(&replace_all(mention, &working, " "));
    }

    let contextual_with_lead = CONTEXTUAL_WITH_LEAD.get_or_init(|| {
        ascii_regex(r"(?i)^\s*(?:meeting|meet(?:ing)?|sync|chat|coffee|lunch|dinner|breakfast|call|review)\s+with\b")
    });
    let [to_clause, for_clause, with_clause] = CLAUSES.get_or_init(|| {
        let boundary = temporal_boundary();
        let people = person_list();
        [
            ascii_regex(&format!(r"(?i)\bto\s+({people}){boundary}")),
            ascii_regex(&format!(r"(?i)\bfor\s+({people}){boundary}")),
            ascii_regex(&format!(r"(?i)\bwith\s+({people}){boundary}")),
        ]
    });
    let normalize_list = |captures: &Captures| {
        split_entity_list(&group(captures, 1))
            .iter()
            .filter_map(|part| normalize_assignee_value(part, &known_assignees))
            .collect::<Vec<_>>()
    };
    let to_value = |captures: &Captures| Some(normalize_list(captures));
    let for_value = |captures: &Captures| {
        let values = normalize_list(captures);
        (!values.is_empty()).then_some(values)
    };
    let with_value = |captures: &Captures| {
        if is_match(contextual_with_lead, input) {
            return None;
        }
        let values = normalize_list(captures);
        (!values.is_empty()).then_some(values)
    };
    let definitions: [(&Regex, ValueFn<'_, Vec<String>>); 3] = [
        (to_clause, &to_value),
        (for_clause, &for_value),
        (with_clause, &with_value),
    ];

    while let Some(extraction) = find_best_regex_extraction(&working, &definitions) {
        if extraction.value.is_empty() {
            break;
        }
        for value in extraction.value {
            add_assignee(value, &mut assignees);
        }
        working = extraction.working;
    }

    (assignees, working)
}

fn merge_structured_recurrence_exceptions(
    current: &mut Vec<StructuredRecurrenceException>,
    raw: &str,
    context: &Context,
) {
    if raw.is_empty() || current.iter().any(|entry| entry.raw == raw) {
        return;
    }
    current.push(classify_recurrence_exception(raw, context));
}

fn is_valid_calendar_date(year: i64, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || (0..=99).contains(&year) {
        return false;
    }
    i32::try_from(year)
        .ok()
        .and_then(|year| NaiveDate::from_ymd_opt(year, month, day))
        .is_some()
}

fn normalize_iso_date_year(
    month: u32,
    day: u32,
    explicit_year: Option<i64>,
    context: &Context,
) -> Option<NaiveDate> {
    let reference = context.now.with_timezone(&context.zone).date_naive();
    let mut year = explicit_year.unwrap_or_else(|| i64::from(reference.year()));
    if !is_valid_calendar_date(year, month, day) {
        return None;
    }
    let candidate = NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)?;
    if explicit_year.is_none() && candidate < reference && is_valid_calendar_date(year + 1, month, day)
    {
        year += 1;
    }
    NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)
}

fn month_number(token: &str) -> Option<u32> {
    let months = [
        ("jan", 1),
        ("january", 1),
        ("feb", 2),
        ("february", 2),
        ("mar", 3),
        ("march", 3),
        ("apr", 4),
        ("april", 4),
        ("may", 5),
        ("jun", 6),
        ("june", 6),
        ("jul", 7),
        ("july", 7),
        ("aug", 8),
        ("august", 8),
        ("sep", 9),
        ("sept", 9),
        ("september", 9),
        ("oct", 10),
        ("october", 10),
        ("nov", 11),
        ("november", 11),
        ("dec", 12),
        ("december", 12),
    ];
    months
        .iter()
        .find(|(name, _)| *name == token)
        .map(|(_, month)| *month)
}

fn extract_specific_date(raw: &str, context: &Context) -> Option<NaiveDate> {
    static REGEXES: OnceLock<[Regex; 3]> = OnceLock::new();
    let [iso, slash, month_name] = REGEXES.get_or_init(|| {
        [
            ascii_regex(r"\b(\d{4})-(\d{2})-(\d{2})\b"),
            ascii_regex(r"\b(\d{1,2})[/-](\d{1,2})(?:[/-](\d{2,4}))?\b"),
            ascii_regex(&format!(
                r"(?i)\b{MONTH_TOKEN}\s+(\d{{1,2}})(?:st|nd|rd|th)?(?:,?\s+(\d{{4}}))?\b"
            )),
        ]
    });
    let number = |captures: &Captures, index: usize| group(captures, index).parse::<i64>().ok();

    if let Some(captures) = iso.captures(raw).ok().flatten() {
        let year = number(&captures, 1)?;
        let month = u32::try_from(number(&captures, 2)?).ok()?;
        let day = u32::try_from(number(&captures, 3)?).ok()?;
        return is_valid_calendar_date(year, month, day)
            .then(|| NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day))
            .flatten();
    }

    if let Some(captures) = slash.captures(raw).ok().flatten() {
        let month = u32::try_from(number(&captures, 1)?).ok()?;
        let day = u32::try_from(number(&captures, 2)?).ok()?;
        let year_text = group(&captures, 3);
        let explicit_year = if year_text.is_empty() {
            None
        } else if year_text.len() == 2 {
            Some(2000 + year_text.parse::<i64>().ok()?)
        } else {
            Some(year_text.parse::<i64>().ok()?)
        };
        return normalize_iso_date_year(month, day, explicit_year, context);
    }

    let captures = month_name.captures(raw).ok().flatten()?;
    let month = month_number(&group(&captures, 1).to_lowercase())?;
    let day = u32::try_from(number(&captures, 2)?).ok()?;
    let year_text = group(&captures, 3);
    let explicit_year = if year_text.is_empty() {
        None
    } else {
        Some(year_text.parse::<i64>().ok()?)
    };
    normalize_iso_date_year(month, day, explicit_year, context)
}

fn classify_recurrence_exception_condition(raw: &str, context: &Context) -> Option<&'static str> {
    static REGEXES: OnceLock<[Regex; 2]> = OnceLock::new();
    static ORDINALS: OnceLock<Vec<Regex>> = OnceLock::new();
    let [holiday, weekend] = REGEXES.get_or_init(|| {
        [
            ascii_regex(r"(?i)\bholidays?\b"),
            ascii_regex(r"(?i)\bweekends?\b"),
        ]
    });
    if is_match(holiday, raw) {
        return Some("holiday");
    }
    if is_match(weekend, raw) {
        return Some("weekend");
    }
    let ordinals = ORDINALS.get_or_init(|| {
        let mut patterns = compile_all(&[
            r"(?i)\bskip\s+the\s+(first|second|third|last)(?:\s+(?:one|occurrence|instance))?\b",
            r"(?i)\bbut\s+not\s+the\s+(first|second|third|last)\b",
            r"(?i)\bnot\s+the\s+(first|second|third|last)\b",
            r"(?i)\bthe\s+(first|second|third|last)\s+(?:one|occurrence|instance)\b",
        ]);
        patterns.push(ascii_regex(&format!(
            r"(?i)\bthe\s+(first|second|third|last)\s+{WEEKDAY_TOKEN}\s+of\s+(?:the\s+|every\s+)?month\b"
        )));
        patterns
    });
    if ordinals.iter().any(|pattern| is_match(pattern, raw)) {
        return Some("ordinal");
    }
    extract_specific_date(raw, context).map(|_| "specific_date")
}

fn classify_recurrence_exception_action(raw: &str) -> Option<&'static str> {
    static REGEXES: OnceLock<[Regex; 4]> = OnceLock::new();
    let [shift, replace, skip, but_not] = REGEXES.get_or_init(|| {
        [
            ascii_regex(&format!(
                r"(?i)\b(?:then\s+)?(?:move|shift)?(?:\s+to)?\s*(?:the\s+)?(following|next|preceding|previous|prior)\s+{WEEKDAY_TOKEN}\b"
            )),
            ascii_regex(r"(?i)\b(?:replace|replaced)\s+with\s+(.+?)$"),
            ascii_regex(r"(?i)\bskip\b"),
            ascii_regex(r"(?i)\bbut\s+not\b"),
        ]
    });
    if is_match(shift, raw) {
        return Some("shift");
    }
    if let Some(captures) = replace.captures(raw).ok().flatten() {
        let replacement = group(&captures, 1);
        if !replacement
            .trim()
            .trim_end_matches(['.', ',', ';'])
            .is_empty()
        {
            return Some("replace");
        }
    }
    if is_match(skip, raw) || is_match(but_not, raw) {
        return Some("skip");
    }
    None
}

fn classify_recurrence_exception(raw: &str, context: &Context) -> StructuredRecurrenceException {
    static LEAD: OnceLock<Regex> = OnceLock::new();
    let condition = classify_recurrence_exception_condition(raw, context);
    let mut action = classify_recurrence_exception_action(raw);
    let lead = LEAD.get_or_init(|| ascii_regex(r"(?i)^(?:unless|except)\b"));
    if action.is_none() && condition.is_some() && is_match(lead, raw.trim()) {
        action = Some("skip");
    }
    StructuredRecurrenceException {
        raw: raw.to_string(),
        condition,
        action,
    }
}

fn build_task_title(input: &str) -> (String, f64) {
    let cleaned = finalize_task_title(input);
    if cleaned.is_empty() {
        return ("Task".to_string(), 0.3);
    }
    let token_count = whitespace_tokens(&cleaned).len();
    let confidence = match token_count {
        5.. => 0.98,
        3..=4 => 0.9,
        2 => 0.75,
        _ => 0.55,
    };
    (cleaned, confidence)
}
