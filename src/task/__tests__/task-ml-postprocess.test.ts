/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { postprocessTaskMlEntities } from '../ml.ts';

describe('task ML postprocess', () => {
  test('trims decoded spans and drops assignee noise', () => {
    const processed = postprocessTaskMlEntities('finish report to Sara and', [
      { label: 'assignee', text: ' to Sara ', start: 13, end: 22, confidence: 0.82 },
      { label: 'assignee', text: 'and', start: 23, end: 26, confidence: 0.31 },
    ]);

    assert.deepEqual(processed, [{ label: 'assignee', text: 'Sara', start: 17, end: 21, confidence: 0.82 }]);
  });

  test('merges due_time around bare by fragments', () => {
    const processed = postprocessTaskMlEntities('finish report by 10 AM', [
      { label: 'due_date', text: 'by', start: 14, end: 16, confidence: 0.74 },
      { label: 'due_time', text: '10 AM', start: 17, end: 22, confidence: 0.91 },
    ]);

    assert.deepEqual(processed, [{ label: 'due_time', text: 'by 10 AM', start: 14, end: 22, confidence: 0.91 }]);
  });

  test('merges recurrence exception tails such as of every month', () => {
    const processed = postprocessTaskMlEntities(
      'every tuesday except the second tuesday of every month',
      [
        {
          label: 'recurrence_exception',
          text: 'except the second tuesday of every',
          start: 14,
          end: 49,
          confidence: 0.71,
        },
        { label: 'recurrence', text: 'month', start: 49, end: 54, confidence: 0.66 },
      ],
    );

    assert.deepEqual(processed, [
      {
        label: 'recurrence_exception',
        text: 'except the second tuesday of every month',
        start: 14,
        end: 54,
        confidence: 0.71,
      },
    ]);
  });

  test('merges fragmented priority spans and drops malformed status-before-priority fragments', () => {
    const processed = postprocessTaskMlEntities('with Highpriorty', [
      { label: 'status', text: 'with', start: 0, end: 4, confidence: 0.65 },
      { label: 'priority', text: 'High', start: 5, end: 9, confidence: 0.41 },
      { label: 'status', text: 'pr', start: 9, end: 11, confidence: 0.59 },
      { label: 'status', text: 'ior', start: 11, end: 14, confidence: 0.63 },
      { label: 'status', text: 'ty', start: 14, end: 16, confidence: 0.62 },
    ]);

    assert.deepEqual(processed, [{ label: 'priority', text: 'Highpriorty', start: 5, end: 16, confidence: 0.5625 }]);
  });

  test('normalizes fragmented tags into hashtag tokens', () => {
    const text = 'tags #finance #board';
    const processed = postprocessTaskMlEntities(text, [
      { label: 'tags', text: '#', start: 5, end: 6, confidence: 0.84 },
      { label: 'tags', text: 'finance #board', start: 6, end: 20, confidence: 0.76 },
    ]);

    assert.deepEqual(processed, [
      { label: 'tags', text: '#finance', start: 5, end: 13, confidence: 0.84 },
      { label: 'tags', text: '#board', start: 14, end: 20, confidence: 0.84 },
    ]);
  });
});
