import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "fs";
import { dirname, join } from "path";
import { pathToFileURL } from "url";
import { encode, FLAG_ADDED, FLAG_MOON_SIGHTING, KINDS } from "./format.mjs";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((pairs, a, i, all) => (a.startsWith("--") ? [...pairs, [a.slice(2), all[i + 1] && !all[i + 1].startsWith("--") ? all[i + 1] : "true"]] : pairs), []),
);
const FIRST = Number(args.first ?? 1970);
const LAST = Number(args.last ?? 2100);
const ALL_LANGUAGES = (args.languages ?? "all") === "all";
const PRIORITY_SAMPLES = ["US", "US-LA", "US-LA-NO", "GB", "GB-SCT", "CA", "CA-QC", "AU", "AU-NSW", "IN", "DE", "DE-BY", "FR", "JP", "CN", "SG", "BR", "MX", "ZA", "IL", "SA", "AE", "NZ", "IE", "ES-CT", "CH-ZH", "AT", "IT", "NL", "SE"];
const SAMPLE_YEARS = [1970, 1988, 2000, 2016, 2024, 2025, 2026, 2027, 2033, 2050, 2077, 2100];

const progress = (msg) => process.stderr.write(`${msg}\n`);

async function loadPackage(dir) {
  const { default: Holidays } = await import(pathToFileURL(join(dir, "src/index.js")).href);
  const { data } = await import(pathToFileURL(join(dir, "src/data.js")).href);
  return { Holidays, data };
}

function civil(h) {
  const [day, clock = "00:00:00"] = h.date.split(" ");
  const [hh, mm] = clock.split(":").map(Number);
  const minutes = Math.max(1, Math.round((h.end - h.start) / 60000));
  return { date: day, offset: hh * 60 + mm, minutes };
}

function rawNames(data, r, rule) {
  const c = data.holidays[r.country];
  const state = c?.states?.[r.state];
  const levels = [
    [c, r.country],
    [state, `${r.country}-${r.state}`.toUpperCase()],
    [state?.regions?.[r.region], r.code],
  ];
  let names = null;
  let id = null;
  for (const [level, code] of levels) {
    const entry = level?.days?.[rule];
    if (!entry || typeof entry !== "object") continue;
    if (entry._name && data.names?.[entry._name]?.name) {
      names = { ...(names ?? {}), ...data.names[entry._name].name };
      id = entry._name;
    }
    if (entry.name && typeof entry.name === "object") {
      names = { ...(names ?? {}), ...entry.name };
      id = `${code}:${rule}`;
    }
  }
  return names ? { id, names } : null;
}

const slug = (s) => s.normalize("NFKD").toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");

function definitionId(baseId, english) {
  const existing = definitions.get(baseId);
  if (!existing || existing.names.en === english) return baseId;
  return `${baseId}@${slug(english)}`;
}

const levelData = (data, country, state, region) => {
  const c = data.holidays[country];
  return [c?.states?.[state]?.regions?.[region], c?.states?.[state], c].filter(Boolean);
};

function regionList(Holidays) {
  const hd = new Holidays();
  const out = [];
  for (const country of Object.keys(hd.getCountries("en") ?? {}).sort()) {
    const countryName = hd.getCountries("en")[country];
    out.push({ code: country, country, state: undefined, region: undefined, name: countryName, parentCode: null });
    for (const state of Object.keys(hd.getStates(country, "en") ?? {}).sort()) {
      out.push({ code: `${country}-${state}`.toUpperCase(), country, state, region: undefined, name: hd.getStates(country, "en")[state], parentCode: country });
      for (const region of Object.keys(hd.getRegions(country, state, "en") ?? {}).sort()) {
        out.push({ code: `${country}-${state}-${region}`.toUpperCase(), country, state, region, name: hd.getRegions(country, state, "en")[region], parentCode: `${country}-${state}`.toUpperCase() });
      }
    }
  }
  return out;
}

const definitions = new Map();
function definition(id, names, flags) {
  if (!definitions.has(id)) definitions.set(id, { id, names: {}, flags });
  const d = definitions.get(id);
  for (const [lang, name] of Object.entries(names)) {
    if (typeof name !== "string" || !name.trim()) continue;
    if (!ALL_LANGUAGES && lang !== "en") continue;
    if (!d.names[lang]) d.names[lang] = name.trim();
  }
  return d;
}

