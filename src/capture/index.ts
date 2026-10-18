import { parseEventInput } from '../calendar/index.ts';
import { classifyIntent } from '../intent/index.ts';
import { parseReminderInput } from '../reminder/index.ts';
import { DEFAULT_LOCALE, DEFAULT_TIMEZONE } from '../shared/constants.ts';
import { normalizeWhitespace } from '../shared/utils.ts';
import type { PreviewSchedule } from '../dates/index.ts';
import { parseTaskInput } from '../task/index.ts';
import {
  CAPTURE_PREVIEW_CONTRACT_VERSION,
  CAPTURE_PREVIEW_IMPLEMENTATIONS,
  capturePreviewImplementationDescriptor,
} from './implementations.ts';
import type {
  CapturePreviewCalendarEventDraft,
  CapturePreviewCandidate,
  CapturePreviewContractVersion,
  CapturePreviewImplementationBoundary,
  CapturePreviewImplementationDescriptor,
  CapturePreviewImplementationHost,
  CapturePreviewImplementationStatus,
  CapturePreviewOptions,
  CapturePreviewParser,
  CapturePreviewParserConfig,
  CapturePreviewRecurrenceException,
  CapturePreviewReminderDraft,
  CapturePreviewRequest,
  CapturePreviewResult,
  CapturePreviewTaskDraft,
  CapturePreviewWarning,
  CoreAiCapturePreviewConfig,
  RustCliCapturePreviewConfig,
} from './types.ts';

type WarningLike = {
  code: string;
  severity: string;
  message: string;
};

type NodeProcessEnv = Record<string, string | undefined>;

type SpawnedChildProcess = {
  stdout: {
    on(event: 'data', listener: (chunk: unknown) => void): void;
  };
  stderr: {
    on(event: 'data', listener: (chunk: unknown) => void): void;
  };
  stdin: {
    end(input: string): void;
  };
  on(event: 'error', listener: (error: Error) => void): void;
  on(event: 'close', listener: (code: number | null, signal: string | null) => void): void;
  kill(signal: string): void;
};

type SpawnFunction = (
  command: string,
  args: string[],
  options: {
    env?: NodeProcessEnv;
    stdio: ['pipe', 'pipe', 'pipe'];
    windowsHide: boolean;
  },
) => SpawnedChildProcess;

const DEFAULT_RUST_TIMEOUT_MS = 30_000;

const isObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value && typeof value === 'object' && !Array.isArray(value));

const toIsoString = (value: Date | string | undefined): string | null => {
  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value.toISOString();
  }
  if (typeof value === 'string' && value.trim()) {
    return value;
  }
  return null;
};

export const normalizeCapturePreviewRequest = (
  input: string | CapturePreviewRequest,
  opts?: CapturePreviewOptions,
): CapturePreviewRequest => {
  if (typeof input === 'string') {
    return {
      text: input,
      now_iso: toIsoString(opts?.now),
      timezone: opts?.timezone || DEFAULT_TIMEZONE,
      locale: opts?.locale || DEFAULT_LOCALE,
      known_assignees: opts?.knownAssignees ?? null,
    };
  }

  return {
    text: input.text,
    now_iso: input.now_iso ?? toIsoString(opts?.now),
    timezone: input.timezone || opts?.timezone || DEFAULT_TIMEZONE,
    locale: input.locale ?? opts?.locale ?? DEFAULT_LOCALE,
    known_assignees: input.known_assignees ?? opts?.knownAssignees ?? null,
  };
};

const unsupported = (unavailableReason: string, message: string): CapturePreviewResult => ({
  kind: 'unsupported',
  confidence: 0,
  candidates: [],
  task: null,
  event: null,
  reminder: null,
  thought: null,
  unavailable_reason: unavailableReason,
  warnings: [
    {
      code: unavailableReason,
      severity: 'warning',
      message,
    },
  ],
  schedule: null,
});

const stripPrefixCaseInsensitive = (text: string, prefix: string): string | null =>
  text.slice(0, prefix.length).toLowerCase() === prefix.toLowerCase() ? text.slice(prefix.length).trim() : null;

const recurrencePartToString = (value: unknown): string | null => {
  if (!value) {
    return null;
  }
  if (typeof value === 'string') {
    return value;
  }
  if (isObject(value) && typeof value.type === 'string') {
    return value.type;
  }
  return JSON.stringify(value);
};

