export const DEFAULT_LOCALE = 'en-US';
export const DEFAULT_TIMEZONE = 'UTC';

export const WEEKDAY_TOKEN =
  '(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';

export const WEEKDAY_INDEX_BY_TOKEN: Record<string, number> = {
  sunday: 0,
  sun: 0,
  monday: 1,
  mon: 1,
  tuesday: 2,
  tue: 2,
  tues: 2,
  wednesday: 3,
  wed: 3,
  thursday: 4,
  thu: 4,
  thur: 4,
  thurs: 4,
  friday: 5,
  fri: 5,
  saturday: 6,
  sat: 6,
};

export const WEEKDAY_NAME_BY_INDEX = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

export const SHARED_ACRONYM_MAP: Record<string, string> = {
  api: 'API',
  eod: 'EOD',
  fcp: 'FCP',
  hr: 'HR',
  os: 'OS',
  pr: 'PR',
  ssl: 'SSL',
  svg: 'SVG',
  usb: 'USB',
  aws: 'AWS',
  readme: 'README',
};

export const TITLE_SMALL_WORDS = new Set([
  'a',
  'an',
  'the',
  'and',
  'but',
  'or',
  'nor',
  'for',
  'to',
  'in',
  'on',
  'at',
  'by',
  'of',
  'with',
  'from',
  'up',
  'as',
  'is',
]);
