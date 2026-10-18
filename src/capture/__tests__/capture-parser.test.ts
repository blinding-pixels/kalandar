/// <reference types="node" />

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { test } from 'node:test';

import {
  CAPTURE_PREVIEW_CONTRACT_VERSION,
  CAPTURE_PREVIEW_IMPLEMENTATIONS,
  createCapturePreviewParser,
  parseCapturePreviewInput,
} from '../index.ts';

const now = '2026-03-27T10:00:00Z';
const timezone = 'UTC';

test('parseCapturePreviewInput maps prefixed task input to capture preview DTO', () => {
  const result = parseCapturePreviewInput('task: finish report tomorrow', { now, timezone });

  assert.equal(result.kind, 'task');
  assert.equal(result.confidence, 0.99);
  assert.equal(result.task?.title, 'finish report');
  assert.equal(result.task?.due_date_text, 'tomorrow');
  assert.deepEqual(result.candidates, []);
  assert.equal(result.event, null);
  assert.equal(result.reminder, null);
});

test('parseCapturePreviewInput routes unprefixed calendar text through intent', () => {
  const result = parseCapturePreviewInput('lunch with Sara tomorrow at noon', { now, timezone });

  assert.equal(result.kind, 'calendar-event');
  assert.equal(result.event?.title, 'Lunch with Sara');
  assert.equal(result.event?.date_text, 'tomorrow');
  assert.equal(result.event?.time_text, 'noon');
  assert.equal(result.schedule?.start?.date, '2026-03-28');
  assert.equal(result.schedule?.start?.time, '12:00');
  assert.ok(result.candidates.some((candidate) => candidate.kind === 'calendar-event'));
});

test('default capture parser uses TypeScript implementation', async () => {
  const parser = await createCapturePreviewParser();
  const result = await parser.parse('thought about parser options', { now, timezone });

  assert.equal(parser.mode, 'typescript');
  assert.equal(parser.info.contract_version, CAPTURE_PREVIEW_CONTRACT_VERSION);
  assert.equal(parser.info.boundary, 'in-process');
  assert.equal(parser.info.host, 'javascript');
  assert.equal(result.kind, 'thought');
  assert.equal(result.thought?.text, 'thought about parser options');
});

test('capture parser implementation registry names the public surfaces', () => {
  assert.deepEqual(Object.keys(CAPTURE_PREVIEW_IMPLEMENTATIONS).sort(), ['coreai', 'rust-cli', 'typescript']);
  assert.equal(CAPTURE_PREVIEW_IMPLEMENTATIONS.typescript.contract_version, 'capture-preview-v1');
  assert.equal(CAPTURE_PREVIEW_IMPLEMENTATIONS['rust-cli'].boundary, 'json-stdio-cli');
  assert.equal(CAPTURE_PREVIEW_IMPLEMENTATIONS.coreai.boundary, 'native-host-bridge');
  assert.equal(CAPTURE_PREVIEW_IMPLEMENTATIONS.coreai.status, 'host-required');
});

test('rust-cli capture parser sends request JSON and reads result JSON', async () => {
  const directory = mkdtempSync(path.join(tmpdir(), 'kalandar-capture-test-'));
  const script = path.join(directory, 'fake-rust-cli.mjs');
  writeFileSync(
    script,
    [
      "import { readFileSync } from 'node:fs';",
      "const request = JSON.parse(readFileSync(0, 'utf8'));",
      "const resourceIndex = process.argv.indexOf('--resource-root');",
      "const resourceRoot = resourceIndex >= 0 ? process.argv[resourceIndex + 1] : null;",
      'process.stdout.write(JSON.stringify({',
      "  kind: 'thought',",
      '  confidence: 0.42,',
      '  candidates: [],',
      '  task: null,',
      '  event: null,',
      '  reminder: null,',
      '  thought: { text: `${request.text}|${resourceRoot}` },',
      '  unavailable_reason: null,',
      '  warnings: []',
      '}));',
    ].join('\n'),
  );

  const parser = await createCapturePreviewParser({
    implementation: 'rust-cli',
    command: process.execPath,
    args: [script],
    resourceRoot: directory,
  });
  const result = await parser.parse('hello from rust', { now, timezone });

  assert.equal(parser.mode, 'rust-cli');
  assert.equal(parser.info.contract_version, CAPTURE_PREVIEW_CONTRACT_VERSION);
  assert.equal(parser.info.boundary, 'json-stdio-cli');
  assert.equal(parser.info.host, 'native');
  assert.equal(parser.info.command, process.execPath);
  assert.equal(parser.info.resourceRoot, directory);
  assert.equal(result.kind, 'thought');
  assert.equal(result.thought?.text, `hello from rust|${directory}`);
});

test('coreai capture parser delegates app-facing results through a native host bridge', async () => {
  const requests: unknown[] = [];
  let disposed = false;
  const parser = await createCapturePreviewParser({
    implementation: 'coreai',
    bridge: {
      previewCapture(request) {
        requests.push(request);
        return parseCapturePreviewInput(request);
      },
      dispose() {
        disposed = true;
      },
    },
  });
  const expected = parseCapturePreviewInput('lunch with Sara tomorrow at noon', { now, timezone });
  const actual = await parser.parse('lunch with Sara tomorrow at noon', { now, timezone });

  assert.equal(parser.mode, 'coreai');
  assert.equal(parser.info.contract_version, CAPTURE_PREVIEW_CONTRACT_VERSION);
  assert.equal(parser.info.boundary, 'native-host-bridge');
  assert.equal(parser.info.host, 'apple-native');
  assert.deepEqual(actual, expected);
  assert.deepEqual(requests, [
    {
      text: 'lunch with Sara tomorrow at noon',
      now_iso: now,
      timezone,
      locale: 'en-US',
      known_assignees: null,
    },
  ]);
  await parser.dispose();
  assert.equal(disposed, true);
});

test('capture parser factory rejects unknown implementation names', async () => {
  await assert.rejects(
    () => createCapturePreviewParser({ implementation: 'swift-coreai' } as never),
    /Unsupported capture preview implementation: swift-coreai/,
  );
});

test('copied Rust CLI source references the package binary name', () => {
  const root = new URL('../../..', import.meta.url);
  const manifest = readFileSync(new URL('rust/kalandar/Cargo.toml', root), 'utf8');
  const binarySource = readFileSync(
    new URL('rust/kalandar/src/bin/kalandar-capture-preview.rs', root),
    'utf8',
  );

  assert.match(manifest, /name = "kalandar"/);
  assert.match(binarySource, /CaptureParser::new/);
});
