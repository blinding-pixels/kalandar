use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::capture::recurrence::{ByDay, Frequency, RecurrenceException};
use crate::capture::resolve::{
    schedule_parts, Alert, CaptureKind, LabelledSpan, LineResolution, Moment, ResolvedRecurrence,
    Role,
};
use crate::capture::{Context, Warning};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScheduleSpan {
    pub role: Role,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ScheduleTexts {
    pub moment: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub end_time: Option<String>,
    pub duration: Option<String>,
    pub lead: Option<String>,
    pub alert: Option<String>,
    pub recurrence: Option<String>,
    pub recurrence_start: Option<String>,
    pub recurrence_end: Option<String>,
    pub recurrence_window: Option<String>,
    pub recurrence_exceptions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewMoment {
    pub date: String,
    pub time: Option<String>,
    pub local: Option<String>,
    pub instant: Option<String>,
    pub all_day: bool,
    pub implied: Vec<&'static str>,
    pub alternative_date: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewWeekday {
    pub position: Option<i8>,
    pub weekday: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewRecurrence {
    pub rrule: String,
    pub frequency: Frequency,
    pub interval: u32,
    pub days: Option<Vec<&'static str>>,
    pub weekdays: Vec<PreviewWeekday>,
    pub month_days: Vec<i8>,
    pub months: Vec<u32>,
    pub time: Option<String>,
    pub count: Option<u32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub until: Option<String>,
    pub exdates: Vec<String>,
    pub exceptions: Vec<RecurrenceException>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewSchedule {
    pub start: Option<PreviewMoment>,
    pub end: Option<PreviewMoment>,
    pub duration_minutes: Option<i64>,
    pub recurrence: Option<PreviewRecurrence>,
    pub lead_minutes: Option<i64>,
    pub alert_minutes_before: Option<i64>,
    pub alert_at: Option<PreviewMoment>,
    pub alerts_off: bool,
    pub notify_at: Option<String>,
    pub task_signal: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreviewWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureSchedule {
    pub spans: Vec<ScheduleSpan>,
    pub texts: ScheduleTexts,
    pub schedule: Option<PreviewSchedule>,
    pub warnings: Vec<PreviewWarning>,
}

static TIME_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:\b(?:at|by|@|around|from)\s*)?(?:\b\d{1,2}(?::\d{2})?\s*(?:a\.?m\.?|p\.?m\.?|am|pm)(?:\W|$)|\b\d{1,2}:\d{2}\b|\b\d{3,4}\s*h(?:rs)?\b|\b\d{1,2}\s*h\b|\b(?:noon|midday|midnight|eod|cob|end\s+of\s+(?:the\s+)?day|close\s+of\s+business)\b|\b(?:this\s+|in\s+the\s+)?(?:morning|afternoon|evening|night|tonight|tonite)\b)",
    )
    .unwrap_or_else(|e| panic!("time phrase: {e}"))
});

static CONNECTOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:by|before|on)\s+$").unwrap_or_else(|e| panic!("connector: {e}"))
});

static DAYPART: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:morning|afternoon|evening|night)\b").unwrap_or_else(|e| panic!("daypart: {e}"))
});

static IN_THE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bin\s+the\s+$").unwrap_or_else(|e| panic!("in the: {e}"))
});

static ALERT_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:with\s+an?\s+)?(?:alert|alarm|reminder|notification|notify(?:\s+me)?|ping(?:\s+me)?|remind\s+me)\s*:?\s*$")
        .unwrap_or_else(|e| panic!("alert word: {e}"))
});

fn trim_connectors(text: &str) -> Option<String> {
    let mut t = text.trim();
    loop {
        let before = t;
        for edge in [",", ";", "-", "–"] {
            t = t.trim_start_matches(edge).trim_end_matches(edge).trim();
        }
        for word in ["at", "on", "by", "and"] {
            let lower = t.to_lowercase();
            if lower.ends_with(&format!(" {word}")) {
                t = t[..t.len() - word.len()].trim();
            }
            if lower == word {
                t = "";
            }
        }
        if t == before {
            break;
        }
    }
    (!t.is_empty()).then(|| t.to_string())
}

fn strip_time_prefix(t: String) -> String {
    let lower = t.to_lowercase();
    ["at ", "by ", "from ", "in the ", "@"]
        .iter()
        .find(|p| lower.starts_with(*p))
        .map_or(t.clone(), |p| t[p.len()..].trim().to_string())
}

