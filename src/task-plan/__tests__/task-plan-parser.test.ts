/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { loadMlArtifact } from '../../runtime/index.ts';
import { createTaskDecompositionTestBundle, createTaskTestBundle } from '../../runtime/__tests__/helpers.ts';
import { parseTaskDecompositionInput } from '../../task-decomposition/index.ts';
import { parseTaskInput } from '../../task/index.ts';
import { createTaskPlanParser, parseTaskPlanInput } from '../index.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('task plan parser', () => {
  test('returns only the base task parse by default', () => {
    const input = 'finish the board deck tomorrow at 3 pm';
    const result = parseTaskPlanInput(input, { now, timezone });

    assert.deepEqual(result.task, parseTaskInput(input, { now, timezone }));
    assert.equal(result.decomposition, null);
    assert.equal(result.meta.decomposition_enabled, false);
    assert.equal(result.meta.task_runtime?.mode, 'lightweight');
  });

  test('optionally includes lightweight decomposition without changing the task parse', () => {
    const input = 'Create 2 briefs, one for Anthony and one for Daliah in the branding project by 8 PM next week.';
    const result = parseTaskPlanInput(input, { now, timezone, decompose: true });

    assert.deepEqual(result.task, parseTaskInput(input, { now, timezone }));
    assert.deepEqual(result.decomposition, parseTaskDecompositionInput(input, { now, timezone }));
    assert.equal(result.meta.decomposition_enabled, true);
    assert.equal(result.meta.decomposition_runtime?.mode, 'lightweight');
  });

  test('exposes an async parser object with optional decomposition', async () => {
    const parser = await createTaskPlanParser({
      task: { mode: 'lightweight' },
      decomposition: { mode: 'lightweight' },
    });

    const input = 'Learn jazz standards: Maiden Voyage, Autumn Leaves, and All the Things You Are; for each song learn the melody, harmony, and solos.';
    const withDecomposition = await parser.parse(input, { now, timezone, decompose: true });
    const withoutDecomposition = await parser.parse(input, { now, timezone, decompose: false });

    assert.equal(parser.mode, 'lightweight');
    assert.equal(parser.info.parser, 'task_plan');
    assert.equal(parser.info.task.mode, 'lightweight');
    assert.equal(parser.info.decomposition?.mode, 'lightweight');
    assert.equal(withDecomposition.decomposition?.kind, 'cross_product');
    assert.equal(withDecomposition.decomposition?.children.length, 9);
    assert.equal(withoutDecomposition.decomposition, null);
    assert.equal(withoutDecomposition.meta.decomposition_enabled, false);
  });

  test('composes ML task and decomposition parsers without changing the wrapper contract', async () => {
    const taskBundle = await createTaskTestBundle();
    const decompositionBundle = await createTaskDecompositionTestBundle();
    try {
      const taskArtifact = await loadMlArtifact({ kind: 'directory', path: taskBundle.dir }, 'task');
      const decompositionArtifact = await loadMlArtifact(
        { kind: 'directory', path: decompositionBundle.dir },
        'task_decomposition',
      );

      const taskInput = 'finish report tomorrow at 3 pm';
      const taskEncoding = taskArtifact.tokenizer.encode(taskInput);
      const taskLabelCount = Object.keys(taskArtifact.idToLabel).length;

      const decompositionInput = 'Create 2 briefs, one for Anthony and one for Daliah by 8 PM next week.';
      const canonicalDecompositionInput = decompositionInput.slice(0, -1);
      const decompositionEncoding = decompositionArtifact.tokenizer.encode(canonicalDecompositionInput);
      const decompositionLabelCount = Object.keys(decompositionArtifact.idToLabel).length;

      const parser = await createTaskPlanParser({
        task: {
          mode: 'ml',
          artifact: taskArtifact,
          inference: {
            kind: 'fake-task',
            async run(request) {
              const sequenceLength = request.inputs.input_ids.shape[1];
              const logits = new Array<number>(sequenceLength * taskLabelCount).fill(-8);
              const setLabel = (tokenIndex: number, labelIndex: number) => {
                logits[tokenIndex * taskLabelCount + 0] = -8;
                logits[tokenIndex * taskLabelCount + labelIndex] = 8;
              };

              setLabel(1, 1);
              setLabel(2, 2);
              setLabel(3, 3);
              setLabel(4, 5);
              setLabel(5, 6);
              setLabel(6, 6);

              return {
                outputs: {
                  logits: {
                    dtype: 'float32',
                    shape: [1, sequenceLength, taskLabelCount],
                    data: logits,
                  },
                },
              };
            },
          },
        },
        decomposition: {
          mode: 'ml',
          artifact: decompositionArtifact,
          inference: {
            kind: 'fake-task-decomposition',
            async run(request) {
              const sequenceLength = request.inputs.input_ids.shape[1];
              const logits = new Array<number>(sequenceLength * decompositionLabelCount).fill(-8);
              const setLabelForSpan = (start: number, end: number, label: string): void => {
                for (let index = 0; index < decompositionEncoding.offsets.length; index += 1) {
                  const [tokenStart, tokenEnd] = decompositionEncoding.offsets[index] || [0, 0];
                  if (tokenEnd <= tokenStart) {
                    continue;
                  }
                  if (tokenStart < start || tokenEnd > end) {
                    continue;
                  }
                  const prefix = tokenStart === start ? 'B' : 'I';
                  const labelIndex = decompositionArtifact.labelToId[`${prefix}-${label}`];
                  if (typeof labelIndex !== 'number') {
                    throw new Error(`Missing label ${prefix}-${label}`);
                  }
                  logits[index * decompositionLabelCount + 0] = -8;
                  logits[index * decompositionLabelCount + labelIndex] = 8;
                }
              };

              setLabelForSpan(0, 15, 'parent_title');
              setLabelForSpan(21, 28, 'child_target');
              setLabelForSpan(37, 43, 'child_target');
              setLabelForSpan(47, 49, 'shared_due_time');
              setLabelForSpan(50, 52, 'shared_due_time');
              setLabelForSpan(53, 62, 'shared_due_date');

              return {
                outputs: {
                  logits: {
                    dtype: 'float32',
                    shape: [1, sequenceLength, decompositionLabelCount],
                    data: logits,
                  },
                },
              };
            },
          },
        },
      });

      const taskOnly = await parser.parse(taskInput, { now, timezone });
      assert.equal(taskOnly.task.meta.runtime?.mode, 'ml');
      assert.equal(taskOnly.decomposition, null);
      assert.equal(taskOnly.meta.decomposition_enabled, false);

      const withDecomposition = await parser.parse(decompositionInput, { now, timezone, decompose: true });
      assert.equal(parser.mode, 'ml');
      assert.equal(parser.info.task.mode, 'ml');
      assert.equal(parser.info.decomposition?.mode, 'ml');
      assert.equal(withDecomposition.task.meta.runtime?.mode, 'ml');
      assert.equal(withDecomposition.decomposition?.meta.runtime.mode, 'ml');
      assert.equal(withDecomposition.decomposition?.kind, 'paired_targets');
      assert.equal(withDecomposition.decomposition?.children.length, 2);
    } finally {
      await taskBundle.cleanup();
      await decompositionBundle.cleanup();
    }
  });
});
