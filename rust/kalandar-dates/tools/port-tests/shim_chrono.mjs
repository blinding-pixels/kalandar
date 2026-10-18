import { createRequire } from "module";
const require = createRequire(import.meta.url);
const ROOT = process.env.CHRONO_PATH;
const real = require(ROOT);
const realTimezone = require(ROOT + "/dist/cjs/timezone.js");
const realTypes = require(ROOT + "/dist/cjs/types.js");
const realCasualRelative = require(ROOT + "/dist/cjs/locales/en/parsers/ENTimeUnitCasualRelativeFormatParser.js").default;
const realTimeExp = require(ROOT + "/dist/cjs/locales/en/parsers/ENTimeExpressionParser.js").default;
const rec = globalThis.__recorder;

const meta = new WeakMap();
export function metaOf(v) {
  return (v !== null && (typeof v === "object" || typeof v === "function")) ? meta.get(v) : undefined;
}

function track(target, parseId, path) {
  if (target === null || target === undefined) return target;
  if (typeof target !== "object") return target;
  const proxy = new Proxy(target, {
    get(t, prop, receiver) {
      if (typeof prop === "symbol") {
        const v = Reflect.get(t, prop, t);
        return typeof v === "function" ? v.bind(t) : v;
      }
      const v = Reflect.get(t, prop, t);
      const segPath = [...path, prop];
      if (typeof v === "function") {
        const fn = (...args) => {
          if (prop === "imply" || prop === "assign") {
            rec.event({ type: "mutate", parse: parseId, path, method: prop, args });
          }
          const out = v.apply(t, args.map(a => metaOf(a) ? metaOf(a).target : a));
          const callPath = [...path, { call: prop, args }];
          rec.access(parseId, callPath, out);
          if (out === t) return proxy;
          return track(out, parseId, callPath);
        };
        return fn;
      }
      rec.access(parseId, segPath, v);
      return track(v, parseId, segPath);
    },
  });
  meta.set(proxy, { target, parseId, path });
  return proxy;
}

function serializeRef(ref) {
  if (ref === undefined || ref === null) return { kind: "now" };
  if (ref instanceof Date) return { kind: "date", instant: ref.getTime() };
  const out = { kind: "ref", instant: ref.instant ? ref.instant.getTime() : null };
  if ("timezone" in ref) out.timezone = ref.timezone === undefined ? null : ref.timezone;
  return out;
}

function serializeTimezones(tz) {
  const out = {};
  for (const [k, v] of Object.entries(tz)) {
    if (typeof v === "number") out[k] = { fixed: v };
    else out[k] = { dst: v.timezoneOffsetDuringDst, nonDst: v.timezoneOffsetNonDst, start: findRule(v.dstStart), end: findRule(v.dstEnd) };
  }
  return out;
}

function findRule(fn) {
  for (let month = 1; month <= 12; month++)
    for (let wd = 0; wd < 7; wd++)
      for (let hour = 0; hour < 24; hour++) {
        let ok = true;
        for (let y = 2000; y <= 2030 && ok; y++) ok = fn(y).getTime() === realTimezone.getLastWeekdayOfMonth(y, month, wd, hour).getTime();
        if (ok) return { last: true, month, weekday: wd, hour };
        for (let n = 1; n <= 4; n++) {
          let ok2 = true;
          for (let y = 2000; y <= 2030 && ok2; y++) ok2 = fn(y).getTime() === realTimezone.getNthWeekdayOfMonth(y, month, wd, n, hour).getTime();
          if (ok2) return { last: false, month, weekday: wd, n, hour };
        }
      }
  rec.unportable("unrecognised custom DST rule");
  return null;
}

function serializeOption(opt) {
  const out = {};
  if (!opt) return out;
  for (const [k, v] of Object.entries(opt)) {
    if (k === "debug") continue;
    if (k === "forwardDate") out.forwardDate = !!v;
    else if (k === "timezones") out.timezones = serializeTimezones(v);
    else rec.unportable("unknown option " + k);
  }
  return out;
}

function wrapChrono(realChrono, config) {
  const wrapper = {
    __config: config,
    __real: realChrono,
    get parsers() { return realChrono.parsers; },
    set parsers(v) { realChrono.parsers = v; },
    get refiners() { return realChrono.refiners; },
    parse(text, ref, opt) {
      const cfg = resolveConfig(wrapper);
      const id = rec.event({ type: "parse", config: cfg, text, ref: serializeRef(ref), option: serializeOption(opt) });
      const results = realChrono.parse(text, ref, opt);
      return track(results, id, []);
    },
    parseDate(text, ref, opt) {
      const results = wrapper.parse(text, ref, opt);
      return results.length > 0 ? results[0].start.date() : null;
    },
    clone() { return wrapChrono(realChrono.clone(), config); },
  };
  wrapper.__baseParsers = realChrono.parsers.length;
  return wrapper;
}

function resolveConfig(w) {
  const parsers = w.__real.parsers;
  if (parsers.length === w.__baseParsers) return w.__config;
  if (w.__config === "casual" && parsers.length === w.__baseParsers - 1 && !parsers.some(p => p instanceof realTimeExp)) {
    return "casual-timeExpression";
  }
  const extra = parsers.slice(w.__baseParsers);
  if (w.__config === "strict" && extra.length === 1 && extra[0] instanceof realCasualRelative && extra[0].allowAbbreviations === false) {
    return "strict+casualRelativeNoAbbr";
  }
  rec.unportable("custom parser configuration");
  return w.__config + "+custom";
}

const casualW = wrapChrono(real.casual, "casual");
const strictW = wrapChrono(real.strict, "strict");
const gbW = wrapChrono(real.en.GB, "gb");
const frW = wrapChrono(real.fr.casual, "fr");

export const casual = casualW;
export const strict = strictW;
export const parse = (...a) => casualW.parse(...a);
export const parseDate = (...a) => casualW.parseDate(...a);
export const Meridiem = real.Meridiem;
export const Weekday = real.Weekday;
export const ParsingResult = real.ParsingResult;
export const fr = Object.assign({}, frW, { casual: frW, parse: (...a) => frW.parse(...a) });
export const configuration = {
  createCasualConfiguration(littleEndian = false) {
    return { __marker: littleEndian ? "gb" : "casual", real: real.en.configuration.createCasualConfiguration(littleEndian) };
  },
  createConfiguration(strictMode = true, littleEndian = false) {
    if (strictMode && !littleEndian) return { __marker: "strict", real: real.en.configuration.createConfiguration(true, false) };
    rec.unportable("custom configuration");
    return { __marker: "custom", real: real.en.configuration.createConfiguration(strictMode, littleEndian) };
  },
};
export function Chrono(cfg) {
  if (cfg && cfg.__marker) return wrapChrono(new real.Chrono(cfg.real), cfg.__marker);
  rec.unportable("custom Chrono");
  return wrapChrono(new real.Chrono(cfg), "custom");
}
export const en = { casual: casualW, strict: strictW, GB: gbW, configuration, parse: (...a) => casualW.parse(...a) };
export const getLastWeekdayOfMonth = realTimezone.getLastWeekdayOfMonth;
export const Month = realTypes.Month;
export const ENTimeUnitCasualRelativeFormatParser = realCasualRelative;
export const ENTimeExpressionParser = realTimeExp;
export default casualW;
