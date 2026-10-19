use super::intent::{classify_intent_embedded, IntentKind};
use super::{
    js::{self, normalize_whitespace, replace_all},
    postprocess::finalize_task_title,
    recurrence_exceptions::{
        classify_recurrence_exception, to_recurrence_exceptions, StructuredRecurrenceException,
    },
    schedule::{context_for, ScheduleOutput},
    CapturePreviewRequest, CapturePreviewTaskDraft, CapturePreviewWarning, PreviewSchedule,
};
use crate::capture_spans::CaptureSpanSource;
use fancy_regex::{Captures, Regex};
use kalandar_dates::capture::{capture_schedule, CaptureKind, Context};
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub struct TaskParseOutput {
    pub draft: CapturePreviewTaskDraft,
    pub warnings: Vec<CapturePreviewWarning>,
    pub schedule: Option<PreviewSchedule>,
}

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

type ValueFn<'a, T> = &'a dyn Fn(&Captures) -> Option<T>;

struct RegexExtraction<T> {
    value: T,
    working: String,
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
    raw_input: String,
    working: String,
    known_assignees: Option<&'a [String]>,
    context: Context,
    fields: TaskFields,
    warnings: Vec<CapturePreviewWarning>,
    schedule: Option<PreviewSchedule>,
}

fn compile_all(patterns: &[&str]) -> Vec<Regex> {
    patterns.iter().map(|pattern| js::regex(pattern)).collect()
}

fn temporal_boundary() -> String {
    format!(
        r"(?=\s+(?:with|to|every|daily|weekly|monthly|yearly|starting|beginning|from|until|till|through|ending|except|unless|by|before|at|high\b|medium\b|low\b|urgent\b|in\s+progress\b|blocked\b|waiting\b|done\b|completed?\b|not\s+started\b|todo\b|tags?\b|{DURATION_TOKEN}\b)|\s*[,.;]|$)"
    )
}

fn person_list() -> String {
    format!(r"{PERSON_TOKEN}(?:\s*(?:,|and|&)\s*{PERSON_TOKEN})*")
}

