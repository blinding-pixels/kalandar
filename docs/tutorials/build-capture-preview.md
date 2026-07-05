# Build A Capture Preview Flow

This tutorial shows how to build a small capture preview flow that accepts user text, parses it, and decides what confirmation UI to show.

## Objectives

You will:

- create a parser
- parse text with timezone context
- branch on `preview.kind`
- show warnings and alternate candidates
- dispose the parser

## Before You Begin

Install Kalandar:

```sh
npm install kalandar
```

## Create The Parser

Use the default JavaScript implementation:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser();
```

## Parse User Input

Pass the text and app context:

```ts
const preview = await parser.parse('remind me to submit expenses every Friday at 4pm', {
  timezone: 'America/New_York',
  locale: 'en-US',
});
```

## Render By Kind

Switch on `preview.kind`:

```ts
switch (preview.kind) {
  case 'task':
    renderTaskDraft(preview.task);
    break;
  case 'reminder':
    renderReminderDraft(preview.reminder);
    break;
  case 'calendar-event':
    renderEventDraft(preview.event);
    break;
  case 'thought':
    renderThought(preview.thought);
    break;
  case 'ambiguous':
    renderCandidates(preview.candidates);
    break;
  case 'unsupported':
    renderUnsupported(preview.warnings);
    break;
}
```

## Show Warnings

Warnings are part of the contract and should be visible to the app, even when the parse succeeds:

```ts
for (const warning of preview.warnings) {
  console.warn(`${warning.code}: ${warning.message}`);
}
```

Use warnings to explain incomplete parses, unsupported details, or host-runtime problems.

## Offer Alternate Candidates

Candidates help a user recover when intent routing is close:

```ts
const alternatives = preview.candidates.filter((candidate) => candidate.kind !== preview.kind);
```

For example, a user can switch an ambiguous text from a task draft to a calendar-event draft.

## Dispose The Parser

Dispose the parser when your host no longer needs it:

```ts
await parser.dispose();
```

The default JavaScript parser has no native resources, but Rust and Core AI hosts can use disposal to release process or model handles.

## Cleanup

No external resources were created by this tutorial.

## What's Next

- Use a native implementation with [Use the Rust CLI](../tasks/use-rust-cli.md).
- Use an Apple-native bridge with [Use Swift and Core AI](../tasks/use-coreai.md).
- Look up exact fields in [Capture preview v1](../reference/capture-preview-v1.md).
