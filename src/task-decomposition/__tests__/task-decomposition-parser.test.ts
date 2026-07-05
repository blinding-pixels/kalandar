/// <reference types="node" />

import { readFileSync } from 'node:fs';
import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createTaskDecompositionParser, parseTaskDecompositionInput } from '../index.ts';
import type { TaskDecompositionResult } from '../types.ts';
import { loadMlArtifact } from '../../runtime/index.ts';
import { createTaskDecompositionTestBundle } from '../../runtime/__tests__/helpers.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

const parse = (input: string) => parseTaskDecompositionInput(input, { now, timezone });

interface FixtureRow {
  id: string;
  bucket?: string;
  text: string;
  expected: {
    should_suggest: boolean;
    kind: string;
    parent_title?: string;
    child_titles?: string[];
    child_due_date_states?: string[];
    child_due_date_values?: Array<string | null>;
    child_due_time_values?: Array<string | null>;
    child_project_states?: string[];
    child_project_values?: Array<string | null>;
  };
}

const loadFixture = (path: string): FixtureRow[] => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8')) as FixtureRow[];

const assertFixtureParity = (row: FixtureRow, result: TaskDecompositionResult): void => {
  assert.equal(result.should_suggest, row.expected.should_suggest, row.id);
  assert.equal(result.kind, row.expected.kind, row.id);
  if (row.expected.parent_title) {
    assert.equal(result.parent?.title, row.expected.parent_title, row.id);
  }

  if (!row.expected.should_suggest) {
    assert.deepEqual(result.children, [], row.id);
    return;
  }

  assert.equal(result.parent?.title, row.expected.parent_title, row.id);
  assert.deepEqual(result.children.map((child) => child.title), row.expected.child_titles || [], row.id);
  assert.deepEqual(result.children.map((child) => child.due_date.state), row.expected.child_due_date_states || [], row.id);
  assert.deepEqual(result.children.map((child) => child.due_date.value), row.expected.child_due_date_values || [], row.id);
  if (row.expected.child_due_time_values) {
    assert.deepEqual(result.children.map((child) => child.due_time.value), row.expected.child_due_time_values, row.id);
  }
  assert.deepEqual(result.children.map((child) => child.project.state), row.expected.child_project_states || [], row.id);
  if (row.expected.child_project_values) {
    assert.deepEqual(result.children.map((child) => child.project.value), row.expected.child_project_values, row.id);
  }
};

