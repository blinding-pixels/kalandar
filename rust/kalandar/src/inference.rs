use crate::{
    CaptureParserArtifactBundle, CaptureParserArtifactManifest, CaptureParserError,
    TokenizedCaptureInput, BERT_TOKEN_CLASSIFIER_RUNTIME_KIND,
};
use std::collections::BTreeMap;

pub const INPUT_IDS_TENSOR_NAME: &str = "input_ids";
pub const ATTENTION_MASK_TENSOR_NAME: &str = "attention_mask";
pub const LOGITS_OUTPUT_TENSOR_NAME: &str = "logits";

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeTensorData {
    Int64(Vec<i64>),
    Float32(Vec<f32>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTensor {
    pub dtype: &'static str,
    pub shape: Vec<usize>,
    pub data: RuntimeTensorData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferenceOutput {
    pub outputs: BTreeMap<String, RuntimeTensor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceSessionDescriptor {
    pub parser_family: String,
    pub artifact_id: String,
    pub suite_version: String,
    pub tensor_contract_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureParserTensorRequest {
    pub descriptor: InferenceSessionDescriptor,
    pub inputs: BTreeMap<String, RuntimeTensor>,
    pub outputs: Vec<String>,
}

pub trait CaptureParserInferenceAdapter {
    fn kind(&self) -> &'static str;
    fn run(
        &self,
        request: &CaptureParserTensorRequest,
    ) -> Result<InferenceOutput, CaptureParserError>;
}

pub fn build_token_classification_request(
    manifest: &CaptureParserArtifactManifest,
    bundle: &CaptureParserArtifactBundle,
    tokenized: &TokenizedCaptureInput,
) -> Result<CaptureParserTensorRequest, CaptureParserError> {
    if bundle.runtime_kind != BERT_TOKEN_CLASSIFIER_RUNTIME_KIND {
        return Err(CaptureParserError::ManifestInvalid);
    }

    let tensor_contract_version = bundle
        .tensor_contract_version
        .clone()
        .ok_or(CaptureParserError::ManifestInvalid)?;
    let sequence_length = tokenized.input_ids.len();
    if sequence_length == 0 || sequence_length != tokenized.attention_mask.len() {
        return Err(CaptureParserError::ManifestInvalid);
    }

    let mut inputs = BTreeMap::new();
    inputs.insert(
        INPUT_IDS_TENSOR_NAME.to_string(),
        RuntimeTensor {
            dtype: "int64",
            shape: vec![1, sequence_length],
            data: RuntimeTensorData::Int64(tokenized.input_ids.clone()),
        },
    );
    inputs.insert(
        ATTENTION_MASK_TENSOR_NAME.to_string(),
        RuntimeTensor {
            dtype: "int64",
            shape: vec![1, sequence_length],
            data: RuntimeTensorData::Int64(tokenized.attention_mask.clone()),
        },
    );

    Ok(CaptureParserTensorRequest {
        descriptor: InferenceSessionDescriptor {
            parser_family: bundle.parser.clone(),
            artifact_id: bundle.artifact_id.clone(),
            suite_version: manifest.suite_version.clone(),
            tensor_contract_version,
        },
        inputs,
        outputs: vec![LOGITS_OUTPUT_TENSOR_NAME.to_string()],
    })
}
