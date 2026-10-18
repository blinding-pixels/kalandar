import type {
  BertWordPieceTokenizerManifest,
  LoadedArtifactFiles,
  RuntimeTokenizer,
  TokenizedText,
} from './types.ts';

interface TokenChar {
  char: string;
  sourceStart: number;
  sourceEnd: number;
}

interface BasicToken {
  text: string;
  chars: TokenChar[];
}

const MAX_INPUT_CHARS_PER_WORD = 100;
const NONSPACING_MARK_REGEX = /\p{Mn}/u;
const PUNCTUATION_REGEX = /\p{P}/u;
const ASCII_PUNCTUATION_REGEX = /^[!-/:-@[-`{-~]$/;
const CONTROL_REGEX = /\p{Cc}|\p{Cf}|\p{Co}|\p{Cn}|\p{Cs}/u;
const WHITESPACE_REGEX = /\s/u;

const isControl = (char: string): boolean => {
  if (char === '\n' || char === '\r' || char === '\t') {
    return false;
  }
  return CONTROL_REGEX.test(char);
};

const isWhitespace = (char: string): boolean => WHITESPACE_REGEX.test(char);

const isPunctuation = (char: string): boolean => ASCII_PUNCTUATION_REGEX.test(char) || PUNCTUATION_REGEX.test(char);

const isChineseChar = (codePoint: number): boolean =>
  (codePoint >= 0x4e00 && codePoint <= 0x9fff) ||
  (codePoint >= 0x3400 && codePoint <= 0x4dbf) ||
  (codePoint >= 0x20000 && codePoint <= 0x2a6df) ||
  (codePoint >= 0x2a700 && codePoint <= 0x2b73f) ||
  (codePoint >= 0x2b740 && codePoint <= 0x2b81f) ||
  (codePoint >= 0x2b920 && codePoint <= 0x2ceaf) ||
  (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
  (codePoint >= 0x2f800 && codePoint <= 0x2fa1f);

const pushMappedChar = (target: TokenChar[], char: string, sourceStart: number, sourceEnd: number): void => {
  target.push({
    char,
    sourceStart,
    sourceEnd,
  });
};

/**
 * Mirrors the Hugging Face BertNormalizer the models were trained with: drop NUL, U+FFFD and
 * control characters, map whitespace to a space, pad CJK ideographs, decompose (NFD) and drop
 * nonspacing marks, then lowercase. Offsets stay in UTF-16 units of the input.
 */
const normalizeInput = (input: string, doLowerCase: boolean): TokenChar[] => {
  const output: TokenChar[] = [];

  for (let index = 0; index < input.length; ) {
    const codePoint = input.codePointAt(index) ?? 0;
    const rawChar = String.fromCodePoint(codePoint);
    const sourceStart = index;
    const sourceEnd = index + rawChar.length;
    index = sourceEnd;

    if (codePoint === 0 || codePoint === 0xfffd || isControl(rawChar)) {
      continue;
    }

    if (isWhitespace(rawChar)) {
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      continue;
    }

    if (isChineseChar(codePoint)) {
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      pushMappedChar(output, rawChar, sourceStart, sourceEnd);
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      continue;
    }

    for (const piece of rawChar.normalize('NFD')) {
      if (doLowerCase && NONSPACING_MARK_REGEX.test(piece)) {
        continue;
      }
      for (const lowered of doLowerCase ? piece.toLowerCase() : piece) {
        pushMappedChar(output, lowered, sourceStart, sourceEnd);
      }
    }
  }

  return output;
};

const basicTokenize = (input: TokenChar[]): BasicToken[] => {
  const tokens: BasicToken[] = [];
  let current: TokenChar[] = [];

  const flushCurrent = (): void => {
    if (current.length === 0) {
      return;
    }
    tokens.push({
      text: current.map((entry) => entry.char).join(''),
      chars: current,
    });
    current = [];
  };

  for (const entry of input) {
    if (entry.char === ' ') {
      flushCurrent();
      continue;
    }

    if (isPunctuation(entry.char)) {
      flushCurrent();
      tokens.push({
        text: entry.char,
        chars: [entry],
      });
      continue;
    }

    current.push(entry);
  }

  flushCurrent();
  return tokens;
};

const encodeWordPiece = (
  token: BasicToken,
  vocab: Map<string, number>,
  unkToken: string,
): Array<{ id: number; start: number; end: number }> => {
  if (token.chars.length > MAX_INPUT_CHARS_PER_WORD) {
    const unkId = vocab.get(unkToken);
    if (typeof unkId !== 'number') {
      throw new Error(`Tokenizer vocabulary is missing ${unkToken}`);
    }
    return [{ id: unkId, start: token.chars[0].sourceStart, end: token.chars[token.chars.length - 1].sourceEnd }];
  }
  if (vocab.has(token.text)) {
    return [
      {
        id: vocab.get(token.text) ?? 0,
        start: token.chars[0].sourceStart,
        end: token.chars[token.chars.length - 1].sourceEnd,
      },
    ];
  }

  const pieces: Array<{ id: number; start: number; end: number }> = [];
  const characters = token.chars.map((entry) => entry.char);
  let start = 0;
  while (start < characters.length) {
    let end = characters.length;
    let bestPiece: string | null = null;
    let bestEnd = start;

    while (end > start) {
      const fragment = characters.slice(start, end).join('');
      const candidate = start === 0 ? fragment : `##${fragment}`;
      if (vocab.has(candidate)) {
        bestPiece = candidate;
        bestEnd = end;
        break;
      }
      end -= 1;
    }

    if (!bestPiece) {
      const unkId = vocab.get(unkToken);
      if (typeof unkId !== 'number') {
        throw new Error(`Tokenizer vocabulary is missing ${unkToken}`);
      }
      return [
        {
          id: unkId,
          start: token.chars[0].sourceStart,
          end: token.chars[token.chars.length - 1].sourceEnd,
        },
      ];
    }

    pieces.push({
      id: vocab.get(bestPiece) ?? 0,
      start: token.chars[start].sourceStart,
      end: token.chars[bestEnd - 1].sourceEnd,
    });
    start = bestEnd;
  }

  return pieces;
};

