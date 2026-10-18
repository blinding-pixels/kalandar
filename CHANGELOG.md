# Changelog

## 0.2.0

### Added

- **`kalandar-dates`.** A new Rust crate in `rust/kalandar-dates` resolves dates, times, repeats, exceptions, durations, lead times, alerts and public holidays. It is a Rust port of chrono-node v2.10.1 with a capture resolution layer, imported with its history. JavaScript, Rust and Swift all use it.
- **`schedule` on capture previews.** Results carry the resolved start, end, duration, repeat (as an `RRULE`, with exception dates), lead time, alert and notify time. The resolver's warnings appear in `warnings` with severity `info`.
- **Public holidays.** Holidays for every country and region in date-holidays 3.37.0 are embedded, plus computed Indian lunisolar festivals, resolved for the capture's region.
- **JavaScript date resolution through WebAssembly.** `src/dates` embeds the `kalandar-dates` WebAssembly build and exports `captureSchedule`, `resolveUnlabelled` and `holidayDataInfo` from `kalandar/dates`.
- **Swift date resolution through UniFFI.** The new `KalandarDates` product calls `kalandar-dates` through a UniFFI binding to a static-library XCFramework (`KalandarDatesFFI`). A `kalandar-dates-schedule` command-line tool is used for parity checks.
- **Build and check scripts:**
  - `npm run dates:wasm` and `npm run dates:xcframework` rebuild the WebAssembly and XCFramework.
  - `npm run dates:parity` checks that WebAssembly and Swift match native Rust on every evaluation row.
  - `npm test` runs the JavaScript tests.
- **`ort-download` cargo feature, on by default.** It holds ONNX Runtime's `download-binaries`, `copy-dylibs` and `tls-native` features. With `default-features = false`, `kalandar` downloads nothing at build time and builds no TLS stack.

### Models

- **Retrained models.** Task, calendar and reminder ship the round-two "repair" bundles. The labels and vocabulary are unchanged. The suite is `kalandar-capture-parser-onnx-v2`, and the postprocess versions are v2.
- **Decoding** reads the first subword piece of each whitespace word, as the models were evaluated. Task and reminder take the argmax. Calendar uses the constrained BIO Viterbi decoder from the round-two tensor contract, whose rules ship as `calendar/sequence_decoder.json`. JavaScript, Rust (`kalandar::sequence_decoder`) and Swift decode identically.
- **Manifests** carry a `decoding` field. Manifests that still say v1 are rejected.
- **Release status.** These models are marked experimental pending a blind evaluation. The training report flags field regressions for task (project 7/10 against 8/10 shipped, assignee 8/13 against 9/13) and reminder (trigger phrases 107/124 against 110/124, location 5/8 against 6/8).

### Changed

- **Draft `_text` fields hold the phrase from the capture**, such as `tomorrow`, `3pm` or `every other Tuesday`, in all three implementations. In the JavaScript parsers, fields that used to hold resolved ISO dates or `RRULE` strings (`date`, `time`, `remind_at`, `recurrence`) now hold the phrase too; the resolved values are in `schedule`.
- **Date and time splitting follows `kalandar-dates`.**
  - Times drop `at` (`3pm`, not `at 3pm`).
  - A `by`, `before` or `on` that introduces a date stays with it (`by friday`).
  - A clock time after a repeat goes to the time field.
- **A date with no time gives an all-day result** instead of an assumed time.
- **One parity case changes.** `Lunch with Sara every other Tuesday starting next month` has a 60 minute duration, because lunch sets a 12:00 time. Its locked parity value and the Rust golden fixtures are updated.
- **The Swift lightweight parser and the Core AI path** take every date field from `kalandar-dates`.
- **The JavaScript warning set follows the resolver.** `ambiguous_time`, `remind_at_past` and `recurrence_day_mismatch` are no longer raised.

### Removed

- **The `chrono-node` dependency** and all JavaScript date-parsing code built on it. This removes the calendar `chronoPass`, `recurrencePass`, `alertsPass` and `durationPass`, the reminder time passes, and the task `durationPass` and `dateTypoCorrectionPass`.
- **The regular-expression date extraction** in the Rust lightweight parsers.
- **The Swift parsers' hand-written date, repeat and duration matching.**

### Licensing

- **`holidays.bin` is CC BY-SA 3.0.** `rust/kalandar-dates/src/capture/holidays.bin` is generated from date-holidays. It ships as a separate data file, with the licence and attribution in `rust/kalandar-dates/LICENSES/date-holidays.txt`, and the npm package licence is now `(MIT AND CC-BY-SA-3.0)`.
- **Everything else stays MIT,** including the code and the computed `festivals.bin`.
