# Kalandar Documentation

Kalandar parses short natural-language captures into app-facing task, reminder, calendar-event, or thought drafts.

Use this documentation by goal:

| Goal | Start here |
| --- | --- |
| Install Kalandar from private Git or parse text in JavaScript. | [Getting started](./getting-started.md) |
| Choose JavaScript, Rust, or Swift/Core AI. | [Implementation paths](./concepts/implementation-paths.md) |
| Understand how dates, repeats and holidays are resolved. | [Date resolution](./concepts/date-resolution.md) |
| Understand model artifacts and inference adapters. | [Artifacts and inference](./concepts/artifacts-and-inference.md) |
| Use the default JavaScript parser. | [Use the JavaScript parser](./tasks/use-typescript.md) |
| Use the Rust CLI from a host app. | [Use the Rust CLI](./tasks/use-rust-cli.md) |
| Use the Apple-native path. | [Use Swift and Core AI](./tasks/use-coreai.md) |
| Self-host model inference. | [Self-host inference](./tasks/self-host-inference.md) |
| Build a full capture preview flow. | [Build a capture preview flow](./tutorials/build-capture-preview.md) |
| Look up exact request and response fields. | [Capture preview reference](./reference/capture-preview-v1.md) |
| Prepare Git tags and optional registry releases. | [Publishing Kalandar](./release/publishing.md) |

## Documentation Map

### Concepts

Concept pages explain how Kalandar is designed. Read these before making integration decisions.

- [Overview](./concepts/overview.md)
- [Implementation paths](./concepts/implementation-paths.md)
- [Date resolution](./concepts/date-resolution.md)
- [Artifacts and inference](./concepts/artifacts-and-inference.md)

### Tasks

Task pages show how to do one thing.

- [Use the JavaScript parser](./tasks/use-typescript.md)
- [Use the Rust CLI](./tasks/use-rust-cli.md)
- [Use Swift and Core AI](./tasks/use-coreai.md)
- [Self-host inference](./tasks/self-host-inference.md)

### Tutorials

Tutorial pages combine multiple concepts and tasks.

- [Build a capture preview flow](./tutorials/build-capture-preview.md)

### Reference

Reference pages define contracts and package contents.

- [Capture preview v1](./reference/capture-preview-v1.md)
- [Runtime adapters](./reference/runtime-adapters.md)
- [Package layout](./reference/package-layout.md)

### Release

Release pages cover verification and publishing readiness.

- [Publishing Kalandar](./release/publishing.md)
- [Release checklist](./release/checklist.md)

### Contributing To Docs

- [Documentation style guide](./style-guide.md)

## What's Next

Install from Git with [Getting started](./getting-started.md), then choose an implementation path.
