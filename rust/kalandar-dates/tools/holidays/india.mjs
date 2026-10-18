import { elongation, siderealSun, siderealMoon, sunrise, sunset, findCrossing } from "./astro.mjs";

const HOUR = 3600000;
const DAY = 86400000;
const IST = 330 * 60000;
export const PLACES = {
  delhi: { lat: 28.6139, lon: 77.209, offsetMinutes: 330 },
  chennai: { lat: 13.0827, lon: 80.2707, offsetMinutes: 330 },
  thiruvananthapuram: { lat: 8.5241, lon: 76.9366, offsetMinutes: 330 },
};

const MONTHS = [
  "chaitra", "vaishakha", "jyeshtha", "ashadha", "shravana", "bhadrapada",
  "ashvin", "kartika", "margashirsha", "pausha", "magha", "phalguna",
];

const civil = (ms) => {
  const d = new Date(ms + IST);
  return [d.getUTCFullYear(), d.getUTCMonth() + 1, d.getUTCDate()];
};
const civilKey = ([y, m, d]) => `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
const addDays = ([y, m, d], n) => civil(Date.UTC(y, m - 1, d) - IST + n * DAY + 12 * HOUR);

function newMoonsBetween(startMs, endMs) {
  const out = [];
  let t = startMs;
  while (t < endMs) {
    const step = t + DAY;
    const a = elongation(t);
    const b = elongation(step);
    if (b < a) out.push(findCrossing(elongation, 0, t, step));
    t = step;
  }
  return out;
}

export function lunarMonths(year) {
  const moons = newMoonsBetween(Date.UTC(year - 1, 10, 1), Date.UTC(year + 1, 2, 1));
  const months = [];
  for (let i = 0; i + 1 < moons.length; i++) {
    const sign = Math.floor(siderealSun(moons[i]) / 30);
    const nextSign = Math.floor(siderealSun(moons[i + 1]) / 30);
    months.push({
      name: MONTHS[(sign + 1) % 12],
      adhika: sign === nextSign,
      start: moons[i],
      end: moons[i + 1],
    });
    if ((nextSign - sign + 12) % 12 === 2) {
      months.push({ name: MONTHS[(sign + 2) % 12], adhika: false, kshaya: true, start: moons[i], end: moons[i + 1] });
    }
  }
  return months;
}

function tithiInterval(month, tithi) {
  const lo = month.start + (tithi - 1) * 0.9 * DAY - 2 * DAY;
  const begin = tithi === 1 ? month.start : findCrossing(elongation, (tithi - 1) * 12, Math.max(month.start, lo), Math.min(month.end, lo + 6 * DAY));
  const finish = tithi === 30 ? month.end : findCrossing(elongation, tithi * 12, Math.max(month.start, lo + DAY), Math.min(month.end, lo + 7 * DAY));
  return [begin, finish];
}

function overlap(a0, a1, b0, b1) {
  return Math.max(0, Math.min(a1, b1) - Math.max(a0, b0));
}

function candidateDays([begin, finish]) {
  const first = addDays(civil(begin), -1);
  return [0, 1, 2, 3].map((n) => addDays(first, n));
}

const WINDOWS = {
  sunrise: (day, place) => {
    const r = sunrise(...day, place);
    return [r, r + 1];
  },
  madhyahna: (day, place) => {
    const r = sunrise(...day, place);
    const s = sunset(...day, place);
    const part = (s - r) / 5;
    return [r + 2 * part, r + 3 * part];
  },
  aparahna: (day, place) => {
    const r = sunrise(...day, place);
    const s = sunset(...day, place);
    const part = (s - r) / 5;
    return [r + 3 * part, r + 4 * part];
  },
  pradosh: (day, place) => {
    const s = sunset(...day, place);
    return [s, s + 2.4 * HOUR];
  },
  nishita: (day, place) => {
    const s = sunset(...day, place);
    const r = sunrise(...addDays(day, 1), place);
    const mid = (s + r) / 2;
    return [mid - 0.4 * HOUR, mid + 0.4 * HOUR];
  },
  moonrise: (day, place) => {
    const s = sunset(...day, place);
    return [s + HOUR, s + 4 * HOUR];
  },
};

function windowCover(interval, rule, day, place) {
  const [w0, w1] = WINDOWS[rule](day, place);
  return { cover: overlap(w0, w1, interval[0], interval[1]), length: w1 - w0, window: [w0, w1] };
}

function udaya(interval, place, minimumAfterSunrise = 0) {
  for (const d of candidateDays(interval)) {
    const r = sunrise(...d, place);
    if (r >= interval[0] && r < interval[1]) {
      if (interval[1] - r >= minimumAfterSunrise) return d;
      return addDays(d, -1);
    }
  }
  return civil(interval[0]);
}

function pickDay(interval, rule, place, preferLater = false) {
  const touched = candidateDays(interval)
    .map((d) => ({ d, ...windowCover(interval, rule, d, place) }))
    .filter((c) => c.cover > 0.1 * c.length);
  if (touched.length === 0) return udaya(interval, place);
  if (preferLater) return touched[touched.length - 1].d;
  return touched.reduce((a, b) => (b.cover > a.cover + 1000 ? b : a)).d;
}

const inShravana = (ms) => {
  const l = siderealMoon(ms);
  return l >= 280 && l < 280 + 40 / 3;
};

function vijayadashami(interval, place) {
  const days = candidateDays(interval).map((d) => ({ d, ...windowCover(interval, "aparahna", d, place) }));
  const touched = days.filter((c) => c.cover > 0.1 * c.length);
  const star = (c) => inShravana((c.window[0] + c.window[1]) / 2);
  if (touched.length >= 2) return (touched.find(star) ?? touched[0]).d;
  if (touched.length === 0) return udaya(interval, place);
  const only = touched[0];
  if (only.cover >= 0.85 * only.length) return only.d;
  const next = addDays(only.d, 1);
  const other = days.find((c) => c.d.join() === next.join());
  if (other && star(other)) {
    const r = sunrise(...other.d, place);
    if (r >= interval[0] && r < interval[1]) return other.d;
  }
  return only.d;
}

function holikaDahan(interval, place) {
  const bhadraEnd = (interval[0] + interval[1]) / 2;
  const days = candidateDays(interval).map((d) => ({ d, ...windowCover(interval, "pradosh", d, place) }));
  const first = days.find((c) => c.cover > 0);
  if (!first) return udaya(interval, place);
  const free = overlap(first.window[0], first.window[1], Math.max(interval[0], bhadraEnd), interval[1]);
  if (free > 0) return first.d;
  const next = addDays(first.d, 1);
  const r = sunrise(...next, place);
  const s = sunset(...next, place);
  const lasting = r >= interval[0] && r < interval[1] ? interval[1] - r : 0;
  return lasting >= 0.75 * (s - r) ? next : first.d;
}

function lunarFestival(year, monthName, tithi, choose) {
  const results = lunarMonths(year)
    .filter((m) => m.name === monthName && !m.adhika)
    .map((m) => choose(tithiInterval(m, tithi)))
    .filter(([y]) => y === year);
  return results[0] ?? null;
}

function makarCrossing(year) {
  const approx = Date.UTC(year, 0, 14);
  return findCrossing(siderealSun, 270, approx - 10 * DAY, approx + 10 * DAY);
}

function makarSankranti(year, place) {
  const t = makarCrossing(year);
  const day = civil(t);
  return t < sunrise(...day, place) ? addDays(day, -1) : day;
}

function thaiPongal(year, place) {
  const t = makarCrossing(year);
  const day = civil(t);
  const r = sunrise(...day, place);
  const s = sunset(...day, place);
  return t < r + (4 * (s - r)) / 5 ? day : addDays(day, 1);
}

const utc = ([y, m, d]) => Date.UTC(y, m - 1, d);

function onam(year) {
  const place = PLACES.thiruvananthapuram;
  const sankranti = findCrossing(siderealSun, 120, Date.UTC(year, 7, 5), Date.UTC(year, 7, 30));
  const chingamEnd = findCrossing(siderealSun, 150, Date.UTC(year, 8, 5), Date.UTC(year, 8, 30));
  let first = civil(sankranti);
  const r0 = sunrise(...first, place);
  const s0 = sunset(...first, place);
  if (sankranti > r0 + (3 * (s0 - r0)) / 5) first = addDays(first, 1);
  const earliest = addDays(first, 4);
  let day = earliest;
  for (let i = 0; i < 32; i++) {
    const r = sunrise(...day, place);
    if (r > chingamEnd) break;
    if (inShravana(r)) {
      if (inShravana(r + 2.4 * HOUR)) return day;
      const previous = addDays(day, -1);
      if (inShravana(r - HOUR) && utc(previous) >= utc(earliest)) return previous;
    } else if (inShravana(r + 2.4 * HOUR) && !inShravana(sunrise(...addDays(day, 1), place))) {
      return day;
    }
    day = addDays(day, 1);
  }
  let best = null;
  let bestHours = 0;
  for (let d = earliest; utc(d) < chingamEnd; d = addDays(d, 1)) {
    const r = sunrise(...d, place);
    const s = sunset(...d, place);
    let hours = 0;
    for (let t = r; t < s; t += HOUR / 4) if (inShravana(t)) hours += 0.25;
    if (hours > bestHours) {
      best = d;
      bestHours = hours;
    }
  }
  return best;
}

export const FESTIVALS = [
  { id: "diwali", names: { en: "Diwali", hi: "दीपावली" }, kind: "public", compute: (y) => lunarFestival(y, "ashvin", 30, (i) => pickDay(i, "pradosh", PLACES.delhi)) },
  { id: "holi", names: { en: "Holi", hi: "होली" }, kind: "optional", compute: (y) => lunarFestival(y, "phalguna", 15, (i) => addDays(holikaDahan(i, PLACES.delhi), 1)) },
  { id: "dussehra", names: { en: "Dussehra", hi: "दशहरा" }, kind: "public", compute: (y) => lunarFestival(y, "ashvin", 10, (i) => vijayadashami(i, PLACES.delhi)) },
  { id: "janmashtami", names: { en: "Janmashtami", hi: "जन्माष्टमी" }, kind: "optional", compute: (y) => lunarFestival(y, "shravana", 23, (i) => udaya(i, PLACES.delhi)) },
  { id: "ganesh_chaturthi", names: { en: "Ganesh Chaturthi", hi: "गणेश चतुर्थी" }, kind: "optional", compute: (y) => lunarFestival(y, "bhadrapada", 4, (i) => pickDay(i, "madhyahna", PLACES.delhi)) },
  { id: "maha_shivaratri", names: { en: "Maha Shivaratri", hi: "महाशिवरात्रि" }, kind: "optional", compute: (y) => lunarFestival(y, "magha", 29, (i) => pickDay(i, "nishita", PLACES.delhi, true)) },
  { id: "raksha_bandhan", names: { en: "Raksha Bandhan", hi: "रक्षा बन्धन" }, kind: "optional", compute: (y) => lunarFestival(y, "shravana", 15, (i) => udaya(i, PLACES.delhi, 2.4 * HOUR)) },
  { id: "ram_navami", names: { en: "Ram Navami", hi: "राम नवमी" }, kind: "optional", compute: (y) => lunarFestival(y, "chaitra", 9, (i) => pickDay(i, "madhyahna", PLACES.delhi, true)) },
  { id: "navratri", names: { en: "Navratri", hi: "नवरात्रि" }, kind: "observance", compute: (y) => lunarFestival(y, "ashvin", 1, (i) => udaya(i, PLACES.delhi)) },
  { id: "chaitra_navratri", names: { en: "Chaitra Navratri", hi: "चैत्र नवरात्रि" }, kind: "observance", compute: (y) => lunarFestival(y, "chaitra", 1, (i) => udaya(i, PLACES.delhi)) },
  { id: "karva_chauth", names: { en: "Karva Chauth", hi: "करवा चौथ" }, kind: "optional", compute: (y) => lunarFestival(y, "ashvin", 19, (i) => pickDay(i, "moonrise", PLACES.delhi)) },
  { id: "makar_sankranti", names: { en: "Makar Sankranti", hi: "मकर संक्रान्ति" }, kind: "optional", compute: (y) => makarSankranti(y, PLACES.delhi) },
  { id: "pongal", names: { en: "Pongal", ta: "பொங்கல்" }, kind: "optional", compute: (y) => thaiPongal(y, PLACES.chennai) },
  { id: "onam", names: { en: "Onam", ml: "ഓണം" }, kind: "optional", compute: onam },
];


export function indianFestivals(year) {
  return FESTIVALS.map((f) => {
    const day = f.compute(year);
    return day ? { id: f.id, date: civilKey(day), kind: f.kind, names: f.names } : null;
  }).filter(Boolean);
}

export const debugInterval = (year, monthName, tithi) =>
  lunarMonths(year).filter((m) => m.name === monthName && !m.adhika).map((m) => tithiInterval(m, tithi).map((t) => new Date(t + IST).toISOString().slice(0, 16)));
