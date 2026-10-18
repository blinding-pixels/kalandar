use super::ParseContext;
use crate::recurrence_exceptions::classify_recurrence_exception;
use crate::schedule::{context_for, ScheduleOutput};
use crate::CapturePreviewRequest;
use kalandar_dates::capture::{capture_schedule, CaptureKind};

pub(super) fn schedule_pass(ctx: &mut ParseContext, request: &CapturePreviewRequest) {
    let context = context_for(request);
    let capture = capture_schedule(&ctx.raw_input, CaptureKind::CalendarEvent, None, &context);
    let output = ScheduleOutput::new(capture, &ctx.masked_input);
    ctx.duration_minutes = output.duration_minutes();
    ctx.alerts = output.alert_offsets();
    ctx.recurrence_exception = output
        .texts
        .recurrence_exceptions
        .iter()
        .map(|raw| classify_recurrence_exception(raw, &context))
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
