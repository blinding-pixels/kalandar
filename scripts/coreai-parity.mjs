import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { parseCapturePreviewInput } from '../src/capture/index.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = path.resolve(root, '..');
const fixturesRoot = path.join(root, 'rust', 'kalandar', 'tests', 'fixtures', 'kalandar');
const jsLockPath = path.join(root, 'coreai', 'app-parity-lock.json');
const args = process.argv.slice(2);

const readJson = (filePath) => JSON.parse(fs.readFileSync(filePath, 'utf8'));

const hasArg = (name) => args.includes(name);

const valueArg = (name, envName) => {
  const index = args.indexOf(name);
  if (index >= 0) {
    return args[index + 1];
  }
  return envName ? process.env[envName] : undefined;
};

const firstExistingPath = (paths) => paths.find((candidate) => fs.existsSync(candidate));

const defaultRustEnvPath = firstExistingPath([
  path.join(root, '.build', 'rust-parity', 'env.json'),
  path.join(workspaceRoot, '.build', 'rust-parity', 'env.json'),
]);
const rustEnvPath = valueArg('--rust-env-file', 'KALANDAR_RUST_PARITY_ENV')
  || defaultRustEnvPath;
const rustEnv = rustEnvPath && fs.existsSync(rustEnvPath) ? readJson(rustEnvPath) : null;

const requireRust = hasArg('--require-rust') || process.env.KALANDAR_REQUIRE_RUST_PARITY === '1';
const requirePython = hasArg('--require-python') || process.env.KALANDAR_REQUIRE_PYTHON_PARITY === '1';
const requireCoreAi = hasArg('--require-coreai') || process.env.KALANDAR_REQUIRE_COREAI_PARITY === '1';
const requireCoreAiApp = hasArg('--require-coreai-app') || process.env.KALANDAR_REQUIRE_COREAI_APP_PARITY === '1';
const tensorAtol = Number(valueArg('--tensor-atol', 'KALANDAR_COREAI_TENSOR_ATOL') || '0.001');
const embeddingCosineMin = Number(valueArg('--embedding-cosine-min', 'KALANDAR_COREAI_EMBEDDING_COSINE_MIN') || '0.999');

const pythonReportPath = valueArg('--python-report', 'KALANDAR_PYTHON_PARITY_REPORT');
const onnxReportPath = valueArg('--onnx-report', 'KALANDAR_ONNX_PARITY_REPORT');
const coreAiReportPath = valueArg('--coreai-report', 'KALANDAR_COREAI_PARITY_REPORT');
const coreAiAppCommand = valueArg('--coreai-app-command', 'KALANDAR_COREAI_CAPTURE_PREVIEW_COMMAND');

const failures = [];
const summaries = [];

const normalizeText = (value) => {
  if (value === null || value === undefined) {
    return null;
  }
  const normalized = String(value).split(/\s+/).filter(Boolean).join(' ').toLowerCase();
  return normalized || null;
};

const field = (object, snakeName, camelName = snakeName) => object?.[snakeName] ?? object?.[camelName];

