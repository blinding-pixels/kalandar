use kalandar::{
    build_token_classification_request, load_capture_parser_artifact_manifest_from_root,
    load_ort_session, CaptureWordPieceTokenizer, RuntimeTensorData, LOGITS_OUTPUT_TENSOR_NAME,
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
fn bundled_task_ort_session_loads_and_runs_inference() {
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

    let mut session = load_ort_session(&manifest, bundle).expect("ort session should load");
    let output = session.run(&request).expect("ort inference should run");

    assert_eq!(session.input_names(), vec!["input_ids", "attention_mask"]);
    assert_eq!(session.output_names(), vec!["logits"]);
    match &output.outputs[LOGITS_OUTPUT_TENSOR_NAME].data {
        RuntimeTensorData::Float32(values) => assert!(!values.is_empty()),
        RuntimeTensorData::Int64(_) => panic!("logits output should be float32"),
    }
    assert_eq!(
        output.outputs[LOGITS_OUTPUT_TENSOR_NAME].shape,
        vec![1, encoded.input_ids.len(), bundle.labels.len()]
    );
}

#[test]
fn bundled_reminder_ort_session_loads_and_runs_inference() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");
    let bundle = manifest
        .bundle("reminder")
        .expect("reminder bundle should exist");
    let tokenizer =
        CaptureWordPieceTokenizer::load(&manifest, bundle).expect("tokenizer should load");
    let encoded = tokenizer.encode("remind me to water plants every friday at 9am");
    let request = build_token_classification_request(&manifest, bundle, &encoded)
        .expect("tensor request should build");

    let mut session = load_ort_session(&manifest, bundle).expect("ort session should load");
    let output = session.run(&request).expect("ort inference should run");

    assert_eq!(
        output.outputs[LOGITS_OUTPUT_TENSOR_NAME].shape,
        vec![1, encoded.input_ids.len(), bundle.labels.len()]
    );
}

#[test]
fn bundled_calendar_ort_session_loads_and_runs_inference() {
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
    let encoded = tokenizer.encode("lunch with sara tomorrow at noon");
    let request = build_token_classification_request(&manifest, bundle, &encoded)
        .expect("tensor request should build");

    let mut session = load_ort_session(&manifest, bundle).expect("ort session should load");
    let output = session.run(&request).expect("ort inference should run");

    assert_eq!(
        output.outputs[LOGITS_OUTPUT_TENSOR_NAME].shape,
        vec![1, encoded.input_ids.len(), bundle.labels.len()]
    );
}