const parseVocab = (text: string): Map<string, number> => {
  const lines = text.split(/\r?\n/);
  const vocab = new Map<string, number>();
  for (let index = 0; index < lines.length; index += 1) {
    const token = lines[index];
    if (!token) {
      continue;
    }
    vocab.set(token, index);
  }
  return vocab;
};

export const createBertWordPieceTokenizer = (
  manifest: BertWordPieceTokenizerManifest,
  files: LoadedArtifactFiles,
): RuntimeTokenizer => {
  const vocabText = files.vocab.text ?? new TextDecoder().decode(files.vocab.bytes);
  const vocab = parseVocab(vocabText);
  const clsId = vocab.get(manifest.cls_token);
  const sepId = vocab.get(manifest.sep_token);
  const padId = vocab.get(manifest.pad_token);
  const unkId = vocab.get(manifest.unk_token);

  if (
    typeof clsId !== 'number' ||
    typeof sepId !== 'number' ||
    typeof padId !== 'number' ||
    typeof unkId !== 'number'
  ) {
    throw new Error('Tokenizer vocabulary is missing required special tokens');
  }

  const specialTokenIds = new Set<number>([clsId, sepId, padId, unkId]);

  return {
    kind: 'bert_wordpiece',
    maxSequenceLength: manifest.max_sequence_length,
    specialTokenIds,
    encode(input: string): TokenizedText {
      const normalized = normalizeInput(input, manifest.do_lower_case);
      const basicTokens = basicTokenize(normalized);
      const maxWordPieces = Math.max(0, manifest.max_sequence_length - 2);
      const wordPieces: Array<{ id: number; start: number; end: number }> = [];
      let truncated = false;

      for (const token of basicTokens) {
        const encodedPieces = encodeWordPiece(token, vocab, manifest.unk_token);
        if (wordPieces.length + encodedPieces.length > maxWordPieces) {
          const remaining = maxWordPieces - wordPieces.length;
          if (remaining > 0) {
            wordPieces.push(...encodedPieces.slice(0, remaining));
          }
          truncated = true;
          break;
        }
        wordPieces.push(...encodedPieces);
      }

      const inputIds = [clsId, ...wordPieces.map((entry) => entry.id), sepId];
      const attentionMask = new Array<number>(inputIds.length).fill(1);
      const offsets: Array<[number, number]> = [
        [0, 0],
        ...wordPieces.map((entry): [number, number] => [entry.start, entry.end]),
        [0, 0],
      ];

      return {
        inputIds,
        attentionMask,
        offsets,
        truncated,
      };
    },
  };
};
