# Documentation Style Guide

This page defines the style for Kalandar documentation.

## Content Types

Use one content type per page:

| Type | Purpose | Directory |
| --- | --- | --- |
| Concept | Explain how something works. | `docs/concepts/` |
| Task | Show how to do one thing. | `docs/tasks/` |
| Tutorial | Walk through a complete goal. | `docs/tutorials/` |
| Reference | Define exact contracts. | `docs/reference/` |
| Release | Verify and publish packages. | `docs/release/` |

## Page Structure

Concept pages should include:

- a short definition
- when to use the concept
- important constraints
- links to related tasks

Task pages should include:

- `Before You Begin`
- steps that a reader can run
- troubleshooting when useful
- `What's Next`

Tutorial pages should include:

- `Objectives`
- `Before You Begin`
- ordered work sections
- `Cleanup`
- `What's Next`

Reference pages should include:

- exact names
- exact types
- tables for fields
- examples only when they clarify the contract

## Writing Style

Use:

- U.S. English.
- Present tense.
- Active voice.
- Direct language.
- `you` for the reader.
- `Kalandar` for the package.
- Backticks for code, commands, paths, package names, and field names.

Avoid:

- vague link text such as "click here".
- hidden assumptions about local machines.
- undocumented placeholders.
- future promises.
- putting task steps inside concept pages.

## Code Blocks

Do not include a shell prompt in commands.

Use placeholders in angle brackets:

```sh
cargo build --manifest-path <path-to-Cargo.toml>
```

Describe each placeholder before or after the command.

Separate commands from output:

```sh
npm audit --omit=dev
```

The output is similar to:

```text
found 0 vulnerabilities
```

## Links

Use descriptive link text:

- Good: `Read Capture preview v1 for the result schema.`
- Bad: `Click here for the result schema.`

Prefer relative links for files in this repository.

## What's Next

Add new docs to [the docs index](./README.md) when they become part of the public documentation set.