function addOccurrence(entries, id, kind, occurrence) {
  if (!entries.has(id)) entries.set(id, { kinds: new Map(), list: new Map() });
  const e = entries.get(id);
  const key = `${occurrence.date}|${occurrence.substitute ? 1 : 0}`;
  if (!e.list.has(key)) e.list.set(key, { ...occurrence, type: kind });
  if (!occurrence.substitute) e.kinds.set(kind, (e.kinds.get(kind) ?? 0) + 1);
}

const ISLAMIC_INDIA = [
  { rule: "1 Shawwal", id: "IN:eid_ul_fitr", names: { en: "Eid al-Fitr", hi: "ईद उल-फ़ित्र" }, kind: "public" },
  { rule: "10 Dhu al-Hijjah", id: "IN:eid_ul_adha", names: { en: "Eid al-Adha", hi: "ईद उल-अज़हा" }, kind: "public" },
  { rule: "10 Muharram", id: "IN:muharram", names: { en: "Muharram", hi: "मुहर्रम" }, kind: "public" },
  { rule: "12 Rabi al-awwal", id: "IN:milad_un_nabi", names: { en: "Milad-un-Nabi", hi: "मिलाद-उन-नबी" }, kind: "public" },
];

function islamicIndia(Holidays, entries, coverage) {
  for (const spec of ISLAMIC_INDIA) {
    const hd = new Holidays("IN");
    const ok = hd.setHoliday(spec.rule, { name: { en: spec.names.en }, type: spec.kind });
    if (!ok) throw new Error(`date-holidays rejected the rule ${spec.rule}`);
    const d = definition(spec.id, spec.names, FLAG_MOON_SIGHTING | FLAG_ADDED);
    let years = 0;
    for (let y = FIRST; y <= LAST; y++) {
      const found = (hd.getHolidays(y) ?? []).filter((h) => h.rule === spec.rule && !h.substitute);
      if (found.length) years++;
      for (const h of found) addOccurrence(entries, d.id, spec.kind, { ...civil(h), substitute: false });
    }
    coverage[spec.id] = years;
  }
}

function sampleFixture(Holidays, regions) {
  const codes = new Set(PRIORITY_SAMPLES);
  regions.forEach((r, i) => {
    if (i % 23 === 0) codes.add(r.code);
  });
  const samples = [];
  for (const r of regions.filter((x) => codes.has(x.code))) {
    const hd = new Holidays(r.country, r.state, r.region);
    for (const year of SAMPLE_YEARS) {
      const holidays = [];
      for (const y of [year - 1, year, year + 1]) {
        for (const h of hd.getHolidays(y, "en") ?? []) {
          const c = civil(h);
          if (Number(c.date.slice(0, 4)) !== year) continue;
          holidays.push({ date: c.date, name: h.name, type: h.type, substitute: !!h.substitute });
        }
      }
      const unique = [...new Map(holidays.map((h) => [JSON.stringify(h), h])).values()];
      unique.sort((a, b) => (a.date + a.name).localeCompare(b.date + b.name));
      samples.push({ region: r.code, year, holidays: unique });
    }
  }
  return samples;
}

function attribution(dir) {
  const parts = [];
  const license = join(dir, "LICENSE");
  if (existsSync(license)) parts.push(readFileSync(license, "utf8").trim());
  const countries = join(dir, "data", "countries");
  if (existsSync(countries)) {
    const lines = [];
    for (const file of readdirSync(countries).filter((f) => f.endsWith(".yaml")).sort()) {
      const urls = [...readFileSync(join(countries, file), "utf8").matchAll(/@attrib\s+(\S+)/g)].map((m) => m[1]);
      if (urls.length) lines.push(`${file.replace(".yaml", "")}: ${[...new Set(urls)].join(" ")}`);
    }
    if (lines.length) parts.push(`Attribution listed in data/countries/*.yaml:\n${lines.join("\n")}`);
  }
  return parts.join("\n\n");
}

