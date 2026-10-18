import { readFileSync } from "fs";
import { FESTIVALS } from "./india.mjs";

const official = JSON.parse(readFileSync(new URL("./india_official.json", import.meta.url)));
const only = process.argv[2];
const summary = {};
for (const f of FESTIVALS) {
  if (only && f.id !== only) continue;
  const rows = [];
  for (const [year, entry] of Object.entries(official.years)) {
    const want = entry.festivals[f.id === "chaitra_navratri" ? "navratri_start" : f.id]?.date;
    if (!want) continue;
    const day = f.compute(Number(year));
    const got = day ? `${day[0]}-${String(day[1]).padStart(2, "0")}-${String(day[2]).padStart(2, "0")}` : null;
    rows.push({ year, want, got, ok: want === got });
  }
  summary[f.id] = { agree: rows.filter((r) => r.ok).length, total: rows.length, differ: rows.filter((r) => !r.ok).map((r) => `${r.year}: official ${r.want}, computed ${r.got}`) };
}
console.log(JSON.stringify(summary, null, 1));
