# Kalandar Architecture

This document is the canonical architecture for Kalandar v1.

It exists to freeze the product shape before more integration work lands. If code and this document diverge, treat that as a bug and resolve it deliberately rather than letting the code define policy by accident.

## Product Shape

Kalandar is a client-first parsing library for productivity text.

The system has four parser surfaces:

- `intent`
- `calendar`
- `task`
- `reminder`

The package must support two parser families:

- lightweight rule-based parsing for small JS/TS apps
- ML-backed parsing for higher accuracy

Both families must converge on the same outward contracts wherever possible so applications can swap parser modes without redesigning their integration.

## Core Principles

1. Parsers remain independently usable.
2. Shared contracts exist underneath them for interoperability.
3. Cross-object behavior happens through adapters, not parser coupling.
4. Kalandar ships as a library, not as a hosted service.
5. Telemetry is opt-in only and disabled by default.
6. ML artifacts are optional add-ons, not mandatory npm payload.

## Public Package Shape

Kalandar should ship as one package with tree-shakeable exports:

- `kalandar/intent`
- `kalandar/calendar`
- `kalandar/task`
- `kalandar/reminder`
- shared planning/adaptor exports

The package should not force all consumers to install or load every parser path. Lightweight users must be able to consume only the rule-based or tiny-runtime paths they need.

## Implementation Surfaces

Kalandar has two API layers that must stay separate:

- high-level parser APIs, such as `createCapturePreviewParser`, accept app text and return app-facing parser results
- low-level runtime APIs, such as `InferenceAdapter`, accept tensor requests and return tensor responses

The high-level capture preview contract is `capture-preview-v1`. Current implementations are:

- `typescript`: in-process JavaScript/TypeScript rules, intended for web and JS apps
- `rust-cli`: JSON-over-stdio Rust parser, intended for cross-platform native hosts
- `coreai`: native host bridge for Apple apps; the Swift/Core AI host must return the same app-facing DTO

The package owns the `coreai` app-facing contract, not the Apple-only runtime. A Swift or native Apple host is responsible for loading Core AI models, mapping model output back into `capture-preview-v1`, and passing that result through the bridge. Treat a Core AI host as shippable only after it passes the same locked app-output parity fixtures as `typescript` and `rust-cli`.

The checked local Core AI parity report is currently tensor-level. Full Core AI app-output parity requires passing a Swift/native capture command to `scripts/coreai-parity.mjs` with `--coreai-app-command` and `--require-coreai-app`.

## Parser Contracts

The product has parser-specific outputs and a shared planning layer.

Parser-specific results remain first-class because each domain has behavior the others do not:

- `TaskParseResult`
- `ReminderParseResult`
- `EventParseResult`
- `IntentResult`

The shared planning layer exists for downstream composition:

- task to reminder
- task to calendar block
- event to reminder
- app-specific notification or workflow adapters

The shared planning layer must standardize temporal fields wherever possible, including:

- `date`
- `time`
- `end_time`
- `duration`
- `lead_time`
- `location`
- `recurrence`
- `recurrence_window`
- `recurrence_start`
- `recurrence_end`
- `recurrence_exception`

Parser-specific fields remain available and are not erased by the shared layer.

## Intent Routing

Intent routing is a four-way problem:

- `task`
- `calendar`
- `reminder`
- `none`

The default runtime intent classifier is the tiny hashed linear model in `intent/`, not the large training artifact.

## ML Runtime Strategy

Kalandar supports two ML integration modes:

1. Local ML
2. Remote inference on user-owned infrastructure

In both cases, tokenization and postprocessing live on the client/application side.

### Local ML

The host application performs:

- normalization
- tokenization
- ONNX inference
- postprocessing

### Remote ML

The host application performs:

- normalization
- tokenization
- sends tensors to user-owned inference infrastructure
- receives model outputs
- postprocessing

Kalandar must not ship a first-party maintained server package for inference.

## Why Tokenization Lives Client Side

Tokenization and inference must share a strict artifact contract, but Kalandar should not solve this by shipping server software.

The design choice is:

- client-side tokenization
- client-side postprocessing
- optional local or remote inference backends

This avoids first-party server maintenance while still allowing self-hosted inference on arbitrary user infrastructure.

## InferenceAdapter Contract

Kalandar should define an inference abstraction that isolates parser logic from backend choice.

The package should know how to talk to:

- a local ONNX runtime adapter
- any user-supplied remote adapter that implements the same tensor contract

Kalandar must not know about specific platforms like SageMaker, Modal, Triton, or Cloudflare. Users can build adapters for those systems without Kalandar taking on a platform support surface.

## Atomic Artifact Bundles

ML artifacts must be distributed as immutable atomic bundles.

Each bundle includes:

