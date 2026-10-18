/// <reference types="node" />

import { strict as assert } from 'node:assert';
import fs from 'node:fs';
import { describe, test } from 'node:test';

import {
  alignFirstPieces,
  decodeArgmax,
  decodeBioViterbi,
  decodeFirstPieceSpans,
  rebuildWordSpans,
  splitWhitespaceWords,
  validateSequenceDecoderSidecar,
} from '../sequence-decoder.ts';

interface GoldenCase {
  labels: string[];
  logits: number[][];
  argmax: number[];
  bio: number[];
}

const goldenCases = JSON.parse(
  fs.readFileSync(new URL('./fixtures/decoder_golden.json', import.meta.url), 'utf8'),
) as GoldenCase[];

const SMALL_LABELS = ['O', 'B-a', 'I-a', 'B-b', 'I-b'];

const isLegalPath = (labels: string[], path: number[]): boolean =>
  path.every((labelId, position) => {
    const label = labels[labelId];
    if (!label.startsWith('I-')) {
      return true;
    }
    if (position === 0) {
      return false;
    }
    const previous = labels[path[position - 1]];
    return previous === label || previous === `B-${label.slice(2)}`;
  });

const allPaths = (length: number, labelCount: number): number[][] => {
  if (length === 0) {
    return [[]];
  }
  return allPaths(length - 1, labelCount).flatMap((prefix) => Array.from({ length: labelCount }, (_, label) => [...prefix, label]));
};

const pathScore = (logits: number[][], path: number[]): number => path.reduce((total, label, position) => total + logits[position][label], 0);

const seededRandom = (seed: number): (() => number) => {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
};

describe('sequence decoder golden cases', () => {
  for (const [index, golden] of goldenCases.entries()) {
    test(`case ${index} matches the reference argmax path`, () => {
      assert.deepEqual(decodeArgmax(golden.logits, golden.labels), golden.argmax);
    });

    test(`case ${index} matches the reference BIO Viterbi path`, () => {
      assert.deepEqual(decodeBioViterbi(golden.logits, golden.labels), golden.bio);
    });
  }

  test('covers every golden case', () => {
    assert.equal(goldenCases.length, 6);
  });
});

describe('BIO Viterbi', () => {
  test('returns an empty path for an empty sequence', () => {
    assert.deepEqual(decodeBioViterbi([], SMALL_LABELS), []);
    assert.deepEqual(decodeArgmax([], SMALL_LABELS), []);
  });

  test('breaks exact ties toward the lowest label ids', () => {
    assert.deepEqual(decodeBioViterbi([[0, 0, 0, 0, 0], [0, 0, 0, 0, 0], [0, 0, 0, 0, 0]], SMALL_LABELS), [0, 0, 0]);
    assert.deepEqual(decodeBioViterbi([[0, 1, 0, 1, 0], [0, 0, 1, 0, 1]], SMALL_LABELS), [1, 2]);
  });

  test('never starts with I and never continues a different field', () => {
    assert.deepEqual(decodeBioViterbi([[0, 0, 5, 0, 0]], SMALL_LABELS), [0]);
    assert.deepEqual(decodeBioViterbi([[0, 3, 0, 0, 0], [0, 0, 0, 0, 5]], SMALL_LABELS), [3, 4]);
    assert.deepEqual(decodeBioViterbi([[0, 0, 0, 3, 0], [0, 0, 5, 0, 0]], SMALL_LABELS), [1, 2]);
  });

  test('finds a highest-scoring legal path on exhaustive random cases', () => {
    const random = seededRandom(729);
    for (let length = 1; length <= 4; length += 1) {
      const candidates = allPaths(length, SMALL_LABELS.length).filter((path) => isLegalPath(SMALL_LABELS, path));
      for (let sample = 0; sample < 25; sample += 1) {
        const logits = Array.from({ length }, () =>
          Array.from({ length: SMALL_LABELS.length }, () => (sample % 5 === 0 ? Math.round(random() * 2) : random() * 4 - 2)),
        );
        const actual = decodeBioViterbi(logits, SMALL_LABELS);
        const best = Math.max(...candidates.map((path) => pathScore(logits, path)));
        assert.equal(isLegalPath(SMALL_LABELS, actual), true);
        assert.equal(pathScore(logits, actual), best);
      }
    }
  });

  test('rejects invalid input', () => {
    assert.throws(() => decodeBioViterbi([[0, 0]], ['O']), /logits per word/);
    assert.throws(() => decodeBioViterbi([[Number.NaN]], ['O']), /finite/);
    assert.throws(() => decodeBioViterbi([[1, 2]], ['O', 'I-a']), /requires B-a/);
    assert.throws(() => decodeBioViterbi([[1, 2]], ['O', 'X-a']), /invalid BIO label/);
    assert.throws(() => decodeBioViterbi([[1, 2]], ['O', 'O']), /unique/);
    assert.throws(() => decodeArgmax([[Number.POSITIVE_INFINITY, 0]], ['O', 'B-a']), /finite/);
  });
});

