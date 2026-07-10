# Use The JavaScript Parser

This task shows how to use the default in-process JavaScript implementation.

## Before You Begin

You need:

- Node.js 18 or newer for Node usage.
- An ESM-capable project.
- Git access to `blinding-pixels/kalandar`.

```sh
npm install git+ssh://git@github.com/blinding-pixels/kalandar.git#<git-ref>
# or
pnpm add git+ssh://git@github.com/blinding-pixels/kalandar.git#<git-ref>
```

Replace `<git-ref>` with a commit SHA, branch, or release tag. npm installation is also supported once a registry release exists.

## Create A Parser

Create the default parser:

```ts
import { createCapturePreviewParser } from 'kalandar/capture';

const parser = await createCapturePreviewParser();
```

The default parser has `mode: "typescript"`.

## Parse User Text

Pass user text and optional context:

```ts
const preview = await parser.parse('lunch with Sara tomorrow at noon', {
  timezone: 'America/Los_Angeles',
  locale: 'en-US',
});

if (preview.kind === 'calendar-event') {
  console.log(preview.event?.title);
  console.log(preview.event?.date_text);
  console.log(preview.event?.time_text);
}
```

## Use Known Assignees

Pass known assignees when task ownership matters:

```ts
const preview = await parser.parse('ask Maya to review the contract by Friday', {
  knownAssignees: ['Maya', 'Jon', 'Priya'],
  timezone: 'America/New_York',
});

console.log(preview.task?.assignees);
```

## Handle Unsupported Input

Handle unsupported or empty text before creating app objects:

```ts
const preview = await parser.parse('');

if (preview.kind === 'unsupported') {
  console.warn(preview.unavailable_reason);
  console.warn(preview.warnings);
}
```

## Use Explicit Prefixes

Use prefixes when your UI already knows the target object type:

```ts
await parser.parse('task: finish report tomorrow');
await parser.parse('reminder: water plants every Friday at 9am');
await parser.parse('event: planning review next Tuesday at 2pm');
```

Prefixes bypass intent ambiguity and parse the payload as the requested kind.

## What's Next

- Inspect result fields in [Capture preview v1](../reference/capture-preview-v1.md).
- Compare implementations in [Implementation paths](../concepts/implementation-paths.md).
- Build a larger flow in [Build a capture preview flow](../tutorials/build-capture-preview.md).
