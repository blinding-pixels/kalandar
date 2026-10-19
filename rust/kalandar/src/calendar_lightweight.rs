mod attendees;
mod location;
mod schedule;

use super::{
    js,
    postprocess::finalize_calendar_title,
    recurrence_exceptions::{to_recurrence_exceptions, StructuredRecurrenceException},
    CapturePreviewCalendarEventDraft, CapturePreviewRequest, CapturePreviewResult,
    CapturePreviewWarning,
};
use crate::capture_spans::CaptureSpanSource;
use kalandar_dates::capture::PreviewSchedule;

#[derive(Default)]
pub(crate) struct ParseContext {
    raw_input: String,
    cleaned_input: String,
    masked_input: String,
    title: Option<String>,
    date: Option<String>,
    time: Option<String>,
    end_time: Option<String>,
    duration_minutes: Option<u32>,
    location: Option<String>,
    location_confidence: f64,
    recurrence: Option<String>,
    recurrence_start: Option<String>,
    recurrence_end: Option<String>,
    recurrence_exception: Vec<StructuredRecurrenceException>,
    alerts: Vec<u32>,
    attendees: Option<Vec<String>>,
    warnings: Vec<CapturePreviewWarning>,
    schedule: Option<PreviewSchedule>,
}

impl ParseContext {
    fn new(input: &str) -> Self {
        let cleaned_input = js::normalize_whitespace(input);
        Self {
            raw_input: cleaned_input.clone(),
            masked_input: cleaned_input.clone(),
            cleaned_input,
            ..Self::default()
        }
    }

    fn mask_span(&mut self, start: usize, end: usize) {
        if end <= start || end > self.masked_input.len() {
            return;
        }
        if self.masked_input.is_char_boundary(start) && self.masked_input.is_char_boundary(end) {
            self.masked_input
                .replace_range(start..end, &" ".repeat(end - start));
        }
    }

    fn is_span_available(&self, start: usize, end: usize) -> bool {
        if end <= start || end > self.masked_input.len() {
            return false;
        }
        self.masked_input
            .get(start..end)
            .is_some_and(js::has_non_space)
    }
}

fn is_blank(value: Option<&String>) -> bool {
    value.is_none_or(String::is_empty)
}

fn title_pass(ctx: &mut ParseContext) {
    let title = finalize_calendar_title(&ctx.masked_input, &ctx.cleaned_input);
    ctx.title = (!title.is_empty()).then_some(title);
}

fn finalize_calendar_parse_context(ctx: &mut ParseContext) {
    let mut warnings = Vec::new();
    if is_blank(ctx.date.as_ref())
        && is_blank(ctx.time.as_ref())
        && is_blank(ctx.recurrence.as_ref())
    {
        warnings.push(CapturePreviewWarning {
            code: "calendar-time-missing".to_string(),
            severity: "warning".to_string(),
            message: "No date, time, or recurrence extracted.".to_string(),
        });
    }
    warnings.append(&mut ctx.warnings);
    ctx.warnings = warnings;
}

fn parse_event_input(
    input: &str,
    request: &CapturePreviewRequest,
    spans: &dyn CaptureSpanSource,
) -> ParseContext {
    let mut ctx = ParseContext::new(input);
    attendees::attendees_pass(&mut ctx);
    schedule::schedule_pass(&mut ctx, request, spans);
    location::location_pass(&mut ctx);
    title_pass(&mut ctx);
    finalize_calendar_parse_context(&mut ctx);
    ctx
}

fn calendar_result(ctx: ParseContext) -> CapturePreviewResult {
    let recurrence_exceptions =
        to_recurrence_exceptions(&ctx.recurrence_exception, ctx.schedule.as_ref());
    let event = CapturePreviewCalendarEventDraft {
        title: ctx.title,
        date_text: ctx.date,
        time_text: ctx.time,
        end_time_text: ctx.end_time,
        duration_minutes: ctx.duration_minutes,
        location: ctx.location,
        recurrence_text: ctx.recurrence,
        recurrence_start_text: ctx.recurrence_start,
        recurrence_end_text: ctx.recurrence_end,
        recurrence_exceptions,
        alert_offset_minutes: ctx.alerts,
        attendees: ctx.attendees.unwrap_or_default(),
    };
    CapturePreviewResult::calendar_event(event, 0.99, Vec::new(), ctx.warnings)
        .with_schedule(ctx.schedule)
}

pub fn parse_calendar_preview(
    text: &str,
    request: &CapturePreviewRequest,
    spans: &dyn CaptureSpanSource,
) -> CapturePreviewResult {
    calendar_result(parse_event_input(text, request, spans))
}