const canonicalTag = (value) => normalizeText(value)?.replace(/^#/, '') ?? null;

const normalizeList = (values, mapper = normalizeText) =>
  Array.from(new Set((values || []).map((value) => mapper(value)).filter(Boolean))).sort();

const canonicalDateText = (value) => {
  const text = normalizeText(value);
  if (text === '2026-05-17') {
    return 'tomorrow';
  }
  if (text === '2026-06-01') {
    return 'starting next month';
  }
  return text;
};

const canonicalClockText = (value) => {
  const text = normalizeText(value)?.replace(/^at\s+/, '').replace(/\s+/g, '') ?? null;
  if (!text) {
    return null;
  }
  const isoTime = text.match(/^\d{4}-\d{2}-\d{2}t(\d{2}):(\d{2})(?::\d{2})?$/);
  if (isoTime) {
    return `${isoTime[1]}:${isoTime[2]}`;
  }
  const meridiem = text.match(/^(\d{1,2})(?::(\d{2}))?(am|pm)$/);
  if (meridiem) {
    let hour = Number(meridiem[1]);
    const minute = meridiem[2] || '00';
    if (meridiem[3] === 'pm' && hour < 12) {
      hour += 12;
    }
    if (meridiem[3] === 'am' && hour === 12) {
      hour = 0;
    }
    return `${String(hour).padStart(2, '0')}:${minute}`;
  }
  return text;
};

const canonicalRecurrenceText = (value) => {
  const text = normalizeText(value);
  if (text === 'freq=weekly;interval=2;byday=tu') {
    return 'every other tuesday';
  }
  if (text === 'freq=weekly;interval=1;byday=tu') {
    return 'every tuesday';
  }
  return text;
};

const normalizeExceptions = (values) =>
  (values || []).map((entry) => ({
    raw: normalizeText(entry.raw),
  }));

const normalizeTaskPreview = (preview) => {
  const task = preview.task ?? preview;
  if (!task || !task.title) {
    throw new Error(`expected task preview, received ${preview.kind}`);
  }
  return {
    kind: preview.kind,
    title: normalizeText(task.title),
    assignees: normalizeList(task.assignees),
    projectHint: normalizeText(field(task, 'project_hint', 'projectHint')),
    tags: normalizeList(task.tags, canonicalTag),
    priority: normalizeText(task.priority),
    status: normalizeText(task.status),
    dueDateText: canonicalDateText(field(task, 'due_date_text', 'dueDateText')),
    dueTimeText: canonicalClockText(field(task, 'due_time_text', 'dueTimeText')),
  };
};

const normalizeReminderPreview = (preview) => {
  const reminder = preview.reminder ?? preview;
  if (!reminder || !reminder.title) {
    throw new Error(`expected reminder preview, received ${preview.kind}`);
  }
  const recurrence = reminder.recurrence;
  return {
    kind: preview.kind,
    title: normalizeText(reminder.title),
    remindAtText: recurrence ? null : canonicalDateText(field(reminder, 'remind_at_text', 'remindAtText')),
    recurrenceStartText: canonicalDateText(field(reminder, 'recurrence_start_text', 'recurrenceStartText')),
    recurrenceEndText: canonicalDateText(field(reminder, 'recurrence_end_text', 'recurrenceEndText')),
    recurrenceWindowText: normalizeText(field(reminder, 'recurrence_window_text', 'recurrenceWindowText')),
    contextHint: normalizeText(field(reminder, 'context_hint', 'contextHint')),
    recurrenceExceptions: normalizeExceptions(field(reminder, 'recurrence_exceptions', 'recurrenceExceptions')),
    recurrence: recurrence
      ? {
          frequency: normalizeText(recurrence.frequency),
          interval: recurrence.interval,
          days: recurrence.days ? normalizeList(recurrence.days) : null,
        }
      : null,
  };
};

const normalizeCalendarPreview = (preview) => {
  const event = preview.event ?? preview;
  if (!event || !('title' in event)) {
    throw new Error(`expected calendar event preview, received ${preview.kind}`);
  }
  return {
    kind: preview.kind,
    title: normalizeText(event.title),
    dateText: canonicalDateText(field(event, 'date_text', 'dateText')),
    timeText: canonicalClockText(field(event, 'time_text', 'timeText')),
    endTimeText: canonicalClockText(field(event, 'end_time_text', 'endTimeText')),
    durationMinutes: field(event, 'duration_minutes', 'durationMinutes') ?? null,
    location: normalizeText(event.location),
    recurrenceText: canonicalRecurrenceText(field(event, 'recurrence_text', 'recurrenceText')),
    recurrenceStartText: canonicalDateText(field(event, 'recurrence_start_text', 'recurrenceStartText')),
    recurrenceEndText: canonicalDateText(field(event, 'recurrence_end_text', 'recurrenceEndText')),
    recurrenceExceptions: normalizeExceptions(field(event, 'recurrence_exceptions', 'recurrenceExceptions')),
  };
};

const normalizePreview = (family, preview) => {
  if (family === 'task') {
    return normalizeTaskPreview(preview);
  }
  if (family === 'reminder') {
    return normalizeReminderPreview(preview);
  }
  if (family === 'calendar') {
    return normalizeCalendarPreview(preview);
  }
  return { kind: preview.kind };
};

const requestForFixture = (fixture, family) => ({
  text: fixture.input,
  now_iso: fixture.nowIso || fixture.now_iso || '2026-05-16T12:00:00.000Z',
  timezone: fixture.timezone || 'America/New_York',
  locale: fixture.locale || 'en-US',
  known_assignees: fixture.knownAssignees || fixture.known_assignees || null,
});

const pushMismatch = (label, expected, actual) => {
  failures.push(`${label}\nexpected: ${JSON.stringify(expected)}\nactual:   ${JSON.stringify(actual)}`);
};

const assertDeepEqual = (label, expected, actual) => {
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    pushMismatch(label, expected, actual);
  }
};

