import { normalizeShadowText } from '../core/shadow-text.ts';
import type { IntentOptions, IntentScores, IntentType } from './types.ts';

export const DEFAULT_INTENT_THRESHOLD = 0.55;
export const GAP_THRESHOLD = 0.15;
export const ROUTE_TO_NONE_THRESHOLD = 0.45;

const FILLER_WORDS = new Set(['lol', 'lmao', 'pls', 'plz', 'yo', 'hey', 'uhhh', 'uhh', 'ummm', 'umm', 'um']);

const WEEKDAY_PATTERN =
  '(?:mon(?:day)?|tue(?:s|sday)?|wed(?:nesday)?|thu(?:r|rs|rsday|ursday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)';
const MONTH_PATTERN =
  '(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)';
const RELATIVE_DATE_PATTERN =
  '(?:today|tonight|tomorrow|tmr|tmrw|next\\s+(?:week|month|' +
  WEEKDAY_PATTERN +
  ')|this\\s+(?:week|month|' +
  WEEKDAY_PATTERN +
  ')|in\\s+a\\s+fortnight|fortnight|end\\s+of\\s+(?:day|week)|eod)';
const TIME_PATTERN =
  '(?:[01]?\\d(?::[0-5]\\d)?\\s?(?:a|p|am|pm)|[01]?\\d(?:[ap])|\\d{1,2}|noon|midnight|half\\s+[01]?\\d|\\d{3,4})';

const TIME_REGEX = new RegExp(`\\b${TIME_PATTERN}\\b`, 'i');
const DATE_REGEX = new RegExp(
  `\\b(?:${RELATIVE_DATE_PATTERN}|${WEEKDAY_PATTERN}|${MONTH_PATTERN}\\s+\\d{1,2}(?:-\\d{1,2})?|\\d{1,2}[/-]\\d{1,2}|the\\s+\\d{1,2}(?:st|nd|rd|th)?)\\b`,
  'i',
);
const RECURRENCE_REGEX = new RegExp(
  `\\b(?:every\\s+(?:other\\s+)?(?:weekday|week|day|month|${WEEKDAY_PATTERN})|weekly|daily|monthly|yearly|every\\s+week\\s+on\\s+${WEEKDAY_PATTERN})\\b`,
  'i',
);

const stripEmoji = (input: string): string =>
  input.replace(/(?:\p{Extended_Pictographic}|\p{Emoji_Presentation}|\uFE0F)/gu, ' ');

export const TASK_LEAD_WORDS = new Set([
  'finish',
  'fix',
  'build',
  'write',
  'send',
  'update',
  'deploy',
  'review',
  'submit',
  'complete',
  'refactor',
  'cancel',
  'order',
  'buy',
  'get',
  'prepare',
  'draft',
  'ship',
  'call',
  'check',
  'book',
  'renew',
  'follow',
  'prep',
  'organize',
  'schedule',
  'grab',
  'pick',
  'clean',
  'set',
  'sign',
  'water',
  'email',
  'plan',
  'brainstorm',
]);

export const DATE_TASK_LEAD_WORDS = new Set([
  'finish',
  'fix',
  'build',
  'write',
  'send',
  'update',
  'deploy',
  'submit',
  'complete',
  'refactor',
  'cancel',
  'order',
  'buy',
  'get',
  'prepare',
  'draft',
  'ship',
  'check',
  'renew',
  'follow',
  'organize',
  'grab',
  'pick',
  'clean',
  'set',
  'sign',
  'water',
  'email',
  'brainstorm',
]);

export const APPOINTMENT_WORDS = new Set(['dentist', 'doctor', 'appointment', 'appt', 'haircut', 'gym']);

const STRONG_CALENDAR_WORDS = new Set([
  'meeting',
  'session',
  'standup',
  'sync',
  '1on1',
  'interview',
  'lunch',
  'dinner',
  'brunch',
  'coffee',
  'drinks',
  'breakfast',
  'appointment',
  'appt',
  'flight',
  'rehearsal',
  'class',
  'yoga',
  'reservation',
  'workshop',
  'party',
  'birthday',
  'offsite',
  'town',
  'hall',
  'concert',
  'doctor',
  'dentist',
  'haircut',
  'gym',
  'trip',
]);

export const normalizeIntentInput = (input: string): string => {
  const shadowNormalized = normalizeShadowText(input);
  const collapsed = shadowNormalized
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase()
    .replace(/[’']/g, "'");
  const withoutEmoji = stripEmoji(collapsed);
  const withoutNoise = withoutEmoji
    .replace(/[!?.,]{2,}/g, ' ')
    .replace(/[()[\]{}"]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  const filteredTokens = withoutNoise
    .split(' ')
    .filter(Boolean)
    .filter((token) => !FILLER_WORDS.has(token.replace(/[^a-z]/g, '')));

  return filteredTokens.join(' ');
};

export const hasTimeSignal = (value: string): boolean => TIME_REGEX.test(value);

export const hasDateSignal = (value: string): boolean => DATE_REGEX.test(value);

export const hasDurationSignal = (value: string): boolean =>
  /\b\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?)\b/i.test(value);

export const hasRecurrenceSignal = (value: string): boolean => RECURRENCE_REGEX.test(value);

export const hasStrongCalendarFrame = (tokens: string[]): boolean =>
  tokens.some((token) => STRONG_CALENDAR_WORDS.has(token));

export const rankIntentScores = (scores: IntentScores): Array<[IntentType, number]> =>
  (Object.entries(scores) as Array<[IntentType, number]>).sort((left, right) => right[1] - left[1]);

export const resolveIntentThreshold = (options?: IntentOptions): number => {
  const candidate = options?.threshold;
  if (typeof candidate !== 'number' || Number.isNaN(candidate)) {
    return DEFAULT_INTENT_THRESHOLD;
  }
  return Math.max(0, Math.min(1, candidate));
};
