use kalandar::{
    build_token_classification_request, load_capture_parser_artifact_manifest_from_root,
    CaptureWordPieceTokenizer, RuntimeTensorData, ATTENTION_MASK_TENSOR_NAME,
    INPUT_IDS_TENSOR_NAME, LOGITS_OUTPUT_TENSOR_NAME,
};
use std::{env, path::PathBuf};

fn capture_parser_resource_root() -> Option<PathBuf> {
    env::var_os("KALANDAR_CAPTURE_RESOURCE_ROOT").map(PathBuf::from)
}

fn resource_root_or_skip() -> Option<PathBuf> {
    let resource_root = capture_parser_resource_root();
    if resource_root.is_none() {
        eprintln!("skipping resource-backed capture parser test: KALANDAR_CAPTURE_RESOURCE_ROOT is not set");
    }
    resource_root
}

#[test]
fn bundled_task_tokenizer_encodes_known_tokens() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let bundle = manifest.bundle("task").expect("task bundle should exist");
    let tokenizer =
        CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");

    let encoded = tokenizer.encode("finish report tomorrow");

    assert_eq!(
        encoded.input_ids,
        vec![
            tokenizer.token_id("[CLS]").expect("cls token should exist") as i64,
            tokenizer
                .token_id("finish")
                .expect("finish token should exist") as i64,
            tokenizer
                .token_id("report")
                .expect("report token should exist") as i64,
            tokenizer
                .token_id("tomorrow")
                .expect("tomorrow token should exist") as i64,
            tokenizer.token_id("[SEP]").expect("sep token should exist") as i64,
        ]
    );
    assert_eq!(encoded.attention_mask, vec![1, 1, 1, 1, 1]);
    assert_eq!(
        encoded.offsets,
        vec![(0, 0), (0, 6), (7, 13), (14, 22), (0, 0)]
    );
    assert!(!encoded.truncated);
}

#[test]
fn bundled_calendar_tokenizer_splits_punctuation_and_lowercases_input() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let bundle = manifest
        .bundle("calendar")
        .expect("calendar bundle should exist");
    let tokenizer =
        CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");

    let encoded = tokenizer.encode("Lunch with Sara.");

    assert_eq!(
        encoded.input_ids,
        vec![
            tokenizer.token_id("[CLS]").expect("cls token should exist") as i64,
            tokenizer
                .token_id("lunch")
                .expect("lunch token should exist") as i64,
            tokenizer.token_id("with").expect("with token should exist") as i64,
            tokenizer.token_id("sara").expect("sara token should exist") as i64,
            tokenizer.token_id(".").expect("period token should exist") as i64,
            tokenizer.token_id("[SEP]").expect("sep token should exist") as i64,
        ]
    );
    assert_eq!(encoded.attention_mask, vec![1, 1, 1, 1, 1, 1]);
    assert_eq!(
        encoded.offsets,
        vec![(0, 0), (0, 5), (6, 10), (11, 15), (15, 16), (0, 0)]
    );
}

#[test]
fn token_classification_request_uses_tensor_contract_shape() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let bundle = manifest.bundle("task").expect("task bundle should exist");
    let tokenizer =
        CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");
    let encoded = tokenizer.encode("finish report tomorrow");

    let request = build_token_classification_request(&manifest, bundle, &encoded)
        .expect("tensor request should build");

    assert_eq!(request.descriptor.parser_family, "task");
    assert_eq!(
        request.descriptor.tensor_contract_version,
        "task-bert-token-classification-v1"
    );
    assert_eq!(request.outputs, vec![LOGITS_OUTPUT_TENSOR_NAME.to_string()]);

    match &request.inputs[INPUT_IDS_TENSOR_NAME].data {
        RuntimeTensorData::Int64(values) => assert_eq!(
            values,
            &vec![
                tokenizer.token_id("[CLS]").expect("cls token should exist") as i64,
                tokenizer
                    .token_id("finish")
                    .expect("finish token should exist") as i64,
                tokenizer
                    .token_id("report")
                    .expect("report token should exist") as i64,
                tokenizer
                    .token_id("tomorrow")
                    .expect("tomorrow token should exist") as i64,
                tokenizer.token_id("[SEP]").expect("sep token should exist") as i64,
            ]
        ),
        RuntimeTensorData::Float32(_) => panic!("input_ids should use int64 tensors"),
    }
    match &request.inputs[ATTENTION_MASK_TENSOR_NAME].data {
        RuntimeTensorData::Int64(values) => assert_eq!(values, &vec![1, 1, 1, 1, 1]),
        RuntimeTensorData::Float32(_) => panic!("attention_mask should use int64 tensors"),
    }
    assert_eq!(request.inputs[INPUT_IDS_TENSOR_NAME].shape, vec![1, 5]);
    assert_eq!(request.inputs[ATTENTION_MASK_TENSOR_NAME].shape, vec![1, 5]);
}

#[test]
fn tokenizer_marks_truncated_sequences_when_wordpieces_exceed_budget() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let bundle = manifest.bundle("task").expect("task bundle should exist");
    let tokenizer =
        CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");
    let repeated = std::iter::repeat_n("finish", 200)
        .collect::<Vec<_>>()
        .join(" ");

    let encoded = tokenizer.encode(&repeated);

    assert!(encoded.truncated);
    assert_eq!(encoded.input_ids.len(), tokenizer.config.max_tokens);
    assert_eq!(encoded.attention_mask.len(), tokenizer.config.max_tokens);
}
