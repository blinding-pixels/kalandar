# chrono-english-rs

A Rust port of the English parser in [chrono-node](https://github.com/wanasit/chrono) **v2.10.1**.

```
chrono-node: Copyright (c) 2014, Wanasit Tanakitrungruang
Released under the MIT License. The full notice is in LICENSE.
This crate is a derivative work and keeps that copyright and licence.
```

The goal is the same output as chrono-node for the same text, reference instant and time zone. It is a line-by-line port, not a redesign. Parser order, refiner order, regular expressions and the date arithmetic quirks all follow the TypeScript source.

## Usage

```rust
use chrono::{TimeZone, Utc};
use chrono_english_rs::{chrono_tz, parse, Component, ParsingOption, ParsingReference, SystemZone};

let now = Utc.with_ymd_and_hms(2026, 10, 18, 13, 0, 0).unwrap();
let reference = ParsingReference::new(now)
    .with_system_zone(SystemZone::Iana(chrono_tz::America::New_York));
let results = parse("lunch with Sam tomorrow at 1", &reference, &ParsingOption::forward());

let first = &results[0];
assert_eq!(first.text, "tomorrow at 1");
assert_eq!(first.start.get(Component::Day), Some(19));
assert_eq!(first.start.get(Component::Hour), Some(1));
assert!(first.start.is_certain(Component::Hour));
assert!(!first.start.is_certain(Component::Second));
```

Entry points:

- `parse`, `parse_date`: the casual English parser (`chrono.parse`, `chrono.parseDate`).
- `casual()`, `strict()`, `gb()`: the three English configurations (`chrono.casual`, `chrono.strict`, `chrono.en.GB`). Each returns a `&'static Chrono` with `parse` and `parse_date`.

`ParsingReference` carries:

- `instant`: the reference instant (`ParsingReference.instant`).
- `timezone`: `Some(TimezoneInput::Offset(minutes))` or `Some(TimezoneInput::Abbreviation("PST"))`, as in `ParsingReference.timezone`. `None` means "use the system zone", as in JavaScript.
- `system_zone`: the zone that plays the part of the JavaScript host's local time zone. See below.

`ParsingOption` carries `forward_date` and `timezones` (extra abbreviations, fixed or DST-dependent).

Each `ParsedResult` has `index`, `text`, `start`, and an optional `end`. `ParsedComponents` has `get`, `is_certain`, `is_implied`, `certain_components`, `date` and `tags`, matching `ParsedComponents` in chrono-node.

## The system zone

chrono-node does its calendar arithmetic with JavaScript `Date` objects, so the host's local time zone leaks into results. It decides the reference wall clock when no `timezone` is given, which local day "last night" and "this week" start from, and how DST gaps are resolved. The JavaScript capture parser runs in the browser, so its "system zone" is the user's zone.

This crate makes that zone explicit. `SystemZone::Utc` (the default) behaves like node with `TZ=UTC`. `SystemZone::Fixed(minutes)` is a fixed offset. `SystemZone::Iana(tz)` (feature `tz`, on by default, uses chrono-tz) behaves like node with `TZ=<that zone>`. That includes DST gap and overlap handling as the ECMAScript spec defines it. Years after 2037 are mapped to a calendar-equivalent year inside the tz tables, which matches how V8 extends the last DST rule.

To reproduce the browser parser, pass the user's IANA zone as `system_zone` and leave `timezone` empty. To pin the reference to a known offset, also set `with_timezone_offset`.

## How the port works

- `jsdate.rs` reimplements the parts of the ECMAScript `Date` that chrono-node uses: `MakeDay`/`MakeTime` overflow (Jan 31 plus one month is March 3), the 0 to 99 year rule in the `Date` constructor and `Date.UTC`, local and UTC getters and setters, and `TimeClip`. All of chrono-node's arithmetic runs on this type, so the quirks carry over too.
- `js_regex.rs` translates each JavaScript pattern into a `fancy-regex` pattern with JavaScript semantics. `\d`, `\w`, `\s`, `\b`, `.` and `/i` become explicit ASCII classes, because the Rust defaults are Unicode-aware and would match more (for example, `(?i)s` matches the long s `ſ`). Lookarounds need `fancy-regex`. A plain `regex` copy of each pattern with the lookarounds removed runs first. It rejects most non-matching text and finds where the real search should start. With it, parsing runs about as fast as chrono-node on node 23.
- Parsers run on the remaining text after each match, exactly like `Chrono.executeParser`. Skips after a failed extraction follow the JavaScript `match.index` updates.
- `clone()` in chrono-node drops tags, and `addDuration` mutates its argument, adding carried fraction units. Both behaviours are kept because later checks depend on them.
- Result positions are UTF-8 byte offsets, so `&text[r.index..]` works. chrono-node reports UTF-16 offsets. The test helpers convert.

## Files

- `src/`: the port (about 4,800 lines with rustfmt layout, 4,400 non-blank, no comments).
- `src/tests/`: chrono-node's English test suite, generated from the upstream jest files by `tools/port-tests`. One hand-ported test lives in `hand_ported.rs`.
- `tools/port-tests/`: runs the upstream jest files under node against the real chrono-node with recording shims, then writes the Rust tests from what each `expect` checked. Run `./record.sh` then `python3 generate.py`.
- `difftest/`: the differential harness. `./difftest/run_all.sh` builds the phrase sets, runs node and the crate on every case and writes any mismatch to `difftest/*diffs.jsonl`.
- `examples/diff_runner.rs`: reads JSON lines of cases and prints every component of every result.

## Known differences from chrono-node

None showed up in the differential runs. These are the deliberate or theoretical ones:

- `index` is a byte offset, not a UTF-16 offset.
- `date()` returns `Option<DateTime<Utc>>`. `None` is where JavaScript would return an Invalid Date.
- Component values are `i32`. JavaScript can carry `NaN` after absurd overflows (billions of days). Those results fail the validity filter in both versions.
- `ENMergeRelativeAfterDateRefiner` throws a `TypeError` in JavaScript when the text after a date starts with `+` or `-` but holds no time unit. The port treats that as an empty duration. No English parser produces such a result.
- Custom DST rules for extra time zone abbreviations are `DstRule` values (nth or last weekday of a month, or a `fn(i32) -> NaiveDateTime`) rather than arbitrary closures.
- Not ported: other locales, the debug handler, and adding custom parsers or refiners at runtime.

## Facets layer (`facets` module, feature `facets`, on by default)

The `facets` module sits on top of the port and doesn't change it. The port's tests and the differential harness still run against the unmodified parser. The layer turns the date and time phrases that Facets' models mark in a captured line into dates, times and repeat rules, following the written conventions in `docs/facets-conventions.md`.

### Calling it

```rust
use chrono_english_rs::facets::{resolve_capture, CaptureKind, Context, LabelledSpan, Role, Settings};

let ctx = Context::new(now_utc, chrono_tz::America::Los_Angeles).with_settings(Settings::default());
let spans = [LabelledSpan::new(Role::EventStart, 15, 23), LabelledSpan::new(Role::EventStart, 24, 28)];
let resolution = resolve_capture("lunch with Sam tomorrow at 1", CaptureKind::CalendarEvent, &spans, &ctx);
```

- `resolve_line(line, kind, spans, ctx)` trusts the spans as given. Each span has a role (`Role::from_label` maps the model labels such as `start_time`, `due_date`, `trigger_time`, `lead_time` and `recurrence`) and byte offsets into the line. The whole line is parsed once, and only the results that overlap a span of the right role are used.
- `resolve_capture(line, kind, spans, ctx)` is the recommended entry point. The spans say where the date and time words are, and the layer decides what they are. It merges fragmented spans ("last" + "Friday of" + "every month"), finds repeat phrases the model labelled as a due date, and falls back to its own detection when the model marked nothing, with a warning.
- `resolve_unlabelled(line, kind, ctx)` and `resolve_phrase(phrase, role, kind, ctx)` cover no spans and a single labelled phrase.
- `parse_recurrence(text, settings)` parses a repeat phrase on its own.

The result (`LineResolution`, serialisable with serde) has the start (event start, task due or reminder trigger), the end (calendar events), the recurrence (`rule`, an `RRULE` string, the shape Facets stores today as `{frequency, interval, days}`, start and end dates, exceptions, and the first occurrence), an alert (relative, absolute or off), lead minutes, `notify_at` (for a date-only task or all-day reminder, the date at the default notification time; it never changes the due), the task signal, and warnings. Every moment lists which parts were `implied` and carries warnings such as `time_assumed`, `rolled_to_tomorrow`, `moved_to_next_week`, `ambiguous_meridiem`, `approximate_time`, `end_assumed`, `start_adjusted`, `dst_gap_shifted` and `dst_overlap_earlier`. The app can show the guess instead of refusing to save.

### What it adds to chrono-node

- Role defaults: bare hours read by context ("lunch at 1" is 13:00), past reminder times roll to tomorrow, a date without a time is date-only for every kind (a task's due is a plain date, a reminder or event is all-day), timed events get the default length, and lead times resolve against their event.
- Recurrence: daily, weekly, monthly and yearly rules with intervals, weekday lists, positions ("first Monday", "every second and fourth Sunday"), month days including the last day, yearly dates, workdays and weekends, counts, spans ("for 6 weeks"), start, end and exceptions. The first occurrence is checked against the rule.
- Bare ordinals and ordinal ranges ("on the 14th", "from the 8th to the 10th"), which chrono-node returns nothing for.
- Public holidays for every country and region in date-holidays (embedded at build time, see below), resolved for the user's region, plus computed Indian festivals. A match is always that holiday's date, seasons, "quarter past", "half past", "quarter to", "half an hour", fortnights, business days, approximate times ("around 3", "3ish"), meal and activity times, "after work", "end of day/week/month", alert phrases, task signals, and a small typo and shorthand table ("wednsday", "2moro", "7a", "@ 4").
- IANA zones through chrono-tz. Local times in a DST gap move forward by the gap; times in an overlap take the earlier instant. Both are flagged, and neither drops the date.
- Settings: 12 or 24-hour clock, week start, weekend days, the hour the day starts, default event length, and default times for morning, afternoon, evening, tonight, after work, end of day, reminders and each meal, plus the default notification time for date-only items.

### WebAssembly

`wasm/` is a separate crate that wraps the layer and the raw parser with wasm-bindgen: `resolveLine`, `resolveCapture`, `resolveUnlabelled` and `parse`. Each function takes and returns a JSON string, and span offsets are UTF-16 as in JavaScript. `wasm/build.sh` builds the Node and web packages (wasm-bindgen-cli 0.2.121, installed under `../.tools`). `wasm/test.sh` runs 1,946 calls through both the wasm build and the same code compiled natively, and checks that the outputs are identical. The wasm file is 3.2 MB, 0.76 MB gzipped. Most of that is the time zone database.

### Fuzzing

`fuzz/` has three cargo-fuzz targets: `phrase_resolver`, `recurrence_parser` and `chrono_parse`. On this Mac, AddressSanitizer hangs at start-up, so `fuzz/run.sh` builds with `--sanitizer none` and runs each binary directly under a wall-clock kill.


## Holiday data

`src/facets/holidays.bin` is a generated table. It is built from [date-holidays](https://github.com/commenthol/date-holidays) by `tools/holidays/regenerate.sh`, at the version and npm integrity hash pinned in `tools/holidays/date-holidays.pin`.

- **No npm in the tree.** The script installs the pinned package into a throwaway folder, checks the hash, runs `tools/holidays/generate.mjs`, writes the table, the differential-test fixture (`tests/data/holiday_samples.json`) and the licence and attribution file (`LICENSES/date-holidays.txt`), then deletes the folder.
- **Bumping the version.** `regenerate.sh --bump <version>` pins a new version and prints which holidays were added, removed or moved per country.
- **Staleness.** `tools/holidays/check.sh` reports how old the pinned version is and whether a newer one exists. It needs the network and is for developers only.
- **Header.** The table records the source version, its integrity hash and the generation date. `facets::holiday_data_info()` exposes them.
- **Indian festivals.** The table also holds the computed Indian festivals, documented in `docs/indian-festivals.md`.

date-holidays is licensed "(ISC AND CC-BY-3.0)". The holiday data is the CC part; see `LICENSES/date-holidays.txt` for the licence text and the attribution that must ship with it.
