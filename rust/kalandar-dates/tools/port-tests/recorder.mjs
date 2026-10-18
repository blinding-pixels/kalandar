import fs from "fs";
import path from "path";
import { pathToFileURL } from "url";

const out = { tests: [] };
let current = null;
let lastAccess = null;
let nextParse = 0;

function jsonable(v) {
  if (v instanceof Date) return { __date: v.getTime() };
  if (v instanceof Set) return { __set: Array.from(v) };
  if (v === undefined) return { __undefined: true };
  if (typeof v === "number" && Number.isNaN(v)) return { __nan: true };
  return v;
}

globalThis.__recorder = {
  event(e) {
    if (e.type === "parse") e.id = nextParse++;
    if (!current) throw new Error("event outside test");
    current.events.push(e);
    lastAccess = null;
    return e.id;
  },
  access(parseId, p, value) { lastAccess = { parseId, path: p, value }; },
  unportable(reason) { if (current) current.unportable.push(reason); },
  fail(msg) { if (current) current.failures.push(msg); },
};

const { metaOf } = await import("./shim_chrono.mjs");

function sourceOf(actual) {
  const m = metaOf(actual);
  if (m) return { parse: m.parseId, path: m.path };
  if (lastAccess && Object.is(lastAccess.value, actual)) return { parse: lastAccess.parseId, path: lastAccess.path };
  if (actual && actual.__perf) return { perf: true };
  return null;
}

function matcherSet(actual, src, negate) {
  const record = (matcher, expected, ok) => {
    let expSrc = null;
    if (lastAccess && expected !== undefined && Object.is(lastAccess.value, expected)) expSrc = { parse: lastAccess.parseId, path: lastAccess.path };
    let s = src, e = expected, constant = actual;
    if (!s && expSrc) { s = expSrc; e = actual; }
    const pass = negate ? !ok : ok;
    if (!pass) current.failures.push(`${matcher} failed`);
    if (!s) { current.unportable.push(`expect() on untracked value with ${matcher}`); lastAccess = null; return; }
    current.events.push({ type: "assert", parse: s.parse, path: s.path, perf: s.perf, matcher, expected: jsonable(e), not: negate, ok: pass });
    lastAccess = null;
  };
  const real = (v) => (metaOf(v) ? metaOf(v).target : v);
  const a = real(actual);
  return {
    toBe: (v) => record("toBe", v, Object.is(a, v)),
    toEqual: (v) => record("toEqual", v, JSON.stringify(a) === JSON.stringify(v)),
    toStrictEqual: (v) => record("toStrictEqual", v, JSON.stringify(a) === JSON.stringify(v)),
    toBeNull: () => record("toBeNull", null, a === null),
    toBeUndefined: () => record("toBeUndefined", undefined, a === undefined),
    toBeDefined: () => record("toBeDefined", undefined, a !== undefined),
    toBeTruthy: () => record("toBeTruthy", undefined, !!a),
    toBeFalsy: () => record("toBeFalsy", undefined, !a),
    toContain: (v) => record("toContain", v, a instanceof Set ? a.has(v) : a.includes(v)),
    toBeCloseTo: (v) => record("toBeCloseTo", v, Math.abs(a - v) < 0.005),
    toBeLessThan: (v) => record("toBeLessThan", v, (a && a.__perf ? a.ms : a) < v),
    toBeGreaterThan: (v) => record("toBeGreaterThan", v, a > v),
    toBeLessThanOrEqual: (v) => record("toBeLessThanOrEqual", v, a <= v),
    toHaveLength: (v) => record("toHaveLength", v, a.length === v),
    toBeDate: (d) => {
      const date = typeof a.date === "function" ? a.date() : null;
      lastAccess = null;
      record("toBeDate", d, date !== null && date.getTime() === d.getTime());
    },
  };
}

globalThis.expect = (actual) => {
  const src = sourceOf(actual);
  lastAccess = null;
  const base = matcherSet(actual, src, false);
  base.not = matcherSet(actual, src, true);
  return base;
};

const pending = [];
globalThis.test = (name, fn) => pending.push({ name, fn });
globalThis.it = globalThis.test;
globalThis.describe = (name, fn) => fn();

const files = process.argv.slice(3);
for (const f of files) {
  pending.length = 0;
  await import(pathToFileURL(path.resolve(f)).href);
  for (const t of pending) {
    current = { file: path.basename(f), name: t.name, events: [], unportable: [], failures: [] };
    try {
      t.fn();
    } catch (e) {
      current.failures.push("exception: " + e.message);
    }
    out.tests.push(current);
    current = null;
  }
}
fs.writeFileSync(process.argv[2], JSON.stringify(out, null, 1));
const failing = out.tests.filter(t => t.failures.length);
console.log(`tests=${out.tests.length} failing_in_node=${failing.length} unportable=${out.tests.filter(t => t.unportable.length).length}`);
for (const t of failing) console.log("FAIL", t.file, t.name, t.failures.slice(0, 3));
for (const t of out.tests.filter(t => t.unportable.length)) console.log("UNPORTABLE", t.file, t.name, [...new Set(t.unportable)]);