const loadGoldens = (family) => readJson(path.join(fixturesRoot, family, 'goldens.json'));

const runJsParity = () => {
  const lock = readJson(jsLockPath);
  if (lock.version !== 1 || !Array.isArray(lock.cases)) {
    failures.push(`${jsLockPath} must contain version 1 cases`);
    return;
  }

  let count = 0;
  for (const fixture of lock.cases) {
    const request = requestForFixture({ ...fixture, nowIso: lock.nowIso }, fixture.family);
    const preview = parseCapturePreviewInput(request);
    const actual = normalizePreview(fixture.family, preview);
    const expected = normalizePreview(fixture.family, fixture.expected);
    assertDeepEqual(`js:${fixture.family}:${fixture.id}`, expected, actual);
    count += 1;
  }
  summaries.push(`js app parity checked ${count} locked fixture cases`);
};

const runRustPreview = (fixture, family, command, resourceRoot) => {
  const request = requestForFixture(fixture, family);
  const result = spawnSync(command, ['--resource-root', resourceRoot], {
    cwd: root,
    input: `${JSON.stringify(request)}\n`,
    encoding: 'utf8',
    env: rustProcessEnv(),
  });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `rust command exited with ${result.status}`);
  }
  return JSON.parse(result.stdout);
};

const runRustParity = () => {
  const defaultRustCommand = firstExistingPath([
    path.join(root, 'rust', 'target', 'debug', 'kalandar-capture-preview'),
    path.join(workspaceRoot, '.build', 'cargo-target', 'debug', 'kalandar-capture-preview'),
  ]);
  const command = valueArg('--rust-command', 'KALANDAR_CAPTURE_PREVIEW_COMMAND')
    || defaultRustCommand
    || path.join(root, 'rust', 'target', 'debug', 'kalandar-capture-preview');
  const resourceRoot = valueArg('--rust-resource-root', 'KALANDAR_CAPTURE_RESOURCE_ROOT') || rustEnv?.resourceRoot;
  if (!resourceRoot || !fs.existsSync(command)) {
    const reason = !resourceRoot ? 'missing KALANDAR_CAPTURE_RESOURCE_ROOT' : `missing rust command ${command}`;
    if (requireRust) {
      failures.push(`rust app parity required but unavailable: ${reason}`);
    } else {
      summaries.push(`rust app parity skipped: ${reason}`);
    }
    return;
  }

  let count = 0;
  for (const family of ['intent', 'task', 'reminder', 'calendar']) {
    for (const fixture of loadGoldens(family)) {
      const preview = runRustPreview(fixture, family, command, resourceRoot);
      const actual = normalizePreview(family, preview);
      const expected = normalizePreview(family, fixture.oracle);
      assertDeepEqual(`rust:${family}:${fixture.id}`, expected, actual);
      count += 1;
    }
  }
  summaries.push(`rust app parity checked ${count} locked fixture cases`);
};

const commandExists = (command) => !command.includes(path.sep) || fs.existsSync(command);

const runCoreAiAppPreview = (fixture, family, command) => {
  const request = requestForFixture(fixture, family);
  const result = spawnSync(command, [], {
    cwd: root,
    input: `${JSON.stringify(request)}\n`,
    encoding: 'utf8',
    env: process.env,
  });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `Core AI app command exited with ${result.status}`);
  }
  return JSON.parse(result.stdout);
};

