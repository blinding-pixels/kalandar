use super::intent_model::{classify_probabilities, load_tiny_intent_model};
use super::intent_rules::{
    has_date_signal, has_recurrence_signal, has_time_signal, is_explicit_reminder,
    is_short_appointment_ambiguous, looks_like_dated_task, looks_like_reflective_note,
    looks_like_soft_action_idea, looks_like_time_blocked_task, normalize_intent_input,
    starts_with_task_lead, tokens,
};
use super::{CaptureParserArtifactManifest, CaptureParserError};

const DEFAULT_INTENT_THRESHOLD: f64 = 0.55;
const GAP_THRESHOLD: f64 = 0.15;
const ROUTE_TO_NONE_THRESHOLD: f64 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    Task,
    Calendar,
    Reminder,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntentRoute {
    pub intent: IntentKind,
    pub task: f64,
    pub calendar: f64,
    pub reminder: f64,
    pub none: f64,
    pub ambiguous: bool,
    pub top_two_gap: f64,
}

impl IntentRoute {
    pub fn primary_score(&self) -> f64 {
        match self.intent {
            IntentKind::Task => self.task,
            IntentKind::Calendar => self.calendar,
            IntentKind::Reminder => self.reminder,
            IntentKind::None => self.none,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RankedIntent {
    kind: IntentKind,
    score: f64,
}

pub fn classify_intent(
    manifest: &CaptureParserArtifactManifest,
    input: &str,
) -> Result<IntentRoute, CaptureParserError> {
    let model = load_tiny_intent_model(manifest)?;
    let normalized = normalize_intent_input(input);
    let tokens = tokens(&normalized);

    if normalized.is_empty() {
        return Ok(route(IntentKind::None, 0.0, 0.0, 0.0, 1.0, true, 1.0));
    }

    if is_explicit_reminder(&normalized) {
        return Ok(route(
            IntentKind::Reminder,
            0.03,
            0.03,
            0.92,
            0.02,
            false,
            0.89,
        ));
    }

    if looks_like_reflective_note(&normalized, &tokens) {
        return Ok(route(IntentKind::None, 0.03, 0.03, 0.0, 0.94, false, 0.91));
    }

    if looks_like_soft_action_idea(&normalized, &tokens) {
        return Ok(route(IntentKind::None, 0.12, 0.03, 0.0, 0.85, false, 0.73));
    }

    if starts_with_task_lead(&tokens)
        && !has_time_signal(&normalized)
        && !has_date_signal(&normalized)
        && !has_recurrence_signal(&normalized)
    {
        return Ok(route(IntentKind::Task, 0.86, 0.12, 0.0, 0.02, false, 0.74));
    }

    if looks_like_time_blocked_task(&normalized, &tokens) {
        return Ok(route(IntentKind::Task, 0.78, 0.2, 0.0, 0.02, false, 0.58));
    }

    if looks_like_dated_task(&normalized, &tokens) {
        return Ok(route(IntentKind::Task, 0.72, 0.24, 0.0, 0.04, false, 0.48));
    }

    if is_short_appointment_ambiguous(&normalized, &tokens) {
        return Ok(route(IntentKind::None, 0.18, 0.42, 0.0, 0.4, true, 0.02));
    }

    let [task, calendar, none] = classify_probabilities(model.as_ref(), &normalized);
    let mut ranked = [
        RankedIntent {
            kind: IntentKind::Task,
            score: task,
        },
        RankedIntent {
            kind: IntentKind::Calendar,
            score: calendar,
        },
        RankedIntent {
            kind: IntentKind::None,
            score: none,
        },
        RankedIntent {
            kind: IntentKind::Reminder,
            score: 0.0,
        },
    ];
    ranked.sort_by(|left, right| right.score.total_cmp(&left.score));

    let top = ranked[0];
    let second = ranked[1];
    let top_two_gap = top.score - second.score;
    let ambiguous = top_two_gap <= GAP_THRESHOLD || top.score < DEFAULT_INTENT_THRESHOLD;
    let intent = if top.kind == IntentKind::None || top.score < ROUTE_TO_NONE_THRESHOLD {
        IntentKind::None
    } else {
        top.kind
    };

    Ok(route(
        intent,
        task,
        calendar,
        0.0,
        none,
        ambiguous,
        top_two_gap,
    ))
}

fn route(
    intent: IntentKind,
    task: f64,
    calendar: f64,
    reminder: f64,
    none: f64,
    ambiguous: bool,
    top_two_gap: f64,
) -> IntentRoute {
    IntentRoute {
        intent,
        task,
        calendar,
        reminder,
        none,
        ambiguous,
        top_two_gap,
    }
}
