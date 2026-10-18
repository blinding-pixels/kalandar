/// <reference types="node" />

import { strict as assert } from 'node:assert';
import fs from 'node:fs/promises';
import { describe, test } from 'node:test';

import { buildReminderDeliverySpec, createReminderParser, reminderResultToPlanningObject } from '../index.ts';
import { loadMlArtifact } from '../../runtime/index.ts';
import { createReminderTestBundle } from '../../runtime/__tests__/helpers.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('reminder ML parser', () => {
  test('returns ReminderParseResult through the app-facing contract with a fake adapter', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const artifact = await loadMlArtifact({ kind: 'directory', path: bundle.dir }, 'reminder');
      const input = 'water plants every other saturday except the week of the 21st and add soil';
      const encoding = artifact.tokenizer.encode(input);
      assert.equal(encoding.inputIds.length, 16);

      const parser = await createReminderParser({
        mode: 'ml',
        artifact,
        inference: {
          kind: 'fake',
          async run(request) {
            const labelCount = 19;
            const sequenceLength = request.inputs.input_ids.shape[1];
            const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
            const setLabel = (tokenIndex: number, labelIndex: number) => {
              logits[tokenIndex * labelCount + 0] = -8;
              logits[tokenIndex * labelCount + labelIndex] = 8;
            };

            setLabel(1, 1);
            setLabel(2, 2);
            setLabel(3, 5);
            setLabel(4, 6);
            setLabel(5, 6);
            setLabel(7, 11);
            setLabel(8, 5);
            setLabel(9, 6);
            setLabel(10, 6);
            setLabel(11, 6);
            setLabel(13, 1);
            setLabel(14, 2);

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

      const result = await parser.parse(input, { now, timezone });

      assert.equal(result.fields.title, 'Water Plants and Add Soil');
      assert.equal(result.fields.remind_at, null);
      assert.equal(result.schedule?.start?.date, '2026-03-28');
      assert.deepEqual(result.fields.recurrence, {
        frequency: 'weekly',
        interval: 2,
        days: ['saturday'],
      });
      assert.deepEqual(result.fields.recurrence_exception, [
        {
          raw: 'except the week of the 21st',
          condition: null,
          action: null,
        },
      ]);
      assert.equal(result.meta.runtime?.mode, 'ml');
      assert.equal(result.meta.runtime?.artifact_id, artifact.artifact_id);
      assert.equal('token_ids' in result, false);
      assert.equal('logits' in result, false);

      const planning = reminderResultToPlanningObject(result);
      assert.equal(planning.kind, 'reminder');
      assert.deepEqual(planning.temporal.recurrence_exception, ['except the week of the 21st']);

      const spec = buildReminderDeliverySpec(planning, { destination: 'notifications' });
      assert.equal(spec.trigger.trigger_time, null);
      assert.deepEqual(spec.schedule.recurrence_exception, ['except the week of the 21st']);
    } finally {
      await bundle.cleanup();
    }
  });

  test('fails loudly on invalid artifact setup', async () => {
    const bundle = await createReminderTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

      await assert.rejects(
        () =>
          createReminderParser({
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
    const bundle = await createReminderTestBundle();
    try {
      const parser = await createReminderParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run() {
            throw new Error('shape mismatch from remote backend');
          },
        },
      });

      await assert.rejects(
        () => parser.parse('remind me to call mom tomorrow at 9am', { now, timezone }),
        /shape mismatch from remote backend/i,
      );
    } finally {
      await bundle.cleanup();
    }
  });
});
