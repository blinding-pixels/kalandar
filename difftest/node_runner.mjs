import { createRequire } from "module";
import readline from "readline";
const require = createRequire(import.meta.url);
const chrono = require(process.env.CHRONO_PATH);

const COMPONENTS = ["year","month","day","weekday","hour","minute","second","millisecond","meridiem","timezoneOffset"];
function comps(c) {
  if (!c) return null;
  const known = {}, implied = {};
  for (const k of COMPONENTS) {
    if (k in c.knownValues) known[k] = c.knownValues[k];
    else if (k in c.impliedValues) implied[k] = c.impliedValues[k];
  }
  const t = c.date().getTime();
  return { known, implied, date: Number.isNaN(t) ? null : t };
}
const configs = { casual: chrono.casual, strict: chrono.strict, gb: chrono.en.GB };
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of rl) {
  if (!line.trim()) continue;
  const c = JSON.parse(line);
  const ref = { instant: new Date(c.ref) };
  if (c.tz !== null && c.tz !== undefined) ref.timezone = c.tz;
  let results;
  try {
    results = configs[c.config || "casual"].parse(c.text, ref, { forwardDate: !!c.forward }).map(r => ({
      index: r.index, text: r.text, start: comps(r.start), end: comps(r.end), tags: Array.from(r.tags()).sort(),
    }));
  } catch (e) {
    results = { error: String(e) };
  }
  process.stdout.write(JSON.stringify({ id: c.id, results }) + "\n");
}
