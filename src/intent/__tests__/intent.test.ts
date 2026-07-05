/// <reference types="node" />

import { existsSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { classifyIntent } from '../index.ts';

type IntentType = 'task' | 'calendar' | 'none' | 'reminder' | 'event' | 'note' | 'ambiguous' | 'calendar_event';

interface DatasetExample {
  id: string;
  input: string;
  expected_intent: IntentType;
  ambiguous: boolean;
}

interface DatasetTier {
  description: string;
  examples: DatasetExample[];
}

interface DatasetFile {
  tiers: Record<string, DatasetTier>;
}

const datasetPath = new URL('../../../../../working files/merged-intent-classifier.json', import.meta.url);
const dataset = existsSync(datasetPath) ? (JSON.parse(readFileSync(datasetPath, 'utf8')) as DatasetFile) : null;
const datasetTest = dataset ? test : test.skip;

datasetTest('classifyIntent matches the merged dataset', () => {
  const datasetFile = dataset as DatasetFile;
  for (const [tierName, tier] of Object.entries(datasetFile.tiers)) {
    for (const example of tier.examples) {
      const result = classifyIntent(example.input);
      try {
        const expectedIntent =
          example.expected_intent === 'calendar_event'
            ? 'calendar'
            : example.expected_intent === 'event'
              ? 'calendar'
              : example.expected_intent === 'note'
                ? 'none'
                : example.expected_intent;
        assert.equal(result.intent, expectedIntent, `${example.id} expected ${expectedIntent} but got ${result.intent} for "${example.input}"`);
        assert.equal(
          result.ambiguous,
          example.ambiguous,
          `${example.id} expected ambiguous=${example.ambiguous} but got ${result.ambiguous}`,
        );
      } catch (error) {
        const details = [
          `${tierName}:${example.id} "${example.input}"`,
          `expected intent=${example.expected_intent} ambiguous=${example.ambiguous}`,
          `received intent=${result.intent} ambiguous=${result.ambiguous} topTwoGap=${result.meta.topTwoGap.toFixed(2)}`,
          `scores=${JSON.stringify(result.scores)}`,
        ].join(' | ');
        throw new assert.AssertionError({
          message: details,
          actual: error,
          expected: 'matching classification',
          operator: 'classifyIntent',
        });
      }
    }
  }
});
