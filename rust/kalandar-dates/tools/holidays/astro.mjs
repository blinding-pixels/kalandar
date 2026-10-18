const RAD = Math.PI / 180;
const norm = (x) => ((x % 360) + 360) % 360;

export function julianDay(ms) {
  return ms / 86400000 + 2440587.5;
}

export function deltaTSeconds(year) {
  if (year < 1986) {
    const t = year - 1975;
    return 45.45 + 1.067 * t - t * t / 260 - t * t * t / 718;
  }
  if (year < 2005) {
    const t = year - 2000;
    return 63.86 + 0.3345 * t - 0.060374 * t * t + 0.0017275 * t ** 3 + 0.000651814 * t ** 4 + 0.00002373599 * t ** 5;
  }
  if (year < 2050) {
    const t = year - 2000;
    return 62.92 + 0.32217 * t + 0.005589 * t * t;
  }
  const u = (year - 1820) / 100;
  return -20 + 32 * u * u - 0.5628 * (2150 - year);
}

function centuries(ms) {
  const year = 1970 + ms / (365.2425 * 86400000);
  const jde = julianDay(ms) + deltaTSeconds(year) / 86400;
  return (jde - 2451545) / 36525;
}

function nutation(t) {
  const omega = 125.04452 - 1934.136261 * t;
  const l = 280.4665 + 36000.7698 * t;
  const lp = 218.3165 + 481267.8813 * t;
  return (-17.2 * Math.sin(omega * RAD) - 1.32 * Math.sin(2 * l * RAD) - 0.23 * Math.sin(2 * lp * RAD) + 0.21 * Math.sin(2 * omega * RAD)) / 3600;
}

export function sunLongitude(ms) {
  const t = centuries(ms);
  const l0 = 280.46646 + 36000.76983 * t + 0.0003032 * t * t;
  const m = 357.52911 + 35999.05029 * t - 0.0001537 * t * t;
  const c =
    (1.914602 - 0.004817 * t - 0.000014 * t * t) * Math.sin(m * RAD) +
    (0.019993 - 0.000101 * t) * Math.sin(2 * m * RAD) +
    0.000289 * Math.sin(3 * m * RAD);
  return norm(l0 + c - 0.00569 + nutation(t));
}

const MOON_TERMS = [
  [0, 0, 1, 0, 6288774], [2, 0, -1, 0, 1274027], [2, 0, 0, 0, 658314], [0, 0, 2, 0, 213618],
  [0, 1, 0, 0, -185116], [0, 0, 0, 2, -114332], [2, 0, -2, 0, 58793], [2, -1, -1, 0, 57066],
  [2, 0, 1, 0, 53322], [2, -1, 0, 0, 45758], [0, 1, -1, 0, -40923], [1, 0, 0, 0, -34720],
  [0, 1, 1, 0, -30383], [2, 0, 0, -2, 15327], [0, 0, 1, 2, -12528], [0, 0, 1, -2, 10980],
  [4, 0, -1, 0, 10675], [0, 0, 3, 0, 10034], [4, 0, -2, 0, 8548], [2, 1, -1, 0, -7888],
  [2, 1, 0, 0, -6766], [1, 0, -1, 0, -5163], [1, 1, 0, 0, 4987], [2, -1, 1, 0, 4036],
  [2, 0, 2, 0, 3994], [4, 0, 0, 0, 3861], [2, 0, -3, 0, 3665], [0, 1, -2, 0, -2689],
  [2, 0, -1, 2, -2602], [2, -1, -2, 0, 2390], [1, 0, 1, 0, -2348], [2, -2, 0, 0, 2236],
  [0, 1, 2, 0, -2120], [0, 2, 0, 0, -2069], [2, -2, -1, 0, 2048], [2, 0, 1, -2, -1773],
  [2, 0, 0, 2, -1595], [4, -1, -1, 0, 1215], [0, 0, 2, 2, -1110], [3, 0, -1, 0, -892],
  [2, 1, 1, 0, -810], [4, -1, -2, 0, 759], [0, 2, -1, 0, -713], [2, 2, -1, 0, -700],
  [2, 1, -2, 0, 691], [2, -1, 0, -2, 596], [4, 0, 1, 0, 549], [0, 0, 4, 0, 537],
  [4, -1, 0, 0, 520], [1, 0, -2, 0, -487], [2, 1, 0, -2, -399], [0, 0, 2, -2, -381],
  [1, 1, 1, 0, 351], [3, 0, -2, 0, -340], [4, 0, -3, 0, 330], [2, -1, 2, 0, 327],
  [0, 2, 1, 0, -323], [1, 1, -1, 0, 299], [2, 0, 3, 0, 294],
];