describe('lightweight task decomposition parser', () => {
  test('supports parallel flat lists with a shared deadline', () => {
    const result = parse(
      'Plan out future of the YouTube channel, script videos, make the calendar, and assign packaging responsibilities by Friday at 8 PM.',
    );

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'parallel');
    assert.equal(result.parent?.title, 'Plan out future of the YouTube channel');
    assert.deepEqual(result.children.map((child) => child.title), [
      'Script videos',
      'Make the calendar',
      'Assign packaging responsibilities',
    ]);
    assert.deepEqual(result.children.map((child) => child.due_date), [
      { state: 'inherited', value: 'by Friday' },
      { state: 'inherited', value: 'by Friday' },
      { state: 'inherited', value: 'by Friday' },
    ]);
    assert.deepEqual(result.children.map((child) => child.due_time), [
      { state: 'inherited', value: '8 PM' },
      { state: 'inherited', value: '8 PM' },
      { state: 'inherited', value: '8 PM' },
    ]);
  });

  test('supports paired targets with a shared schedule and project', () => {
    const result = parse('Create 2 briefs, one for Anthony and one for Daliah in the branding project by 8 PM next week.');

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'paired_targets');
    assert.equal(result.parent?.title, 'Create 2 briefs');
    assert.deepEqual(result.children.map((child) => child.title), ['Create brief for Anthony', 'Create brief for Daliah']);
    assert.deepEqual(result.children.map((child) => child.due_date), [
      { state: 'inherited', value: 'next week' },
      { state: 'inherited', value: 'next week' },
    ]);
    assert.deepEqual(result.children.map((child) => child.due_time), [
      { state: 'inherited', value: '8 PM' },
      { state: 'inherited', value: '8 PM' },
    ]);
    assert.deepEqual(result.children.map((child) => child.project), [
      { state: 'inherited', value: 'in the branding project' },
      { state: 'inherited', value: 'in the branding project' },
    ]);
  });

  test('supports paired targets with child-specific dates', () => {
    const result = parse('Create 2 briefs, one for Anthony by Tuesday and one for Daliah by Wednesday in the branding project.');

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'paired_targets');
    assert.deepEqual(result.children.map((child) => child.title), ['Create brief for Anthony', 'Create brief for Daliah']);
    assert.deepEqual(result.children.map((child) => child.due_date), [
      { state: 'explicit', value: 'by Tuesday' },
      { state: 'explicit', value: 'by Wednesday' },
    ]);
    assert.deepEqual(result.children.map((child) => child.due_time), [
      { state: 'unspecified', value: null },
      { state: 'unspecified', value: null },
    ]);
    assert.deepEqual(result.children.map((child) => child.project), [
      { state: 'inherited', value: 'in the branding project' },
      { state: 'inherited', value: 'in the branding project' },
    ]);
  });

  test('supports simple cross-product expansion', () => {
    const result = parse(
      'Learn jazz standards: Maiden Voyage, Autumn Leaves, and All the Things You Are; for each song learn the melody, harmony, and solos.',
    );

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'cross_product');
    assert.equal(result.parent?.title, 'Learn jazz standards');
    assert.equal(result.children.length, 9);
    assert.equal(result.children[0].title, 'Learn the melody for Maiden Voyage');
    assert.equal(result.children[8].title, 'Learn the solos for All the Things You Are');
  });

  test('supports referential cross-product expansion', () => {
    const result = parse(
      'One part of my plan to learn jazz standards is to go through form, changes, and head for each of these tunes: So What, Blue Bossa, and Solar.',
    );

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'cross_product');
    assert.equal(result.parent?.title, 'learn jazz standards');
    assert.equal(result.children[0].title, 'Go through form for So What');
    assert.equal(result.children[2].title, 'Go through head for So What');
  });

  test('supports alternate paired targets with project prefix and child-specific dates', () => {
    const result = parse('For the venue refresh project, create packets for Maya and Nina, one due Wednesday and the other due Thursday at 4 PM.');

    assert.equal(result.should_suggest, true);
    assert.equal(result.kind, 'paired_targets');
    assert.equal(result.parent?.title, 'create packets');
    assert.deepEqual(result.children.map((child) => child.title), ['Create packet for Maya', 'Create packet for Nina']);
    assert.deepEqual(result.children.map((child) => child.due_date), [
      { state: 'explicit', value: 'Wednesday' },
      { state: 'explicit', value: 'Thursday' },
    ]);
    assert.deepEqual(result.children.map((child) => child.due_time), [
      { state: 'explicit', value: '4 PM' },
      { state: 'unspecified', value: null },
    ]);
    assert.deepEqual(result.children.map((child) => child.project), [
      { state: 'inherited', value: 'For the venue refresh project' },
      { state: 'inherited', value: 'For the venue refresh project' },
    ]);
  });

  test('preserves recoverable parent titles on abstentions', () => {
    const single = parse('Just finish the poster draft by tomorrow.');
    assert.equal(single.should_suggest, false);
    assert.equal(single.kind, 'none');
    assert.equal(single.parent?.title, 'finish the poster draft');
    assert.deepEqual(single.parent?.due_date, { state: 'explicit', value: 'by tomorrow' });

    const compound = parse('Plan the documentary rollout and then call Sam by Friday.');
    assert.equal(compound.should_suggest, false);
    assert.equal(compound.kind, 'none');
    assert.equal(compound.parent?.title, 'Plan the documentary rollout');
  });

  test('abstains on conditional branch language', () => {
    const result = parse('Deploy update: run tests, if they fail fix errors, if they pass push to production.');

    assert.equal(result.should_suggest, false);
    assert.equal(result.kind, 'none');
    assert.deepEqual(result.children, []);
    assert.ok(result.issues.includes('flow_control_signal'));
  });

  test('abstains on multiple parent tasks in one sentence', () => {
    const result = parse('Morning routine: coffee, check email, then work tasks: finish parser, write tests.');

    assert.equal(result.should_suggest, false);
    assert.equal(result.kind, 'none');
    assert.ok(result.issues.includes('multiple_parent_tasks_signal'));
  });

  test('abstains on nested branch structures', () => {
    const result = parse(
      'Plan tour: book flights, then for NYC show confirm venue (email John, check availability), and for LA line up band, rehearse twice.',
    );

    assert.equal(result.should_suggest, false);
    assert.equal(result.kind, 'none');
    assert.ok(result.issues.includes('nested_branch_signal'));
  });

  test('exposes an async lightweight parser object', async () => {
    const parser = await createTaskDecompositionParser();
    const result = await parser.parse('Create 2 briefs, one for Anthony and one for Daliah by 8 PM next week.', {
      now,
      timezone,
    });

    assert.equal(parser.mode, 'lightweight');
    assert.equal(parser.info.parser, 'task_decomposition');
    assert.equal(result.kind, 'paired_targets');
    assert.deepEqual(result.children.map((child) => child.due_time), [
      { state: 'inherited', value: '8 PM' },
      { state: 'inherited', value: '8 PM' },
    ]);
  });

  test('matches the manual decomposition fixture contract', () => {
    const rows = loadFixture('../../../../training/data/fixtures/task_decomposition_manual_eval.json');

    for (const row of rows) {
      assertFixtureParity(row, parse(row.text));
    }
  });

  test('matches the generated decomposition benchmark contract', () => {
    const rows = loadFixture('../../../../training/data/fixtures/task_decomposition_benchmark_eval.json');

    for (const row of rows) {
      assertFixtureParity(row, parse(row.text));
    }
  });
});

