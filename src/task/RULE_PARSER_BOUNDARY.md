# Task Rule Parser Boundary

This rule parser is meant to be a deterministic, tiny fallback for small apps. It targets short imperative task inputs and extracts cleaned surface spans for the current task contract:

- `title`
- `assignee[]`
- `project`
- `tags[]`
- `priority`
- `status`
- `duration`
- `due_date`
- `due_time`
- `recurrence`
- `recurrence_start`
- `recurrence_end`
- `recurrence_exception`

## Supported

- Short task commands with explicit metadata clauses.
- Assignees from `to <name>`, `with <name>`, `for <name>`, and `@mentions`.
- Projects from explicit `project` phrases such as `for Project Atlas` and `in project Helios`.
- Tags from trailing `tags:` clauses and hashtags.
- Priority, status, and duration from common deterministic phrases.
- One-off `due_date` and `due_time` spans for common date/time wording.
- Recurrence spans such as `every Tuesday`, `every other Tuesday`, `starting next month`, `till July 1st`, and basic `except ...` / `unless ...` clauses.

## Intentionally Unsupported

- Large fuzzy planning prompts or multi-step agendas.
- Deeply nested recurrence exception logic.
- Free-form rescheduling or date arithmetic from vague intent.
- Full parity with the ML parser on ambiguous person-vs-project cases.
- Turning recurrence into concrete generated dates inside this parser.
