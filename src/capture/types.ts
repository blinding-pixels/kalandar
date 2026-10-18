import type { EventParseResult } from '../calendar/types.ts';
import type { PreviewSchedule } from '../dates/index.ts';
import type { IntentResult } from '../intent/types.ts';
import type { ReminderParseResult, ReminderRecurrence } from '../reminder/types.ts';
import type { TaskParseResult } from '../task/types.ts';

export type CapturePreviewKind = 'task' | 'calendar-event' | 'reminder' | 'thought' | 'ambiguous' | 'unsupported';
export type CapturePreviewCandidateKind = 'task' | 'calendar-event' | 'reminder';
export type CapturePreviewImplementation = 'typescript' | 'rust-cli' | 'coreai';
export type CapturePreviewContractVersion = 'capture-preview-v1';
export type CapturePreviewImplementationBoundary = 'in-process' | 'json-stdio-cli' | 'native-host-bridge';
export type CapturePreviewImplementationHost = 'javascript' | 'native' | 'apple-native';
export type CapturePreviewImplementationStatus = 'ready' | 'host-required';

export interface CapturePreviewRequest {
  text: string;
  now_iso?: string | null;
  timezone: string;
  locale?: string | null;
  known_assignees?: string[] | null;
}

export interface CapturePreviewOptions {
  now?: Date | string;
  timezone?: string;
  locale?: string;
  knownAssignees?: string[];
}

export interface CapturePreviewCandidate {
  kind: CapturePreviewCandidateKind;
  score: number;
}

export interface CapturePreviewRecurrenceException {
  raw: string;
  condition: string | null;
  action: string | null;
}

export interface CapturePreviewTaskDraft {
  title: string;
  assignees: string[];
  project_hint: string | null;
  tags: string[];
  priority: string | null;
  status: string | null;
  due_date_text: string | null;
  due_time_text: string | null;
  duration_text: string | null;
  recurrence_text: string | null;
  recurrence_start_text: string | null;
  recurrence_end_text: string | null;
  recurrence_exceptions: CapturePreviewRecurrenceException[];
}

export interface CapturePreviewReminderDraft {
  title: string;
  remind_at_text: string | null;
  recurrence: ReminderRecurrence | null;
  recurrence_start_text: string | null;
  recurrence_end_text: string | null;
  recurrence_exceptions: CapturePreviewRecurrenceException[];
  recurrence_window_text: string | null;
  location: string | null;
  context_hint: string | null;
}

export interface CapturePreviewCalendarEventDraft {
  title: string | null;
  date_text: string | null;
  time_text: string | null;
  end_time_text: string | null;
  duration_minutes: number | null;
  location: string | null;
  recurrence_text: string | null;
  recurrence_start_text: string | null;
  recurrence_end_text: string | null;
  recurrence_exceptions: CapturePreviewRecurrenceException[];
  alert_offset_minutes: number[];
  attendees: string[];
}

export interface CapturePreviewThoughtDraft {
  text: string;
}

export interface CapturePreviewWarning {
  code: string;
  severity: string;
  message: string;
}

export interface CapturePreviewResult {
  kind: CapturePreviewKind;
  confidence: number;
  candidates: CapturePreviewCandidate[];
  task: CapturePreviewTaskDraft | null;
  event: CapturePreviewCalendarEventDraft | null;
  reminder: CapturePreviewReminderDraft | null;
  thought: CapturePreviewThoughtDraft | null;
  unavailable_reason: string | null;
  warnings: CapturePreviewWarning[];
  schedule?: PreviewSchedule | null;
}

export interface TypeScriptCapturePreviewConfig {
  implementation?: 'typescript';
}

export interface RustCliCapturePreviewConfig {
  implementation: 'rust-cli';
  command: string;
  args?: string[];
  env?: Record<string, string | undefined>;
  resourceRoot: string;
  timeoutMs?: number;
}

export interface CoreAiCapturePreviewBridge {
  previewCapture(request: CapturePreviewRequest): Promise<CapturePreviewResult> | CapturePreviewResult;
  dispose?(): Promise<void> | void;
}

export interface CoreAiCapturePreviewConfig {
  implementation: 'coreai';
  bridge: CoreAiCapturePreviewBridge;
}

export type CapturePreviewParserConfig = TypeScriptCapturePreviewConfig | RustCliCapturePreviewConfig | CoreAiCapturePreviewConfig;

export interface CapturePreviewImplementationDescriptor {
  parser: 'capture';
  implementation: CapturePreviewImplementation;
  contract_version: CapturePreviewContractVersion;
  boundary: CapturePreviewImplementationBoundary;
  host: CapturePreviewImplementationHost;
  status: CapturePreviewImplementationStatus;
}

export interface CapturePreviewParserInfo {
  parser: 'capture';
  implementation: CapturePreviewImplementation;
  contract_version: CapturePreviewContractVersion;
  boundary: CapturePreviewImplementationBoundary;
  host: CapturePreviewImplementationHost;
  command?: string;
  resourceRoot?: string;
}

export interface CapturePreviewParser {
  mode: CapturePreviewImplementation;
  info: CapturePreviewParserInfo;
  parse(input: string | CapturePreviewRequest, opts?: CapturePreviewOptions): Promise<CapturePreviewResult>;
  dispose(): Promise<void>;
}

export interface CapturePreviewSourceResults {
  intent: IntentResult;
  task?: TaskParseResult;
  event?: EventParseResult;
  reminder?: ReminderParseResult;
}