static TIME_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:-|–|to|until|till)\s*").unwrap_or_else(|e| panic!("time range: {e}"))
});

fn split_moment(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let Some(m) = TIME_PHRASE.find(text) else {
        return (trim_connectors(text), None, None);
    };
    let time = trim_connectors(m.as_str()).map(strip_time_prefix);
    let mut end_time = None;
    let mut rest_end = m.end();
    if let Some(sep) = TIME_RANGE.find(&text[m.end()..]) {
        let tail = &text[m.end() + sep.end()..];
        if let Some(second) = TIME_PHRASE.find(tail).filter(|x| x.start() == 0) {
            end_time = trim_connectors(second.as_str()).map(strip_time_prefix);
            rest_end = m.end() + sep.end() + second.end();
        }
    }
    let rest = format!("{} {}", &text[..m.start()], &text[rest_end..]);
    let date = trim_connectors(&rest.split_whitespace().collect::<Vec<_>>().join(" "));
    (date, time, end_time)
}

fn joinable(gap: &str) -> bool {
    let gap = gap.trim().to_lowercase();
    gap.is_empty()
        || gap.chars().all(|c| !c.is_alphanumeric())
        || matches!(gap.as_str(), "and" | "at" | "on" | "the" | "of")
}

fn regions(line: &str, spans: &[&LabelledSpan]) -> Vec<(usize, usize)> {
    let mut sorted: Vec<(usize, usize)> = spans.iter().map(|s| (s.start, s.end)).collect();
    sorted.sort_unstable();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (start, end) in sorted {
        match out.last_mut() {
            Some(last) if start <= last.1 || joinable(line.get(last.1..start).unwrap_or("x")) => {
                last.1 = last.1.max(end);
            }
            _ => out.push((start, end)),
        }
    }
    out
}

fn text_of(line: &str, (start, end): (usize, usize)) -> Option<String> {
    trim_connectors(line.get(start..end).unwrap_or(""))
}

fn role_text(line: &str, spans: &[LabelledSpan], roles: &[Role]) -> Option<String> {
    let chosen: Vec<&LabelledSpan> = spans.iter().filter(|s| roles.contains(&s.role)).collect();
    regions(line, &chosen)
        .into_iter()
        .find_map(|r| text_of(line, r))
}

fn moment(m: &Moment) -> PreviewMoment {
    PreviewMoment {
        date: m.date.format("%Y-%m-%d").to_string(),
        time: m.time.map(|t| t.format("%H:%M").to_string()),
        local: m.local.map(|l| l.format("%Y-%m-%dT%H:%M:%S").to_string()),
        instant: m.instant.map(|i| i.to_rfc3339()),
        all_day: m.all_day,
        implied: m.implied.clone(),
        alternative_date: m.alternative.map(|d| d.format("%Y-%m-%d").to_string()),
    }
}

fn weekday_name(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "monday",
        chrono::Weekday::Tue => "tuesday",
        chrono::Weekday::Wed => "wednesday",
        chrono::Weekday::Thu => "thursday",
        chrono::Weekday::Fri => "friday",
        chrono::Weekday::Sat => "saturday",
        chrono::Weekday::Sun => "sunday",
    }
}

fn recurrence(r: &ResolvedRecurrence) -> PreviewRecurrence {
    let all_day = r.first.as_ref().is_some_and(|f| f.all_day);
    PreviewRecurrence {
        rrule: r.rrule.clone(),
        frequency: r.rule.frequency,
        interval: r.rule.interval,
        days: r.reminder_shape.as_ref().and_then(|s| s.days.clone()),
        weekdays: r
            .rule
            .weekdays
            .iter()
            .map(|ByDay { position, weekday }| PreviewWeekday {
                position: *position,
                weekday: weekday_name(*weekday),
            })
            .collect(),
        month_days: r.rule.month_days.clone(),
        months: r.rule.months.clone(),
        time: r.rule.time.map(|t| t.format("%H:%M").to_string()),
        count: r.rule.count,
        start_date: r.start_date.map(|d| d.format("%Y-%m-%d").to_string()),
        end_date: r.end_date.map(|d| d.format("%Y-%m-%d").to_string()),
        until: r.until.map(|d| d.format("%Y-%m-%d").to_string()),
        exdates: r
            .exdates
            .iter()
            .map(|d| {
                if all_day {
                    d.format("%Y-%m-%d").to_string()
                } else {
                    d.format("%Y-%m-%dT%H:%M:%S").to_string()
                }
            })
            .collect(),
        exceptions: r.exceptions.clone(),
    }
}

