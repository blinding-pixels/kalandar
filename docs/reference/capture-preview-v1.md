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
  schedule?: PreviewSchedule | null;
}
```

Exactly one draft field should be populated for `task`, `calendar-event`, `reminder`, or `thought` results.

The draft fields that end in `_text` hold the phrase from the capture, such as `tomorrow`, `3pm` or `every other Tuesday`. The resolved dates, times and repeat rule are in `schedule`. All three implementations compute the date fields and `schedule` with the same Rust resolver, `kalandar-dates`; see [Date resolution](../concepts/date-resolution.md). `schedule` was added in 0.2.0. Swift encodes a missing value by leaving the key out, while JavaScript and Rust emit `null`.

## Direct Swift Extensions

`CoreAIParserSuite` returns the typed Swift draft together with entity spans and
model metadata. Its reminder draft can include `leadTimeText`, serialized as
`lead_time_text`, and its calendar draft can include `descriptionText`, serialized
as `description_text`. These optional fields are model enrichments for direct
Swift callers; the portable `capture-preview-v1` contract above does not require
JavaScript or Rust callers to emit or consume them.

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

Warnings do not always mean parsing failed. They can also describe ambiguity or partial extraction. Warnings from the date resolver use severity `info` and the resolver's codes, such as `time_assumed`, `no_time`, `ambiguous_next_weekday`, `converted_timezone` or `holiday_other_region`. [Resolution conventions](../../rust/kalandar-dates/docs/resolution-conventions.md) lists every code.

## Schedule

```ts
interface PreviewSchedule {
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

interface PreviewMoment {
  date: string;
  time: string | null;
  local: string | null;
  instant: string | null;
  all_day: boolean;
  implied: string[];
  alternative_date: string | null;
}

interface PreviewRecurrence {
  rrule: string;
  frequency: 'minutely' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'yearly';
  interval: number;
  days: string[] | null;
  weekdays: { position: number | null; weekday: string }[];
  month_days: number[];
  months: number[];
  time: string | null;
  count: number | null;
  start_date: string | null;
  end_date: string | null;
  until: string | null;
  exdates: string[];
  exceptions: CapturePreviewRecurrenceException[];
}
```

| Field | Description |
| --- | --- |
| `start` | Event start, task due, or reminder trigger. A date with no time is all-day: `all_day` is `true`, and `time`, `local` and `instant` are `null`. |
| `end` | Event end. An end the capture didn't state lists `end` in `implied`. |
| `duration_minutes` | End minus start, or a stated duration. |
| `recurrence` | The repeat as an RFC 5545 `RRULE`, the `{frequency, interval, days}` shape the reminder draft also uses, and the resolved exception dates. |
| `lead_minutes`, `alert_minutes_before`, `alert_at`, `alerts_off` | Lead time and alert instructions. |
| `notify_at` | For an all-day task or reminder, the date at the default notification time, 09:00. It is a display hint and doesn't change the due date. |
| `date`, `time` | Local date `YYYY-MM-DD` and time `HH:MM` in the request time zone. |
| `local` | Local date and time `YYYY-MM-DDTHH:MM:SS`. |
| `instant` | The same moment with its UTC offset, in RFC 3339. |
| `implied` | Parts the resolver assumed, such as `time`, `date`, `year`, `meridiem` or `end`. |
| `alternative_date` | Another reading of an ambiguous phrase, such as `next Friday` said on a Wednesday. |

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
  resourceRoot: 'node_modules/kalandar/resources/capture-parser',
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
- Use direct model parsing in [Use Swift and Core AI](../tasks/use-coreai.md).
- Review runtime tensor adapters in [Runtime adapters](./runtime-adapters.md).
