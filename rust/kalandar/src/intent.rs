use super::intent_model::{
    classify_probabilities, embedded_tiny_intent_model, TinyIntentModel,
};
use super::intent_rules::{
    has_date_signal, has_recurrence_signal, has_time_signal, is_explicit_reminder,
    is_short_appointment_ambiguous, looks_like_dated_task, looks_like_reflective_note,
    looks_like_soft_action_idea, looks_like_time_blocked_task, normalize_intent_input,
    normalize_js_whitespace, starts_with_task_lead, tokens,
};
use super::{
    CapturePreviewCandidate, CapturePreviewWarning,
};

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

impl IntentKind {
    pub fn capture_kind(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Calendar => "calendar-event",
            Self::Reminder => "reminder",
            Self::None => "thought",
        }
    }
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

    pub fn candidates(&self) -> Vec<CapturePreviewCandidate> {
        let mut candidates = [
            ("task", self.task),
            ("calendar-event", self.calendar),
            ("reminder", self.reminder),
        ]
        .into_iter()
        .filter(|(_, score)| *score > 0.0)
        .map(|(kind, score)| CapturePreviewCandidate { kind, score })
        .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates
    }

    pub fn warnings(&self) -> Vec<CapturePreviewWarning> {
        if !self.ambiguous {
            return Vec::new();
        }
        let message = if self.intent == IntentKind::None {
            "Intent is ambiguous and was routed to none."
        } else {
            "Intent is ambiguous between multiple categories."
        };
        vec![CapturePreviewWarning {
            code: "intent_ambiguous".to_string(),
            severity: "info".to_string(),
            message: message.to_string(),
        }]
    }

    pub fn capture_intent(&self) -> CaptureIntent {
        CaptureIntent {
            kind: self.intent.capture_kind(),
            confidence: self.primary_score(),
            candidates: self.candidates(),
            warnings: self.warnings(),
            route: *self,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureIntent {
    pub kind: &'static str,
    pub confidence: f64,
    pub candidates: Vec<CapturePreviewCandidate>,
    pub warnings: Vec<CapturePreviewWarning>,
    pub route: IntentRoute,
}


pub fn classify_intent_embedded(input: &str) -> IntentRoute {
    classify_intent_with_model(embedded_tiny_intent_model(), input)
}

pub fn route_capture_intent(text: &str) -> CaptureIntent {
    classify_intent_embedded(&normalize_js_whitespace(text)).capture_intent()
}

pub(crate) fn classify_intent_with_model(model: &TinyIntentModel, input: &str) -> IntentRoute {
    let normalized = normalize_intent_input(input);
    let tokens = tokens(&normalized);

    if normalized.is_empty() {
        return route(IntentKind::None, [0.0, 0.0, 0.0, 1.0], true, 1.0);
    }

    if is_explicit_reminder(&normalized) {
        return route(IntentKind::Reminder, [0.03, 0.03, 0.92, 0.02], false, 0.89);
    }

    if looks_like_reflective_note(&normalized, &tokens) {
        return route(IntentKind::None, [0.03, 0.03, 0.0, 0.94], false, 0.91);
    }

    if looks_like_soft_action_idea(&normalized, &tokens) {
        return route(IntentKind::None, [0.12, 0.03, 0.0, 0.85], false, 0.73);
    }

    if starts_with_task_lead(&tokens)
        && !has_time_signal(&normalized)
        && !has_date_signal(&normalized)
        && !has_recurrence_signal(&normalized)
    {
        return route(IntentKind::Task, [0.86, 0.12, 0.0, 0.02], false, 0.74);
    }

    if looks_like_time_blocked_task(&normalized, &tokens) {
        return route(IntentKind::Task, [0.78, 0.2, 0.0, 0.02], false, 0.58);
    }

    if looks_like_dated_task(&normalized, &tokens) {
        return route(IntentKind::Task, [0.72, 0.24, 0.0, 0.04], false, 0.48);
    }

    if is_short_appointment_ambiguous(&normalized, &tokens) {
        return route(IntentKind::None, [0.18, 0.42, 0.0, 0.4], true, 0.02);
    }

    let scores = classify_probabilities(model, &normalized);
    let mut ranked = [
        IntentKind::Task,
        IntentKind::Calendar,
        IntentKind::Reminder,
        IntentKind::None,
    ]
    .into_iter()
    .zip(scores)
    .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let (top_kind, top_score) = ranked[0];
    let top_two_gap = top_score - ranked[1].1;
    let ambiguous = top_two_gap <= GAP_THRESHOLD || top_score < DEFAULT_INTENT_THRESHOLD;
    let intent = if top_kind == IntentKind::None || top_score < ROUTE_TO_NONE_THRESHOLD {
        IntentKind::None
    } else {
        top_kind
    };

    route(intent, scores, ambiguous, top_two_gap)
}

fn route(
    intent: IntentKind,
    [task, calendar, reminder, none]: [f64; 4],
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