pub fn parse_task_preview(
    text: &str,
    request: &CapturePreviewRequest,
    spans: &dyn CaptureSpanSource,
) -> TaskParseOutput {
    let mut ctx = create_task_parse_context(text, request);
    priority_pass(&mut ctx);
    status_pass(&mut ctx);
    schedule_pass(&mut ctx, spans);
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
        raw_input: normalize_whitespace(input),
        working: normalize_whitespace(input),
        known_assignees: request.known_assignees.as_deref(),
        context: context_for(request),
        fields: TaskFields::default(),
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

fn schedule_pass(ctx: &mut TaskParseContext, spans: &dyn CaptureSpanSource) {
    let labelled = spans.schedule_spans(CaptureKind::Task, &ctx.working);
    let found = capture_schedule(
        &ctx.working,
        CaptureKind::Task,
        labelled.as_deref(),
        &ctx.context,
    );
    let ScheduleOutput {
        remaining,
        texts,
        schedule,
        warnings,
    } = ScheduleOutput::new(found, &ctx.working);
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
    ctx.schedule = schedule;
    ctx.warnings.extend(warnings);
    ctx.working = remaining;
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
    ctx.fields.title = build_task_title(&ctx.working);
}

fn build_task_warnings(ctx: &TaskParseContext) -> Vec<CapturePreviewWarning> {
    let mut warnings = Vec::new();
    let route = classify_intent_embedded(&ctx.raw_input);
    let mismatch = match route.intent {
        IntentKind::Calendar => Some((
            "intent_mismatch_calendar",
            "warning",
            "Input looks more like a calendar item than a task.",
        )),
        IntentKind::Reminder => Some((
            "intent_mismatch_reminder",
            "warning",
            "Input looks more like a reminder than a task.",
        )),
        IntentKind::None if !route.ambiguous => Some((
            "intent_mismatch_none",
            "info",
            "Input did not route cleanly to task or calendar.",
        )),
        _ => None,
    };
    if let Some((code, severity, message)) = mismatch {
        warnings.push(CapturePreviewWarning {
            code: code.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
        });
    }
    if let Some(known) = ctx.known_assignees.filter(|known| !known.is_empty()) {
        let known_set = get_known_assignee_set(Some(known));
        let unknown_assignees = ctx
            .fields
            .assignee
            .iter()
            .filter(|hint| {
                !hint.starts_with('@')
                    && !hint.contains('@')
                    && !known_set.contains(&js::trim(hint).to_lowercase())
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
        schedule: ctx.schedule,
    }
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
        let Some(captures) = js::exec(regex, working) else {
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
        .get_or_init(|| js::regex(r"(?i)\b(?:not|non)[\s-]*(?:urgent|important|critical)\b"));
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
            if js::test(pattern, working) {
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

    if js::test(negated, &working) {
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
            .map(|(pattern, _)| js::regex(pattern))
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

fn edge_punctuation() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| js::regex(r"^[,;:]+|[,;:!?]+$"))
}

fn cleanup_tag_token(value: &str) -> Option<String> {
    let trimmed = replace_all(edge_punctuation(), js::trim(value), "");
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
    REGEX.get_or_init(|| js::regex(r"#[A-Za-z0-9][A-Za-z0-9_-]*"))
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
            .get_or_init(|| js::regex(r"(?i)\s*(?:,|and)\s*"))
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

    let clause = CLAUSE
        .get_or_init(|| js::regex(r"(?i)\b(?:with\s+)?(?:tags?\b\s*:?\s*|tagged\s+)([^.;]+)$"));
    let clause_value = |captures: &Captures| Some(js::group(captures, 1).to_string());
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
        let tag_word = TAG_WORD.get_or_init(|| js::regex(r"(?i)\b(?:with\s+)?tags?\b[:]?"));
        working = normalize_whitespace(&replace_all(
            tag_word,
            &replace_all(hashtag, &working, " "),
            " ",
        ));
    }

    (tags, working)
}

fn title_case_name(value: &str) -> String {
    js::words(value)
        .map(|part| {
            if part.contains('@') {
                return part.to_string();
            }
            let (head, tail) = part.split_at(part.chars().next().map_or(0, char::len_utf8));
            js::capitalize(&format!("{head}{}", tail.to_lowercase()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn clean_named_value(value: &str) -> String {
    normalize_whitespace(&replace_all(edge_punctuation(), value, ""))
}

fn split_entity_list(value: &str) -> Vec<String> {
    static SPLIT: OnceLock<Regex> = OnceLock::new();
    SPLIT
        .get_or_init(|| js::regex(r"(?i)\s*(?:,|and|&)\s*"))
        .split(value)
        .filter_map(Result::ok)
        .map(clean_named_value)
        .filter(|part| !part.is_empty())
        .collect()
}

fn is_weekday_or_month(token: &str) -> bool {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    js::test(
        REGEX.get_or_init(|| {
            js::regex(r"(?i)^(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?|january|february|march|april|may|june|july|august|september|october|november|december|jan|feb|mar|apr|jun|jul|aug|sep|sept|oct|nov|dec)$")
        }),
        token,
    )
}

fn is_likely_person(value: &str, known_assignees: &BTreeSet<String>) -> bool {
    static REGEXES: OnceLock<[Regex; 4]> = OnceLock::new();
    let [mention, email, reserved, name_token] = REGEXES.get_or_init(|| {
        [
            js::regex(r"(?i)^@[a-z0-9_.+-]+$"),
            js::regex(r"(?i)^[a-z0-9.+-]+@[a-z0-9.-]+\.[a-z]{2,}$"),
            js::regex(
                r"(?i)^(?:project|tag|tags|today|tomorrow|tonight|next|this|every|daily|weekly|monthly)$",
            ),
            js::regex(r"^[A-Z][a-z]+(?:[-'][A-Z][a-z]+)?$"),
        ]
    });
    let cleaned = clean_named_value(value);
    if cleaned.is_empty() {
        return false;
    }
    if js::test(mention, &cleaned) || js::test(email, &cleaned) {
        return true;
    }
    if known_assignees.contains(&cleaned.to_lowercase()) {
        return true;
    }
    if js::test(reserved, &cleaned) {
        return false;
    }
    let tokens: Vec<&str> = js::words(&cleaned).collect();
    if tokens.iter().any(|token| is_weekday_or_month(token)) {
        return false;
    }
    if cleaned.chars().any(|char| char.is_ascii_digit()) {
        return false;
    }
    if tokens.len() > 2 {
        return false;
    }
    tokens.iter().all(|token| js::test(name_token, token))
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
            js::regex(
                r"(?i)^(?:with|to|by|before|every|starting|until|till|high|medium|low|urgent)$",
            ),
            js::regex(r"^(?:[A-Z][a-z]+[A-Z][A-Za-z]*|[A-Z]{2,}|[A-Z][a-z]+\d+|[A-Z][a-z]+)$"),
        ]
    });
    let cleaned = clean_named_value(value);
    if cleaned.is_empty() || js::test(reserved, &cleaned) {
        return false;
    }
    let tokens: Vec<&str> = js::words(&cleaned).collect();
    if tokens.iter().any(|token| is_weekday_or_month(token)) {
        return false;
    }
    if tokens.len() >= 2 {
        return true;
    }
    js::test(single, &cleaned)
}

fn extract_project(input: &str) -> (Option<String>, String) {
    static REGEXES: OnceLock<[Regex; 3]> = OnceLock::new();
    let [explicit, for_project, titlecase] = REGEXES.get_or_init(|| {
        let boundary = temporal_boundary();
        [
            js::regex(&format!(r"(?i)\b(?:in|under)\s+project\s+(.+?){boundary}")),
            js::regex(&format!(r"(?i)\bfor\s+project\s+(.+?){boundary}")),
            js::regex(&format!(
                r"\b[Ff][Oo][Rr]\s+([A-Z][a-z0-9-]*(?:\s+(?!(?:{PROJECT_CHAIN_RESERVED})\b)[A-Za-z][A-Za-z0-9-]*){{0,3}}){boundary}"
            )),
        ]
    });
    let keyword_value = |captures: &Captures| {
        let candidate = clean_named_value(js::group(captures, 1));
        (!candidate.is_empty()).then(|| format!("Project {candidate}"))
    };
    let titlecase_value = |captures: &Captures| {
        let candidate = clean_named_value(js::group(captures, 1));
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
        .map(|value| js::trim(value).to_lowercase())
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

    let mention = MENTION.get_or_init(|| js::regex(r"(?<![A-Za-z0-9_.+-])@[A-Za-z0-9_.+-]+"));
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
        js::regex(r"(?i)^\s*(?:meeting|meet(?:ing)?|sync|chat|coffee|lunch|dinner|breakfast|call|review)\s+with\b")
    });
    let [to_clause, for_clause, with_clause] = CLAUSES.get_or_init(|| {
        let boundary = temporal_boundary();
        let people = person_list();
        [
            js::regex(&format!(r"(?i)\bto\s+({people}){boundary}")),
            js::regex(&format!(r"(?i)\bfor\s+({people}){boundary}")),
            js::regex(&format!(r"(?i)\bwith\s+({people}){boundary}")),
        ]
    });
    let normalize_list = |captures: &Captures| {
        split_entity_list(js::group(captures, 1))
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
        if js::test(contextual_with_lead, input) {
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

fn build_task_title(input: &str) -> String {
    let cleaned = finalize_task_title(input);
    if cleaned.is_empty() {
        return "Task".to_string();
    }
    cleaned
}
