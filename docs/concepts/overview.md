# Overview

Kalandar is a client-first parsing library for productivity capture text.

It turns short text into one of these app-facing drafts:

- task
- reminder
- calendar event
- thought
- unsupported input

## Capture Preview

The main public contract is `capture-preview-v1`.

A capture preview request contains user text plus optional context, such as timezone, locale, current time, and known assignees. A capture preview result contains one selected kind, confidence, alternate candidates, one populated draft object, and warnings.

Use capture preview when your app needs to show a confirmation screen before creating a real task, reminder, or event.

## Parser Surfaces

Kalandar exposes both high-level and low-level APIs.

High-level APIs accept text and return app-facing drafts:

- `parseCapturePreviewInput`
- `createCapturePreviewParser`
- parser-specific exports such as `kalandar/task`, `kalandar/reminder`, and `kalandar/calendar`

Low-level runtime APIs accept tensors and return tensors:

- `createLocalOnnxInferenceAdapter`
- `createLocalCoreAiInferenceAdapter`
- `createRemoteInferenceAdapter`

Most apps should start with the high-level capture API.

## Implementation Paths

Kalandar has three capture-preview implementations:

- `typescript`: in-process JavaScript.
- `rust-cli`: JSON over standard input and standard output.
- `coreai`: host-provided Apple-native bridge.

All implementations return the same `capture-preview-v1` result shape. This lets an app change implementation without changing its confirmation UI.

## Model Artifacts

The base package does not bundle large model artifacts. Instead, model-backed paths use artifact bundles that are loaded separately by a host app.

An artifact bundle includes:

- model file
- tokenizer files
- label config
- tensor contract metadata
- normalization and postprocess versions
- content hashes

The runtime validates the bundle before inference.

## What Kalandar Is Not

Kalandar is not:

- a hosted inference service
- a model artifact registry
- a calendar, task, or reminder database
- a telemetry pipeline
- a replacement for your app's confirmation and validation UI

## What's Next

- Choose an implementation in [Implementation paths](./implementation-paths.md).
- Learn how artifacts work in [Artifacts and inference](./artifacts-and-inference.md).
- Parse your first input in [Getting started](../getting-started.md).
