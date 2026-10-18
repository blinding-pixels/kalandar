// First-piece sequence decoding for token-classification artifacts.
// Implements the round-two tensor contract: split text into whitespace words,
// keep only the first subword piece of each represented word, decode the
// [words, labels] logits matrix with argmax or constrained BIO Viterbi, and
// rebuild spans from whitespace word offsets.

export type SequenceDecoding = 'argmax' | 'bio';

export const SEQUENCE_DECODINGS: readonly SequenceDecoding[] = ['argmax', 'bio'];

export interface WhitespaceWord {
  start: number;
  end: number;
}

export interface DecodedSpan {
  label: string;
  start: number;
  end: number;
  text: string;
  confidence: number;
}

export interface SequenceDecoderSidecar {
  algorithm: string;
  first_subword_piece_only: boolean;
  accumulator: string;
  start_scores: Array<number | null>;
  transition_scores_previous_by_next: Array<Array<number | null>>;
  end_scores: Array<number | null>;
  null_score_means: string;
  tie_break: string;
  labels: string[];
  runtime_max_tokens?: number;
}

// Python str.split() whitespace, so word boundaries match the training contract.
const WHITESPACE_WORD_REGEX = /[^\t\n\v\f\r\x1c-\x1f \x85\xa0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+/g;

export const isSequenceDecoding = (value: unknown): value is SequenceDecoding =>
  typeof value === 'string' && (SEQUENCE_DECODINGS as readonly string[]).includes(value);

export const splitWhitespaceWords = (text: string): WhitespaceWord[] =>
  Array.from(text.matchAll(WHITESPACE_WORD_REGEX), (match) => ({
    start: match.index ?? 0,
    end: (match.index ?? 0) + match[0].length,
  }));

/**
 * Maps each whitespace word to the token index of its first subword piece.
 * Tokens with empty offsets (special tokens and padding) are ignored. Words
 * without a token, such as words past truncation, map to null.
 */
export const alignFirstPieces = (words: WhitespaceWord[], offsets: ReadonlyArray<readonly [number, number]>): Array<number | null> => {
  const firstPieces = new Array<number | null>(words.length).fill(null);
  let wordIndex = 0;
  for (let tokenIndex = 0; tokenIndex < offsets.length; tokenIndex += 1) {
    const offset = offsets[tokenIndex];
    if (!offset) {
      continue;
    }
    const [start, end] = offset;
    if (end <= start) {
      continue;
    }
    while (wordIndex < words.length && words[wordIndex].end <= start) {
      wordIndex += 1;
    }
    if (wordIndex >= words.length) {
      break;
    }
    if (start >= words[wordIndex].start && firstPieces[wordIndex] === null) {
      firstPieces[wordIndex] = tokenIndex;
    }
  }
  return firstPieces;
};

export const validateBioLabels = (labels: readonly string[]): void => {
  if (new Set(labels).size !== labels.length || !labels.includes('O')) {
    throw new Error('Sequence decoder requires unique BIO labels including O');
  }
  for (const label of labels) {
    if (label === 'O') {
      continue;
    }
    const prefix = label.slice(0, 2);
    if ((prefix !== 'B-' && prefix !== 'I-') || label.length <= 2) {
      throw new Error(`Sequence decoder received invalid BIO label ${label}`);
    }
    if (prefix === 'I-' && !labels.includes(`B-${label.slice(2)}`)) {
      throw new Error(`Sequence decoder label ${label} requires B-${label.slice(2)}`);
    }
  }
};

const validateLogits = (logits: ReadonlyArray<ArrayLike<number>>, labelCount: number): void => {
  for (const row of logits) {
    if (row.length !== labelCount) {
      throw new Error(`Sequence decoder expected ${labelCount} logits per word, received ${row.length}`);
    }
    for (let index = 0; index < row.length; index += 1) {
      if (!Number.isFinite(row[index])) {
        throw new Error('Sequence decoder requires finite logits');
      }
    }
  }
};

const argmaxRow = (row: ArrayLike<number>): number => {
  let best = 0;
  for (let index = 1; index < row.length; index += 1) {
    if (row[index] > row[best]) {
      best = index;
    }
  }
  return best;
};

/** Legal BIO starts, indexed by label id. */
export const bioStartAllowed = (labels: readonly string[]): boolean[] => labels.map((label) => !label.startsWith('I-'));

/** Legal BIO transitions, indexed [previous][next]. */
export const bioTransitionAllowed = (labels: readonly string[]): boolean[][] =>
  labels.map((previous) =>
    labels.map((next) => !next.startsWith('I-') || previous === next || previous === `B-${next.slice(2)}`),
  );

export const decodeArgmax = (logits: ReadonlyArray<ArrayLike<number>>, labels: readonly string[]): number[] => {
  validateBioLabels(labels);
  validateLogits(logits, labels.length);
  return logits.map((row) => argmaxRow(row));
};

/**
 * Constrained BIO Viterbi over raw first-piece logits. Accumulates in float64;
 * on exact ties picks the lowest predecessor id, then the lowest final id.
 */
export const decodeBioViterbi = (logits: ReadonlyArray<ArrayLike<number>>, labels: readonly string[]): number[] => {
  validateBioLabels(labels);
  validateLogits(logits, labels.length);
  if (logits.length === 0) {
    return [];
  }

  const labelCount = labels.length;
  const startAllowed = bioStartAllowed(labels);
  const allowed = bioTransitionAllowed(labels);
  let scores = Array.from({ length: labelCount }, (_, label) => (startAllowed[label] ? Number(logits[0][label]) : -Infinity));
  const backpointers: number[][] = [];

  for (let position = 1; position < logits.length; position += 1) {
    const row = logits[position];
    const next = new Array<number>(labelCount);
    const back = new Array<number>(labelCount);
    for (let label = 0; label < labelCount; label += 1) {
      let bestScore = -Infinity;
      let bestPrevious = 0;
      for (let previous = 0; previous < labelCount; previous += 1) {
        if (allowed[previous][label] && scores[previous] > bestScore) {
          bestScore = scores[previous];
          bestPrevious = previous;
        }
      }
      next[label] = Number(row[label]) + bestScore;
      back[label] = bestPrevious;
    }
    backpointers.push(back);
    scores = next;
  }

  let current = argmaxRow(scores);
  const path = [current];
  for (let position = backpointers.length - 1; position >= 0; position -= 1) {
    current = backpointers[position][current];
    path.push(current);
  }
  return path.reverse();
};

export const decodeSequence = (
  logits: ReadonlyArray<ArrayLike<number>>,
  labels: readonly string[],
  decoding: SequenceDecoding,
): number[] => (decoding === 'bio' ? decodeBioViterbi(logits, labels) : decodeArgmax(logits, labels));

/**
 * Rebuilds spans from per-word labels. O ends a span, B always starts one, and
 * I extends the current span only when the previous word has the same field.
 */
export const rebuildWordSpans = (
  text: string,
  words: WhitespaceWord[],
  wordLabels: readonly string[],
  wordConfidences?: readonly number[],
): DecodedSpan[] => {
  const spans: Array<DecodedSpan & { confidenceSum: number; confidenceCount: number }> = [];
  for (let index = 0; index < words.length; index += 1) {
    const label = wordLabels[index] ?? 'O';
    if (label === 'O') {
      continue;
    }
    const separator = label.indexOf('-');
    const prefix = label.slice(0, separator);
    const field = label.slice(separator + 1);
    const previous = index > 0 ? (wordLabels[index - 1] ?? 'O') : 'O';
    const confidence = wordConfidences?.[index] ?? 1;
    const current = spans[spans.length - 1];
    if (prefix === 'B' || previous === 'O' || previous.slice(previous.indexOf('-') + 1) !== field || !current) {
      spans.push({
        label: field,
        start: words[index].start,
        end: words[index].end,
        text: '',
        confidence: 0,
        confidenceSum: confidence,
        confidenceCount: 1,
      });
      continue;
    }
    current.end = words[index].end;
    current.confidenceSum += confidence;
    current.confidenceCount += 1;
  }
  return spans.map(({ confidenceSum, confidenceCount, ...span }) => ({
    ...span,
    text: text.slice(span.start, span.end),
    confidence: Number((confidenceSum / confidenceCount).toFixed(4)),
  }));
};

const softmaxProbability = (row: ArrayLike<number>, selected: number): number => {
  let max = -Infinity;
  for (let index = 0; index < row.length; index += 1) {
    max = Math.max(max, row[index]);
  }
  let sum = 0;
  for (let index = 0; index < row.length; index += 1) {
    sum += Math.exp(row[index] - max);
  }
  return Number.isFinite(sum) && sum > 0 ? Math.exp(row[selected] - max) / sum : 0;
};

export interface FirstPieceDecodeInput {
  text: string;
  offsets: ReadonlyArray<readonly [number, number]>;
  /** Row-major [sequence, labels] logits for a single batch item. */
  logits: ArrayLike<number>;
  labels: readonly string[];
  decoding: SequenceDecoding;
}

/**
 * Decodes one tokenized input end to end: aligns first pieces, decodes their
 * logits, and rebuilds whitespace-word spans. Span confidence is the mean
 * softmax probability of the selected label on each word's first piece; it
 * does not influence decoding.
 */
export const decodeFirstPieceSpans = ({ text, offsets, logits, labels, decoding }: FirstPieceDecodeInput): DecodedSpan[] => {
  const labelCount = labels.length;
  if (logits.length !== offsets.length * labelCount) {
    throw new Error(`Sequence decoder expected ${offsets.length * labelCount} logits, received ${logits.length}`);
  }
  const words = splitWhitespaceWords(text);
  const firstPieces = alignFirstPieces(words, offsets);
  const represented: number[] = [];
  const rows: number[][] = [];
  firstPieces.forEach((tokenIndex, wordIndex) => {
    if (tokenIndex === null) {
      return;
    }
    represented.push(wordIndex);
    rows.push(Array.from({ length: labelCount }, (_, label) => logits[tokenIndex * labelCount + label]));
  });

  const labelIds = decodeSequence(rows, labels, decoding);
  const wordLabels = new Array<string>(words.length).fill('O');
  const wordConfidences = new Array<number>(words.length).fill(0);
  represented.forEach((wordIndex, rowIndex) => {
    wordLabels[wordIndex] = labels[labelIds[rowIndex]];
    wordConfidences[wordIndex] = softmaxProbability(rows[rowIndex], labelIds[rowIndex]);
  });
  return rebuildWordSpans(text, words, wordLabels, wordConfidences);
};

/** Orders labels by contiguous id, failing when ids are missing. */
export const orderedLabels = (idToLabel: Record<number, string>): string[] => {
  const count = Object.keys(idToLabel).length;
  return Array.from({ length: count }, (_, id) => {
    const label = idToLabel[id];
    if (typeof label !== 'string') {
      throw new Error(`Artifact labels are not contiguous: missing id ${id}`);
    }
    return label;
  });
};

/**
 * Checks that a sequence_decoder.json sidecar declares exactly the BIO Viterbi
 * rules this module implements for the given labels.
 */
export const validateSequenceDecoderSidecar = (sidecar: unknown, labels: readonly string[]): SequenceDecoderSidecar => {
  const value = sidecar as SequenceDecoderSidecar;
  const fail = (reason: string): never => {
    throw new Error(`Invalid sequence_decoder.json: ${reason}`);
  };
  if (!value || typeof value !== 'object') {
    fail('expected an object');
  }
  const checks: Array<[boolean, string]> = [
    [value.algorithm === 'BIO Viterbi', 'algorithm must be "BIO Viterbi"'],
    [value.first_subword_piece_only === true, 'first_subword_piece_only must be true'],
    [value.accumulator === 'float64', 'accumulator must be float64'],
    [value.null_score_means === 'negative infinity', 'null_score_means must be "negative infinity"'],
    [value.tie_break === 'lowest predecessor ID, then lowest final ID', 'unsupported tie_break'],
    [
      Array.isArray(value.labels) &&
        value.labels.length === labels.length &&
        value.labels.every((label, index) => label === labels[index]),
      'labels do not match the artifact config',
    ],
  ];
  for (const [passed, reason] of checks) {
    if (!passed) {
      fail(reason);
    }
  }
  validateBioLabels(labels);
  const legalScore = (allowedValue: boolean) => (allowedValue ? 0 : null);
  const startAllowed = bioStartAllowed(labels);
  const allowed = bioTransitionAllowed(labels);
  const sameVector = (actual: unknown, expected: Array<number | null>) =>
    Array.isArray(actual) && actual.length === expected.length && actual.every((entry, index) => entry === expected[index]);
  if (!sameVector(value.start_scores, startAllowed.map(legalScore))) {
    fail('start_scores do not match the BIO start rules');
  }
  if (!sameVector(value.end_scores, labels.map(() => 0))) {
    fail('end_scores must allow every label');
  }
  if (
    !Array.isArray(value.transition_scores_previous_by_next) ||
    value.transition_scores_previous_by_next.length !== labels.length ||
    !value.transition_scores_previous_by_next.every((row, index) => sameVector(row, allowed[index].map(legalScore)))
  ) {
    fail('transition_scores_previous_by_next do not match the BIO transition rules');
  }
  return value;
};
