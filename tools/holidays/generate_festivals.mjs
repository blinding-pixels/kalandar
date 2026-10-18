import { createHash } from "crypto";
import { readFileSync, writeFileSync } from "fs";
import { encode, FLAG_COMPUTED, FLAG_ADDED } from "./format.mjs";
import { FESTIVALS } from "./india.mjs";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((pairs, a, i, all) => (a.startsWith("--") ? [...pairs, [a.slice(2), all[i + 1]]] : pairs), []),
);
const FIRST = 1970;
const LAST = 2100;
const RULES_VERSION = "1";

const hash = createHash("sha512");
for (const file of ["astro.mjs", "india.mjs"]) hash.update(readFileSync(new URL(`./${file}`, import.meta.url)));

const definitions = [];
const series = [];
const entries = [];
for (const f of [...FESTIVALS].sort((a, b) => a.id.localeCompare(b.id))) {
  const list = [];
  for (let y = FIRST; y <= LAST; y++) {
    const day = f.compute(y);
    if (!day) continue;
    const date = `${day[0]}-${String(day[1]).padStart(2, "0")}-${String(day[2]).padStart(2, "0")}`;
    list.push({ date, offset: 0, minutes: 1440, substitute: false, kind: null });
  }
  entries.push({ definition: definitions.length, series: series.length, kind: f.kind });
  definitions.push({ id: `facets:${f.id}`, flags: FLAG_COMPUTED | FLAG_ADDED, names: f.names });
  series.push(list);
}
const bytes = encode({
  source: "facets-astronomy",
  version: RULES_VERSION,
  integrity: `sha512-${hash.digest("base64")}`,
  generated: args.generated ?? new Date().toISOString().slice(0, 10),
  firstYear: FIRST,
  lastYear: LAST,
  definitions,
  series,
  regions: [{ code: "IN", parent: null, name: "India", zones: [], languages: [], entries }],
});
writeFileSync(args.out, bytes);
process.stderr.write(`wrote ${args.out}: ${bytes.length} bytes, ${definitions.length} festivals\n`);