const toRecurrenceExceptions = (
  entries: Array<{ raw: string; condition?: unknown; action?: unknown }>,
  schedule?: PreviewSchedule | null,
): CapturePreviewRecurrenceException[] => {
  const resolved = schedule?.recurrence?.exceptions || [];
  const sameCount = resolved.length === entries.length;
  return entries.map((entry, index) => {
    const matched = sameCount
      ? resolved[index]
      : resolved.find((exception) => exception.raw.includes(entry.raw) || entry.raw.includes(exception.raw));
    if (resolved.length > 0) {
      return { raw: entry.raw, condition: matched?.condition ?? null, action: matched?.action ?? null };
    }
    return {
      raw: entry.raw,
      condition: recurrencePartToString(entry.condition),
      action: recurrencePartToString(entry.action),
    };
  });
};

const toPreviewWarnings = (warnings: WarningLike[] | null | undefined): CapturePreviewWarning[] =>
  (warnings || []).map((warning) => ({
    code: warning.code,
    severity: warning.severity,
    message: warning.message,
  }));

const buildCandidates = (scores: { task: number; calendar: number; reminder: number }): CapturePreviewCandidate[] =>
  ([
    { kind: 'task', score: scores.task },
    { kind: 'calendar-event', score: scores.calendar },
    { kind: 'reminder', score: scores.reminder },
  ] satisfies CapturePreviewCandidate[])
    .filter((candidate) => candidate.score > 0)
    .sort((left, right) => right.score - left.score);

const toTaskDraft = (result: ReturnType<typeof parseTaskInput>): CapturePreviewTaskDraft => ({
  title: result.fields.title,
  assignees: result.fields.assignee,
  project_hint: result.fields.project,
  tags: result.fields.tags,
  priority: result.fields.priority,
  status: result.fields.status,
  due_date_text: result.fields.due_date,
  due_time_text: result.fields.due_time,
  duration_text: result.fields.duration,
  recurrence_text: result.fields.recurrence,
  recurrence_start_text: result.fields.recurrence_start,
  recurrence_end_text: result.fields.recurrence_end,
  recurrence_exceptions: toRecurrenceExceptions(result.fields.recurrence_exception, result.schedule),
});

const toReminderDraft = (result: ReturnType<typeof parseReminderInput>): CapturePreviewReminderDraft => ({
  title: result.fields.title,
  remind_at_text: result.fields.remind_at,
  recurrence: result.fields.recurrence,
  recurrence_start_text: result.fields.recurrence_start,
  recurrence_end_text: result.fields.recurrence_end,
  recurrence_exceptions: toRecurrenceExceptions(result.fields.recurrence_exception, result.schedule),
  recurrence_window_text: result.fields.recurrence_window,
  location: result.fields.location,
  context_hint: result.fields.context_hint,
});

const toEventDraft = (result: ReturnType<typeof parseEventInput>): CapturePreviewCalendarEventDraft => ({
  title: result.fields.title,
  date_text: result.fields.date,
  time_text: result.fields.time,
  end_time_text: result.fields.end_time,
  duration_minutes: result.fields.duration_minutes,
  location: result.fields.location,
  recurrence_text: result.fields.recurrence,
  recurrence_start_text: result.fields.recurrence_start,
  recurrence_end_text: result.fields.recurrence_end,
  recurrence_exceptions: toRecurrenceExceptions(result.fields.recurrence_exception, result.schedule),
  alert_offset_minutes: (result.fields.alerts || []).map((alert) => alert.offset_minutes),
  attendees: result.fields.attendees || [],
});

const taskResult = (
  task: ReturnType<typeof parseTaskInput>,
  confidence: number,
  candidates: CapturePreviewCandidate[],
  intentWarnings: CapturePreviewWarning[] = [],
): CapturePreviewResult => ({
  kind: 'task',
  confidence,
  candidates,
  task: toTaskDraft(task),
  event: null,
  reminder: null,
  thought: null,
  unavailable_reason: null,
  warnings: [...intentWarnings, ...toPreviewWarnings(task.warnings)],
  schedule: task.schedule ?? null,
});

const reminderResult = (
  reminder: ReturnType<typeof parseReminderInput>,
  confidence: number,
  candidates: CapturePreviewCandidate[],
  intentWarnings: CapturePreviewWarning[] = [],
): CapturePreviewResult => ({
  kind: 'reminder',
  confidence,
  candidates,
  task: null,
  event: null,
  reminder: toReminderDraft(reminder),
  thought: null,
  unavailable_reason: null,
  warnings: [...intentWarnings, ...toPreviewWarnings(reminder.warnings)],
  schedule: reminder.schedule ?? null,
});

const calendarResult = (
  event: ReturnType<typeof parseEventInput>,
  confidence: number,
  candidates: CapturePreviewCandidate[],
  intentWarnings: CapturePreviewWarning[] = [],
): CapturePreviewResult => ({
  kind: 'calendar-event',
  confidence,
  candidates,
  task: null,
  event: toEventDraft(event),
  reminder: null,
  thought: null,
  unavailable_reason: null,
  warnings: [...intentWarnings, ...toPreviewWarnings(event.warnings)],
  schedule: event.schedule ?? null,
});

