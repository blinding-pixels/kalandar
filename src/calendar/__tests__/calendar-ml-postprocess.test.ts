/// <reference types="node" />

import { strict as assert } from 'node:assert';
import { describe, test } from 'node:test';

import { createCalendarParser } from '../index.ts';
import { postprocessCalendarMlEntities } from '../ml.ts';
import { createCalendarTestBundle } from '../../runtime/__tests__/helpers.ts';

const now = '2026-03-27T10:00:00-04:00';
const timezone = 'America/New_York';

describe('calendar ML postprocess', () => {
  test('trims decoded spans and structures recurrence exceptions in TS', () => {
    const processed = postprocessCalendarMlEntities(
      [
        {
          label: 'recurrence_exception',
          text: " unless it's a holiday then the following Thursday starting next month, ",
          start: 10,
          end: 87,
          confidence: 0.94,
        },
      ],
      { now, timezone },
    );

    assert.deepEqual(processed, [
      {
        label: 'recurrence_exception',
        text: "unless it's a holiday then the following Thursday",
        start: 11,
        end: 60,
        confidence: 0.94,
        value: {
          raw: "unless it's a holiday then the following Thursday",
          condition: { type: 'holiday' },
          action: { type: 'shift', to: 'thursday', direction: 'following' },
        },
      },
    ]);
  });

  test('surfaces contact suggestions and warning generation through the ML runtime path', async () => {
    const bundle = await createCalendarTestBundle({ extraVocab: ['starting', 'july', '1st'] });
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

      const result = await parser.parse('lunch with sara every tuesday starting july 1st', { now, timezone });
      assert.deepEqual(result.fields.contact_suggestions, [{ text: 'Sara', source: 'title_with_phrase' }]);
      assert.ok(result.warnings);
      assert.ok(result.warnings?.some((warning) => warning.code === 'start_adjusted' && warning.severity === 'info'));
    } finally {
      await bundle.cleanup();
    }
  });
});
