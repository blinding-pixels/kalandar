import { KALANDAR_DATES_WASM_BASE64 } from './wasm/bytes.ts';
import initAsync, {
  captureSchedule as wasmCaptureSchedule,
  holidayData as wasmHolidayData,
  initSync,
  resolveUnlabelled as wasmResolveUnlabelled,
} from './wasm/glue.ts';

export type ScheduleKind = 'task' | 'reminder' | 'calendar';

export type ScheduleRole =
  | 'event_start'
  | 'event_end'
  | 'duration'
  | 'due_date'
  | 'due_time'
  | 'trigger'
  | 'lead_time'
  | 'alert'
  | 'recurrence'
  | 'recurrence_start'
  | 'recurrence_end'
  | 'recurrence_exception'
  | 'recurrence_window';

export interface ScheduleSpan {
  role: ScheduleRole;
  start: number;
  end: number;
  text: string;
}

export interface ScheduleTexts {
  moment: string | null;
  date: string | null;
  time: string | null;
  end_time: string | null;
  duration: string | null;
  lead: string | null;
  alert: string | null;
  recurrence: string | null;
  recurrence_start: string | null;
  recurrence_end: string | null;
  recurrence_window: string | null;
  recurrence_exceptions: string[];
}

export interface PreviewMoment {
  date: string;
  time: string | null;
  local: string | null;
  instant: string | null;
  all_day: boolean;
  implied: string[];
  alternative_date: string | null;
}

export interface PreviewWeekday {
  position: number | null;
  weekday: string;
}

export interface PreviewRecurrenceException {
  raw: string;
  condition: string | null;
  action: string | null;
}

export interface PreviewRecurrence {
  rrule: string;
  frequency: 'minutely' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly';
  interval: number;
  days: string[] | null;
  weekdays: PreviewWeekday[];
  month_days: number[];
  months: number[];
  time: string | null;
  count: number | null;
  start_date: string | null;
  end_date: string | null;
  until: string | null;
  exdates: string[];
  exceptions: PreviewRecurrenceException[];
}

export interface PreviewSchedule {
  start: PreviewMoment | null;
  end: PreviewMoment | null;
  duration_minutes: number | null;
  recurrence: PreviewRecurrence | null;
  lead_minutes: number | null;
  alert_minutes_before: number | null;
  alert_at: PreviewMoment | null;
  alerts_off: boolean;
  notify_at: string | null;
  task_signal: string | null;
}

export interface PreviewWarning {
  code: string;
  message: string;
}

export interface CaptureSchedule {
  spans: ScheduleSpan[];
  texts: ScheduleTexts;
  schedule: PreviewSchedule | null;
  warnings: PreviewWarning[];
}

export interface HolidayDataInfo {
  source: string;
  version: string;
  integrity: string;
  generated: string;
  first_year: number;
  last_year: number;
}

export interface ScheduleOptions {
  now?: Date | string;
  timezone?: string;
  locale?: string | null;
  settings?: Record<string, unknown>;
  spans?: Array<{ label: string; start: number; end: number }>;
}

let ready = false;

const decodeBase64 = (value: string): Uint8Array => {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
};

export type KalandarDatesWasmSource =
  | string
  | URL
  | Request
  | Response
  | Promise<Response>
  | ArrayBuffer
  | ArrayBufferView
  | object;

const embeddedBytes = (): Uint8Array => {
  if (!KALANDAR_DATES_WASM_BASE64) {
    throw new Error(
      'kalandar-dates: this build leaves out the embedded WebAssembly; await initKalandarDatesAsync(url) before parsing',
    );
  }
  return decodeBase64(KALANDAR_DATES_WASM_BASE64);
};

/** Compiles the embedded WebAssembly synchronously. Every resolver call does this on first use. */
export const initKalandarDates = (): void => {
  if (ready) {
    return;
  }
  initSync({ module: embeddedBytes() });
  ready = true;
};

/**
 * Compiles the WebAssembly asynchronously. Pass the URL of `kalandar/dates/kalandar_dates_wasm_bg.wasm`,
 * a `fetch` Response or its promise to use `WebAssembly.instantiateStreaming`; with no argument it
 * uses the embedded copy. Once it resolves, the synchronous parsers run without the embedded copy.
 */
export const initKalandarDatesAsync = async (source?: KalandarDatesWasmSource): Promise<void> => {
  if (ready) {
    return;
  }
  await initAsync({ module_or_path: source ?? embeddedBytes() });
  ready = true;
};

const isoOf = (value: Date | string | undefined): string => {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'string' && value.trim()) {
    return value;
  }
  return new Date().toISOString();
};

const callJson = <T>(call: (request: string) => string, request: unknown): T => {
  initKalandarDates();
  const parsed = JSON.parse(call(JSON.stringify(request)));
  if (parsed && typeof parsed === 'object' && 'error' in parsed && Object.keys(parsed).length === 1) {
    throw new Error(`kalandar-dates: ${String(parsed.error)}`);
  }
  return parsed as T;
};

export const captureSchedule = (text: string, kind: ScheduleKind, options: ScheduleOptions = {}): CaptureSchedule =>
  callJson<CaptureSchedule>(wasmCaptureSchedule, {
    text,
    kind,
    now: isoOf(options.now),
    timezone: options.timezone || 'UTC',
    locale: options.locale ?? null,
    settings: options.settings ?? null,
    spans: options.spans,
    offsets: 'utf16',
  });

export const resolveUnlabelled = (text: string, kind: ScheduleKind, options: ScheduleOptions = {}): unknown =>
  callJson<unknown>(wasmResolveUnlabelled, {
    line: text,
    kind,
    now: isoOf(options.now),
    zone: options.timezone || 'UTC',
    settings: options.settings ?? null,
  });

export const holidayDataInfo = (): HolidayDataInfo => {
  initKalandarDates();
  return JSON.parse(wasmHolidayData()) as HolidayDataInfo;
};

export const removeScheduleSpans = (text: string, spans: ScheduleSpan[]): string => {
  const ordered = [...spans].sort((left, right) => left.start - right.start || left.end - right.end);
  let output = '';
  let cursor = 0;
  for (const span of ordered) {
    if (span.start >= cursor) {
      output += `${text.slice(cursor, span.start)} `;
    }
    cursor = Math.max(cursor, span.end);
  }
  output += text.slice(cursor);
  return output.split(/\s+/).filter(Boolean).join(' ');
};
