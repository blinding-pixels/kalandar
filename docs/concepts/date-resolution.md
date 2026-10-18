# Date Resolution

Kalandar resolves every date, time, repeat, lead time and holiday in a capture with one Rust library, `kalandar-dates`, in `rust/kalandar-dates`. The JavaScript, Rust and Swift implementations all call it, so the same text gives the same dates on every platform.

## What It Does

`kalandar-dates` is a Rust port of the English parser in [chrono-node](https://github.com/wanasit/chrono) v2.10.1, plus a capture resolution layer on top. For each capture line it:

- finds the date, time, repeat, exception, duration, lead-time and alert phrases;
- resolves them in the request's time zone, using written conventions for words such as `next Friday`, `this weekend`, `end of month` or `in the morning`;
- builds repeat rules as an RFC 5545 `RRULE`, with exception dates;
- resolves public holidays for the user's region;
- reports every guess as a warning.

The parsers use the phrases for the draft `_text` fields and remove them from the title. They return the resolved values in the result's `schedule` field. The written conventions are in [Resolution conventions](../../rust/kalandar-dates/docs/resolution-conventions.md).

## How Each Implementation Calls It

| Implementation | How it calls `kalandar-dates` |
| --- | --- |
| Rust | A normal crate dependency of `kalandar`. |
| JavaScript | A WebAssembly build embedded in `src/dates` and compiled on first use. It needs no network access and no native addon. |
| Swift | A static library in an XCFramework, called through a UniFFI binding in the `KalandarDates` target. |

`npm run dates:parity` runs every evaluation row in `rust/kalandar-dates/eval` through native Rust, the WebAssembly build and the Swift binding, and fails if any output differs.

## Holiday Data

Holidays come from two embedded tables:

- `rust/kalandar-dates/src/capture/holidays.bin`, generated from [date-holidays](https://github.com/commenthol/date-holidays). This file is under CC BY-SA 3.0. Its licence and the required attribution are in `rust/kalandar-dates/LICENSES/date-holidays.txt`.
- `rust/kalandar-dates/src/capture/festivals.bin`, Indian lunisolar festivals computed by Kalandar's own astronomy routines. This file is under Kalandar's MIT licence. [Indian festivals](../../rust/kalandar-dates/docs/indian-festivals.md) documents how they're computed and how well they match the published government lists.

`rust/kalandar-dates/tools/holidays/regenerate.sh` rebuilds the tables from the pinned date-holidays version. Nothing is downloaded at runtime.

## Constraints

- The resolver reads English. Other languages are left unresolved.
- The JavaScript build adds about 4.3 MB of WebAssembly to the package, embedded as about 5.7 MB of base64. It is compiled synchronously on first use, so the parsers stay synchronous. Recent Chrome versions allow synchronous compilation of modules up to 8 MB on the main thread; on older browsers, run the parser in a worker.

## Loading The WebAssembly In A Browser

The package also ships the module as a file, `dist/dates/wasm/kalandar_dates_wasm_bg.wasm`, exported as `kalandar/dates/kalandar_dates_wasm_bg.wasm`. Serve it as a static asset, then compile it before the first parse with `initKalandarDatesAsync`, which takes a URL, a `Response`, a promise of one, or the bytes:

```ts
import { initKalandarDatesAsync, parseCapturePreviewInput } from 'kalandar/capture';

await initKalandarDatesAsync(fetch('/assets/kalandar_dates_wasm_bg.wasm'));
parseCapturePreviewInput('pay rent by Friday 5pm');
```

A served `.wasm` file with the `application/wasm` content type compiles with `WebAssembly.instantiateStreaming` and caches like any other asset, at about 0.67 MB with brotli. Once the async init resolves, the parsers never touch the embedded copy, so a bundler can drop it: alias `kalandar/dates/wasm/bytes` to `kalandar/dates/wasm/no-bytes`. Without the alias, calling a parser before the async init resolves compiles the embedded copy; with it, that call throws.
- The Swift binding needs the `KalandarDatesFFI` XCFramework. A source checkout builds it with `npm run dates:xcframework`; tagged releases attach it as a release asset.

## What's Next

- Read the `schedule` fields in [Capture preview v1](../reference/capture-preview-v1.md).
- Rebuild the WebAssembly or XCFramework in [Publishing Kalandar](../release/publishing.md).
