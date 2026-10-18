import { readFileSync } from "fs";
import { createRequire } from "module";
import { execFileSync } from "child_process";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import assert from "assert";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const wasm = require(join(here, "..", "pkg", "chrono_english_wasm.js"));

const callsText = readFileSync(join(here, "calls.jsonl"), "utf8");
const calls = callsText.split("\n").filter((l) => l.trim()).map((l) => JSON.parse(l));
const native = execFileSync(process.env.NATIVE_BIN, { input: callsText, maxBuffer: 1 << 28 })
  .toString()
  .split("\n")
  .filter((l) => l.trim());

assert.strictEqual(native.length, calls.length, "native output count");
let compared = 0;
let nonEmpty = 0;
for (let i = 0; i < calls.length; i++) {
  const call = calls[i];
  const fromWasm = wasm[call.fn](JSON.stringify(call.request));
  assert.ok(!JSON.parse(fromWasm).error, `wasm error for ${JSON.stringify(call)}: ${fromWasm}`);
  assert.deepStrictEqual(JSON.parse(fromWasm), JSON.parse(native[i]), `mismatch for ${JSON.stringify(call)}`);
  compared++;
  if (fromWasm !== "[]" && !fromWasm.includes('"start":null')) nonEmpty++;
}

const lunch = JSON.parse(
  wasm.resolveUnlabelled(JSON.stringify({ line: "lunch with Sam tomorrow at 1", kind: "calendar", now: "2026-10-18T17:00:00Z", zone: "America/Los_Angeles" }))
);
assert.strictEqual(lunch.start.local, "2026-10-19T13:00:00");
assert.strictEqual(lunch.end.local, "2026-10-19T14:00:00");

const weekly = JSON.parse(
  wasm.resolveUnlabelled(JSON.stringify({ line: "remind me every Monday and Wednesday", kind: "reminder", now: "2026-10-18T17:00:00Z", zone: "America/Los_Angeles" }))
);
assert.strictEqual(weekly.recurrence.rrule, "FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE");
assert.deepStrictEqual(weekly.recurrence.facets, { frequency: "weekly", interval: 1, days: ["monday", "wednesday"] });

const parsed = JSON.parse(wasm.parse(JSON.stringify({ text: "next friday at 5pm", ref: 1760792400000, zone: "UTC", tz: null, forward: false, config: "casual" })));
assert.strictEqual(parsed[0].start.date, 1761325200000);
console.log(`wasm parity: ${compared} calls identical to native Rust (${nonEmpty} with a result), spot checks passed`);
