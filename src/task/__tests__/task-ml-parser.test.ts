/// <reference types="node" />

import { strict as assert } from 'node:assert';
import fs from 'node:fs/promises';
import { describe, test } from 'node:test';

import {
  buildTaskCalendarBlockSpec,
  buildTaskReminderDeliverySpec,
  createTaskParser,
  parseTaskInput,
  taskResultToPlanningObject,
} from '../index.ts';
import { loadMlArtifact } from '../../runtime/index.ts';
import { createTaskTestBundle } from '../../runtime/__tests__/helpers.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('task ML parser', () => {
  test('returns TaskParseResult through the app-facing contract with a fake adapter', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const parser = await createTaskParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run(request) {
            const labelCount = 27;
            const sequenceLength = request.inputs.input_ids.shape[1];
            const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
            const setLabel = (tokenIndex: number, labelIndex: number) => {
              logits[tokenIndex * labelCount + 0] = -8;
              logits[tokenIndex * labelCount + labelIndex] = 8;
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
                  shape: [1, sequenceLength, labelCount],
                  data: logits,
                },
              },
            };
          },
        },
      });

      const result = await parser.parse('finish report tomorrow at 3 pm', { now, timezone });

      assert.equal(result.fields.title, 'finish report');
      assert.equal(result.fields.due_date, 'tomorrow');
      assert.equal(result.fields.due_time, 'at 3 pm');
      assert.equal(result.meta.runtime?.mode, 'ml');
      assert.equal('token_ids' in result, false);
      assert.equal('logits' in result, false);

      const planning = taskResultToPlanningObject(result);
      assert.equal(planning.kind, 'task');
      assert.equal(planning.temporal.date, 'tomorrow');
      assert.equal(planning.temporal.time, 'at 3 pm');

      const reminder = buildTaskReminderDeliverySpec(result, { destination: 'notifications' });
      assert.equal(reminder.trigger.trigger_time, 'at 3 pm');
      assert.equal(reminder.schedule.recurrence, null);

      const calendarBlock = buildTaskCalendarBlockSpec(result);
      assert.equal(calendarBlock.date, 'tomorrow');
      assert.equal(calendarBlock.time, 'at 3 pm');
    } finally {
      await bundle.cleanup();
    }
  });

  test('lightweight and ML modes expose the same app-facing task field shape', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const parser = await createTaskParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run(request) {
            const labelCount = 27;
            const sequenceLength = request.inputs.input_ids.shape[1];
            const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
            const setLabel = (tokenIndex: number, labelIndex: number) => {
              logits[tokenIndex * labelCount + 0] = -8;
              logits[tokenIndex * labelCount + labelIndex] = 8;
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
                  shape: [1, sequenceLength, labelCount],
                  data: logits,
                },
              },
            };
          },
        },
      });

      const input = 'finish report tomorrow at 3 pm';
      const lightweight = parseTaskInput(input, { now, timezone });
      const ml = await parser.parse(input, { now, timezone });
      const fieldKeys = Object.keys(lightweight.fields).sort();

      assert.deepEqual(Object.keys(ml.fields).sort(), fieldKeys);

      for (const fieldKey of fieldKeys) {
        const lightweightValue = lightweight.fields[fieldKey as keyof typeof lightweight.fields];
        const mlValue = ml.fields[fieldKey as keyof typeof ml.fields];
        const lightweightShape = Array.isArray(lightweightValue) ? 'array' : lightweightValue === null ? 'null' : typeof lightweightValue;
        const mlShape = Array.isArray(mlValue) ? 'array' : mlValue === null ? 'null' : typeof mlValue;
        assert.equal(mlShape, lightweightShape, `field ${fieldKey} should expose the same app-facing type shape`);
      }
    } finally {
      await bundle.cleanup();
    }
  });

  test('fails loudly on invalid artifact setup', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

      await assert.rejects(
        () =>
          createTaskParser({
            mode: 'ml',
            artifact: { kind: 'directory', path: bundle.dir },
            inference: {
              kind: 'fake',
              async run() {
                throw new Error('should not be called');
              },
            },
          }),
        /artifact_id/i,
      );
    } finally {
      await bundle.cleanup();
    }
  });

  test('does not silently downgrade to lightweight mode on ML inference failure', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const parser = await createTaskParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run() {
            throw new Error('shape mismatch from remote backend');
          },
        },
      });

      await assert.rejects(() => parser.parse('finish report tomorrow at 3 pm', { now, timezone }), /shape mismatch from remote backend/i);
    } finally {
      await bundle.cleanup();
    }
  });

  test('accepts an explicitly loaded task artifact', async () => {
    const bundle = await createTaskTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'task');
      const parser = await createTaskParser({
        mode: 'ml',
        artifact,
        inference: {
          kind: 'fake',
          async run(request) {
            const labelCount = 27;
            const sequenceLength = request.inputs.input_ids.shape[1];
            const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
            const setLabel = (tokenIndex: number, labelIndex: number) => {
              logits[tokenIndex * labelCount + 0] = -8;
              logits[tokenIndex * labelCount + labelIndex] = 8;
            };

            setLabel(1, 1);
            setLabel(2, 2);

            return {
              outputs: {
                logits: {
                  dtype: 'float32',
                  shape: [1, sequenceLength, labelCount],
                  data: logits,
                },
              },
            };
          },
        },
      });

      const result = await parser.parse('finish report', { now, timezone });
      assert.equal(result.meta.runtime?.artifact_id, artifact.artifact_id);
      assert.equal(result.fields.title, 'finish report');
    } finally {
      await bundle.cleanup();
    }
  });
});
