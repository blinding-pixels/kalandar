pub use kalandar_dates::capture::{
    PreviewMoment, PreviewRecurrence, PreviewSchedule, PreviewWeekday,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePreviewRequest {
    pub text: String,
    pub now_iso: Option<String>,
    pub timezone: String,
    pub locale: Option<String>,
    pub known_assignees: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CapturePreviewCandidate {
    pub kind: &'static str,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewRecurrenceException {
    pub raw: String,
    pub condition: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewTaskDraft {
    pub title: String,
    pub assignees: Vec<String>,
    pub project_hint: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    pub due_date_text: Option<String>,
    pub due_time_text: Option<String>,
    pub duration_text: Option<String>,
    pub recurrence_text: Option<String>,
    pub recurrence_start_text: Option<String>,
    pub recurrence_end_text: Option<String>,
    pub recurrence_exceptions: Vec<CapturePreviewRecurrenceException>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewReminderRecurrence {
    pub frequency: String,
    pub interval: u32,
    pub days: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewReminderDraft {
    pub title: String,
    pub remind_at_text: Option<String>,
    pub recurrence: Option<CapturePreviewReminderRecurrence>,
    pub recurrence_start_text: Option<String>,
    pub recurrence_end_text: Option<String>,
    pub recurrence_exceptions: Vec<CapturePreviewRecurrenceException>,
    pub recurrence_window_text: Option<String>,
    pub location: Option<String>,
    pub context_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewCalendarEventDraft {
    pub title: Option<String>,
    pub date_text: Option<String>,
    pub time_text: Option<String>,
    pub end_time_text: Option<String>,
    pub duration_minutes: Option<u32>,
    pub location: Option<String>,
    pub recurrence_text: Option<String>,
    pub recurrence_start_text: Option<String>,
    pub recurrence_end_text: Option<String>,
    pub recurrence_exceptions: Vec<CapturePreviewRecurrenceException>,
    pub alert_offset_minutes: Vec<u32>,
    pub attendees: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewThoughtDraft {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapturePreviewWarning {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CapturePreviewResult {
    pub kind: &'static str,
    pub confidence: f64,
    pub candidates: Vec<CapturePreviewCandidate>,
    pub task: Option<CapturePreviewTaskDraft>,
    pub event: Option<CapturePreviewCalendarEventDraft>,
    pub reminder: Option<CapturePreviewReminderDraft>,
    pub thought: Option<CapturePreviewThoughtDraft>,
    pub unavailable_reason: Option<String>,
    pub warnings: Vec<CapturePreviewWarning>,
    pub schedule: Option<PreviewSchedule>,
}

impl CapturePreviewResult {
    pub fn unsupported(unavailable_reason: &'static str, message: &'static str) -> Self {
        Self {
            kind: "unsupported",
            confidence: 0.0,
            candidates: Vec::new(),
            task: None,
            event: None,
            reminder: None,
            thought: None,
            unavailable_reason: Some(unavailable_reason.to_string()),
            warnings: vec![CapturePreviewWarning {
                code: unavailable_reason.to_string(),
                severity: "warning".to_string(),
                message: message.to_string(),
            }],
            schedule: None,
        }
    }

    pub fn thought(
        text: String,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
    ) -> Self {
        Self {
            kind: "thought",
            confidence,
            candidates,
            task: None,
            event: None,
            reminder: None,
            thought: Some(CapturePreviewThoughtDraft { text }),
            unavailable_reason: None,
            warnings: Vec::new(),
            schedule: None,
        }
    }

    pub fn ambiguous(confidence: f64, candidates: Vec<CapturePreviewCandidate>) -> Self {
        Self {
            kind: "ambiguous",
            confidence,
            candidates,
            task: None,
            event: None,
            reminder: None,
            thought: None,
            unavailable_reason: None,
            warnings: Vec::new(),
            schedule: None,
        }
    }

    pub fn task(
        task: CapturePreviewTaskDraft,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
        warnings: Vec<CapturePreviewWarning>,
    ) -> Self {
        Self {
            kind: "task",
            confidence,
            candidates,
            task: Some(task),
            event: None,
            reminder: None,
            thought: None,
            unavailable_reason: None,
            warnings,
            schedule: None,
        }
    }

    pub fn reminder(
        reminder: CapturePreviewReminderDraft,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
        warnings: Vec<CapturePreviewWarning>,
    ) -> Self {
        Self {
            kind: "reminder",
            confidence,
            candidates,
            task: None,
            event: None,
            reminder: Some(reminder),
            thought: None,
            unavailable_reason: None,
            warnings,
            schedule: None,
        }
    }

    pub fn calendar_event(
        event: CapturePreviewCalendarEventDraft,
        confidence: f64,
        candidates: Vec<CapturePreviewCandidate>,
        warnings: Vec<CapturePreviewWarning>,
    ) -> Self {
        Self {
            kind: "calendar-event",
            confidence,
            candidates,
            task: None,
            event: Some(event),
            reminder: None,
            thought: None,
            unavailable_reason: None,
            warnings,
            schedule: None,
        }
    }
}

impl CapturePreviewResult {
    pub fn with_schedule(mut self, schedule: Option<PreviewSchedule>) -> Self {
        self.schedule = schedule;
        self
    }
}
