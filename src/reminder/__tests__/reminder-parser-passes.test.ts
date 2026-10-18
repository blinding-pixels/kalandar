/// <reference types="node" />

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { prefixPass } from '../passes/prefixPass.ts';
import { schedulePass } from '../passes/schedulePass.ts';
import { titlePass } from '../passes/titlePass.ts';
import { createReminderParseContext } from '../utils.ts';

test('reminder passes extract recurrence and time across stages', () => {
  const ctx = createReminderParseContext('remind me every monday at 3pm to call dentist');

  prefixPass(ctx);
  schedulePass(ctx);
  assert.equal(ctx.result.fields.recurrence?.frequency, 'weekly');
  assert.ok(ctx.result.fields.remind_at);

  titlePass(ctx);
  assert.equal(ctx.result.fields.title, 'Call Dentist');
  assert.ok(ctx.result.meta.debugSteps.length > 0);
});

test('schedule pass extracts reminder recurrence boundaries and windows', () => {
  const ctx = createReminderParseContext('attach mailers to phil every wednesday night for 3 months till 9th august');

  prefixPass(ctx);
  schedulePass(ctx);

  assert.equal(ctx.result.fields.recurrence_window, 'for 3 months');
  assert.equal(ctx.result.fields.recurrence_end, 'till 9th august');
  assert.equal(ctx.result.fields.context_hint, 'night');
});

test('schedule pass splits chained reminder recurrence exceptions and preserves trailing action text', () => {
  const ctx = createReminderParseContext(
    "water plants every other saturday unless it's a holiday then the following Thursday but skip the first one and add soil",
  );

  prefixPass(ctx);
  schedulePass(ctx);

  assert.deepEqual(ctx.result.fields.recurrence_exception, [
    {
      raw: "unless it's a holiday then the following Thursday",
      condition: { type: 'holiday' },
      action: { type: 'shift', to: 'thursday', direction: 'following' },
    },
    {
      raw: 'but skip the first one',
      condition: { type: 'ordinal', which: 'first' },
      action: { type: 'skip' },
    },
  ]);
  assert.match(ctx.working, /\band add soil\b/i);
});
