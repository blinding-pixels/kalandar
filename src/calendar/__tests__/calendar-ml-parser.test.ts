/// <reference types="node" />

import { strict as assert } from 'node:assert';
import fs from 'node:fs/promises';
import { describe, test } from 'node:test';

import { createCalendarParser, eventResultToPlanningObject } from '../index.ts';
import { CALENDAR_LABELS, buildBioSequenceDecoderSidecar, createCalendarTestBundle } from '../../runtime/__tests__/helpers.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('calendar ML parser', () => {
  test('decodes first pieces with the decoding the artifact declares', async () => {
    const parseWith = async (decoding: 'argmax' | 'bio') => {
      const bundle = await createCalendarTestBundle({
        decoding,
        ...(decoding === 'bio' ? { sequenceDecoder: buildBioSequenceDecoderSidecar(CALENDAR_LABELS) } : {}),
      });
      try {
        const parser = await createCalendarParser({
          mode: 'ml',
          artifact: { kind: 'directory', path: bundle.dir },
          inference: {
            kind: 'fake',
            async run(request) {
              const labelCount = 21;
              const sequenceLength = request.inputs.input_ids.shape[1];
              const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
              const setLogit = (tokenIndex: number, labelIndex: number, value: number) => {
                logits[tokenIndex * labelCount + labelIndex] = value;
              };
              setLogit(0, 0, 8);
              setLogit(1, 1, 8);
              setLogit(2, 2, 8);
              setLogit(3, 2, 8);
              setLogit(4, 16, 8);
              setLogit(4, 3, 6);
              setLogit(5, 0, 8);
              return { outputs: { logits: { dtype: 'float32', shape: [1, sequenceLength, labelCount], data: logits } } };
            },
          },
        });
        return await parser.parse('lunch with sara tomorrow', { now, timezone });
      } finally {
        await bundle.cleanup();
      }
    };

    const argmax = await parseWith('argmax');
    assert.equal(argmax.fields.location, 'tomorrow');

    const bio = await parseWith('bio');
    assert.equal(bio.fields.location, null);
    assert.equal(bio.fields.title, 'Lunch with Sara');
    assert.notEqual(bio.fields.date, null);
  });

  test('returns EventParseResult through the app-facing contract with a fake adapter', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const parser = await createCalendarParser({
        mode: 'ml',
        artifact: { kind: 'directory', path: bundle.dir },
        inference: {
          kind: 'fake',
          async run(request) {
            const labelCount = 21;
            const sequenceLength = request.inputs.input_ids.shape[1];
            const logits = new Array<number>(sequenceLength * labelCount).fill(-8);
            const setLabel = (tokenIndex: number, labelIndex: number) => {
              logits[tokenIndex * labelCount + 0] = -8;
              logits[tokenIndex * labelCount + labelIndex] = 8;
            };

            setLabel(1, 1);
            setLabel(2, 2);
            setLabel(3, 2);
            setLabel(4, 17);
            setLabel(5, 18);
            setLabel(6, 7);
            setLabel(7, 8);
            setLabel(8, 8);

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

      const result = await parser.parse('lunch with sara every tuesday starting next month', { now, timezone });

      assert.equal(result.fields.title, 'Lunch with Sara');
      assert.equal(result.fields.recurrence, 'every tuesday');
      assert.equal(result.fields.recurrence_start, 'starting next month');
      assert.equal(result.fields.date, null);
      assert.deepEqual(result.fields.contact_suggestions, [{ text: 'Sara', source: 'title_with_phrase' }]);
      assert.equal(result.meta.runtime?.mode, 'ml');
      assert.equal('token_ids' in result, false);
      assert.equal('logits' in result, false);

      const planning = eventResultToPlanningObject(result);
      assert.equal(planning.kind, 'event');
      assert.equal(planning.temporal.recurrence, 'every tuesday');
      assert.deepEqual(planning.contact_suggestions, [{ text: 'Sara', source: 'title_with_phrase' }]);
    } finally {
      await bundle.cleanup();
    }
  });

  test('fails loudly on invalid artifact setup', async () => {
    const bundle = await createCalendarTestBundle();
    try {
      const manifest = JSON.parse(await fs.readFile(bundle.manifestPath, 'utf8'));
      manifest.artifact_id = 'bad-id';
      await fs.writeFile(bundle.manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');

      await assert.rejects(
        () =>
          createCalendarParser({
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
    const bundle = await createCalendarTestBundle();
    try {
      const parser = await createCalendarParser({
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
        () => parser.parse('design review tomorrow at 3 pm', { now, timezone }),
        /shape mismatch from remote backend/i,
      );
    } finally {
      await bundle.cleanup();
    }
  });
});