export const parseCapturePreviewInput = (
  input: string | CapturePreviewRequest,
  opts?: CapturePreviewOptions,
): CapturePreviewResult => {
  const request = normalizeCapturePreviewRequest(input, opts);
  const text = normalizeWhitespace(request.text || '');
  if (!text) {
    return unsupported('capture-text-required', 'Capture preview requires non-empty text.');
  }

  const parserOptions = {
    now: request.now_iso || undefined,
    timezone: request.timezone,
    locale: request.locale || undefined,
  };
  const knownAssignees = request.known_assignees || undefined;

  const taskPayload = stripPrefixCaseInsensitive(text, 'task:');
  if (taskPayload !== null) {
    return taskPayload
      ? taskResult(parseTaskInput(taskPayload, { ...parserOptions, knownAssignees }), 0.99, [])
      : unsupported('capture-text-required', 'Capture preview requires non-empty text.');
  }

  const reminderPayload = stripPrefixCaseInsensitive(text, 'reminder:');
  if (reminderPayload !== null) {
    return reminderPayload
      ? reminderResult(parseReminderInput(reminderPayload, parserOptions), 0.99, [])
      : unsupported('capture-text-required', 'Capture preview requires non-empty text.');
  }

  const eventPayload = stripPrefixCaseInsensitive(text, 'event:');
  if (eventPayload !== null) {
    return eventPayload
      ? calendarResult(parseEventInput(eventPayload, parserOptions), 0.99, [])
      : unsupported('capture-text-required', 'Capture preview requires non-empty text.');
  }

  const intent = classifyIntent(text);
  const candidates = buildCandidates(intent.scores);
  const intentWarnings = toPreviewWarnings(intent.warnings);

  switch (intent.intent) {
    case 'task': {
      const parsed = parseTaskInput(text, { ...parserOptions, knownAssignees });
      return taskResult(parsed, intent.scores.task, candidates, intentWarnings);
    }
    case 'calendar': {
      const parsed = parseEventInput(text, parserOptions);
      return calendarResult(parsed, intent.scores.calendar, candidates, intentWarnings);
    }
    case 'reminder': {
      const parsed = parseReminderInput(text, parserOptions);
      return reminderResult(parsed, intent.scores.reminder, candidates, intentWarnings);
    }
    case 'none':
      return {
        kind: 'thought',
        confidence: intent.scores.none,
        candidates,
        task: null,
        event: null,
        reminder: null,
        thought: { text },
        unavailable_reason: null,
        warnings: intentWarnings,
        schedule: null,
      };
  }
};

const assertCapturePreviewResult = (value: unknown): CapturePreviewResult => {
  if (
    !isObject(value) ||
    typeof value.kind !== 'string' ||
    typeof value.confidence !== 'number' ||
    !Array.isArray(value.candidates) ||
    !Array.isArray(value.warnings)
  ) {
    throw new Error('Rust capture preview command returned invalid JSON.');
  }
  return value as unknown as CapturePreviewResult;
};

