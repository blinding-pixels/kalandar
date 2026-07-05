# Getting Started

This page shows how to install Kalandar and parse your first capture with the default JavaScript implementation.

## Before You Begin

You need:

- Node.js 18 or newer.
- An ESM-capable project.

## Install Kalandar

Install the npm package:

```sh
npm install kalandar
# or
pnpm add kalandar
```

## Parse A Capture

Create a parser and parse user input:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser();

const preview = await parser.parse('task: send renewal invoice tomorrow at 3pm #finance', {
  timezone: 'America/New_York',
  locale: 'en-US',
});

console.log(preview.kind);
console.log(preview.task);
```

The output is similar to:

```json
{
  "kind": "task",
  "confidence": 0.99,
  "task": {
    "title": "send renewal invoice",
    "tags": ["finance"],
    "due_date_text": "tomorrow",
    "due_time_text": "3pm"
  }
}
```

The actual result also includes `candidates`, `event`, `reminder`, `thought`, `unavailable_reason`, and `warnings`.

## Use The Sync Convenience API

For lightweight JavaScript-only apps, use `parseCapturePreviewInput`:

```ts
import { parseCapturePreviewInput } from 'kalandar';

const preview = parseCapturePreviewInput('lunch with Sara tomorrow at noon', {
  timezone: 'America/Los_Angeles',
});

console.log(preview.event?.title);
```

## Select Another Implementation

The default parser uses the in-process JavaScript implementation. Native hosts can choose another implementation through `createCapturePreviewParser`.

| Implementation | Use when |
| --- | --- |
| `typescript` | You need web or Node parsing without native binaries. |
| `rust-cli` | You want a native process boundary for Windows, Linux, or cross-platform desktop apps. |
| `coreai` | You have an Apple-native host that can return `capture-preview-v1` results. |

## What's Next

- Learn how implementations differ in [Implementation paths](./concepts/implementation-paths.md).
- Use the default parser with [Use the JavaScript parser](./tasks/use-typescript.md).
- Inspect the exact result schema in [Capture preview v1](./reference/capture-preview-v1.md).