async function main() {
  const regionsOut = [];
  const series = [];
  const seriesIndex = new Map();
  const seriesId = (list) => {
    const key = JSON.stringify(list);
    if (!seriesIndex.has(key)) {
      seriesIndex.set(key, series.length);
      series.push(list);
    }
    return seriesIndex.get(key);
  };
  const perRegion = [];
  const coverage = {};
  let pkg = null;
  if (args.package) {
    pkg = await loadPackage(args.package);
    const regions = regionList(pkg.Holidays);
    progress(`${regions.length} countries and regions`);
    regions.forEach((r, n) => {
      if (n % 50 === 0) progress(`  ${n}/${regions.length} ${r.code}`);
      const hd = new pkg.Holidays(r.country, r.state, r.region);
      const entries = new Map();
      for (let y = FIRST - 1; y <= LAST + 1; y++) {
        for (const h of hd.getHolidays(y, "en") ?? []) {
          const c = civil(h);
          const year = Number(c.date.slice(0, 4));
          if (year < FIRST || year > LAST) continue;
          const english = h.name.replace(/\s*\(substitute day\)$/, "");
          const raw = rawNames(pkg.data, r, h.rule);
          const id = definitionId(raw?.id ?? `${r.code}:${h.rule}`, english);
          definition(id, { ...(raw?.names ?? {}), en: english }, 0);
          addOccurrence(entries, id, h.type, { ...c, substitute: !!h.substitute });
        }
      }
      if (r.country === "IN") islamicIndia(pkg.Holidays, entries, coverage);
      const level = levelData(pkg.data, r.country, r.state, r.region);
      perRegion.push({ r, entries, languages: hd.getLanguages() ?? [], zones: hd.getTimezones?.() ?? level.find((l) => l.zones)?.zones ?? [] });
    });
    writeFileSync(args.samples, `${JSON.stringify({ source: `date-holidays ${args.version}`, years: SAMPLE_YEARS, samples: sampleFixture(pkg.Holidays, regions) }, null, 0)}\n`);
    if (args.notice) {
      mkdirSync(dirname(args.notice), { recursive: true });
      writeFileSync(args.notice, `date-holidays ${args.version} (https://github.com/commenthol/date-holidays)\n\n${attribution(args.package)}\n`);
    }
  } else {
    throw new Error("--package <dir> is required");
  }
  const defList = [...definitions.values()].sort((a, b) => a.id.localeCompare(b.id));
  const defIndex = new Map(defList.map((d, i) => [d.id, i]));
  const codeIndex = new Map(perRegion.map((p, i) => [p.r.code, i]));
  for (const { r, entries, languages, zones } of perRegion) {
    const list = [...entries.entries()]
      .map(([id, e]) => {
        const sorted = [...e.list.values()].sort((a, b) => a.date.localeCompare(b.date) || Number(a.substitute) - Number(b.substitute));
        const kind = [...e.kinds.entries()].sort((a, b) => b[1] - a[1] || KINDS.indexOf(a[0]) - KINDS.indexOf(b[0]))[0]?.[0] ?? "observance";
        const occurrences = sorted.map((o) => ({ date: o.date, offset: o.offset, minutes: o.minutes, substitute: o.substitute, kind: o.type && o.type !== kind ? o.type : null }));
        return { definition: defIndex.get(id), series: seriesId(occurrences), kind };
      })
      .sort((a, b) => a.definition - b.definition);
    regionsOut.push({ code: r.code, parent: r.parentCode ? codeIndex.get(r.parentCode) ?? null : null, name: r.name ?? r.code, zones: [...zones].sort(), languages: [...languages], entries: list });
  }
  const table = {
    source: "date-holidays",
    version: args.version ?? "none",
    integrity: args.integrity ?? "none",
    generated: args.generated ?? new Date().toISOString().slice(0, 10),
    firstYear: FIRST,
    lastYear: LAST,
    definitions: defList,
    series,
    regions: regionsOut,
  };
  const bytes = encode(table);
  writeFileSync(args.out, bytes);
  progress(`wrote ${args.out}: ${bytes.length} bytes, ${regionsOut.length} regions, ${defList.length} holidays, ${series.length} distinct date series`);
  if (Object.keys(coverage).length) progress(`Islamic holidays for India, years covered: ${JSON.stringify(coverage)}`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
