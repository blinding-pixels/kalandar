use kalandar::{CaptureParser, CapturePreviewRequest, CapturePreviewResult};
use serde::Deserialize;
use std::{collections::BTreeSet, env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct IntentFixture {
    id: String,
    input: String,
    oracle: IntentOracle,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct IntentOracle {
    kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskFixture {
    id: String,
    input: String,
    timezone: String,
    known_assignees: Option<Vec<String>>,
    oracle: TaskOracle,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TaskOracle {
    kind: String,
    title: Option<String>,
    assignees: Vec<String>,
    project_hint: Option<String>,
    tags: Vec<String>,
    priority: Option<String>,
    status: Option<String>,
    due_date_text: Option<String>,
    due_time_text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReminderFixture {
    id: String,
    input: String,
    timezone: String,
    oracle: ReminderOracle,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ReminderOracle {
    kind: String,
    title: Option<String>,
    remind_at_text: Option<String>,
    recurrence_start_text: Option<String>,
    recurrence_end_text: Option<String>,
    recurrence_window_text: Option<String>,
    context_hint: Option<String>,
    recurrence_exceptions: Vec<ReminderExceptionOracle>,
    recurrence: Option<ReminderRecurrenceOracle>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ReminderRecurrenceOracle {
    frequency: Option<String>,
    interval: u32,
    days: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ReminderExceptionOracle {
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarFixture {
    id: String,
    input: String,
    timezone: String,
    locale: String,
    oracle: CalendarOracle,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CalendarOracle {
    kind: String,
    title: Option<String>,
    date_text: Option<String>,
    time_text: Option<String>,
    end_time_text: Option<String>,
    duration_minutes: Option<u32>,
    location: Option<String>,
    recurrence_text: Option<String>,
    recurrence_start_text: Option<String>,
    recurrence_end_text: Option<String>,
    recurrence_exceptions: Vec<ReminderExceptionOracle>,
    attendees: Vec<String>,
}

fn load_fixtures<T>(relative_path: &str) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/kalandar")
        .join(relative_path);
    let contents = fs::read_to_string(path).expect("fixture json should exist");
    serde_json::from_str(&contents).expect("fixture json should parse")
}

fn capture_parser_resource_root() -> Option<PathBuf> {
    env::var_os("KALANDAR_CAPTURE_RESOURCE_ROOT").map(PathBuf::from)
}

fn parser() -> Option<CaptureParser> {
    let Some(resource_root) = capture_parser_resource_root() else {
        eprintln!("skipping resource-backed capture parser test: KALANDAR_CAPTURE_RESOURCE_ROOT is not set");
        return None;
    };
    Some(CaptureParser::new(resource_root).expect("resource manifest should load"))
}

fn preview_request(
    input: &str,
    timezone: &str,
    locale: Option<&str>,
    known_assignees: Option<Vec<String>>,
) -> CapturePreviewRequest {
    CapturePreviewRequest {
        text: input.to_string(),
        now_iso: Some("2026-05-16T12:00:00.000Z".to_string()),
        timezone: timezone.to_string(),
        locale: locale.map(str::to_string),
        known_assignees,
    }
}

fn normalize_text(value: Option<&str>) -> Option<String> {
    value.and_then(|entry| {
        let normalized = entry.split_whitespace().collect::<Vec<_>>().join(" ");
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.to_lowercase())
        }
    })
}

fn normalize_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_task_preview(preview: &CapturePreviewResult) -> TaskOracle {
    let task = preview
        .task
        .as_ref()
        .expect("task preview should be present");
    TaskOracle {
        kind: preview.kind.to_string(),
        title: normalize_text(Some(task.title.as_str())),
        assignees: normalize_list(&task.assignees),
        project_hint: normalize_text(task.project_hint.as_deref()),
        tags: normalize_list(&task.tags),
        priority: normalize_text(task.priority.as_deref()),
        status: normalize_text(task.status.as_deref()),
        due_date_text: normalize_text(task.due_date_text.as_deref()),
        due_time_text: normalize_text(task.due_time_text.as_deref()),
    }
}

fn normalize_reminder_preview(preview: &CapturePreviewResult) -> ReminderOracle {
    let reminder = preview
        .reminder
        .as_ref()
        .expect("reminder preview should be present");
    ReminderOracle {
        kind: preview.kind.to_string(),
        title: normalize_text(Some(reminder.title.as_str())),
        remind_at_text: normalize_text(reminder.remind_at_text.as_deref()),
        recurrence_start_text: normalize_text(reminder.recurrence_start_text.as_deref()),
        recurrence_end_text: normalize_text(reminder.recurrence_end_text.as_deref()),
        recurrence_window_text: normalize_text(reminder.recurrence_window_text.as_deref()),
        context_hint: normalize_text(reminder.context_hint.as_deref()),
        recurrence_exceptions: reminder
            .recurrence_exceptions
            .iter()
            .map(|exception| ReminderExceptionOracle {
                raw: normalize_text(Some(exception.raw.as_str())),
            })
            .collect(),
        recurrence: reminder
            .recurrence
            .as_ref()
            .map(|recurrence| ReminderRecurrenceOracle {
                frequency: normalize_text(Some(recurrence.frequency.as_str())),
                interval: recurrence.interval,
                days: recurrence.days.as_ref().map(|days| normalize_list(days)),
            }),
    }
}

fn normalize_calendar_preview(preview: &CapturePreviewResult) -> CalendarOracle {
    let event = preview
        .event
        .as_ref()
        .expect("calendar preview should be present");
    CalendarOracle {
        kind: preview.kind.to_string(),
        title: normalize_text(event.title.as_deref()),
        date_text: normalize_text(event.date_text.as_deref()),
        time_text: normalize_text(event.time_text.as_deref()),
        end_time_text: normalize_text(event.end_time_text.as_deref()),
        duration_minutes: event.duration_minutes,
        location: normalize_text(event.location.as_deref()),
        recurrence_text: normalize_text(event.recurrence_text.as_deref()),
        recurrence_start_text: normalize_text(event.recurrence_start_text.as_deref()),
        recurrence_end_text: normalize_text(event.recurrence_end_text.as_deref()),
        recurrence_exceptions: event
            .recurrence_exceptions
            .iter()
            .map(|exception| ReminderExceptionOracle {
                raw: normalize_text(Some(exception.raw.as_str())),
            })
            .collect(),
        attendees: normalize_list(&event.attendees),
    }
}

#[test]
fn preview_capture_matches_intent_oracle_goldens() {
    let Some(parser) = parser() else {
        return;
    };
    let fixtures: Vec<IntentFixture> = load_fixtures("intent/goldens.json");

    for fixture in fixtures {
        let preview = parser.preview_capture(preview_request(
            &fixture.input,
            "America/New_York",
            Some("en-US"),
            None,
        ));
        assert_eq!(
            preview.kind, fixture.oracle.kind,
            "intent fixture {}",
            fixture.id
        );
    }
}

#[test]
fn preview_capture_matches_task_oracle_goldens() {
    let Some(parser) = parser() else {
        return;
    };
    let fixtures: Vec<TaskFixture> = load_fixtures("task/goldens.json");

    for fixture in fixtures {
        let preview = parser.preview_capture(preview_request(
            &fixture.input,
            &fixture.timezone,
            Some("en-US"),
            fixture.known_assignees.clone(),
        ));
        assert_eq!(
            preview.kind, "task",
            "task fixture {} routed to {}",
            fixture.id, preview.kind
        );
        assert_eq!(
            normalize_task_preview(&preview),
            fixture.oracle,
            "task fixture {}",
            fixture.id
        );
    }
}

#[test]
fn preview_capture_matches_reminder_oracle_goldens() {
    let Some(parser) = parser() else {
        return;
    };
    let fixtures: Vec<ReminderFixture> = load_fixtures("reminder/goldens.json");

    for fixture in fixtures {
        let preview = parser.preview_capture(preview_request(
            &fixture.input,
            &fixture.timezone,
            Some("en-US"),
            None,
        ));
        assert_eq!(
            preview.kind, "reminder",
            "reminder fixture {} routed to {}",
            fixture.id, preview.kind
        );
        assert_eq!(
            normalize_reminder_preview(&preview),
            fixture.oracle,
            "reminder fixture {}",
            fixture.id
        );
    }
}

#[test]
fn preview_capture_matches_calendar_oracle_goldens() {
    let Some(parser) = parser() else {
        return;
    };
    let fixtures: Vec<CalendarFixture> = load_fixtures("calendar/goldens.json");

    for fixture in fixtures {
        let preview = parser.preview_capture(preview_request(
            &fixture.input,
            &fixture.timezone,
            Some(&fixture.locale),
            None,
        ));
        assert_eq!(
            preview.kind, "calendar-event",
            "calendar fixture {} routed to {}",
            fixture.id, preview.kind
        );
        assert_eq!(
            normalize_calendar_preview(&preview),
            fixture.oracle,
            "calendar fixture {}",
            fixture.id
        );
    }
}

#[test]
fn preview_capture_routes_task_drafts_through_runtime() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "finish report tomorrow at 3pm with Sara #finance high priority".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: Some(vec!["Sara".to_string()]),
    });

    assert_eq!(preview.kind, "task");
    assert!(preview.confidence > 0.5);
    let task = preview.task.expect("task preview should be present");
    assert_eq!(task.title, "finish report");
    assert_eq!(task.assignees, vec!["Sara"]);
    assert_eq!(task.tags, vec!["finance"]);
    assert_eq!(task.priority.as_deref(), Some("high"));
    assert_eq!(task.due_date_text.as_deref(), Some("tomorrow"));
    assert_eq!(task.due_time_text.as_deref(), Some("3pm"));
}

#[test]
fn preview_capture_preserves_contextual_with_phrases_in_task_titles() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "task: meeting with Sarah next monday at noon".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: Some(vec!["Sarah".to_string()]),
    });

    assert_eq!(preview.kind, "task");
    let task = preview.task.expect("task preview should be present");
    assert_eq!(task.title, "meeting with Sarah");
    assert!(task.assignees.is_empty());
    assert_eq!(task.due_date_text.as_deref(), Some("next monday"));
    assert_eq!(task.due_time_text.as_deref(), Some("noon"));
}

#[test]
fn preview_capture_preserves_imperative_task_title_prefixes() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "task: finally finish the Q2 board deck draft for Project Atlas with Nina and Omar by the first business day next month, high priority, 90 minutes, in progress, tags #finance #board".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: Some(vec!["Nina".to_string(), "Omar".to_string()]),
    });

    assert_eq!(preview.kind, "task");
    let task = preview.task.expect("task preview should be present");
    assert_eq!(task.title, "finally finish the Q2 board deck draft");
    assert_eq!(task.project_hint.as_deref(), Some("Project Atlas"));
    assert_eq!(task.assignees, vec!["Nina", "Omar"]);
}

