/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { mergeReminderMlTitleValue, postprocessReminderMlEntities } from '../ml.ts';

describe('reminder ML postprocess', () => {
  test('merges exception fragments across adjacent recurrence_exception and trigger_time spans', () => {
    const text = 'water plants every monday except tomorrow at 9 am';
    const exceptionStart = text.indexOf('except tomorrow');
    const triggerStart = text.indexOf('at 9 am');

    const processed = postprocessReminderMlEntities(text, [
      {
        label: 'recurrence_exception',
        text: 'except tomorrow',
        start: exceptionStart,
        end: exceptionStart + 'except tomorrow'.length,
        confidence: 0.82,
      },
      {
        label: 'trigger_time',
        text: 'at 9 am',
        start: triggerStart,
        end: triggerStart + 'at 9 am'.length,
        confidence: 0.74,
      },
    ]);

    assert.deepEqual(processed, [
      {
        label: 'recurrence_exception',
        text: 'except tomorrow at 9 am',
        start: exceptionStart,
        end: triggerStart + 'at 9 am'.length,
        confidence: 0.78,
      },
    ]);
  });

  test('expands exception prefixes when decoding starts after except', () => {
    const text = 'Water Plants every other saturday except the week of the 21st and add soil';
    const theStart = text.indexOf('the week');
    const weekStart = text.indexOf('week of the 21st');

    const processed = postprocessReminderMlEntities(text, [
      {
        label: 'title',
        text: 'Water Plants',
        start: 0,
        end: 'Water Plants'.length,
        confidence: 0.9303,
      },
      {
        label: 'recurrence',
        text: 'every other saturday',
        start: text.indexOf('every other saturday'),
        end: text.indexOf('every other saturday') + 'every other saturday'.length,
        confidence: 0.8784,
      },
      {
        label: 'recurrence_exception',
        text: 'the',
        start: theStart,
        end: theStart + 'the'.length,
        confidence: 0.3686,
      },
      {
        label: 'recurrence',
        text: 'week of the 21st',
        start: weekStart,
        end: weekStart + 'week of the 21st'.length,
        confidence: 0.6099,
      },
      {
        label: 'title',
        text: 'add soil',
        start: text.indexOf('add soil'),
        end: text.indexOf('add soil') + 'add soil'.length,
        confidence: 0.9081,
      },
    ]);

    assert.equal(processed.length, 4);
    assert.deepEqual(processed[0], {
      label: 'title',
      text: 'Water Plants',
      start: 0,
      end: 'Water Plants'.length,
      confidence: 0.9303,
    });
    assert.deepEqual(processed[1], {
      label: 'recurrence',
      text: 'every other saturday',
      start: text.indexOf('every other saturday'),
      end: text.indexOf('every other saturday') + 'every other saturday'.length,
      confidence: 0.8784,
    });
    assert.equal(processed[2]?.label, 'recurrence_exception');
    assert.equal(processed[2]?.text, 'except the week of the 21st');
    assert.equal(processed[2]?.start, text.indexOf('except the week of the 21st'));
    assert.equal(processed[2]?.end, text.indexOf('except the week of the 21st') + 'except the week of the 21st'.length);
    assert.ok(Math.abs((processed[2]?.confidence || 0) - 0.48925) < 0.0002);
    assert.deepEqual(processed[3], {
      label: 'title',
      text: 'add soil',
      start: text.indexOf('add soil'),
      end: text.indexOf('add soil') + 'add soil'.length,
      confidence: 0.9081,
    });
  });

  test('merges fragmented title spans while preserving and/or joiners', () => {
    const text = 'Water Plants every other saturday except the week of the 21st and add soil';
    const merged = mergeReminderMlTitleValue(text, [
      {
        label: 'title',
        text: 'Water Plants',
        start: 0,
        end: 'Water Plants'.length,
        confidence: 0.9303,
      },
      {
        label: 'title',
        text: 'add soil',
        start: text.indexOf('add soil'),
        end: text.indexOf('add soil') + 'add soil'.length,
        confidence: 0.9081,
      },
    ]);

    assert.equal(merged, 'Water Plants and add soil');
  });
});
