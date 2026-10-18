import { metaOf } from "./shim_chrono.mjs";
const rec = globalThis.__recorder;

export function testSingleCase(chrono, text, refOrCb, optOrCb, cb) {
  if (cb === undefined && typeof optOrCb === "function") { cb = optOrCb; optOrCb = undefined; }
  if (optOrCb === undefined && typeof refOrCb === "function") { cb = refOrCb; refOrCb = undefined; }
  const results = chrono.parse(text, refOrCb, optOrCb || {});
  const m = metaOf(results);
  const ok = results.length === 1;
  rec.event({ type: "assert", parse: m.parseId, path: [{ call: "__count", args: [] }], matcher: "toBe", expected: 1, not: false, ok });
  if (!ok) rec.fail(`expected single result for '${text}' got ${results.length}`);
  if (cb && ok) cb(results[0], text);
}

export function testWithExpectedDate(chrono, text, expectedDate) {
  testSingleCase(chrono, text, (result) => {
    globalThis.expect(result.start).toBeDate(expectedDate);
  });
}

export function testUnexpectedResult(chrono, text, refDate, options) {
  const results = chrono.parse(text, refDate, options || {});
  const m = metaOf(results);
  const ok = results.length === 0;
  rec.event({ type: "assert", parse: m.parseId, path: [{ call: "__count", args: [] }], matcher: "toBe", expected: 0, not: false, ok });
  if (!ok) rec.fail(`expected no result for '${text}' got ${results.length}`);
}

export function measureMilliSec(block) {
  rec.event({ type: "perf_begin" });
  const start = Date.now();
  block();
  const ms = Date.now() - start;
  rec.event({ type: "perf_end" });
  return { __perf: true, ms };
}
