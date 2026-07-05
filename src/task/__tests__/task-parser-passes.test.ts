/// <reference types="node" />

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { assigneePass } from '../passes/assigneePass.ts';
import { dateTypoCorrectionPass } from '../passes/dateTypoCorrectionPass.ts';
import { durationPass } from '../passes/durationPass.ts';
import { priorityPass } from '../passes/priorityPass.ts';
import { projectPass } from '../passes/projectPass.ts';
import { schedulePass } from '../passes/schedulePass.ts';
import { statusPass } from '../passes/statusPass.ts';
import { tagsPass } from '../passes/tagsPass.ts';
import { titlePass } from '../passes/titlePass.ts';
import { createTaskParseContext } from '../utils.ts';
import { parseTaskInput } from '../index.ts';

test('task passes extract the newer small-app contract in order', () => {
  const ctx = createTaskParseContext(
    'urgent fix login for Project Atlas with @sam by friday, 30 minutes, in progress, tags #ops',
    {
      now: '2026-03-22T12:00:00-04:00',
      timezone: 'America/New_York',
    },
  );

  priorityPass(ctx);
  assert.equal(ctx.result.fields.priority, 'high');

  statusPass(ctx);
  assert.equal(ctx.result.fields.status, 'in progress');

  durationPass(ctx);
  assert.equal(ctx.result.fields.duration, '30 minutes');

  tagsPass(ctx);
  assert.deepEqual(ctx.result.fields.tags, ['#ops']);

  projectPass(ctx);
  assert.equal(ctx.result.fields.project, 'Project Atlas');

  assigneePass(ctx);
  assert.deepEqual(ctx.result.fields.assignee, ['@sam']);

  dateTypoCorrectionPass(ctx);
  schedulePass(ctx);
  assert.equal(ctx.result.fields.due_date, 'by friday');

  titlePass(ctx);
  assert.equal(ctx.result.fields.title, 'fix login');
  assert.ok(ctx.result.meta.debugSteps.length > 0);
});

test('task parser removes recurring spans from the title when recurrence is extracted', () => {
  const result = parseTaskInput('send emails every tuesday at 7 pm', {
    now: '2026-03-22T12:00:00-04:00',
    timezone: 'America/New_York',
  });

  assert.equal(result.fields.title, 'send emails');
  assert.equal(result.fields.recurrence, 'every tuesday');
  assert.equal(result.fields.due_time, 'at 7 pm');
});

test('task parser keeps collaboration-style "meeting with sarah" in the title instead of treating it as assignees', () => {
  const result = parseTaskInput('meeting with sarah next monday at noon', {
    now: '2026-03-27T10:00:00',
    timezone: 'America/New_York',
  });

  assert.deepEqual(result.fields.assignee, []);
  assert.equal(result.fields.title, 'meeting with sarah');
});

test('task parser trims clause boundaries so project and assignee values do not keep leading prepositions', () => {
  const result = parseTaskInput(
    'finally finish the Q2 board deck draft for Project Atlas with Nina and Omar by the first business day next month',
    {
      now: '2026-03-27T10:00:00',
      timezone: 'America/New_York',
    },
  );

  assert.equal(result.fields.project, 'Project Atlas');
  assert.deepEqual(result.fields.assignee, ['Nina', 'Omar']);
});
