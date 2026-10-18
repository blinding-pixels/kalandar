import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { initKalandarDates } from '../src/dates/index.ts';
import { captureSchedule } from '../src/dates/wasm/glue.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const evalDir = path.join(root, 'rust', 'kalandar-dates', 'eval');
const requireSwift = process.argv.includes('--require-swift');

const settingsOf = (row) => {
  const settings = row.settings || {};
  const out = {};
  if (settings.day_first) out.dayFirst = true;
  if (settings.week_start) out.weekStart = settings.week_start;
  if (settings.clock) out.clock = settings.clock;
  if (settings.holidays) out.holidays = settings.holidays;
  if (settings.locale) out.locale = settings.locale;
  return out;
};

const requests = [];
for (const file of fs.readdirSync(evalDir).filter((name) => /rows.*\.jsonl$/.test(name)).sort()) {
  for (const line of fs.readFileSync(path.join(evalDir, file), 'utf8').split('\n')) {
    if (!line.trim()) continue;
    const row = JSON.parse(line);
    requests.push({
      id: row.id,
      request: JSON.stringify({
        text: row.text,
        kind: row.kind,
        now: row.now || '2026-10-18T17:00:00Z',
        timezone: row.zone || 'America/Los_Angeles',
        settings: settingsOf(row),
        offsets: 'utf16',
      }),
    });
  }
}
const input = `${requests.map((r) => r.request).join('\n')}\n`;

const cargo = spawnSync(
  'cargo',
  ['run', '--quiet', '--release', '--manifest-path', path.join(root, 'rust', 'Cargo.toml'), '-p', 'kalandar-dates', '--example', 'schedule_lines'],
  { input, encoding: 'utf8', maxBuffer: 1 << 30 },
);
if (cargo.status !== 0) {
  console.error(cargo.stderr);
  process.exit(1);
}
const native = cargo.stdout.trim().split('\n');

initKalandarDates();
const wasm = requests.map((r) => captureSchedule(r.request));

const swiftBin = path.join(root, 'swift', '.build', 'debug', 'kalandar-dates-schedule');
let swift = null;
if (fs.existsSync(swiftBin)) {
  const run = spawnSync(swiftBin, [], { input, encoding: 'utf8', maxBuffer: 1 << 30 });
  swift = run.status === 0 ? run.stdout.trim().split('\n') : null;
} else if (requireSwift) {
  console.error('Swift parity required but swift/.build/debug/kalandar-dates-schedule is missing; run swift build --package-path swift');
  process.exit(1);
}

const failures = [];
requests.forEach((r, index) => {
  if (wasm[index] !== native[index]) failures.push(`${r.id}: WebAssembly differs from native Rust`);
  if (swift && swift[index] !== native[index]) failures.push(`${r.id}: Swift differs from native Rust`);
});
console.log(`dates parity: ${requests.length} rows, WebAssembly ${wasm.length === native.length ? 'checked' : 'length mismatch'}, Swift ${swift ? 'checked' : 'skipped'}`);
if (failures.length || wasm.length !== native.length || (swift && swift.length !== native.length)) {
  for (const failure of failures.slice(0, 20)) console.error(failure);
  console.error(`${failures.length} differences`);
  process.exit(1);
}
