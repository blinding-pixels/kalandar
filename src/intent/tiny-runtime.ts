import type { IntentOptions, IntentResult, IntentScores, IntentType } from './types.ts';
import { TINY_INTENT_MODEL } from './tiny-model.ts';
import {
  APPOINTMENT_WORDS,
  DATE_TASK_LEAD_WORDS,
  GAP_THRESHOLD,
  ROUTE_TO_NONE_THRESHOLD,
  TASK_LEAD_WORDS,
  hasDateSignal,
  hasDurationSignal,
  hasRecurrenceSignal,
  hasStrongCalendarFrame,
  hasTimeSignal,
  normalizeIntentInput,
  rankIntentScores,
  resolveIntentThreshold,
} from './utils.ts';

const EXPLICIT_REMINDER_PATTERNS = [
  /\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+me\b/,
  /\b(?:re?m(?:i|e)?n?d(?:me)?|rmind|remnd|remmind|remidne)\s+(?:abt|about|to|2)\b/,
  /\b(?:set|create|add|make)\s+(?:me\s+)?(?:an?\s+)?reminder\b/,
  /\breminder\s+(?:to|for|about)\b/,
  /\b(?:remember|remmber|rember|remeber|remembr)\b/,
  /\bneed\s+to\s+remember\b/,
  /\b(?:alert|notify|tell)\s+me\b/,
  /\b(?:dont|don't|do not)\s+forget\b/,
  /\bdont\s+4get\b/,
  /\bdon'?t\s+let\s+me\s+forget\b/,
  /\bnote\s+to\s+self\b/,
  /\bping\s+me\b/,
  /\bheads\s+up\b/,
  /\bnudge\s+me\b/,
  /\bsend\s+me\s+a\s+nudge\b/,
] as const;

const EXPLICIT_NONE_NOTE_PATTERNS = [
  /^(?:just\s+)?(?:an?\s+)?(?:random\s+)?thought\s+(?:about|on|re|regarding)\b/,
  /^(?:just\s+)?(?:an?\s+)?idea\s+(?:about|for|on|re|regarding)\b/,
  /^(?:just\s+)?(?:an?\s+)?question\s+(?:about|for|on|re|regarding)\b/,
  /^(?:just\s+)?(?:some\s+)?notes?\s+(?:about|for|on|re|regarding)\b/,
  /^(?:just\s+)?(?:an?\s+)?(?:observation|reflection|musing)\s+(?:about|on|re|regarding)\b/,
] as const;

const EXPLICIT_NONE_SOFT_ACTION_PATTERNS = [
  /^(?:should\s+i|could\s+we|what\s+if|how\s+about)\b/,
  /^(?:consider|think\s+about|explore)\b/,
  /\bat\s+some\s+point\b/,
  /^(?:ability|option|support|feature)\s+to\b/,
  /^(?:auto|automatic(?:ally)?)\s+[a-z]/,
] as const;

const fnv1a32 = (value: string): number => {
  let hashed = 0x811c9dc5;
  const bytes = new TextEncoder().encode(value);
  for (const byte of bytes) {
    hashed ^= byte;
    hashed = Math.imul(hashed, 0x01000193) >>> 0;
  }
  return hashed >>> 0;
};

const collectCharNgrams = (text: string, minimum = 3, maximum = 5): string[] => {
  const grams: string[] = [];
  for (const token of text.split(' ').filter(Boolean)) {
    const padded = ` ${token} `;
    for (let size = minimum; size <= maximum; size += 1) {
      if (padded.length < size) {
        continue;
      }
      for (let index = 0; index <= padded.length - size; index += 1) {
        grams.push(padded.slice(index, index + size));
      }
    }
  }
  return grams;
};

const featurize = (normalized: string): Map<number, number> => {
  const tokens = normalized ? normalized.split(' ') : [];
  const counts = new Map<number, number>();

  for (const token of tokens) {
    const index = fnv1a32(`w:${token}`) % TINY_INTENT_MODEL.wordBins;
    counts.set(index, (counts.get(index) ?? 0) + 1);
  }

  for (let index = 0; index < tokens.length - 1; index += 1) {
    const bigram = `${tokens[index]} ${tokens[index + 1]}`;
    const featureIndex = fnv1a32(`wb:${bigram}`) % TINY_INTENT_MODEL.wordBins;
    counts.set(featureIndex, (counts.get(featureIndex) ?? 0) + 1);
  }

  const charOffset = TINY_INTENT_MODEL.wordBins;
  for (const gram of collectCharNgrams(normalized)) {
    const featureIndex = charOffset + (fnv1a32(`c:${gram}`) % TINY_INTENT_MODEL.charBins);
    counts.set(featureIndex, (counts.get(featureIndex) ?? 0) + 1);
  }

  let l2 = 0;
  for (const value of counts.values()) {
    l2 += value * value;
  }
  const norm = Math.sqrt(l2) || 1;
  for (const [featureIndex, value] of counts.entries()) {
    counts.set(featureIndex, value / norm);
  }
  return counts;
};

const dotSparse = (features: Map<number, number>, indices: readonly number[], values: readonly number[]): number => {
  let total = 0;
  for (let index = 0; index < indices.length; index += 1) {
    const featureValue = features.get(indices[index]);
    if (featureValue !== undefined) {
      total += featureValue * values[index];
    }
  }
  return total;
};

const softmax = (logits: number[]): number[] => {
  const max = Math.max(...logits);
  const exps = logits.map((value) => Math.exp(value - max));
  const sum = exps.reduce((accumulator, value) => accumulator + value, 0) || 1;
  return exps.map((value) => value / sum);
};

const buildScoreMap = (probabilities: number[]): IntentScores => {
  const result = { task: 0, calendar: 0, reminder: 0, none: 0 } satisfies IntentScores;
  TINY_INTENT_MODEL.labels.forEach((label, index) => {
    result[label as keyof IntentScores] = probabilities[index] ?? 0;
  });
  return result;
};

const buildWarning = (
  rawInput: string,
  intent: IntentType,
  scores: IntentScores,
  topTwoGap: number,
): IntentResult['warnings'] => [
  {
    code: 'intent_ambiguous',
    severity: 'info',
    message: intent === 'none' ? 'Intent is ambiguous and was routed to none.' : 'Intent is ambiguous between multiple categories.',
    fieldHints: ['intent'],
    spans: [{ start: 0, end: rawInput.length, text: rawInput }],
    details: {
      scores,
      routedIntent: intent,
      topTwoGap,
    },
  },
];

const buildResult = (
  rawInput: string,
  intent: IntentType,
  scores: IntentScores,
  ambiguous: boolean,
  debugSteps: IntentResult['meta']['debugSteps'],
  topTwoGap: number,
  signals: IntentResult['meta']['signals'],
): IntentResult => ({
  intent,
  scores,
  ambiguous,
  meta: {
    runtime: {
      mode: 'lightweight',
      model: 'tiny-linear',
      version: TINY_INTENT_MODEL.version,
    },
    debugSteps,
    topTwoGap,
    signals,
  },
  warnings: ambiguous ? buildWarning(rawInput, intent, scores, topTwoGap) : null,
});

const looksLikeTimeBlockedTask = (normalized: string, tokens: string[]): boolean =>
  TASK_LEAD_WORDS.has(tokens[0] ?? '') &&
  hasTimeSignal(normalized) &&
  hasDurationSignal(normalized) &&
  /\b(?:starting\s+at|start(?:ing)?\s+at|for\s+\d+\s*(?:seconds?|secs?|minutes?|mins?|hours?|hrs?))\b/i.test(normalized) &&
  !hasStrongCalendarFrame(tokens);

const looksLikeDatedTask = (normalized: string, tokens: string[]): boolean =>
  DATE_TASK_LEAD_WORDS.has(tokens[0] ?? '') &&
  (hasDateSignal(normalized) || hasRecurrenceSignal(normalized)) &&
  !hasStrongCalendarFrame(tokens);

const looksLikeReflectiveNote = (normalized: string, tokens: string[]): boolean =>
  EXPLICIT_NONE_NOTE_PATTERNS.some((pattern) => pattern.test(normalized)) &&
  !TASK_LEAD_WORDS.has(tokens[0] ?? '');

const looksLikeSoftActionIdea = (normalized: string, tokens: string[]): boolean =>
  EXPLICIT_NONE_SOFT_ACTION_PATTERNS.some((pattern) => pattern.test(normalized)) &&
  !hasTimeSignal(normalized) &&
  !hasDateSignal(normalized) &&
  !hasRecurrenceSignal(normalized) &&
  !hasStrongCalendarFrame(tokens);

export const classifyIntent = (input: string, opts?: IntentOptions): IntentResult => {
  const rawInput = typeof input === 'string' ? input : '';
  const normalized = normalizeIntentInput(rawInput);
  const tokens = normalized ? normalized.split(' ') : [];
  const threshold = resolveIntentThreshold(opts);

  if (!normalized) {
    return buildResult(
      rawInput,
      'none',
      { task: 0, calendar: 0, reminder: 0, none: 1 },
      true,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.empty_input',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 1,
          note: 'empty or non-semantic input routed to none',
        },
      ],
      1,
      [{ intent: 'none', source: 'tiny_intent_empty', weight: 1 }],
    );
  }

  if (EXPLICIT_REMINDER_PATTERNS.some((pattern) => pattern.test(normalized))) {
    return buildResult(
      rawInput,
      'reminder',
      { task: 0.03, calendar: 0.03, reminder: 0.92, none: 0.02 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.explicit_reminder',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.92,
          note: 'explicit reminder/follow-up wording routed to reminder',
        },
      ],
      0.89,
      [{ intent: 'reminder', source: 'tiny_intent_explicit_reminder_phrase', weight: 0.92 }],
    );
  }

  if (looksLikeReflectiveNote(normalized, tokens)) {
    return buildResult(
      rawInput,
      'none',
      { task: 0.03, calendar: 0.03, reminder: 0, none: 0.94 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.reflective_note',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.94,
          note: 'reflective note-like phrasing without task or schedule anchors routed to none',
        },
      ],
      0.91,
      [{ intent: 'none', source: 'tiny_intent_reflective_note', weight: 0.94 }],
    );
  }

  if (looksLikeSoftActionIdea(normalized, tokens)) {
    return buildResult(
      rawInput,
      'none',
      { task: 0.12, calendar: 0.03, reminder: 0, none: 0.85 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.soft_action_idea',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.85,
          note: 'soft ideation or vague possibility phrasing routed to none',
        },
      ],
      0.73,
      [{ intent: 'none', source: 'tiny_intent_soft_action_idea', weight: 0.85 }],
    );
  }

  const startsWithTaskLead = TASK_LEAD_WORDS.has(tokens[0] ?? '');
  if (startsWithTaskLead && !hasTimeSignal(normalized) && !hasDateSignal(normalized) && !hasRecurrenceSignal(normalized)) {
    return buildResult(
      rawInput,
      'task',
      { task: 0.86, calendar: 0.12, reminder: 0, none: 0.02 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.task_lead_override',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.86,
          note: 'task-leading imperative without schedule anchors routed to task',
        },
      ],
      0.74,
      [{ intent: 'task', source: 'tiny_intent_task_lead', weight: 0.86 }],
    );
  }

  if (looksLikeTimeBlockedTask(normalized, tokens)) {
    return buildResult(
      rawInput,
      'task',
      { task: 0.78, calendar: 0.2, reminder: 0, none: 0.02 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.time_blocked_task',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.78,
          note: 'work-framed phrase with duration and start time routed to task',
        },
      ],
      0.58,
      [{ intent: 'task', source: 'tiny_intent_time_blocked_task', weight: 0.78 }],
    );
  }

  if (looksLikeDatedTask(normalized, tokens)) {
    return buildResult(
      rawInput,
      'task',
      { task: 0.72, calendar: 0.24, reminder: 0, none: 0.04 },
      false,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.dated_task',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.72,
          note: 'work-framed phrase with due-date style timing routed to task',
        },
      ],
      0.48,
      [{ intent: 'task', source: 'tiny_intent_dated_task', weight: 0.72 }],
    );
  }

  if (tokens.length <= 4 && hasTimeSignal(normalized) && tokens.some((token) => APPOINTMENT_WORDS.has(token))) {
    return buildResult(
      rawInput,
      'none',
      { task: 0.18, calendar: 0.42, reminder: 0, none: 0.4 },
      true,
      [
        {
          pass: 'tinyIntent',
          ruleId: 'tiny_intent.short_appointment_ambiguous',
          start: 0,
          end: rawInput.length,
          text: rawInput,
          confidence: 0.42,
          note: 'short appointment-like phrase with time is ambiguous and routed to none',
        },
      ],
      0.02,
      [
        { intent: 'calendar', source: 'tiny_intent_short_appointment', weight: 0.42 },
        { intent: 'none', source: 'tiny_intent_short_appointment', weight: 0.4 },
      ],
    );
  }

  const features = featurize(normalized);
  const logits = TINY_INTENT_MODEL.classes.map((classModel, index) =>
    dotSparse(features, classModel.indices, classModel.values) + (TINY_INTENT_MODEL.intercepts[index] ?? 0),
  );
  const probabilities = softmax(logits);
  const scores = buildScoreMap(probabilities);

  const ranked = rankIntentScores(scores);
  const [topIntent, topScore] = ranked[0] ?? ['none', 0];
  const [, secondScore] = ranked[1] ?? ['none', 0];
  const topTwoGap = topScore - secondScore;
  const ambiguous = topTwoGap <= GAP_THRESHOLD || topScore < threshold;
  const intent = topIntent === 'none' || topScore < ROUTE_TO_NONE_THRESHOLD ? 'none' : topIntent;

  return buildResult(
    rawInput,
    intent,
    scores,
    ambiguous,
    [
      {
        pass: 'tinyIntent',
        ruleId: 'tiny_intent.softmax',
        start: 0,
        end: rawInput.length,
        text: rawInput,
        confidence: topScore,
        note: `tiny intent routed=${intent} top=${topIntent} topProb=${topScore.toFixed(3)} gap=${topTwoGap.toFixed(3)} threshold=${threshold.toFixed(3)}`,
      },
    ],
    topTwoGap,
    ranked.map(([label, weight]) => ({
      intent: label,
      source: 'tiny_intent_model',
      weight,
    })),
  );
};
