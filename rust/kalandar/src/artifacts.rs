use crate::{
    validate_bio_labels, validate_sequence_decoder_sidecar, CaptureParserError, SequenceDecoding,
    SEQUENCE_DECODER_LOGICAL_NAME,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const CAPTURE_PARSER_RESOURCE_ROOT: &str = "capture-parser";
pub const CAPTURE_PARSER_MANIFEST_NAME: &str = "manifest.json";
pub const CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION: &str = "kalandar-artifacts-v1";
pub const LEGACY_CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION: &str =
    "facets-capture-parser-artifacts-v1";
pub const BERT_TOKEN_CLASSIFIER_RUNTIME_KIND: &str = "bert-token-classifier";
pub const TINY_LINEAR_RUNTIME_KIND: &str = "tiny-linear";
pub const TRUSTED_CAPTURE_PARSER_MANIFEST_SHA256: &str =
    "c90dd12c907a20419ceae7029a6bae58f38c1f53d9f4038f8af77c8518030d9b";
const INTENT_TINY_LINEAR_LABELS: [&str; 3] = ["calendar", "none", "task"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureParserArtifactManifest {
    #[serde(skip)]
    pub root: PathBuf,
    pub manifest_version: String,
    pub suite_version: String,
    pub bundles: Vec<CaptureParserArtifactBundle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureParserArtifactBundle {
    pub parser: String,
    pub runtime_kind: String,
    pub artifact_id: String,
    pub tensor_contract_version: Option<String>,
    pub normalization_version: String,
    pub postprocess_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoding: Option<String>,
    pub tokenizer: Option<CaptureParserTokenizerManifest>,
    pub labels: Vec<String>,
    pub files: Vec<CaptureParserArtifactFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureParserTokenizerManifest {
    pub kind: String,
    pub do_lower_case: bool,
    pub unk_token: String,
    pub cls_token: String,
    pub sep_token: String,
    pub pad_token: String,
    pub max_sequence_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureParserArtifactFile {
    pub logical_name: String,
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactIdFingerprint<'a> {
    parser: &'a str,
    runtime_kind: &'a str,
    tensor_contract_version: &'a Option<String>,
    normalization_version: &'a str,
    postprocess_version: &'a str,
    tokenizer: &'a Option<CaptureParserTokenizerManifest>,
    labels: Vec<&'a str>,
    files: Vec<&'a CaptureParserArtifactFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decoding: Option<&'a str>,
}

impl CaptureParserArtifactBundle {
    pub fn sequence_decoding(&self) -> Result<SequenceDecoding, CaptureParserError> {
        match self.decoding.as_deref() {
            None => Ok(SequenceDecoding::Argmax),
            Some(value) => {
                SequenceDecoding::parse(value).ok_or(CaptureParserError::ManifestInvalid)
            }
        }
    }
}

impl CaptureParserArtifactManifest {
    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(CAPTURE_PARSER_MANIFEST_NAME)
    }

    pub fn bundle(&self, parser: &str) -> Option<&CaptureParserArtifactBundle> {
        self.bundles.iter().find(|bundle| bundle.parser == parser)
    }

    pub fn resolve_file_path(&self, file: &CaptureParserArtifactFile) -> PathBuf {
        self.root.join(&file.path)
    }

    pub fn validate(&self) -> Result<(), CaptureParserError> {
        if self.manifest_version != CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION
            && self.manifest_version != LEGACY_CAPTURE_PARSER_ARTIFACT_MANIFEST_VERSION
        {
            return Err(CaptureParserError::ManifestInvalid);
        }
        if self.suite_version.trim().is_empty() || self.bundles.is_empty() {
            return Err(CaptureParserError::ManifestInvalid);
        }

        let mut seen_parsers = BTreeSet::new();
        for bundle in &self.bundles {
            if !seen_parsers.insert(bundle.parser.as_str()) {
                return Err(CaptureParserError::ManifestInvalid);
            }
            validate_bundle_shape(bundle)?;
            validate_bundle_artifact_id(bundle)?;
            validate_bundle_files(self, bundle)?;
        }

        Ok(())
    }
}

pub fn manifest_root_from_resources(resources_root: &Path) -> PathBuf {
    resources_root.join(CAPTURE_PARSER_RESOURCE_ROOT)
}

pub fn load_capture_parser_artifact_manifest(
    resources_root: &Path,
) -> Result<CaptureParserArtifactManifest, CaptureParserError> {
    load_capture_parser_artifact_manifest_from_root(&manifest_root_from_resources(resources_root))
}

pub fn load_capture_parser_artifact_manifest_from_root(
    root: &Path,
) -> Result<CaptureParserArtifactManifest, CaptureParserError> {
    load_capture_parser_artifact_manifest_from_root_with_sha256(
        root,
        TRUSTED_CAPTURE_PARSER_MANIFEST_SHA256,
    )
}

pub fn load_capture_parser_artifact_manifest_from_root_with_sha256(
    root: &Path,
    manifest_sha256: &str,
) -> Result<CaptureParserArtifactManifest, CaptureParserError> {
    let expected_manifest_hash = normalized_manifest_sha256(manifest_sha256)?;
    let manifest_path = root.join(CAPTURE_PARSER_MANIFEST_NAME);
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|_| CaptureParserError::ManifestMissing)?;
    if sha256_hex(&manifest_bytes) != expected_manifest_hash {
        return Err(CaptureParserError::ManifestInvalid);
    }
    let manifest_text =
        String::from_utf8(manifest_bytes).map_err(|_| CaptureParserError::ManifestInvalid)?;
    let mut manifest: CaptureParserArtifactManifest =
        serde_json::from_str(&manifest_text).map_err(|_| CaptureParserError::ManifestInvalid)?;
    manifest.root = root
        .canonicalize()
        .map_err(|_| CaptureParserError::ManifestMissing)?;
    manifest.validate()?;
    Ok(manifest)
}

fn validate_bundle_shape(bundle: &CaptureParserArtifactBundle) -> Result<(), CaptureParserError> {
    if bundle.parser.trim().is_empty()
        || bundle.artifact_id.trim().is_empty()
        || bundle.normalization_version.trim().is_empty()
        || bundle.postprocess_version.trim().is_empty()
        || bundle.labels.is_empty()
        || bundle.files.is_empty()
    {
        return Err(CaptureParserError::ManifestInvalid);
    }
    if bundle.sequence_decoding()? == SequenceDecoding::Bio {
        validate_bio_labels(&bundle.labels).map_err(|_| CaptureParserError::ManifestInvalid)?;
    }

    let mut logical_names = BTreeSet::new();
    for file in &bundle.files {
        if file.logical_name.trim().is_empty()
            || file.path.trim().is_empty()
            || file.sha256.len() != 64
            || !file.sha256.chars().all(|char| char.is_ascii_hexdigit())
            || !logical_names.insert(file.logical_name.as_str())
        {
            return Err(CaptureParserError::ManifestInvalid);
        }
    }

    match bundle.runtime_kind.as_str() {
        BERT_TOKEN_CLASSIFIER_RUNTIME_KIND => {
            let tokenizer = bundle
                .tokenizer
                .as_ref()
                .ok_or(CaptureParserError::ManifestInvalid)?;
            if tokenizer.kind != "bert_wordpiece"
                || tokenizer.max_sequence_length < 2
                || tokenizer.unk_token.is_empty()
                || tokenizer.cls_token.is_empty()
                || tokenizer.sep_token.is_empty()
                || tokenizer.pad_token.is_empty()
            {
                return Err(CaptureParserError::ManifestInvalid);
            }
            for required_file in [
                "model",
                "config",
                "tokenizer_config",
                "special_tokens_map",
                "vocab",
            ] {
                if !bundle
                    .files
                    .iter()
                    .any(|file| file.logical_name == required_file)
                {
                    return Err(CaptureParserError::ManifestInvalid);
                }
            }
        }
        TINY_LINEAR_RUNTIME_KIND => {
            if bundle.tokenizer.is_some()
                || !bundle
                    .files
                    .iter()
                    .any(|file| file.logical_name == "model_json")
            {
                return Err(CaptureParserError::ManifestInvalid);
            }
            if bundle.parser == "intent"
                && bundle.labels.iter().map(String::as_str).collect::<Vec<_>>()
                    != INTENT_TINY_LINEAR_LABELS
            {
                return Err(CaptureParserError::ManifestInvalid);
            }
        }
        _ => return Err(CaptureParserError::ManifestInvalid),
    }

    Ok(())
}

fn validate_bundle_artifact_id(
    bundle: &CaptureParserArtifactBundle,
) -> Result<(), CaptureParserError> {
    let labels = bundle.labels.iter().map(String::as_str).collect::<Vec<_>>();
    let mut files = bundle.files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| left.logical_name.cmp(&right.logical_name));

    let fingerprint = ArtifactIdFingerprint {
        parser: &bundle.parser,
        runtime_kind: &bundle.runtime_kind,
        tensor_contract_version: &bundle.tensor_contract_version,
        normalization_version: &bundle.normalization_version,
        postprocess_version: &bundle.postprocess_version,
        tokenizer: &bundle.tokenizer,
        labels,
        files,
        decoding: bundle.decoding.as_deref(),
    };

    let payload =
        serde_json::to_vec(&fingerprint).map_err(|_| CaptureParserError::ManifestInvalid)?;
    let digest = sha256_hex(&payload);
    if bundle.artifact_id != format!("sha256:{digest}") {
        return Err(CaptureParserError::ManifestInvalid);
    }
    Ok(())
}

pub fn read_verified_artifact(
    manifest: &CaptureParserArtifactManifest,
    file: &CaptureParserArtifactFile,
) -> Result<Vec<u8>, CaptureParserError> {
    let canonical = manifest
        .resolve_file_path(file)
        .canonicalize()
        .map_err(|_| CaptureParserError::ArtifactMissing)?;
    if !canonical.starts_with(&manifest.root) {
        return Err(CaptureParserError::ManifestInvalid);
    }
    let bytes = fs::read(&canonical).map_err(|_| CaptureParserError::ArtifactMissing)?;
    if bytes.len() as u64 != file.size_bytes || sha256_hex(&bytes) != file.sha256 {
        return Err(CaptureParserError::ArtifactIntegrityMismatch);
    }
    Ok(bytes)
}

fn validate_bundle_files(
    manifest: &CaptureParserArtifactManifest,
    bundle: &CaptureParserArtifactBundle,
) -> Result<(), CaptureParserError> {
    for file in &bundle.files {
        let bytes = read_verified_artifact(manifest, file)?;

        if file.logical_name == "config" {
            validate_config_labels(bundle, &bytes)?;
        }
        if file.logical_name == SEQUENCE_DECODER_LOGICAL_NAME {
            validate_sequence_decoder_sidecar(&bytes, &bundle.labels)
                .map_err(|_| CaptureParserError::ManifestInvalid)?;
        }
    }

    Ok(())
}

fn validate_config_labels(
    bundle: &CaptureParserArtifactBundle,
    bytes: &[u8],
) -> Result<(), CaptureParserError> {
    #[derive(Deserialize)]
    struct ConfigLabelMap {
        #[serde(rename = "id2label")]
        id_to_label: BTreeMap<String, String>,
        #[serde(rename = "label2id")]
        label_to_id: BTreeMap<String, usize>,
    }

    let config: ConfigLabelMap =
        serde_json::from_slice(bytes).map_err(|_| CaptureParserError::ManifestInvalid)?;
    if config.id_to_label.len() != bundle.labels.len()
        || config.label_to_id.len() != bundle.labels.len()
    {
        return Err(CaptureParserError::ManifestInvalid);
    }

    for (index, label) in bundle.labels.iter().enumerate() {
        let index_key = index.to_string();
        if config.id_to_label.get(&index_key) != Some(label)
            || config.label_to_id.get(label) != Some(&index)
        {
            return Err(CaptureParserError::ManifestInvalid);
        }
    }

    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn normalized_manifest_sha256(value: &str) -> Result<String, CaptureParserError> {
    if value.len() != 64 || !value.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(CaptureParserError::ManifestInvalid);
    }
    Ok(value.to_ascii_lowercase())
}