const runCoreAiAppParity = () => {
  if (!coreAiAppCommand || !commandExists(coreAiAppCommand)) {
    const reason = !coreAiAppCommand ? 'missing KALANDAR_COREAI_CAPTURE_PREVIEW_COMMAND' : `missing Core AI app command ${coreAiAppCommand}`;
    if (requireCoreAiApp) {
      failures.push(`Core AI app parity required but unavailable: ${reason}`);
    } else {
      summaries.push(`Core AI app parity skipped: ${reason}`);
    }
    return;
  }

  const lock = readJson(jsLockPath);
  let count = 0;
  for (const fixture of lock.cases) {
    const preview = runCoreAiAppPreview({ ...fixture, nowIso: lock.nowIso }, fixture.family, coreAiAppCommand);
    const actual = normalizePreview(fixture.family, preview);
    const expected = normalizePreview(fixture.family, fixture.expected);
    assertDeepEqual(`coreai-app:${fixture.family}:${fixture.id}`, expected, actual);
    count += 1;
  }

  for (const family of ['intent', 'task', 'reminder', 'calendar']) {
    for (const fixture of loadGoldens(family)) {
      const preview = runCoreAiAppPreview(fixture, family, coreAiAppCommand);
      const actual = normalizePreview(family, preview);
      const expected = normalizePreview(family, fixture.oracle);
      assertDeepEqual(`coreai-app:${family}:${fixture.id}`, expected, actual);
      count += 1;
    }
  }
  summaries.push(`Core AI app parity checked ${count} locked fixture cases`);
};

const rustProcessEnv = () => {
  const env = { ...process.env };
  if (rustEnv?.manifestSha256 && !env.KALANDAR_CAPTURE_MANIFEST_SHA256) {
    env.KALANDAR_CAPTURE_MANIFEST_SHA256 = rustEnv.manifestSha256;
  }
  if (rustEnv?.ortLibPath) {
    env.ORT_LIB_PATH ||= rustEnv.ortLibPath;
    env.ORT_PREFER_DYNAMIC_LINK ||= rustEnv.ortPreferDynamicLink ?? '1';
    env.DYLD_LIBRARY_PATH = env.DYLD_LIBRARY_PATH
      ? `${rustEnv.ortLibPath}${path.delimiter}${env.DYLD_LIBRARY_PATH}`
      : rustEnv.ortLibPath;
  }
  return env;
};

const reportCases = (report, label) => {
  if (!report || !Array.isArray(report.cases)) {
    throw new Error(`${label} parity report must contain a cases array`);
  }
  return new Map(report.cases.map((entry) => [entry.id, entry]));
};

const tensorOutputs = (entry, label) => {
  if (!entry.outputs || typeof entry.outputs !== 'object') {
    throw new Error(`${label}:${entry.id} must contain outputs`);
  }
  return entry.outputs;
};

const argmax = (values, start, width) => {
  let bestIndex = 0;
  let bestValue = values[start];
  for (let index = 1; index < width; index += 1) {
    const value = values[start + index];
    if (value > bestValue) {
      bestIndex = index;
      bestValue = value;
    }
  }
  return bestIndex;
};

const cosineSimilarity = (left, right) => {
  let dot = 0;
  let leftNorm = 0;
  let rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) {
    dot += left[index] * right[index];
    leftNorm += left[index] * left[index];
    rightNorm += right[index] * right[index];
  }
  return dot / Math.max(Math.sqrt(leftNorm) * Math.sqrt(rightNorm), Number.EPSILON);
};

