# Capture Preview v1

This page defines the `capture-preview-v1` request and response contract.

## Request

```ts
interface CapturePreviewRequest {
  text: string;
  now_iso?: string | null;
  timezone: string;
  locale?: string | null;
  known_assignees?: string[] | null;
}
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `text` | `string` | Yes | User capture text. |
| `now_iso` | `string \| null` | No | Current time as an ISO string. |
| `timezone` | `string` | Yes | IANA timezone used for date and time interpretation. |
| `locale` | `string \| null` | No | Locale hint. |
| `known_assignees` | `string[] \| null` | No | Names the task parser can recognize as assignees. |

## Options

JavaScript callers can pass `CapturePreviewOptions` instead of building the request manually:

```ts
interface CapturePreviewOptions {
  now?: Date | string;
  timezone?: string;
  locale?: string;
  knownAssignees?: string[];
}
```

## Result

```ts
interface CapturePreviewResult {
  kind: 'task' | 'calendar-event' | 'reminder' | 'thought' | 'ambiguous' | 'unsupported';
  confidence: number;
  candidates: CapturePreviewCandidate[];
  task: CapturePreviewTaskDraft | null;
  event: CapturePreviewCalendarEventDraft | null;
  reminder: CapturePreviewReminderDraft | null;
  thought: CapturePreviewThoughtDraft | null;
  unavailable_reason: string | null;
  warnings: CapturePreviewWarning[];
}
```

Exactly one draft field should be populated for `task`, `calendar-event`, `reminder`, or `thought` results.

## Candidate

```ts
interface CapturePreviewCandidate {
  kind: 'task' | 'calendar-event' | 'reminder';
  score: number;
}
```

Candidates are sorted by score descending.

## Task Draft

```ts
interface CapturePreviewTaskDraft {
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
```

## Reminder Draft

```ts
interface CapturePreviewReminderDraft {
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
```

## Calendar Event Draft

```ts
interface CapturePreviewCalendarEventDraft {
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
```

## Recurrence Exception

```ts
interface CapturePreviewRecurrenceException {
  raw: string;
  condition: string | null;
  action: string | null;
}
```

## Warning

```ts
interface CapturePreviewWarning {
  code: string;
  severity: string;
  message: string;
}
```

Warnings do not always mean parsing failed. They can also describe ambiguity or partial extraction.

## Parser Factory

```ts
const parser = await createCapturePreviewParser(config);
const preview = await parser.parse(input, options);
await parser.dispose();
```

## Implementation Configs

### TypeScript

```ts
await createCapturePreviewParser();
await createCapturePreviewParser({ implementation: 'typescript' });
```

### Rust CLI

```ts
await createCapturePreviewParser({
  implementation: 'rust-cli',
  command: '/path/to/kalandar-capture-preview',
  args: [],
  env: {},
  resourceRoot: '/path/to/capture-parser',
  timeoutMs: 30_000,
});
```

### Core AI

```ts
await createCapturePreviewParser({
  implementation: 'coreai',
  bridge: {
    previewCapture(request) {
      return nativeBridge.previewCapture(request);
    },
    dispose() {
      return nativeBridge.dispose?.();
    },
  },
});
```

## Implementation Descriptors

| Implementation | Boundary | Host | Status |
| --- | --- | --- | --- |
| `typescript` | `in-process` | `javascript` | `ready` |
| `rust-cli` | `json-stdio-cli` | `native` | `ready` |
| `coreai` | `native-host-bridge` | `apple-native` | `host-required` |

## What's Next

- Use the JavaScript parser in [Use the JavaScript parser](../tasks/use-typescript.md).
- Use the Rust CLI in [Use the Rust CLI](../tasks/use-rust-cli.md).
- Review runtime tensor adapters in [Runtime adapters](./runtime-adapters.md).