describe('first-piece alignment and span rebuilding', () => {
  test('splits on Python whitespace and keeps character offsets', () => {
    assert.deepEqual(splitWhitespaceWords('  lunch\u00a0with\tsara\n'), [
      { start: 2, end: 7 },
      { start: 8, end: 12 },
      { start: 13, end: 17 },
    ]);
    assert.deepEqual(splitWhitespaceWords('   '), []);
  });

  test('maps each word to its first token piece and skips special tokens', () => {
    const words = splitWhitespaceWords('pick up groceries, today');
    const offsets: Array<[number, number]> = [
      [0, 0],
      [0, 4],
      [5, 7],
      [8, 13],
      [13, 17],
      [17, 18],
      [19, 24],
      [0, 0],
    ];
    assert.deepEqual(alignFirstPieces(words, offsets), [1, 2, 3, 6]);
  });

  test('leaves truncated words without a first piece', () => {
    const words = splitWhitespaceWords('one two three');
    assert.deepEqual(alignFirstPieces(words, [[0, 0], [0, 3], [0, 0]]), [1, null, null]);
  });

  test('rebuilds spans from whitespace words', () => {
    const text = 'lunch with sara tomorrow at noon, downtown';
    const words = splitWhitespaceWords(text);
    const spans = rebuildWordSpans(text, words, ['B-title', 'I-title', 'I-title', 'B-start_time', 'I-start_time', 'I-start_time', 'B-location']);
    assert.deepEqual(
      spans.map(({ label, start, end, text: spanText }) => ({ label, start, end, text: spanText })),
      [
        { label: 'title', start: 0, end: 15, text: 'lunch with sara' },
        { label: 'start_time', start: 16, end: 33, text: 'tomorrow at noon,' },
        { label: 'location', start: 34, end: 42, text: 'downtown' },
      ],
    );
  });

  test('starts a new span on B, on O gaps, and on a field change', () => {
    const text = 'a b c d e f';
    const words = splitWhitespaceWords(text);
    const spans = rebuildWordSpans(text, words, ['B-x', 'B-x', 'I-x', 'O', 'I-x', 'I-y']);
    assert.deepEqual(
      spans.map(({ label, text: spanText }) => [label, spanText]),
      [
        ['x', 'a'],
        ['x', 'b c'],
        ['x', 'e'],
        ['y', 'f'],
      ],
    );
  });

  test('decodes token logits through first pieces only', () => {
    const labels = ['O', 'B-title', 'I-title', 'B-due_date', 'I-due_date'];
    const text = 'finish reports tomorrow';
    const offsets: Array<[number, number]> = [
      [0, 0],
      [0, 6],
      [7, 13],
      [13, 14],
      [15, 23],
      [0, 0],
    ];
    const rows = [
      [9, 0, 0, 0, 0],
      [0, 9, 0, 0, 0],
      [0, 0, 9, 0, 0],
      [0, 0, 0, 9, 0],
      [0, 0, 0, 0, 9],
      [9, 0, 0, 0, 0],
    ];
    const spans = decodeFirstPieceSpans({ text, offsets, logits: rows.flat(), labels, decoding: 'argmax' });
    assert.deepEqual(
      spans.map(({ label, text: spanText }) => [label, spanText]),
      [
        ['title', 'finish reports'],
        ['due_date', 'tomorrow'],
      ],
    );

    const bioSpans = decodeFirstPieceSpans({ text, offsets, logits: rows.flat(), labels, decoding: 'bio' });
    assert.deepEqual(
      bioSpans.map(({ label, text: spanText }) => [label, spanText]),
      [['title', 'finish reports']],
    );
  });
});

describe('sequence_decoder.json sidecar validation', () => {
  const sidecar = (labels: string[]) => ({
    algorithm: 'BIO Viterbi',
    first_subword_piece_only: true,
    accumulator: 'float64',
    start_scores: labels.map((label) => (label.startsWith('I-') ? null : 0)),
    transition_scores_previous_by_next: labels.map((previous) =>
      labels.map((next) => (!next.startsWith('I-') || previous === next || previous === `B-${next.slice(2)}` ? 0 : null)),
    ),
    end_scores: labels.map(() => 0),
    null_score_means: 'negative infinity',
    tie_break: 'lowest predecessor ID, then lowest final ID',
    labels,
    runtime_max_tokens: 128,
  });

  test('accepts a sidecar that matches the BIO rules', () => {
    assert.doesNotThrow(() => validateSequenceDecoderSidecar(sidecar(SMALL_LABELS), SMALL_LABELS));
  });

  test('rejects mismatched labels and transitions', () => {
    assert.throws(() => validateSequenceDecoderSidecar(sidecar(SMALL_LABELS), ['O', 'B-a', 'I-a']), /labels/);
    const tampered = sidecar(SMALL_LABELS);
    tampered.transition_scores_previous_by_next[1][4] = 0;
    assert.throws(() => validateSequenceDecoderSidecar(tampered, SMALL_LABELS), /transition/);
  });

  test('accepts the shipped calendar sidecar', () => {
    const payload = JSON.parse(
      fs.readFileSync(new URL('../../../resources/capture-parser/calendar/sequence_decoder.json', import.meta.url), 'utf8'),
    );
    assert.doesNotThrow(() => validateSequenceDecoderSidecar(payload, payload.labels));
  });
});