describe('task decomposition ML parser', () => {
  test('returns the app-facing decomposition contract through a fake adapter', async () => {
    const bundle = await createTaskDecompositionTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task_decomposition');
      const input = 'Create 2 briefs, one for Anthony and one for Daliah by 8 PM next week.';
      const canonicalInput = input.slice(0, -1);
      const encoding = artifact.tokenizer.encode(canonicalInput);
      const labelCount = Object.keys(artifact.idToLabel).length;
      const logits = new Array<number>(encoding.inputIds.length * labelCount).fill(-8);
      const setLabelForSpan = (start: number, end: number, label: string): void => {
        for (let index = 0; index < encoding.offsets.length; index += 1) {
          const [tokenStart, tokenEnd] = encoding.offsets[index] || [0, 0];
          if (tokenEnd <= tokenStart) {
            continue;
          }
          if (tokenStart < start || tokenEnd > end) {
            continue;
          }
          const prefix = tokenStart === start ? 'B' : 'I';
          const labelId = artifact.labelToId[`${prefix}-${label}`];
          if (labelId == null) {
            continue;
          }
          const rowOffset = index * labelCount;
          logits[rowOffset + 0] = -8;
          logits[rowOffset + labelId] = 8;
        }
      };

      setLabelForSpan(0, 'Create 2 briefs'.length, 'parent_title');
      setLabelForSpan(canonicalInput.indexOf('Anthony'), canonicalInput.indexOf('Anthony') + 'Anthony'.length, 'child_target');
      setLabelForSpan(canonicalInput.indexOf('Daliah'), canonicalInput.indexOf('Daliah') + 'Daliah'.length, 'child_target');
      setLabelForSpan(canonicalInput.indexOf('8 PM'), canonicalInput.indexOf('8 PM') + '8 PM'.length, 'shared_due_time');
      setLabelForSpan(canonicalInput.indexOf('next week'), canonicalInput.indexOf('next week') + 'next week'.length, 'shared_due_date');

      const parser = await createTaskDecompositionParser({
        mode: 'ml',
        artifact,
        inference: {
          kind: 'fake',
          async run(request) {
            assert.equal(request.parser, 'task_decomposition');
            return {
              outputs: {
                logits: {
                  dtype: 'float32',
                  shape: [1, encoding.inputIds.length, labelCount],
                  data: logits,
                },
              },
            };
          },
        },
      });

      const result = await parser.parse(input, { now, timezone });
      assert.equal(result.meta.runtime.mode, 'ml');
      assert.equal(result.meta.runtime.artifact_id, artifact.artifact_id);
      assert.equal(result.should_suggest, true);
      assert.equal(result.kind, 'paired_targets');
      assert.equal(result.parent?.title, 'Create 2 briefs');
      assert.deepEqual(result.children.map((child) => child.title), ['Create brief for Anthony', 'Create brief for Daliah']);
      assert.deepEqual(result.children.map((child) => child.due_time), [
        { state: 'inherited', value: '8 PM' },
        { state: 'inherited', value: '8 PM' },
      ]);
      assert.deepEqual(result.children.map((child) => child.due_date), [
        { state: 'inherited', value: 'next week' },
        { state: 'inherited', value: 'next week' },
      ]);
    } finally {
      await bundle.cleanup();
    }
  });

  test('does not silently downgrade to lightweight mode on ML inference failure', async () => {
    const bundle = await createTaskDecompositionTestBundle();
    try {
      const parser = await createTaskDecompositionParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run() {
            throw new Error('shape mismatch from backend');
          },
        },
      });

      await assert.rejects(
        () => parser.parse('Create 2 briefs, one for Anthony and one for Daliah by 8 PM next week.', { now, timezone }),
        /shape mismatch from backend/i,
      );
    } finally {
      await bundle.cleanup();
    }
  });
});
