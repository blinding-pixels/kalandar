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

const COMBINING_MARK_REGEX = /\p{M}/u;
const PUNCTUATION_REGEX = /[\p{P}\p{S}]/u;
const WHITESPACE_REGEX = /\s/u;

const isControl = (char: string): boolean => {
  if (char === '\n' || char === '\r' || char === '\t') {
    return false;
  }
  return /\p{Cc}|\p{Cf}/u.test(char);
};

const isWhitespace = (char: string): boolean => WHITESPACE_REGEX.test(char);

const isChineseChar = (codePoint: number): boolean =>
  (codePoint >= 0x4e00 && codePoint <= 0x9fff) ||
  (codePoint >= 0x3400 && codePoint <= 0x4dbf) ||
  (codePoint >= 0x20000 && codePoint <= 0x2a6df) ||
  (codePoint >= 0x2a700 && codePoint <= 0x2b73f) ||
  (codePoint >= 0x2b740 && codePoint <= 0x2b81f) ||
  (codePoint >= 0x2b820 && codePoint <= 0x2ceaf) ||
  (codePoint >= 0xf900 && codePoint <= 0xfaff);

const pushMappedChar = (target: TokenChar[], char: string, sourceStart: number, sourceEnd: number): void => {
  target.push({
    char,
    sourceStart,
    sourceEnd,
  });
};

const normalizeInput = (input: string, doLowerCase: boolean): TokenChar[] => {
  const output: TokenChar[] = [];

  for (let index = 0; index < input.length; index += 1) {
    const rawChar = input[index];
    const codePoint = rawChar.codePointAt(0) ?? 0;
    const width = codePoint > 0xffff ? 2 : 1;
    const sourceStart = index;
    const sourceEnd = index + width;

    if (isControl(rawChar)) {
      if (width === 2) {
        index += 1;
      }
      continue;
    }

    if (isWhitespace(rawChar)) {
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      if (width === 2) {
        index += 1;
      }
      continue;
    }

    if (isChineseChar(codePoint)) {
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      pushMappedChar(output, rawChar, sourceStart, sourceEnd);
      pushMappedChar(output, ' ', sourceStart, sourceEnd);
      if (width === 2) {
        index += 1;
      }
      continue;
    }

    const normalized = doLowerCase ? rawChar.toLowerCase() : rawChar;
    const decomposed = normalized.normalize('NFD');
    for (const piece of decomposed) {
      if (COMBINING_MARK_REGEX.test(piece)) {
        continue;
      }
      pushMappedChar(output, piece, sourceStart, sourceEnd);
    }

    if (width === 2) {
      index += 1;
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

    if (PUNCTUATION_REGEX.test(entry.char)) {
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
  let start = 0;
  while (start < token.text.length) {
    let end = token.text.length;
    let bestPiece: string | null = null;
    let bestEnd = start;

    while (end > start) {
      const fragment = token.text.slice(start, end);
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
