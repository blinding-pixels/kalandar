import { readFileSync } from "fs";
import { decode } from "./format.mjs";

const [before, after] = process.argv.slice(2).map((p) => decode(readFileSync(p)));
const thisYear = new Date().getUTCFullYear();

function index(table) {
  const out = new Map();
  for (const region of table.regions) {
    const holidays = new Map();
    for (const e of region.entries) {
      const def = table.definitions[e.definition];
      const dates = table.series[e.series].filter((o) => !o.substitute).map((o) => o.date);
      holidays.set(def.id, { name: def.names.en ?? Object.values(def.names)[0] ?? def.id, dates, kind: e.kind });
    }
    out.set(region.code, holidays);
  }
  return out;
}

const a = index(before);
const b = index(after);
console.log(`date-holidays ${before.version} -> ${after.version}`);
const countries = [...new Set([...a.keys(), ...b.keys()])].sort();
let quiet = true;
for (const code of countries) {
  const old = a.get(code) ?? new Map();
  const now = b.get(code) ?? new Map();
  const lines = [];
  if (!a.has(code)) lines.push("  region added");
  if (!b.has(code)) lines.push("  region removed");
  for (const [id, h] of now) if (!old.has(id)) lines.push(`  + ${h.name} (${h.kind}), next ${h.dates.find((d) => Number(d.slice(0, 4)) >= thisYear) ?? "none"}`);
  for (const [id, h] of old) if (!now.has(id)) lines.push(`  - ${h.name} (${h.kind})`);
  for (const [id, h] of now) {
    const o = old.get(id);
    if (!o) continue;
    const moved = h.dates.filter((d) => !o.dates.includes(d) && Number(d.slice(0, 4)) >= thisYear);
    const dropped = o.dates.filter((d) => !h.dates.includes(d) && Number(d.slice(0, 4)) >= thisYear);
    if (moved.length || dropped.length) lines.push(`  ~ ${h.name}: ${dropped.slice(0, 3).join(", ") || "-"} -> ${moved.slice(0, 3).join(", ") || "-"}${moved.length > 3 ? ` (+${moved.length - 3} more)` : ""}`);
    if (o.kind !== h.kind) lines.push(`  ~ ${h.name}: type ${o.kind} -> ${h.kind}`);
  }
  if (lines.length) {
    quiet = false;
    console.log(code);
    for (const l of lines) console.log(l);
  }
}
if (quiet) console.log("no holidays added, removed or moved");
