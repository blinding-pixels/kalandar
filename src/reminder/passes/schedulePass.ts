import { captureSchedule, removeScheduleSpans } from '../../dates/index.ts';
import { applyReminderSchedule } from '../schedule.ts';
import type { ReminderParsePass } from '../types.ts';

export const schedulePass: ReminderParsePass = (ctx) => {
  const found = captureSchedule(ctx.working, 'reminder', {
    now: ctx.now,
    timezone: ctx.options.timezone,
  });
  applyReminderSchedule(ctx, found, 'schedulePass');
  ctx.working = removeScheduleSpans(ctx.working, found.spans);
  ctx.maskedInput = ctx.working;
  ctx.result.meta.maskedInput = ctx.working;
};