export function moonLongitude(ms) {
  const t = centuries(ms);
  const lp = 218.3164477 + 481267.88123421 * t - 0.0015786 * t * t + t ** 3 / 538841 - t ** 4 / 65194000;
  const d = 297.8501921 + 445267.1114034 * t - 0.0018819 * t * t + t ** 3 / 545868 - t ** 4 / 113065000;
  const m = 357.5291092 + 35999.0502909 * t - 0.0001536 * t * t + t ** 3 / 24490000;
  const mp = 134.9633964 + 477198.8675055 * t + 0.0087414 * t * t + t ** 3 / 69699 - t ** 4 / 14712000;
  const f = 93.272095 + 483202.0175233 * t - 0.0036539 * t * t - t ** 3 / 3526000 + t ** 4 / 863310000;
  const e = 1 - 0.002516 * t - 0.0000074 * t * t;
  let sum = 0;
  for (const [cd, cm, cmp, cf, coeff] of MOON_TERMS) {
    const arg = (cd * d + cm * m + cmp * mp + cf * f) * RAD;
    const factor = Math.abs(cm) === 1 ? e : Math.abs(cm) === 2 ? e * e : 1;
    sum += coeff * factor * Math.sin(arg);
  }
  const a1 = 119.75 + 131.849 * t;
  const a2 = 53.09 + 479264.29 * t;
  sum += 3958 * Math.sin(a1 * RAD) + 1962 * Math.sin((lp - f) * RAD) + 318 * Math.sin(a2 * RAD);
  return norm(lp + sum / 1e6 + nutation(t));
}

export function lahiriAyanamsa(ms) {
  const years = (julianDay(ms) - 2451545) / 365.25;
  return 23.85303 + (50.2788 * years + 0.000111 * years * years) / 3600;
}

export const siderealSun = (ms) => norm(sunLongitude(ms) - lahiriAyanamsa(ms));
export const siderealMoon = (ms) => norm(moonLongitude(ms) - lahiriAyanamsa(ms));
export const elongation = (ms) => norm(moonLongitude(ms) - sunLongitude(ms));

function sunEquatorial(ms) {
  const t = centuries(ms);
  const lambda = sunLongitude(ms) * RAD;
  const eps = (23.439291 - 0.0130042 * t + 0.00256 * Math.cos((125.04 - 1934.136 * t) * RAD)) * RAD;
  const ra = Math.atan2(Math.cos(eps) * Math.sin(lambda), Math.cos(lambda));
  const dec = Math.asin(Math.sin(eps) * Math.sin(lambda));
  return { ra, dec };
}

function gmstDegrees(ms) {
  const d = julianDay(ms) - 2451545;
  return norm(280.46061837 + 360.98564736629 * d);
}

function hourAngleFor(ms, lat, lon) {
  const { ra, dec } = sunEquatorial(ms);
  const lst = (gmstDegrees(ms) + lon) * RAD;
  let h = lst - ra;
  h = Math.atan2(Math.sin(h), Math.cos(h));
  const alt = Math.asin(Math.sin(lat * RAD) * Math.sin(dec) + Math.cos(lat * RAD) * Math.cos(dec) * Math.cos(h));
  return { h, dec, alt };
}

function solarEvent(dayStartUtcMs, place, rising) {
  const h0 = -0.8333 * RAD;
  let guess = dayStartUtcMs + (12 - place.lon / 15) * 3600000 + (rising ? -6 : 6) * 3600000 - place.offsetMinutes * 0;
  for (let i = 0; i < 6; i++) {
    const { h, dec } = hourAngleFor(guess, place.lat, place.lon);
    const cosH0 = (Math.sin(h0) - Math.sin(place.lat * RAD) * Math.sin(dec)) / (Math.cos(place.lat * RAD) * Math.cos(dec));
    const target = (rising ? -1 : 1) * Math.acos(Math.max(-1, Math.min(1, cosH0)));
    let diff = target - h;
    diff = Math.atan2(Math.sin(diff), Math.cos(diff));
    guess += (diff / (2 * Math.PI)) * 86164091;
  }
  return guess;
}

export function civilDayStartUtc(year, month, day, place) {
  return Date.UTC(year, month - 1, day) - place.offsetMinutes * 60000;
}

export function sunrise(year, month, day, place) {
  const start = Date.UTC(year, month - 1, day);
  return solarEvent(start, place, true);
}

export function sunset(year, month, day, place) {
  const start = Date.UTC(year, month - 1, day);
  return solarEvent(start, place, false);
}

export function findCrossing(fn, target, lo, hi) {
  const delta = (ms) => {
    const v = norm(fn(ms) - target);
    return v > 180 ? v - 360 : v;
  };
  let a = lo;
  let b = hi;
  for (let i = 0; i < 60; i++) {
    const mid = (a + b) / 2;
    if (delta(mid) < 0) a = mid;
    else b = mid;
  }
  return (a + b) / 2;
}
