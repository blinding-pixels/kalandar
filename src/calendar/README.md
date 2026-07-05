# Calendar NLP Parser

Offline, tiny-footprint natural-language parser for event-like input.

## Weekday Semantics (Option B)

- Bare weekday (`sunday`) resolves to the upcoming occurrence, including today if today matches.
- `this <weekday>` first targets that weekday in the current week, including today if today matches. If that date would be in the past, it rolls forward to the next occurrence and marks the parse with lower confidence.
- `next <weekday>` resolves to the upcoming occurrence, except when today already is that weekday.
  On same-day input, it resolves to `+7 days`.
- `next week <weekday>` resolves to the weekday in next ISO week (next week's Monday baseline).
- Relative weekday phrases never resolve to a past date.

## API

```ts
import { parseEventInput } from './index';

const result = parseEventInput('dentist thursday at 3 remind me 30m before', {
  now: '2026-02-25T12:00:00-05:00',
  timezone: 'America/New_York',
  locale: 'en-US',
  debug: true,
});
```

ML-capable integrations use the async parser factory:

```ts
import { createCalendarParser } from './index';
import { loadMlArtifact } from '../runtime/index';
import { createLocalOnnxInferenceAdapter } from '../runtime/onnx/index';

const artifact = await loadMlArtifact({
  kind: 'directory',
  path: '/path/to/calendar-artifact-bundle',
});

const parser = await createCalendarParser({
  mode: 'ml',
  artifact,
  inference: createLocalOnnxInferenceAdapter(),
});

const result = await parser.parse('design review next Tuesday at 3', {
  now: '2026-04-18T10:00:00-04:00',
  timezone: 'America/New_York',
});
```

## Output Schema

```ts
EventParseResult {
  fields: {
    title: string | null;
    date: string | null; // YYYY-MM-DD
    time: string | null; // HH:MM (24h)
    end_time: string | null; // HH:MM (24h)
    duration_minutes: number | null;
    location: string | null;
    recurrence: string | null; // RRULE-lite string such as FREQ=WEEKLY;INTERVAL=1;BYDAY=TU
    recurrence_start: string | null; // ISO date for simple recurrence anchors
    recurrence_end: string | null; // ISO date for simple recurrence cutoffs
    recurrence_exception: Array<{
      raw: string;
      condition:
        | { type: 'holiday' }
        | { type: 'weekend' }
        | { type: 'ordinal'; which: 'first' | 'second' | 'third' | 'last' }
        | { type: 'specific_date'; date: string }
        | { type: string; [key: string]: unknown }
        | null;
      action:
        | { type: 'skip' }
        | { type: 'shift'; to: 'monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday'; direction: 'following' | 'preceding' }
        | { type: 'replace'; with: string }
        | { type: string; [key: string]: unknown }
        | null;
    }>;
    alerts: Array<{ offset_minutes: number }> | null; // negative means before
    attendees: string[] | null; // email only
    contact_suggestions: Array<{ text: string; source: 'title_with_phrase' | 'attendee_clause' }> | null;
  };
  meta: {
    locale: string;
    timezone: string;
    runtime?: {
      mode: 'lightweight' | 'ml';
      suite_version?: string;
      artifact_id?: string;
    };
    confidence: {
      title: number;
      date: number;
      time: number;
      end_time: number;
      duration_minutes: number;
      location: number;
      recurrence: number;
      recurrence_start: number;
      recurrence_end: number;
      recurrence_exception: number;
      alerts: number;
      attendees: number;
      contact_suggestions: number;
    };
    spans: {
      title: FieldSpan[];
      date: FieldSpan[];
      time: FieldSpan[];
      end_time: FieldSpan[];
      duration_minutes: FieldSpan[];
      location: FieldSpan[];
      recurrence: FieldSpan[];
      recurrence_start: FieldSpan[];
      recurrence_end: FieldSpan[];
      recurrence_exception: FieldSpan[];
      alerts: FieldSpan[];
      attendees: FieldSpan[];
      contact_suggestions: FieldSpan[];
    };
    cleanedInput: string;
    maskedInput: string;
    debugSteps: DebugStep[];
  };
}
```

## Simple-Parser Boundary

The lightweight rule parser is intentionally small and deterministic.

Supported:
- explicit dates and clock times
- simple recurring rules like `every Tuesday`, `every other Tuesday`, `every second Tuesday of the month`
- recurrence anchors like `starting next month`
- recurrence cutoffs like `till July 1st`
- structured recurrence exception clauses like `unless it's a holiday then the following Thursday`
- preserving `with X` inside the title while also exposing `contact_suggestions`

Intentionally unsupported:
- fuzzy planning language
- relative-time anchors that depend on outside context, such as `from half past the hour of our usual Monday sync`
- full parity with the ML parser on nested or speculative scheduling language

Resolver support:
- built-in exception resolution for `weekend`, `ordinal`, and `specific_date`
- pluggable resolvers for `holiday` and future custom condition types
- missing resolvers degrade to `{ matches: false, unresolvable: true }`

## CLI

```bash
echo "dentist thursday at 3 remind me 30m before" | npm run parse-cli
```

Options:
- `--now <iso>`
- `--timezone <tz>`
- `--locale <locale>`
- `--debug`

When `--timezone` is omitted, the CLI uses the OS timezone and prints the detected timezone to stderr.

## Corpus Tests

The test harness loads external JSON corpus files (no corpus generation).

```bash
npm run test:nlp
```

Override corpus paths:

```bash
CALENDAR_NLP_CORPUS_PATHS="/path/to/corpus-a.json,/path/to/corpus-b.json" npm run test:nlp
```

Optional confidence tolerance override:

```bash
CALENDAR_NLP_CONFIDENCE_TOLERANCE=0.2 npm run test:nlp
```

On mismatch, tests print a golden diff with field-by-field expected/actual values and relevant spans.

Corpus harness behavior:
- `reference_now` and `timezone` in corpus `meta` are injected for deterministic tests.
- If either is omitted, parser defaults are used (`new Date()` and `Intl.DateTimeFormat().resolvedOptions().timeZone`).

## Curated Evaluation

Run the curated lightweight-parser fixture with:

```bash
node calendar/scripts/evaluate-simple-parser.mjs
```