const loadNodeSpawn = async (): Promise<SpawnFunction> => {
  try {
    const dynamicImport = new Function('specifier', 'return import(specifier)') as (specifier: string) => Promise<unknown>;
    const childProcess = (await dynamicImport('node:child_process')) as { spawn?: SpawnFunction };
    if (typeof childProcess.spawn !== 'function') {
      throw new Error('node:child_process did not expose spawn');
    }
    return childProcess.spawn;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Rust capture preview CLI requires a Node.js runtime with node:child_process: ${message}`);
  }
};

export const parseCapturePreviewWithRustCli = async (
  input: string | CapturePreviewRequest,
  config: RustCliCapturePreviewConfig,
  opts?: CapturePreviewOptions,
): Promise<CapturePreviewResult> => {
  const request = normalizeCapturePreviewRequest(input, opts);
  const args = [...(config.args || []), '--resource-root', config.resourceRoot];
  const timeoutMs = config.timeoutMs ?? DEFAULT_RUST_TIMEOUT_MS;
  const spawn = await loadNodeSpawn();

  return await new Promise<CapturePreviewResult>((resolve, reject) => {
    const child = spawn(config.command, args, {
      env: config.env ? { ...process.env, ...config.env } : process.env,
      stdio: ['pipe', 'pipe', 'pipe'],
      windowsHide: true,
    });
    let stdout = '';
    let stderr = '';
    let settled = false;
    let timedOut = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const finish = (callback: () => void): void => {
      if (settled) {
        return;
      }
      settled = true;
      if (timer) {
        clearTimeout(timer);
      }
      callback();
    };

    timer = setTimeout(() => {
      timedOut = true;
      child.kill('SIGKILL');
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      stdout += String(chunk);
    });
    child.stderr.on('data', (chunk) => {
      stderr += String(chunk);
    });
    child.on('error', (error) => {
      finish(() => reject(error));
    });
    child.on('close', (code, signal) => {
      finish(() => {
        if (timedOut) {
          reject(new Error(`Rust capture preview command timed out after ${timeoutMs}ms.`));
          return;
        }
        if (code !== 0) {
          const suffix = stderr.trim() ? `: ${stderr.trim()}` : signal ? `: signal ${signal}` : '';
          reject(new Error(`Rust capture preview command failed with exit code ${code ?? 'null'}${suffix}`));
          return;
        }

        try {
          resolve(assertCapturePreviewResult(JSON.parse(stdout)));
        } catch (error) {
          reject(error);
        }
      });
    });

    child.stdin.end(`${JSON.stringify(request)}\n`);
  });
};

export const createCapturePreviewParser = async (
  config: CapturePreviewParserConfig = {},
): Promise<CapturePreviewParser> => {
  const requestedImplementation = (config as { implementation?: unknown }).implementation;

  if (!config.implementation || config.implementation === 'typescript') {
    return createTypeScriptCapturePreviewParser();
  }

  if (config.implementation === 'rust-cli') {
    return createRustCliCapturePreviewParser(config);
  }

  if (config.implementation === 'coreai') {
    return createCoreAiCapturePreviewParser(config);
  }

  throw new Error(`Unsupported capture preview implementation: ${String(requestedImplementation)}`);
};

const createTypeScriptCapturePreviewParser = (): CapturePreviewParser => {
  const descriptor = capturePreviewImplementationDescriptor('typescript');
  return {
    mode: 'typescript',
    info: {
      ...descriptor,
    },
    async parse(input: string | CapturePreviewRequest, opts?: CapturePreviewOptions): Promise<CapturePreviewResult> {
      return parseCapturePreviewInput(input, opts);
    },
    async dispose(): Promise<void> {},
  };
};

const createRustCliCapturePreviewParser = (config: RustCliCapturePreviewConfig): CapturePreviewParser => {
  const descriptor = capturePreviewImplementationDescriptor('rust-cli');
  return {
    mode: 'rust-cli',
    info: {
      ...descriptor,
      command: config.command,
      resourceRoot: config.resourceRoot,
    },
    async parse(input: string | CapturePreviewRequest, opts?: CapturePreviewOptions): Promise<CapturePreviewResult> {
      return await parseCapturePreviewWithRustCli(input, config, opts);
    },
    async dispose(): Promise<void> {},
  };
};

const createCoreAiCapturePreviewParser = (config: CoreAiCapturePreviewConfig): CapturePreviewParser => {
  const descriptor = capturePreviewImplementationDescriptor('coreai');
  return {
    mode: 'coreai',
    info: {
      ...descriptor,
    },
    async parse(input: string | CapturePreviewRequest, opts?: CapturePreviewOptions): Promise<CapturePreviewResult> {
      const request = normalizeCapturePreviewRequest(input, opts);
      return assertCapturePreviewResult(await config.bridge.previewCapture(request));
    },
    async dispose(): Promise<void> {
      await config.bridge.dispose?.();
    },
  };
};

export { initKalandarDates, initKalandarDatesAsync } from '../dates/index.ts';
export type { KalandarDatesWasmSource } from '../dates/index.ts';

export {
  CAPTURE_PREVIEW_CONTRACT_VERSION,
  CAPTURE_PREVIEW_IMPLEMENTATIONS,
  capturePreviewImplementationDescriptor,
};

export type {
  CapturePreviewCalendarEventDraft,
  CapturePreviewCandidate,
  CapturePreviewCandidateKind,
  CapturePreviewContractVersion,
  CapturePreviewImplementation,
  CapturePreviewImplementationBoundary,
  CapturePreviewImplementationDescriptor,
  CapturePreviewImplementationHost,
  CapturePreviewImplementationStatus,
  CapturePreviewKind,
  CapturePreviewOptions,
  CapturePreviewParser,
  CapturePreviewParserConfig,
  CapturePreviewParserInfo,
  CapturePreviewRecurrenceException,
  CapturePreviewReminderDraft,
  CapturePreviewRequest,
  CapturePreviewResult,
  CapturePreviewTaskDraft,
  CapturePreviewThoughtDraft,
  CapturePreviewWarning,
  CoreAiCapturePreviewBridge,
  CoreAiCapturePreviewConfig,
  RustCliCapturePreviewConfig,
  TypeScriptCapturePreviewConfig,
} from './types.ts';