fn collect_warnings(r: &LineResolution) -> Vec<PreviewWarning> {
    let mut all: Vec<&Warning> = r.warnings.iter().collect();
    for m in [&r.start, &r.end].into_iter().flatten() {
        all.extend(m.warnings.iter());
    }
    if let Some(rec) = &r.recurrence {
        all.extend(rec.warnings.iter());
    }
    let mut out: Vec<PreviewWarning> = Vec::new();
    for w in all {
        if !out
            .iter()
            .any(|o| o.code == w.code.as_str() && o.message == w.message)
        {
            out.push(PreviewWarning {
                code: w.code.as_str(),
                message: w.message.clone(),
            });
        }
    }
    out
}

fn minutes_in(text: &str) -> Option<i64> {
    let caps = DURATION_AMOUNT.captures(text)?;
    let amount = crate::capture::lexicon::number_value(caps.get(1)?.as_str())?;
    let per = crate::capture::lexicon::unit_minutes(caps.get(2)?.as_str())?;
    let minutes = (amount * per).round();
    (minutes > 0.0 && minutes < 1_000_000.0).then_some(minutes as i64)
}

static DURATION_AMOUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\d+(?:\.\d+)?|an?|one|two|three|four|five|six|seven|eight|nine|ten|half\s+an?)\s*(hours?|hrs?|h|minutes?|mins?|m)\b")
        .unwrap_or_else(|e| panic!("duration amount: {e}"))
});

fn schedule_of(r: &LineResolution, duration_text: Option<&str>) -> Option<PreviewSchedule> {
    let (alert_minutes_before, alert_at, alerts_off) = match &r.alert {
        Some(Alert::Relative { minutes_before, at }) => {
            (Some(*minutes_before), at.as_ref().map(moment), false)
        }
        Some(Alert::Absolute { at }) => (None, Some(moment(at)), false),
        Some(Alert::Off) => (None, None, true),
        None => (None, None, false),
    };
    let duration_minutes = match (&r.start, &r.end) {
        (Some(s), Some(e)) => match (s.local, e.local) {
            (Some(a), Some(b)) if b > a => Some((b - a).num_minutes()),
            _ => None,
        },
        _ => None,
    };
    let duration_minutes = duration_minutes.or_else(|| duration_text.and_then(minutes_in));
    let schedule = PreviewSchedule {
        start: r.start.as_ref().map(moment),
        end: r.end.as_ref().map(moment),
        duration_minutes,
        recurrence: r.recurrence.as_ref().map(recurrence),
        lead_minutes: r.lead_minutes,
        alert_minutes_before,
        alert_at,
        alerts_off,
        notify_at: r
            .notify_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
        task_signal: r.task_signal.clone(),
    };
    let empty = schedule.start.is_none()
        && schedule.end.is_none()
        && schedule.recurrence.is_none()
        && schedule.lead_minutes.is_none()
        && schedule.alert_minutes_before.is_none()
        && schedule.alert_at.is_none()
        && !schedule.alerts_off;
    (!empty).then_some(schedule)
}

