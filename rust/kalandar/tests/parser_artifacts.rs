use kalandar::{
    load_capture_parser_artifact_manifest_from_root, CaptureParserError,
    CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

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

fn copy_dir_all(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination directory should be created");
    for entry in fs::read_dir(source).expect("source directory should be readable") {
        let entry = entry.expect("directory entry should be readable");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_all(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path)
                .expect("fixture file should be copied successfully");
        }
    }
}

#[test]
fn bundled_capture_parser_manifest_loads_from_desktop_resources() {
    let Some(resource_root) = resource_root_or_skip() else {
        return;
    };
    let manifest = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect("resource manifest should load");

    assert_eq!(
        manifest.manifest_version,
        CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION
    );
    assert!(manifest.bundle("intent").is_some());
    assert!(manifest.bundle("task").is_some());
    assert!(manifest.bundle("reminder").is_some());
    assert!(manifest.bundle("calendar").is_some());
}

#[test]
fn capture_parser_manifest_rejects_missing_artifact_file() {
    let Some(resource_source) = resource_root_or_skip() else {
        return;
    };
    let temp = tempdir().expect("temp dir should be created");
    let resource_root = temp.path().join("capture-parser");
    copy_dir_all(resource_source.as_path(), resource_root.as_path());

    fs::remove_file(resource_root.join("task").join("vocab.txt"))
        .expect("test should remove copied vocab");

    let error = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect_err("missing vocab should fail");
    assert_eq!(error, CaptureParserError::ArtifactMissing);
}

#[test]
fn capture_parser_manifest_rejects_checksum_mismatch() {
    let Some(resource_source) = resource_root_or_skip() else {
        return;
    };
    let temp = tempdir().expect("temp dir should be created");
    let resource_root = temp.path().join("capture-parser");
    copy_dir_all(resource_source.as_path(), resource_root.as_path());

    fs::write(
        resource_root.join("calendar").join("model.onnx"),
        "tampered-calendar-model\n",
    )
    .expect("test should rewrite copied model");

    let error = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect_err("tampered model should fail integrity verification");
    assert_eq!(error, CaptureParserError::ArtifactIntegrityMismatch);
}

#[test]
fn capture_parser_manifest_rejects_digest_drift_even_when_json_is_well_formed() {
    let Some(resource_source) = resource_root_or_skip() else {
        return;
    };
    let temp = tempdir().expect("temp dir should be created");
    let resource_root = temp.path().join("capture-parser");
    copy_dir_all(resource_source.as_path(), resource_root.as_path());

    let manifest_path = resource_root.join("manifest.json");
    let manifest = fs::read_to_string(&manifest_path).expect("copied manifest should read");
    let tampered = manifest.replace(
        "\"suiteVersion\": \"kalandar-rust-v1\"",
        "\"suiteVersion\": \"kalandar-rust-v1-tampered\"",
    );
    fs::write(&manifest_path, tampered).expect("tampered manifest should write");

    let error = load_capture_parser_artifact_manifest_from_root(resource_root.as_path())
        .expect_err("tampered manifest digest should fail");
    assert_eq!(error, CaptureParserError::ManifestInvalid);
}
