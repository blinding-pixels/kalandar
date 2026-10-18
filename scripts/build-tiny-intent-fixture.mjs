import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { parseCapturePreviewInput } from '../src/capture/index.ts';
import { classifyIntent } from '../src/intent/index.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const kalandarFixturesRoot = path.join(root, 'rust', 'kalandar', 'tests', 'fixtures', 'kalandar');
const appParityLockPath = path.join(root, 'coreai', 'app-parity-lock.json');
const intentTestsRoot = path.join(root, 'src', 'intent', '__tests__');
const trainingDataRoot = process.env.KALANDAR_INTENT_TRAINING_DATA
  || path.resolve(root, '..', '..', 'runtime-split', 'retrain2-out', 'training', 'data');
const intentTrainingPath = path.resolve(root, '..', 'intent-training.json');
const outputPath = path.join(root, 'rust', 'kalandar', 'tests', 'fixtures', 'tiny_intent_parity.json');
const trainingSampleSize = 1500;
const captureOptions = { now: '2026-05-16T12:00:00.000Z', timezone: 'America/New_York', locale: 'en-US' };

const EDGE_CASES = [
  '',
  '   ',
  'task: finish report',
  'meeting with sara tomorrow at 3pm',
  'meeting\u0085with sara tomorrow at 3pm',
  'lunch﻿with sara at noon',
  'dentist at 4',
  'Meeting 3–5pm tomorrow',
  'meeting 3 - 5pm tomorrow',
  'meeting 3—5 tomorrow',
  'call mom 😀 tomorrow at 5',
  'gym ❤️ at 6am',
  '📅 standup at 9',
  'buy 𝐀𝐁 milk',
  'renew 𠜎 passport friday',
  'review the ſpec next week',
  'meeting with Kate at 2pm',
  'MEETING WITH SARA TMRW AT 3PM',
  'ping ME at 5',
  'Remind Me to stretch',
  'İstanbul trip next month',
  'café meeting at 10',
  'naïve plan for wed',
  '¿coffee at 3?',
  'workout 30mins at 7am',
  'focus block for 2hrs starting at 9',
  'write the blog post for 45 min starting at 1pm',
  'pls lol finish report',
  'uhhh dinner at 7',
  'random thought about pricing!!!',
  '(lunch) [with] {sara} "at" noon',
  'every other tues standup',
  'pay rent on the 1st',
  'flight to nyc 12/24',
  'trip march 3-5',
  'ΣΟΣ meeting at 2',
  'sync w/ team 10am tomorrow',
  'yoga\tclass\nat 6',
];

const readJson = (filePath) => JSON.parse(fs.readFileSync(filePath, 'utf8'));

const normalizeWhitespace = (input) => input.replace(/\s+/g, ' ').trim();

const isPrefixed = (text) => ['task:', 'reminder:', 'event:'].some((prefix) => text.slice(0, prefix.length).toLowerCase() === prefix);

const fnv1a32 = (value) => {
  let hashed = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(value)) {
    hashed ^= byte;
    hashed = Math.imul(hashed, 0x01000193) >>> 0;
  }
  return hashed >>> 0;
};

const unescapeLiteral = (value) => value.replace(/\\(.)/g, '$1');

const goldenInputs = () =>
  fs.readdirSync(kalandarFixturesRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(kalandarFixturesRoot, entry.name, 'goldens.json'))
    .filter((filePath) => fs.existsSync(filePath))
    .sort()
    .flatMap((filePath) => readJson(filePath).map((row) => row.input));

const appParityInputs = () => readJson(appParityLockPath).cases.map((row) => row.input);

const intentTestInputs = () =>
  fs.readdirSync(intentTestsRoot)
    .filter((name) => name.endsWith('.test.ts'))
    .sort()
    .flatMap((name) => {
      const source = fs.readFileSync(path.join(intentTestsRoot, name), 'utf8');
      const arrayEntries = [...source.matchAll(/^\s+(['"])((?:\\.|(?!\1).)*)\1,$/gm)].map((match) => match[2]);
      const callArguments = [...source.matchAll(/classifyIntent\((['"])((?:\\.|(?!\1).)*)\1/g)].map((match) => match[2]);
      return [...arrayEntries, ...callArguments].map(unescapeLiteral);
    });

const trainingSample = () => {
  if (!fs.existsSync(trainingDataRoot)) {
    console.warn(`skipping training sample: ${trainingDataRoot} not found`);
    return [];
  }
  const texts = new Set();
  for (const family of ['task', 'calendar', 'reminder']) {
    const familyRoot = path.join(trainingDataRoot, family);
    if (!fs.existsSync(familyRoot)) continue;
    for (const name of fs.readdirSync(familyRoot).filter((file) => file.endsWith('.jsonl')).sort()) {
      for (const line of fs.readFileSync(path.join(familyRoot, name), 'utf8').split('\n')) {
        if (!line.trim()) continue;
        const row = JSON.parse(line);
        if (typeof row.text === 'string') texts.add(row.text);
      }
    }
  }
  return [...texts]
    .map((text) => ({ text, key: fnv1a32(text) }))
    .sort((left, right) => left.key - right.key || (left.text < right.text ? -1 : left.text > right.text ? 1 : 0))
    .slice(0, trainingSampleSize)
    .map((entry) => entry.text);
};

const intentTrainingInputs = () =>
  fs.existsSync(intentTrainingPath)
    ? readJson(intentTrainingPath).map((row) => row.text).filter((text) => typeof text === 'string')
    : [];

const corpus = [
  ...new Set([
    ...EDGE_CASES,
    ...goldenInputs(),
    ...appParityInputs(),
    ...intentTestInputs(),
    ...intentTrainingInputs(),
    ...trainingSample(),
  ]),
];

const toWarnings = (warnings) => (warnings || []).map(({ code, severity, message }) => ({ code, severity, message }));

const cases = [];
let skipped = 0;
for (const text of corpus) {
  const normalized = normalizeWhitespace(text);
  if (!normalized || isPrefixed(normalized)) {
    skipped += 1;
    continue;
  }
  const intent = classifyIntent(normalized);
  const preview = parseCapturePreviewInput(text, captureOptions);
  const intentWarnings = toWarnings(intent.warnings);
  const previewIntentWarnings = preview.warnings.slice(0, intentWarnings.length);
  if (JSON.stringify(previewIntentWarnings) !== JSON.stringify(intentWarnings)) {
    throw new Error(`intent warnings are not the preview warning prefix for ${JSON.stringify(text)}`);
  }
  cases.push({
    text,
    kind: preview.kind,
    confidence: preview.confidence,
    candidates: preview.candidates,
    warnings: intentWarnings,
    intent: intent.intent,
    scores: intent.scores,
    ambiguous: intent.ambiguous,
    topTwoGap: intent.meta.topTwoGap,
  });
}

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, `${JSON.stringify({ version: 1, cases }, null, 2)}\n`);
console.log(`wrote ${cases.length} cases to ${path.relative(root, outputPath)} (skipped ${skipped} empty or prefixed inputs)`);
