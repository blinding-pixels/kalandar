/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { classifyIntent } from '../index.ts';

describe('intent classifier scoring edge cases', () => {
  test('classifies "meeting with sarah at 3pm" as calendar', () => {
    const result = classifyIntent('meeting with sarah at 3pm');
    assert.equal(result.intent, 'calendar');
    assert.equal(result.scores.reminder, 0);
  });

  test('classifies "buy groceries" as task', () => {
    const result = classifyIntent('buy groceries');
    assert.equal(result.intent, 'task');
    assert.equal(result.scores.reminder, 0);
  });

  test('classifies "call mom" as non-ambiguous task', () => {
    const result = classifyIntent('call mom');
    assert.equal(result.intent, 'task');
    assert.equal(result.ambiguous, false);
  });

  test('routes "remind me to call mom" to reminder', () => {
    const result = classifyIntent('remind me to call mom');
    assert.equal(result.intent, 'reminder');
    assert.equal(result.ambiguous, false);
    assert.ok(result.scores.reminder >= 0.9);
  });

  test('routes reminder-style "alert me" phrasing to reminder', () => {
    const result = classifyIntent('alert me in 10 minutes to take the cake out');
    assert.equal(result.intent, 'reminder');
  });

  test('routes reminder-style "tell me" phrasing to reminder', () => {
    const result = classifyIntent('tell me at 9pm to wind down');
    assert.equal(result.intent, 'reminder');
  });

  test('routes "set a reminder" phrasing to reminder', () => {
    const result = classifyIntent('set a reminder to call mom tomorrow at 6');
    assert.equal(result.intent, 'reminder');
    assert.equal(result.ambiguous, false);
    assert.ok(result.scores.reminder >= 0.9);
  });

  test('routes direct "reminder to" phrasing to reminder', () => {
    const result = classifyIntent('reminder to pay rent on friday');
    assert.equal(result.intent, 'reminder');
    assert.equal(result.ambiguous, false);
    assert.ok(result.scores.reminder >= 0.9);
  });

  test('routes "dentist at 4" to none when uncertain', () => {
    const result = classifyIntent('dentist at 4');
    assert.equal(result.intent, 'none');
    assert.equal(result.ambiguous, true);
    assert.ok(result.scores.calendar > 0);
    assert.equal(result.scores.reminder, 0);
  });

  test('threshold can tighten ambiguity without changing the routed intent', () => {
    const baseline = classifyIntent('doctor appointment friday');
    const strict = classifyIntent('doctor appointment friday', { threshold: 0.7 });

    assert.equal(baseline.intent, 'calendar');
    assert.equal(baseline.ambiguous, false);
    assert.equal(strict.intent, 'calendar');
    assert.equal(strict.ambiguous, true);
    assert.deepEqual(strict.scores, baseline.scores);
    assert.equal(strict.warnings?.[0]?.code, 'intent_ambiguous');
  });

  test('classifies "fix the login bug urgent" as task with high confidence', () => {
    const result = classifyIntent('fix the login bug urgent');
    assert.equal(result.intent, 'task');
    assert.ok(result.scores.task >= 0.8);
  });

  test('classifies "birthday party saturday" as calendar', () => {
    const result = classifyIntent('birthday party saturday');
    assert.equal(result.intent, 'calendar');
  });

  test('classifies "pick up the package tomorrow" as non-ambiguous task', () => {
    const result = classifyIntent('pick up the package tomorrow');
    assert.equal(result.intent, 'task');
    assert.equal(result.ambiguous, false);
  });

  test('routes soft suggestion phrasing like "consider adding blocks to the website" to none', () => {
    const result = classifyIntent('Consider adding blocks to the website');
    assert.equal(result.intent, 'none');
    assert.equal(result.ambiguous, false);
  });

  test('routes vague leisure phrasing like "check out Todds movie at some point" to none', () => {
    const result = classifyIntent('Check out Todds movie at some point');
    assert.equal(result.intent, 'none');
    assert.equal(result.ambiguous, false);
  });

  test('routes feature-fragment phrasing like "auto share the graphs" to none', () => {
    const result = classifyIntent('auto share the graphs');
    assert.equal(result.intent, 'none');
    assert.equal(result.ambiguous, false);
  });

  test('keeps direct imperative phrasing like "add blocks to the website" on task', () => {
    const result = classifyIntent('Add blocks to the website');
    assert.equal(result.intent, 'task');
    assert.equal(result.ambiguous, false);
  });

  test('keeps direct imperative phrasing like "share the graphs" on task', () => {
    const result = classifyIntent('Share the graphs');
    assert.equal(result.intent, 'task');
    assert.equal(result.ambiguous, false);
  });

  test('classifies "schedule a call with vendor" as task (no concrete time)', () => {
    const result = classifyIntent('schedule a call with vendor');
    // "schedule" by itself does not promote to calendar without a specific time/date.
    assert.equal(result.intent, 'task');
  });

  test('keeps work-framed time blocks as task using shadow normalization', () => {
    const result = classifyIntent('brainstorm ideas tmrw for 2 hrs starting at 7');
    assert.equal(result.intent, 'task');
    assert.equal(result.ambiguous, false);
  });

  test('keeps occurrence-framed sessions as calendar', () => {
    const result = classifyIntent('brainstorming session at 7 for 2 hrs');
    assert.equal(result.intent, 'calendar');
  });

  test('classifies empty input as none', () => {
    const result = classifyIntent('');
    assert.equal(result.intent, 'none');
    assert.ok(result.scores.none >= 0.3);
  });

  test('classifies "???" as none and marks ambiguous', () => {
    const result = classifyIntent('???');
    assert.equal(result.intent, 'none');
    assert.equal(result.ambiguous, true);
  });
});