- tokenizer config
- ONNX weights
- label maps
- normalization version
- postprocess version
- artifact metadata

Artifacts must not be assembled from independently versioned pieces.

The system must treat a bundle as one atomic unit and reject partial or mismatched combinations.

## Suite-Level Versioning

Kalandar model artifacts move as a suite, not as fully independent parser versions.

That means:

- one suite version for the model family
- optional separate downloads for `calendar`, `task`, and `reminder`
- all downloads must belong to the same suite version

This keeps the support matrix sane and avoids cross-parser drift.

## Artifact Handshake

Every ML inference flow must validate a strict artifact identity.

The artifact identity must be a single content hash over the full artifact bundle, exposed as `artifact_id`.

The runtime must reject:

- stale tokenizer with newer model weights
- stale label maps
- mismatched postprocess logic
- any partial artifact bundle

Failure should be loud and explicit, never silent degradation.

## Tensor Contract

The tensor contract is part of the public architecture and must be documented and frozen.

It must specify:

- input tensor names
- input tensor dtypes
- input tensor shapes
- output tensor names
- output tensor shapes
- maximum sequence length
- padding strategy
- attention mask convention
- truncation behavior
- empty input behavior
- all-whitespace input behavior
- boundary behavior for oversized inputs

This contract is the interoperability layer for remote inference and must remain stable within a suite version.

## Postprocessing

Postprocessing is part of the runtime package, not a separately swappable concern.

That includes logic such as:

- boundary cleanup
- title reconstruction
- exception extraction
- recurrence phrase handling
- contact suggestion surfacing

Postprocess behavior is part of the artifact compatibility story and must be versioned with the artifact metadata.

## Confidence Signals

Kalandar may expose confidence-like signals in parser-specific results, including:

- parser-local field confidence
- span confidence
- ambiguity flags
- warnings

These signals are valid runtime API surface and may be consumed by host applications.

However, they are not part of Kalandar's public product-playbook guidance.

### Documentation Policy For Confidence

Confidence should be documented minimally and neutrally:

- what the field is
- where it appears
- that it is parser-local rather than globally calibrated

Confidence should not be documented in a way that teaches product strategy, monetization strategy, or opinionated UX patterns.

That means public-facing docs should avoid guidance like:

- how to design under-confidence highlighting flows
- when to interrupt users for confirmation
- how to turn confidence into premium quick-capture UX
- how to build correction funnels from parser uncertainty

In practice:

- confidence is allowed in the API
- confidence may be mentioned in type docs or reference docs
- confidence should not become a featured README selling point
- confidence should not be accompanied by prescriptive product examples unless there is an explicit future decision to do that

This is an intentional project policy, not an omission.

## Distribution Strategy

### npm Package

The npm package should contain:

- rule parsers
- tiny intent runtime
- shared contracts
- adapters
- tokenization and postprocess logic
- local runtime integration code

The npm package should not contain large ML model binaries by default.

### Model Artifact Hosting

Primary artifact distribution:

- Hugging Face Hub

Mirror/fallback:

- GitHub Releases

Kalandar should not use npm as the primary model distribution channel, and it should not rely on a first-party CDN.

Artifacts must be self-hostable by users if they need air-gapped or internal deployments.

## Self-Hosting Guidance

Kalandar should provide docs-only guidance for self-hosted inference.

That means:

- one minimal reference example in documentation
- no maintained server package
- no maintained Docker image
- no separate first-party inference server repository

The goal is to enable self-hosting without turning Kalandar into a server product.

## Privacy and Telemetry

Kalandar must emit zero telemetry by default.

Specifically:

- no phone-home behavior
- no built-in analytics
- no default crash reporting
- no hidden diagnostics uploads

Diagnostics may exist only through explicit host-provided hooks, such as:

- logger interfaces
- debug callbacks
- explicit user-owned instrumentation

Privacy is part of the product value and must remain an invariant.

## Rule-Based and ML Coexistence

Rule-based and ML parsers are both first-class modes.

Rule-based parsing is optimized for:

- tiny footprint
- deterministic behavior
- easy distribution

ML parsing is optimized for:

- higher accuracy
- richer boundary handling
- broader natural-language coverage

They should share output shapes and planning adapters as much as possible, but the library should not pretend they have identical capabilities.

## Current v1 Direction

The v1 launch direction is:

- one client-first package
- tree-shakeable parser exports
- tiny runtime intent classifier
- optional suite-versioned ML artifacts
- client-side tokenization and postprocess
- local or user-hosted inference via a stable tensor contract

In one sentence:

Kalandar ships as one client-first package with tree-shakeable parser exports, suite-versioned atomic ML artifacts, client-side tokenization and postprocess, and a frozen tensor contract for optional user-hosted inference backends.
