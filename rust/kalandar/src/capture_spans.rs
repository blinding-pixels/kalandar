use kalandar_dates::capture::{CaptureKind, LabelledSpan};

pub(crate) trait CaptureSpanSource {
    fn schedule_spans(&self, kind: CaptureKind, line: &str) -> Option<Vec<LabelledSpan>>;

    fn model_title(&self) -> Option<String> {
        None
    }
}

pub(crate) struct RuleCaptureSpans;

impl CaptureSpanSource for RuleCaptureSpans {
    fn schedule_spans(&self, _kind: CaptureKind, _line: &str) -> Option<Vec<LabelledSpan>> {
        None
    }
}

#[cfg(feature = "onnx")]
pub(crate) use model::ModelCaptureSpans;

#[cfg(feature = "onnx")]
mod model {
    use super::CaptureSpanSource;
    use crate::{
        build_token_classification_request, decode_first_piece_spans, load_ort_session,
        CaptureParserArtifactManifest, CaptureParserError, CaptureParserOrtSession,
        CaptureWordPieceTokenizer, DecodedSpan, RuntimeTensorData, LOGITS_OUTPUT_TENSOR_NAME,
    };
    use kalandar_dates::capture::{CaptureKind, LabelledSpan, Role};
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    pub(crate) struct ModelCaptureSpans<'a> {
        manifest: &'a CaptureParserArtifactManifest,
        failure: RefCell<Option<CaptureParserError>>,
        title: RefCell<Option<String>>,
    }

    impl<'a> ModelCaptureSpans<'a> {
        pub(crate) fn new(manifest: &'a CaptureParserArtifactManifest) -> Self {
            Self {
                manifest,
                failure: RefCell::new(None),
                title: RefCell::new(None),
            }
        }

        pub(crate) fn take_failure(&self) -> Option<CaptureParserError> {
            self.failure.borrow_mut().take()
        }
    }

    impl CaptureSpanSource for ModelCaptureSpans<'_> {
        fn schedule_spans(&self, kind: CaptureKind, line: &str) -> Option<Vec<LabelledSpan>> {
            match model_spans(self.manifest, kind, line) {
                Ok(decoded) => {
                    *self.title.borrow_mut() = merged_title(line, &decoded);
                    Some(
                        decoded
                            .iter()
                            .filter_map(|span| {
                                Role::from_label(&span.label)
                                    .map(|role| LabelledSpan::new(role, span.start, span.end))
                            })
                            .collect(),
                    )
                }
                Err(error) => {
                    self.failure.borrow_mut().get_or_insert(error);
                    None
                }
            }
        }

        fn model_title(&self) -> Option<String> {
            self.title.borrow().clone()
        }
    }

    fn merged_title(line: &str, decoded: &[DecodedSpan]) -> Option<String> {
        let mut titles = decoded
            .iter()
            .filter(|span| span.label == "title")
            .collect::<Vec<_>>();
        titles.sort_by_key(|span| span.start);
        let mut merged = String::new();
        for (index, span) in titles.iter().enumerate() {
            if index > 0 {
                let gap = line
                    .get(titles[index - 1].end..span.start)
                    .unwrap_or("")
                    .trim_end()
                    .to_lowercase();
                merged.push_str(if gap.ends_with(" and") || gap == "and" {
                    " and "
                } else if gap.ends_with(" or") || gap == "or" {
                    " or "
                } else {
                    " "
                });
            }
            merged.push_str(span.text.trim());
        }
        let last = titles.last()?;
        let mut merged = merged.trim().to_string();
        while let Some((head, tail)) = merged.rsplit_once(' ') {
            if !TITLE_CONNECTORS.contains(&tail.to_lowercase().as_str()) {
                break;
            }
            let follower = decoded
                .iter()
                .filter(|span| span.start >= last.end && Role::from_label(&span.label).is_none())
                .min_by_key(|span| span.start)
                .filter(|span| {
                    line.get(last.end..span.start)
                        .is_some_and(|gap| gap.trim().is_empty())
                });
            match follower {
                Some(span) => {
                    merged.push(' ');
                    merged.push_str(span.text.trim());
                    break;
                }
                None => merged = head.trim_end().to_string(),
            }
        }
        (!merged.is_empty()).then_some(merged)
    }

    const TITLE_CONNECTORS: [&str; 8] = ["with", "for", "to", "about", "and", "or", "of", "from"];

    struct FamilyRunner {
        tokenizer: CaptureWordPieceTokenizer,
        session: CaptureParserOrtSession,
    }

    type RunnerKey = (PathBuf, String);

    fn family_runners() -> &'static Mutex<BTreeMap<RunnerKey, FamilyRunner>> {
        static RUNNERS: OnceLock<Mutex<BTreeMap<RunnerKey, FamilyRunner>>> = OnceLock::new();
        RUNNERS.get_or_init(|| Mutex::new(BTreeMap::new()))
    }

    fn parser_family(kind: CaptureKind) -> &'static str {
        match kind {
            CaptureKind::Task => "task",
            CaptureKind::Reminder => "reminder",
            CaptureKind::CalendarEvent => "calendar",
        }
    }

    fn model_spans(
        manifest: &CaptureParserArtifactManifest,
        kind: CaptureKind,
        line: &str,
    ) -> Result<Vec<DecodedSpan>, CaptureParserError> {
        let bundle = manifest
            .bundle(parser_family(kind))
            .ok_or(CaptureParserError::ManifestInvalid)?;
        let decoding = bundle.sequence_decoding()?;
        let key = (manifest.root.clone(), bundle.artifact_id.clone());
        let mut runners = family_runners()
            .lock()
            .map_err(|_| CaptureParserError::InferenceFailed)?;
        if !runners.contains_key(&key) {
            let runner = FamilyRunner {
                tokenizer: CaptureWordPieceTokenizer::load(manifest, bundle)?,
                session: load_ort_session(manifest, bundle)?,
            };
            runners.insert(key.clone(), runner);
        }
        let runner = runners
            .get_mut(&key)
            .ok_or(CaptureParserError::InferenceFailed)?;
        let encoded = runner.tokenizer.encode(line);
        let request = build_token_classification_request(manifest, bundle, &encoded)?;
        let output = runner.session.run(&request)?;
        drop(runners);

        let RuntimeTensorData::Float32(logits) = &output
            .outputs
            .get(LOGITS_OUTPUT_TENSOR_NAME)
            .ok_or(CaptureParserError::InferenceFailed)?
            .data
        else {
            return Err(CaptureParserError::InferenceFailed);
        };
        decode_first_piece_spans(line, &encoded.offsets, logits, &bundle.labels, decoding)
            .map_err(|_| CaptureParserError::InferenceFailed)
    }
}
