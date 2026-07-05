/// <reference types="node" />

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { absoluteTimePass } from '../passes/absoluteTimePass.ts';
import { chronoFallbackPass } from '../passes/chronoFallbackPass.ts';
import { namedDatePass } from '../passes/namedDatePass.ts';
import { prefixPass } from '../passes/prefixPass.ts';
import { recurrencePass } from '../passes/recurrencePass.ts';
import { scheduleDetailsPass } from '../passes/scheduleDetailsPass.ts';
import { relativeTimePass } from '../passes/relativeTimePass.ts';
import { titlePass } from '../passes/titlePass.ts';
import { createReminderParseContext } from '../utils.ts';

test('reminder passes extract recurrence and time across stages', () => {
  const ctx = createReminderParseContext('remind me every monday at 3pm to call dentist');

  prefixPass(ctx);
  recurrencePass(ctx);
  assert.equal(ctx.result.fields.recurrence?.frequency, 'weekly');
  scheduleDetailsPass(ctx);

  relativeTimePass(ctx);
  absoluteTimePass(ctx);
  assert.ok(ctx.result.fields.remind_at);

  namedDatePass(ctx);
  chronoFallbackPass(ctx);

  titlePass(ctx);
  assert.equal(ctx.result.fields.title, 'Call Dentist');
  assert.ok(ctx.result.meta.debugSteps.length > 0);
});

test('schedule detail pass extracts reminder recurrence boundaries and windows', () => {
  const ctx = createReminderParseContext('attach mailers to phil every wednesday night for 3 months till 9th august');

  prefixPass(ctx);
  recurrencePass(ctx);
  scheduleDetailsPass(ctx);

  assert.equal(ctx.result.fields.recurrence_window, 'for 3 months');
  assert.equal(ctx.result.fields.recurrence_end, 'till 9th august');
  assert.equal(ctx.result.fields.context_hint, 'night');
});

test('schedule detail pass splits chained reminder recurrence exceptions and preserves trailing action text', () => {
  const ctx = createReminderParseContext(
    "water plants every other saturday unless it's a holiday then the following Thursday but skip the first one and add soil",
  );

  prefixPass(ctx);
  recurrencePass(ctx);
  scheduleDetailsPass(ctx);

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