const compareTensor = (label, expected, actual) => {
  assertDeepEqual(`${label}:dtype`, expected.dtype, actual.dtype);
  assertDeepEqual(`${label}:shape`, expected.shape, actual.shape);
  if (!Array.isArray(expected.data) || !Array.isArray(actual.data)) {
    failures.push(`${label}: tensor data must be arrays`);
    return;
  }
  if (expected.data.length !== actual.data.length) {
    failures.push(`${label}: data length mismatch expected ${expected.data.length}, actual ${actual.data.length}`);
    return;
  }

  let maxAbsDiff = 0;
  let sumAbsDiff = 0;
  for (let index = 0; index < expected.data.length; index += 1) {
    const diff = Math.abs(Number(expected.data[index]) - Number(actual.data[index]));
    maxAbsDiff = Math.max(maxAbsDiff, diff);
    sumAbsDiff += diff;
  }
  if (maxAbsDiff > tensorAtol) {
    failures.push(`${label}: max abs diff ${maxAbsDiff} exceeds tolerance ${tensorAtol}`);
  }

  if (expected.shape.length === 3) {
    const [, sequenceLength, labelCount] = expected.shape;
    for (let tokenIndex = 0; tokenIndex < sequenceLength; tokenIndex += 1) {
      const offset = tokenIndex * labelCount;
      const expectedLabel = argmax(expected.data, offset, labelCount);
      const actualLabel = argmax(actual.data, offset, labelCount);
      if (expectedLabel !== actualLabel) {
        failures.push(`${label}: argmax mismatch at token ${tokenIndex}: expected ${expectedLabel}, actual ${actualLabel}`);
        break;
      }
    }
  }

  if (expected.shape.length === 2 && expected.shape[0] === 1) {
    const cosine = cosineSimilarity(expected.data.map(Number), actual.data.map(Number));
    if (cosine < embeddingCosineMin) {
      failures.push(`${label}: cosine ${cosine} below minimum ${embeddingCosineMin}`);
    }
  }

  return {
    maxAbsDiff,
    meanAbsDiff: sumAbsDiff / Math.max(expected.data.length, 1),
  };
};

const compareTensorReports = (expectedLabel, expectedPath, actualLabel, actualPath) => {
  if (!fs.existsSync(expectedPath)) {
    failures.push(`${expectedLabel} report does not exist: ${expectedPath}`);
    return [];
  }
  if (!fs.existsSync(actualPath)) {
    failures.push(`${actualLabel} report does not exist: ${actualPath}`);
    return [];
  }
  const expectedReport = readJson(expectedPath);
  const actualReport = readJson(actualPath);
  const expectedCases = reportCases(expectedReport, expectedLabel);
  const actualCases = reportCases(actualReport, actualLabel);
  const stats = [];

  for (const [caseId, expectedCase] of expectedCases) {
    const actualCase = actualCases.get(caseId);
    if (!actualCase) {
      failures.push(`${actualLabel} report is missing case ${caseId}`);
      continue;
    }
    if (expectedCase.parser && actualCase.parser && expectedCase.parser !== actualCase.parser) {
      failures.push(`${caseId}: parser mismatch expected ${expectedCase.parser}, actual ${actualCase.parser}`);
    }
    const expectedOutputs = tensorOutputs(expectedCase, expectedLabel);
    const actualOutputs = tensorOutputs(actualCase, actualLabel);
    for (const [outputName, expectedTensor] of Object.entries(expectedOutputs)) {
      const actualTensor = actualOutputs[outputName];
      if (!actualTensor) {
        failures.push(`${caseId}: ${actualLabel} missing output ${outputName}`);
        continue;
      }
      const stat = compareTensor(`${expectedLabel}->${actualLabel}:${caseId}:${outputName}`, expectedTensor, actualTensor);
      if (stat) {
        stats.push(stat);
      }
    }
  }

  summaries.push(`${expectedLabel}->${actualLabel} tensor parity checked ${expectedCases.size} cases`);
  return stats;
};

const runTensorParity = () => {
  if (!pythonReportPath) {
    if (requirePython || requireCoreAi) {
      failures.push('python tensor parity reference required but KALANDAR_PYTHON_PARITY_REPORT/--python-report is missing');
    } else {
      summaries.push('python tensor parity skipped: no python report path provided');
    }
    return;
  }

  if (onnxReportPath) {
    compareTensorReports('python', pythonReportPath, 'onnx', onnxReportPath);
  }

  if (!coreAiReportPath) {
    if (requireCoreAi) {
      failures.push('Core AI tensor parity required but KALANDAR_COREAI_PARITY_REPORT/--coreai-report is missing');
    } else {
      summaries.push('Core AI tensor parity skipped: no Core AI report path provided');
    }
    return;
  }

  compareTensorReports('python', pythonReportPath, 'coreai', coreAiReportPath);
};

runJsParity();
runRustParity();
runCoreAiAppParity();
runTensorParity();

for (const summary of summaries) {
  console.log(summary);
}

if (failures.length > 0) {
  console.error('Core AI parity failures:');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
