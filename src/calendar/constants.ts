export const DEFAULT_LOCALE = 'en-US';
export const DEFAULT_TIMEZONE = 'UTC';

export const WEEKDAY_ALIASES: Array<{ token: RegExp; day: string }> = [
  { token: /\bmon(?:day)?\b/gi, day: 'monday' },
  { token: /\btue(?:s|sday)?\b/gi, day: 'tuesday' },
  { token: /\bwed(?:nesday)?\b/gi, day: 'wednesday' },
  { token: /\bthu(?:r|rs|rsday|ursday)?\b/gi, day: 'thursday' },
  { token: /\bfri(?:day)?\b/gi, day: 'friday' },
  { token: /\bsat(?:urday)?\b/gi, day: 'saturday' },
  { token: /\bsun(?:day)?\b/gi, day: 'sunday' },
];

export const WEEKDAY_LIST_PATTERN =
  '(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';

export const NUMBER_WORDS: Record<string, number> = {
  a: 1,
  an: 1,
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
  nine: 9,
  ten: 10,
  eleven: 11,
  twelve: 12,
};

export const TITLE_GLUE_WORDS = new Set([
  'on',
  'at',
  'with',
  'every',
  'reminder',
  'from',
  'to',
  'for',
  'starting',
  'invite',
  'w/',
  'the',
  'and',
]);
