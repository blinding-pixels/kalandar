import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { parseCapturePreviewInput } from '../src/capture/index.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const evalDir = path.join(root, 'rust', 'kalandar-dates', 'eval');
const fixturesDir = path.join(root, 'rust', 'kalandar', 'tests', 'fixtures', 'kalandar');
const requireSwift = process.argv.includes('--require-swift');
const PREFIX = { task: 'task: ', reminder: 'reminder: ', calendar: 'event: ' };
const NOW = '2026-10-18T17:00:00Z';

const familyOf = (row, file) => {
  if (row.kind && PREFIX[row.kind]) return row.kind;
  const match = file.match(/gold_(task|reminder|calendar)/);
  return match ? match[1] : null;
};

const texts = [];
for (const file of fs.readdirSync(evalDir).filter((name) => name.endsWith('.jsonl')).sort()) {
  for (const line of fs.readFileSync(path.join(evalDir, file), 'utf8').split('\n')) {
    if (!line.trim()) continue;
    const row = JSON.parse(line);
    if (row.text) texts.push({ text: row.text, family: familyOf(row, file) });
  }
}
for (const family of ['intent', 'task', 'reminder', 'calendar']) {
  for (const fixture of JSON.parse(fs.readFileSync(path.join(fixturesDir, family, 'goldens.json'), 'utf8'))) {
    texts.push({ text: fixture.input, family: PREFIX[family] ? family : null });
  }
}

const requests = [];
const seen = new Set();
for (const { text, family } of texts) {
  for (const variant of [text, family ? `${PREFIX[family]}${text}` : null]) {
    if (!variant || seen.has(variant)) continue;
    seen.add(variant);
    requests.push({ text: variant, now_iso: NOW, timezone: 'America/Los_Angeles', locale: 'en-US', known_assignees: null });
  }
}
const input = `${requests.map((request) => JSON.stringify(request)).join('\n')}\n`;

const canonical = (value) => {
  if (Array.isArray(value)) return value.map(canonical);
  if (typeof value === 'number') return Number(value.toPrecision(12));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([, entry]) => entry !== null && entry !== undefined)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, entry]) => [key, canonical(entry)]),
    );
  }
  return value;
};
const key = (value) => JSON.stringify(canonical(value));

const rust = spawnSync(
  'cargo',
  [
    'run', '--quiet', '--release', '--no-default-features',
    '--manifest-path', path.join(root, 'rust', 'Cargo.toml'),
    '-p', 'kalandar', '--example', 'rules_lines',
  ],
  { input, encoding: 'utf8', maxBuffer: 1 << 30 },
);
if (rust.status !== 0) {
  console.error(rust.stderr);
  process.exit(1);
}
const rustResults = rust.stdout.trim().split('\n').map((line) => JSON.parse(line));
if (rustResults.length !== requests.length) {
  console.error(`rules parity: Rust returned ${rustResults.length} results for ${requests.length} requests`);
  process.exit(1);
}

const failures = [];
requests.forEach((request, index) => {
  const js = parseCapturePreviewInput(request);
  if (key(js) !== key(rustResults[index])) {
    failures.push(`js vs rust: ${request.text}\n  js:   ${key(js)}\n  rust: ${key(rustResults[index])}`);
  }
});

const swiftBin = path.join(root, 'swift', '.build', 'debug', 'kalandar-coreai-capture-preview');
let swiftChecked = false;
if (fs.existsSync(swiftBin)) {
  const env = { ...process.env };
  delete env.KALANDAR_COREAI_ARTIFACT_ROOT;
  const swift = spawnSync(swiftBin, [], { input, encoding: 'utf8', maxBuffer: 1 << 30, env });
  if (swift.status !== 0) {
    failures.push(`swift command failed: ${swift.stderr}`);
  } else {
    const swiftResults = swift.stdout.trim().split('\n').map((line) => JSON.parse(line));
    requests.forEach((request, index) => {
      if (key(swiftResults[index]) !== key(rustResults[index])) {
        failures.push(`swift vs rust: ${request.text}\n  swift: ${key(swiftResults[index])}\n  rust:  ${key(rustResults[index])}`);
      }
    });
    swiftChecked = true;
  }
} else if (requireSwift) {
  failures.push(`swift parity required but ${swiftBin} is missing; run swift build --package-path swift`);
}

if (failures.length > 0) {
  console.error(failures.slice(0, 20).join('\n'));
  console.error(`rules parity: ${failures.length} mismatches over ${requests.length} requests`);
  process.exit(1);
}
console.log(`rules parity: ${requests.length} requests, JavaScript and Rust agree${swiftChecked ? ', Swift checked' : ''}`);
