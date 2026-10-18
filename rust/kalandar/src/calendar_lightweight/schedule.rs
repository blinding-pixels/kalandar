use super::js::{normalize_whitespace, slice};
use super::recurrence_exceptions::{classify_recurrence_exception, ClassifyOptions};
use super::ParseContext;
use crate::schedule::{context_for, ScheduleOutput};
use crate::{CapturePreviewRecurrenceException, CapturePreviewRequest, CapturePreviewWarning};
use kalandar_dates::capture::{capture_schedule, CaptureKind, PreviewSchedule, ScheduleSpan};

pub(super) fn schedule_pass(
    ctx: &mut ParseContext,
    request: &CapturePreviewRequest,
    options: &ClassifyOptions,
) {
    let capture = capture_schedule(
        &ctx.raw_input,
        CaptureKind::CalendarEvent,
        None,
        &context_for(request),
    );
    let remaining = remove_schedule_spans(&ctx.masked_input, &capture.spans);
    let output = ScheduleOutput {
        remaining,
        texts: capture.texts,
        schedule: capture.schedule,
        warnings: capture
            .warnings
            .iter()
            .map(|warning| CapturePreviewWarning {
                code: warning.code.to_string(),
                severity: "info".to_string(),
                message: warning.message.clone(),
            })
            .collect(),
    };
    ctx.duration_minutes = output.duration_minutes();
    ctx.alerts = output.alert_offsets();
    ctx.recurrence_exception = output
        .texts
        .recurrence_exceptions
        .iter()
        .map(|raw| classify_recurrence_exception(raw, options))
        .collect();
    let ScheduleOutput {
        remaining,
        texts,
        schedule,
        warnings,
    } = output;
    ctx.date = texts.date;
    ctx.time = texts.time;
    ctx.end_time = texts.end_time;
    ctx.recurrence = texts.recurrence;
    ctx.recurrence_start = texts.recurrence_start;
    ctx.recurrence_end = texts.recurrence_end;
    ctx.warnings.extend(warnings);
    ctx.schedule = schedule;
    ctx.raw_input.clone_from(&remaining);
    ctx.masked_input = remaining;
}

fn remove_schedule_spans(text: &str, spans: &[ScheduleSpan]) -> String {
    let mut ordered: Vec<&ScheduleSpan> = spans.iter().collect();
    ordered.sort_by_key(|span| (span.start, span.end));
    let mut output = String::new();
    let mut cursor = 0;
    for span in ordered {
        if span.start >= cursor {
            output.push_str(slice(text, cursor, span.start));
            output.push(' ');
        }
        cursor = cursor.max(span.end);
    }
    output.push_str(slice(text, cursor, text.len()));
    normalize_whitespace(&output)
}

fn recurrence_part_to_string(value: Option<&'static str>) -> Option<String> {
    value.map(str::to_string)
}

pub(super) fn to_recurrence_exceptions(
    entries: &[super::recurrence_exceptions::StructuredRecurrenceException],
    schedule: Option<&PreviewSchedule>,
) -> Vec<CapturePreviewRecurrenceException> {
    let resolved = schedule
        .and_then(|schedule| schedule.recurrence.as_ref())
        .map(|recurrence| recurrence.exceptions.as_slice())
        .unwrap_or(&[]);
    let same_count = resolved.len() == entries.len();
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            if resolved.is_empty() {
                return CapturePreviewRecurrenceException {
                    raw: entry.raw.clone(),
                    condition: recurrence_part_to_string(entry.condition),
                    action: recurrence_part_to_string(entry.action),
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
