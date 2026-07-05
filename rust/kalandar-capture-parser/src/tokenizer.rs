use crate::{
    CaptureParserArtifactBundle, CaptureParserArtifactManifest, CaptureParserError,
    BERT_TOKEN_CLASSIFIER_RUNTIME_KIND,
};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizedCaptureInput {
    pub input_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
    pub offsets: Vec<(usize, usize)>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureTokenizerConfig {
    pub max_tokens: usize,
    pub do_lower_case: bool,
    pub unk_token: String,
    pub cls_token: String,
    pub sep_token: String,
    pub pad_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureWordPieceTokenizer {
    pub config: CaptureTokenizerConfig,
    pub special_token_ids: BTreeSet<u32>,
    vocab: HashMap<String, u32>,
    cls_id: u32,
    sep_id: u32,
    unk_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TokenChar {
    value: char,
    source_start: usize,
    source_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BasicToken {
    text: String,
    chars: Vec<TokenChar>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EncodedPiece {
    id: u32,
    start: usize,
    end: usize,
}

impl CaptureWordPieceTokenizer {
    pub fn load(
        manifest: &CaptureParserArtifactManifest,
        bundle: &CaptureParserArtifactBundle,
    ) -> Result<Self, CaptureParserError> {
        if bundle.runtime_kind != BERT_TOKEN_CLASSIFIER_RUNTIME_KIND {
            return Err(CaptureParserError::ManifestInvalid);
        }

        let tokenizer = bundle
            .tokenizer
            .as_ref()
            .ok_or(CaptureParserError::ManifestInvalid)?;
        if tokenizer.max_sequence_length < 2 {
            return Err(CaptureParserError::ManifestInvalid);
        }
        let vocab_spec = bundle
            .files
            .iter()
            .find(|file| file.logical_name == "vocab")
            .ok_or(CaptureParserError::ManifestInvalid)?;
        let vocab_text = std::fs::read_to_string(manifest.resolve_file_path(vocab_spec))
            .map_err(|_| CaptureParserError::ArtifactMissing)?;
        let vocab = parse_vocab(&vocab_text);

        let cls_id = *vocab
            .get(&tokenizer.cls_token)
            .ok_or(CaptureParserError::ManifestInvalid)?;
        let sep_id = *vocab
            .get(&tokenizer.sep_token)
            .ok_or(CaptureParserError::ManifestInvalid)?;
        let pad_id = *vocab
            .get(&tokenizer.pad_token)
            .ok_or(CaptureParserError::ManifestInvalid)?;
        let unk_id = *vocab
            .get(&tokenizer.unk_token)
            .ok_or(CaptureParserError::ManifestInvalid)?;

        Ok(Self {
            config: CaptureTokenizerConfig {
                max_tokens: tokenizer.max_sequence_length,
                do_lower_case: tokenizer.do_lower_case,
                unk_token: tokenizer.unk_token.clone(),
                cls_token: tokenizer.cls_token.clone(),
                sep_token: tokenizer.sep_token.clone(),
                pad_token: tokenizer.pad_token.clone(),
            },
            special_token_ids: BTreeSet::from([cls_id, sep_id, pad_id, unk_id]),
            vocab,
            cls_id,
            sep_id,
            unk_id,
        })
    }

    pub fn encode(&self, input: &str) -> TokenizedCaptureInput {
        let normalized = normalize_input(input, self.config.do_lower_case);
        let basic_tokens = basic_tokenize(&normalized);
        let max_word_pieces = self.config.max_tokens.saturating_sub(2);
        let mut word_pieces = Vec::new();
        let mut truncated = false;

        for token in basic_tokens {
            let encoded = encode_word_piece(&token, &self.vocab, self.unk_id);
            if word_pieces.len() + encoded.len() > max_word_pieces {
                let remaining = max_word_pieces.saturating_sub(word_pieces.len());
                if remaining > 0 {
                    word_pieces.extend(encoded.into_iter().take(remaining));
                }
                truncated = true;
                break;
            }
            word_pieces.extend(encoded);
        }

        let mut input_ids = Vec::with_capacity(word_pieces.len() + 2);
        input_ids.push(self.cls_id as i64);
        input_ids.extend(word_pieces.iter().map(|piece| piece.id as i64));
        input_ids.push(self.sep_id as i64);

        let mut offsets = Vec::with_capacity(word_pieces.len() + 2);
        offsets.push((0, 0));
        offsets.extend(word_pieces.iter().map(|piece| (piece.start, piece.end)));
        offsets.push((0, 0));

        TokenizedCaptureInput {
            attention_mask: vec![1; input_ids.len()],
            input_ids,
            offsets,
            truncated,
        }
    }

    pub fn token_id(&self, token: &str) -> Option<u32> {
        self.vocab.get(token).copied()
    }
}

fn normalize_input(input: &str, do_lower_case: bool) -> Vec<TokenChar> {
    let mut output = Vec::new();
    for (start, raw_char) in input.char_indices() {
        let end = start + raw_char.len_utf8();

        if is_control(raw_char) {
            continue;
        }

        if raw_char.is_whitespace() {
            output.push(TokenChar {
                value: ' ',
                source_start: start,
                source_end: end,
            });
            continue;
        }

        if is_chinese_char(raw_char as u32) {
            output.push(TokenChar {
                value: ' ',
                source_start: start,
                source_end: end,
            });
            output.push(TokenChar {
                value: raw_char,
                source_start: start,
                source_end: end,
            });
            output.push(TokenChar {
                value: ' ',
                source_start: start,
                source_end: end,
            });
            continue;
        }

        if do_lower_case {
            for lowered in raw_char.to_lowercase() {
                output.push(TokenChar {
                    value: lowered,
                    source_start: start,
                    source_end: end,
                });
            }
        } else {
            output.push(TokenChar {
                value: raw_char,
                source_start: start,
                source_end: end,
            });
        }
    }

    output
}

fn basic_tokenize(input: &[TokenChar]) -> Vec<BasicToken> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();

    let flush = |tokens: &mut Vec<BasicToken>, current: &mut Vec<TokenChar>| {
        if current.is_empty() {
            return;
        }
        tokens.push(BasicToken {
            text: current.iter().map(|entry| entry.value).collect(),
            chars: current.clone(),
        });
        current.clear();
    };

    for entry in input {
        if entry.value == ' ' {
            flush(&mut tokens, &mut current);
            continue;
        }

        if is_punctuation_or_symbol(entry.value) {
            flush(&mut tokens, &mut current);
            tokens.push(BasicToken {
                text: entry.value.to_string(),
                chars: vec![entry.clone()],
            });
            continue;
        }

        current.push(entry.clone());
    }

    flush(&mut tokens, &mut current);
    tokens
}

fn encode_word_piece(
    token: &BasicToken,
    vocab: &HashMap<String, u32>,
    unk_id: u32,
) -> Vec<EncodedPiece> {
    if let Some(id) = vocab.get(&token.text) {
        return vec![EncodedPiece {
            id: *id,
            start: token
                .chars
                .first()
                .map(|entry| entry.source_start)
                .unwrap_or(0),
            end: token
                .chars
                .last()
                .map(|entry| entry.source_end)
                .unwrap_or(0),
        }];
    }

    let token_chars = token.text.chars().collect::<Vec<_>>();
    let mut pieces = Vec::new();
    let mut start = 0;

    while start < token_chars.len() {
        let mut end = token_chars.len();
        let mut best_piece = None;

        while end > start {
            let fragment = token_chars[start..end].iter().collect::<String>();
            let candidate = if start == 0 {
                fragment
            } else {
                format!("##{}", fragment)
            };
            if let Some(id) = vocab.get(&candidate) {
                best_piece = Some((*id, end));
                break;
            }
            end -= 1;
        }

        if let Some((id, best_end)) = best_piece {
            pieces.push(EncodedPiece {
                id,
                start: token.chars[start].source_start,
                end: token.chars[best_end - 1].source_end,
            });
            start = best_end;
        } else {
            return vec![EncodedPiece {
                id: unk_id,
                start: token
                    .chars
                    .first()
                    .map(|entry| entry.source_start)
                    .unwrap_or(0),
                end: token
                    .chars
                    .last()
                    .map(|entry| entry.source_end)
                    .unwrap_or(0),
            }];
        }
    }

    pieces
}

fn parse_vocab(text: &str) -> HashMap<String, u32> {
    text.lines()
        .enumerate()
        .filter_map(|(index, token)| {
            (!token.is_empty()).then_some((token.to_string(), index as u32))
        })
        .collect()
}

fn is_control(value: char) -> bool {
    !matches!(value, '\n' | '\r' | '\t') && value.is_control()
}

fn is_punctuation_or_symbol(value: char) -> bool {
    !value.is_alphanumeric() && !value.is_whitespace()
}

fn is_chinese_char(code_point: u32) -> bool {
    matches!(
        code_point,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
    )
}
