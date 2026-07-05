import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { parseEventInput } from '../index.ts';

const fixturePath = process.argv[2]
  ? path.resolve(process.cwd(), process.argv[2])
  : path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../fixtures/simple-parser-eval.json');

const fixture = JSON.parse(fs.readFileSync(fixturePath, 'utf8'));
const now = new Date(fixture.meta.now);
const timezone = fixture.meta.timezone;

const pretty = (value) => JSON.stringify(value, null, 2);
let failures = 0;

for (const testCase of fixture.cases) {
  const result = parseEventInput(testCase.text, { now, timezone });
  const actual = result.fields;
  const mismatches = [];

  for (const [key, expectedValue] of Object.entries(testCase.expect)) {
    const actualValue = actual[key];
    if (JSON.stringify(actualValue) !== JSON.stringify(expectedValue)) {
      mismatches.push({ key, expected: expectedValue, actual: actualValue });
    }
  }

  const header = mismatches.length === 0 ? 'PASS' : 'FAIL';
  console.log(`${header} ${testCase.name}`);
  console.log(`  text: ${testCase.text}`);
  if (testCase.notes) {
    console.log(`  notes: ${testCase.notes}`);
  }

  if (mismatches.length === 0) {
    console.log(`  recurrence: ${actual.recurrence}`);
    console.log(`  title: ${actual.title}`);
    continue;
  }

  failures += 1;
  for (const mismatch of mismatches) {
    console.log(`  field: ${mismatch.key}`);
    console.log(`    expected: ${pretty(mismatch.expected)}`);
    console.log(`    actual:   ${pretty(mismatch.actual)}`);
  }
}

if (failures > 0) {
  process.exitCode = 1;
}