#[test]
fn preview_capture_routes_reminder_drafts_through_runtime() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "remind me to water plants every other saturday at 9am".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: None,
    });

    assert_eq!(preview.kind, "reminder");
    let reminder = preview
        .reminder
        .expect("reminder preview should be present");
    assert_eq!(reminder.title, "water plants");
    assert_eq!(reminder.remind_at_text.as_deref(), Some("9am"));
    assert_eq!(
        reminder.recurrence.as_ref().map(|value| value.interval),
        Some(2)
    );
}

#[test]
fn preview_capture_routes_calendar_drafts_through_runtime() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "meeting with Sara tomorrow 3pm to 4pm at office".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: None,
    });

    assert_eq!(preview.kind, "calendar-event");
    let event = preview
        .event
        .expect("calendar event preview should be present");
    assert_eq!(event.attendees, vec!["Sara"]);
    assert_eq!(event.date_text.as_deref(), Some("tomorrow"));
    assert_eq!(event.time_text.as_deref(), Some("3pm"));
    assert_eq!(event.end_time_text.as_deref(), Some("4pm"));
    assert_eq!(event.location.as_deref(), Some("office"));
}

#[test]
fn preview_capture_keeps_low_signal_text_as_thought() {
    let Some(parser) = parser() else {
        return;
    };

    let preview = parser.preview_capture(CapturePreviewRequest {
        text: "idea about better shell notifications".to_string(),
        now_iso: Some("2026-05-04T12:00:00.000Z".to_string()),
        timezone: "America/New_York".to_string(),
        locale: Some("en-US".to_string()),
        known_assignees: None,
    });

    assert_eq!(preview.kind, "thought");
    assert_eq!(
        preview
            .thought
            .expect("thought preview should be present")
            .text,
        "idea about better shell notifications"
    );
}
