import type { TaskPriority } from './types.ts';
import { TITLE_SMALL_WORDS } from '../shared/constants.ts';

export const PRIORITY_ORDER: Record<Exclude<TaskPriority, null>, number> = {
  high: 3,
  medium: 2,
  low: 1,
};

export const DEFAULT_KNOWN_ASSIGNEES = [
  'alex',
  'diego',
  'jake',
  'james',
  'jamie',
  'jen',
  'jester',
  'john',
  'lee',
  'mark',
  'mike',
  'maria',
  'nate',
  'priya',
  'rachel',
  'sam',
  'sarah',
];

export const TITLE_WORD_CORRECTIONS: Record<string, string> = {
  adn: 'and',
  borring: 'boring',
  brokn: 'broken',
  cancle: 'cancel',
  cert: 'certificate',
  chck: 'check',
  clnic: 'clinic',
  deplpy: 'deploy',
  desktp: 'desktop',
  docunents: 'documents',
  finishe: 'finish',
  financal: 'financial',
  flrs: 'flowers',
  hotfx: 'hotfix',
  importnt: 'important',
  instanc: 'instance',
  moble: 'mobile',
  monitering: 'monitoring',
  notifcation: 'notification',
  persentation: 'presentation',
  reciepts: 'receipts',
  replce: 'replace',
  reviw: 'review',
  shoud: 'should',
  summry: 'summary',
  teh: 'the',
  uptade: 'update',
  urgnt: 'urgent',
  wrte: 'write',
};

export const PHRASE_CORRECTIONS: Array<[RegExp, string]> = [
  [/\bre-do\b/gi, 'redo'],
  [/\bre do\b/gi, 'redo'],
  [/\bwe dont\b/gi, "we don't"],
  [/\blast years\b/gi, "last year's"],
  [/\bhub os\b/gi, 'hub OS'],
  [/\bprod login\b/gi, 'production login'],
  [/\bpitch deck draft\b/gi, 'pitch deck, draft'],
  [/\bdraft it send\b/gi, 'draft and send'],
  [/\bto clint\b/gi, 'to client'],
  [/\bbday\b/gi, 'bday'],
];

export const HIGH_PRIORITY_PATTERNS: RegExp[] = [
  /\burgent\b/gi,
  /\burgnt\b/gi,
  /\basap\b/gi,
  /\bcritical\b/gi,
  /\bcritcal\b/gi,
  /\bblocker\b/gi,
  /\bp1\b/gi,
  /\bimportant\b/gi,
  /\bimportnt\b/gi,
  /\bhigh\s+priority\b/gi,
  /\bhigh\s*priority\b/gi,
  /\bhighpri\w*\b/gi,
  /\bhigh\s+pri(?:o)?\b/gi,
  /!{3,}\s*urgent!*/gi,
  /🚨/g,
  /\bNOW\b/g,
];

export const MEDIUM_PRIORITY_PATTERNS: RegExp[] = [
  /\bnormal\s+priority\b/gi,
  /\bmedium\s+prio\b/gi,
  /\bmedium\b/gi,
  /\bnormal\b/gi,
];

export const LOW_PRIORITY_PATTERNS: RegExp[] = [
  /\blow\s+priority\b/gi,
  /\blow\s+pri(?:o)?\b/gi,
  /\bno\s+rush\b/gi,
  /\bnice\s+to\s+have\b/gi,
  /\bwhen\s+you\s+can\b/gi,
  /\bwhen\s+you\s+get\s+a\s+chance\b/gi,
  /\bwhenever(?:\s+you\s+can)?\b/gi,
  /\bthis\s+can\s+wait\b/gi,
  /\bnot\s+urgent\b/gi,
];

export const STATUS_PATTERNS: Array<{ regex: RegExp; value: Exclude<import('./types.ts').TaskStatus, null> }> = [
  { regex: /\bin\s+progress\b/gi, value: 'in progress' },
  { regex: /\bnot\s+started\b/gi, value: 'not started' },
  { regex: /\bto\s*do\b/gi, value: 'todo' },
  { regex: /\btodo\b/gi, value: 'todo' },
  { regex: /\bblocked\b/gi, value: 'blocked' },
  { regex: /\bwaiting\b/gi, value: 'waiting' },
  { regex: /\bdone\b/gi, value: 'done' },
  { regex: /\bcompleted?\b/gi, value: 'done' },
];

export const LEADING_FILLER_PATTERNS: RegExp[] = [
  /^\s*i think\b[\s,.-]*/i,
  /^\s*we should\b[\s,.-]*/i,
  /^\s*we shoud\b[\s,.-]*/i,
  /^\s*we need to\b[\s,.-]*/i,
  /^\s*we need\b[\s,.-]*/i,
  /^\s*i need to\b[\s,.-]*/i,
  /^\s*i need\b[\s,.-]*/i,
  /^\s*need to\b[\s,.-]*/i,
  /^\s*need\b[\s,.-]*/i,
  /^\s*pls\b[\s,.-]*/i,
  /^\s*please\b[\s,.-]*/i,
  /^\s*but\b[\s,.-]*/i,
  /^\s*can\b[\s,.-]*/i,
  /^\s*task\b[\s,.:-]*/i,
  /^\s*super\b[\s,.-]*/i,
  /^\s*maybe\b[\s,.-]*/i,
];

export const TRAILING_FILLER_PATTERNS: RegExp[] = [
  /[\s,.-]+\bpls\b\.?$/i,
  /[\s,.-]+\bplease\b\.?$/i,
  /[\s,.-]+\blol\b\.?$/i,
  /[\s,.-]+\bmaybe\b\.?$/i,
  /[\s,.-]+\bidk\b\.?$/i,
  /[\s,.-]+\bboth\b\.?$/i,
  /[\s,.-]+\bbut\b\.?$/i,
];

export const SMALL_WORDS = TITLE_SMALL_WORDS;
