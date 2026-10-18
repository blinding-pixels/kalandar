export const MAGIC = "FHOL";
export const FORMAT_VERSION = 1;
export const KINDS = ["public", "bank", "school", "optional", "observance"];
export const FLAG_COMPUTED = 1;
export const FLAG_MOON_SIGHTING = 2;
export const FLAG_ADDED = 4;

const EPOCH = Date.UTC(1970, 0, 1);
export const dayNumber = (key) => {
  const [y, m, d] = key.split("-").map(Number);
  return Math.round((Date.UTC(y, m - 1, d) - EPOCH) / 86400000);
};
export const dayKey = (n) => new Date(EPOCH + n * 86400000).toISOString().slice(0, 10);

class Writer {
  constructor() {
    this.bytes = [];
  }
  byte(b) {
    this.bytes.push(b & 0xff);
  }
  varint(v) {
    if (!Number.isSafeInteger(v) || v < 0) throw new Error(`bad varint ${v}`);
    while (v >= 0x80) {
      this.bytes.push((v % 0x80) | 0x80);
      v = Math.floor(v / 0x80);
    }
    this.bytes.push(v);
  }
  signed(v) {
    this.varint(v < 0 ? -2 * v - 1 : 2 * v);
  }
  raw(buf) {
    for (const b of buf) this.bytes.push(b);
  }
  text(s) {
    const buf = Buffer.from(s, "utf8");
    this.varint(buf.length);
    this.raw(buf);
  }
}

class Reader {
  constructor(buf) {
    this.buf = buf;
    this.pos = 0;
  }
  byte() {
    if (this.pos >= this.buf.length) throw new Error("truncated");
    return this.buf[this.pos++];
  }
  varint() {
    let v = 0;
    let scale = 1;
    for (;;) {
      const b = this.byte();
      v += (b & 0x7f) * scale;
      if (!(b & 0x80)) return v;
      scale *= 0x80;
    }
  }
  signed() {
    const v = this.varint();
    return v % 2 ? -(v + 1) / 2 : v / 2;
  }
  text() {
    const n = this.varint();
    const s = this.buf.subarray(this.pos, this.pos + n).toString("utf8");
    this.pos += n;
    return s;
  }
}

export function encode(table) {
  const strings = new Map();
  const intern = (s) => {
    if (!strings.has(s)) strings.set(s, strings.size);
    return strings.get(s);
  };
  const languages = [...new Set(table.definitions.flatMap((d) => Object.keys(d.names)).concat(table.regions.flatMap((r) => r.languages)))].sort();
  const langIndex = new Map(languages.map((l, i) => [l, i]));
  languages.forEach(intern);
  for (const d of table.definitions) {
    intern(d.id);
    for (const l of Object.keys(d.names).sort()) intern(d.names[l]);
  }
  for (const r of table.regions) {
    intern(r.code);
    intern(r.name);
    r.zones.forEach(intern);
  }
  const body = new Writer();
  body.raw(Buffer.from(MAGIC));
  body.byte(FORMAT_VERSION);
  body.text(table.source);
  body.text(table.version);
  body.text(table.integrity);
  body.text(table.generated);
  body.varint(table.firstYear);
  body.varint(table.lastYear);
  body.varint(strings.size);
  for (const s of strings.keys()) body.text(s);
  body.varint(languages.length);
  for (const l of languages) body.varint(strings.get(l));
  body.varint(table.definitions.length);
  for (const d of table.definitions) {
    body.varint(strings.get(d.id));
    body.byte(d.flags);
    const langs = Object.keys(d.names).sort();
    body.varint(langs.length);
    for (const l of langs) {
      body.varint(langIndex.get(l));
      body.varint(strings.get(d.names[l]));
    }
  }
  body.varint(table.series.length);
  for (const list of table.series) {
    body.varint(list.length);
    let previous = 0;
    for (const o of list) {
      const day = dayNumber(o.date);
      const extra = o.offset !== 0 || o.minutes !== 1440;
      const kind = o.kind ?? null;
      body.varint((day - previous) * 8 + (kind !== null ? 4 : 0) + (o.substitute ? 2 : 0) + (extra ? 1 : 0));
      if (extra) {
        body.signed(o.offset);
        body.varint(o.minutes);
      }
      if (kind !== null) body.byte(KINDS.indexOf(kind));
      previous = day;
    }
  }
  body.varint(table.regions.length);
  for (const r of table.regions) {
    body.varint(strings.get(r.code));
    body.varint(r.parent === null ? 0 : r.parent + 1);
    body.varint(strings.get(r.name));
    body.varint(r.zones.length);
    for (const z of r.zones) body.varint(strings.get(z));
    body.varint(r.languages.length);
    for (const l of r.languages) body.varint(langIndex.get(l));
    body.varint(r.entries.length);
    for (const e of r.entries) {
      body.varint(e.definition);
      body.varint(e.series);
      body.byte(KINDS.indexOf(e.kind));
    }
  }
  return Buffer.from(body.bytes);
}

export function decode(buf) {
  const r = new Reader(buf);
  if (buf.subarray(0, 4).toString() !== MAGIC) throw new Error("not a holiday table");
  r.pos = 4;
  const format = r.byte();
  if (format !== FORMAT_VERSION) throw new Error(`format ${format}`);
  const out = { source: r.text(), version: r.text(), integrity: r.text(), generated: r.text(), firstYear: r.varint(), lastYear: r.varint() };
  const strings = [];
  for (let n = r.varint(); n > 0; n--) strings.push(r.text());
  const languages = [];
  for (let n = r.varint(); n > 0; n--) languages.push(strings[r.varint()]);
  out.definitions = [];
  for (let n = r.varint(); n > 0; n--) {
    const id = strings[r.varint()];
    const flags = r.byte();
    const names = {};
    for (let k = r.varint(); k > 0; k--) names[languages[r.varint()]] = strings[r.varint()];
    out.definitions.push({ id, flags, names });
  }
  out.series = [];
  for (let n = r.varint(); n > 0; n--) {
    const list = [];
    let day = 0;
    for (let k = r.varint(); k > 0; k--) {
      const head = r.varint();
      day += Math.floor(head / 8);
      const extra = head % 2 === 1;
      const o = { date: dayKey(day), offset: 0, minutes: 1440, substitute: Math.floor(head / 2) % 2 === 1, kind: null };
      if (extra) {
        o.offset = r.signed();
        o.minutes = r.varint();
      }
      if (Math.floor(head / 4) % 2 === 1) o.kind = KINDS[r.byte()];
      list.push(o);
    }
    out.series.push(list);
  }
  out.regions = [];
  for (let n = r.varint(); n > 0; n--) {
    const code = strings[r.varint()];
    const p = r.varint();
    const name = strings[r.varint()];
    const zones = [];
    for (let k = r.varint(); k > 0; k--) zones.push(strings[r.varint()]);
    const langs = [];
    for (let k = r.varint(); k > 0; k--) langs.push(languages[r.varint()]);
    const entries = [];
    for (let k = r.varint(); k > 0; k--) entries.push({ definition: r.varint(), series: r.varint(), kind: KINDS[r.byte()] });
    out.regions.push({ code, parent: p === 0 ? null : p - 1, name, zones, languages: langs, entries });
  }
  if (r.pos !== buf.length) throw new Error("trailing bytes");
  return out;
}