pub fn capture_schedule(
    line: &str,
    kind: CaptureKind,
    labelled: Option<&[LabelledSpan]>,
    ctx: &Context,
) -> CaptureSchedule {
    let parts = schedule_parts(line, kind, labelled, ctx);
    let mut all: Vec<LabelledSpan> = parts.spans.clone();
    all.extend(
        parts
            .alerts
            .iter()
            .map(|(a, b)| LabelledSpan::new(Role::Alert, *a, *b)),
    );
    for span in &mut all {
        if matches!(span.role, Role::EventStart | Role::DueDate | Role::DueTime | Role::Trigger) {
            if let Some(m) = line.get(..span.start).and_then(|before| CONNECTOR.find(before)) {
                span.start = m.start();
            } else if line.get(span.start..span.end).is_some_and(|t| DAYPART.is_match(t)) {
                if let Some(m) = line.get(..span.start).and_then(|before| IN_THE.find(before)) {
                    span.start = m.start();
                }
            }
        }
        if matches!(span.role, Role::LeadTime | Role::Alert) {
            if let Some(m) = line
                .get(..span.start)
                .and_then(|before| ALERT_WORD.find(before))
            {
                span.start = m.start();
            }
        }
    }
    all.retain(|s| {
        s.start < s.end
            && s.end <= line.len()
            && line.is_char_boundary(s.start)
            && line.is_char_boundary(s.end)
    });
    all.sort_by_key(|s| (s.start, s.end));
    let moment_roles = [
        Role::EventStart,
        Role::DueDate,
        Role::DueTime,
        Role::Trigger,
    ];
    let others: Vec<(usize, usize)> = all
        .iter()
        .filter(|x| !moment_roles.contains(&x.role))
        .map(|x| (x.start, x.end))
        .collect();
    let recurrences: Vec<(usize, usize)> = all
        .iter()
        .filter(|x| x.role == Role::Recurrence)
        .map(|x| (x.start, x.end))
        .collect();
    let mut moments: Vec<LabelledSpan> = Vec::new();
    let mut attached: Vec<LabelledSpan> = Vec::new();
    for span in all.iter().filter(|x| moment_roles.contains(&x.role)) {
        let mut pieces = vec![(span.start, span.end)];
        for (a, b) in &others {
            pieces = pieces
                .into_iter()
                .flat_map(|(s0, e0)| {
                    if *b <= s0 || e0 <= *a {
                        vec![(s0, e0)]
                    } else {
                        vec![(s0, (*a).max(s0)), ((*b).min(e0), e0)]
                    }
                })
                .filter(|(s0, e0)| {
                    s0 < e0
                        && line
                            .get(*s0..*e0)
                            .is_some_and(|t| t.chars().any(char::is_alphanumeric))
                })
                .collect();
        }
        for (s0, e0) in pieces {
            let piece = LabelledSpan::new(span.role, s0, e0);
            let near_recurrence = recurrences.iter().any(|(a, b)| {
                let (gap_start, gap_end) = if *b <= s0 { (*b, s0) } else { (e0, *a) };
                gap_start <= gap_end && joinable(line.get(gap_start..gap_end).unwrap_or("x"))
            });
            if near_recurrence {
                attached.push(piece);
            } else {
                moments.push(piece);
            }
        }
    }
    let moment_refs: Vec<&LabelledSpan> = moments.iter().collect();
    let moment_text = regions(line, &moment_refs)
        .into_iter()
        .find_map(|r| text_of(line, r));
    let (date, mut time, range_end) = moment_text
        .as_deref()
        .map(split_moment)
        .unwrap_or((None, None, None));
    let mut recurrence_spans: Vec<LabelledSpan> = all
        .iter()
        .filter(|x| x.role == Role::Recurrence)
        .copied()
        .collect();
    recurrence_spans.extend(
        attached
            .iter()
            .map(|x| LabelledSpan::new(Role::Recurrence, x.start, x.end)),
    );
    let mut recurrence_text = role_text(line, &recurrence_spans, &[Role::Recurrence]);
    if time.is_none() {
        if let Some(text) = recurrence_text.clone() {
            let (rule, rule_time, _) = split_moment(&text);
            if rule_time.is_some() {
                time = rule_time;
                recurrence_text = rule;
            }
        }
    }
    let texts = ScheduleTexts {
        date,
        time,
        moment: moment_text,
        end_time: role_text(line, &all, &[Role::EventEnd])
            .map(|t| {
                let lower = t.to_lowercase();
                ["to ", "until ", "till ", "- ", "– "]
                    .iter()
                    .find(|p| lower.starts_with(*p))
                    .map_or(t.clone(), |p| t[p.len()..].trim().to_string())
            })
            .or(range_end),
        duration: role_text(line, &all, &[Role::Duration]),
        lead: role_text(line, &all, &[Role::LeadTime]),
        alert: role_text(line, &all, &[Role::Alert]),
        recurrence: recurrence_text,
        recurrence_start: role_text(line, &all, &[Role::RecurrenceStart]),
        recurrence_end: role_text(line, &all, &[Role::RecurrenceEnd]),
        recurrence_window: role_text(line, &all, &[Role::RecurrenceWindow]),
        recurrence_exceptions: all
            .iter()
            .filter(|s| s.role == Role::RecurrenceException)
            .filter_map(|s| text_of(line, (s.start, s.end)))
            .collect(),
    };
    let spans = all
        .iter()
        .map(|s| ScheduleSpan {
            role: s.role,
            start: s.start,
            end: s.end,
            text: line[s.start..s.end].to_string(),
        })
        .collect();
    let schedule = schedule_of(&parts.resolution, texts.duration.as_deref());
    CaptureSchedule {
        spans,
        texts,
        schedule,
        warnings: collect_warnings(&parts.resolution),
    }
}
