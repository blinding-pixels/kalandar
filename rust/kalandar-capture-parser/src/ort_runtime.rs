use crate::{
    CaptureParserArtifactBundle, CaptureParserArtifactManifest, CaptureParserError,
    CaptureParserTensorRequest, InferenceOutput, InferenceSessionDescriptor, RuntimeTensor,
    RuntimeTensorData, ATTENTION_MASK_TENSOR_NAME, BERT_TOKEN_CLASSIFIER_RUNTIME_KIND,
    INPUT_IDS_TENSOR_NAME, LOGITS_OUTPUT_TENSOR_NAME,
};
use ndarray::Array2;
use ort::{inputs, session::Session, value::TensorRef};
use std::collections::BTreeMap;

pub struct CaptureParserOrtSession {
    descriptor: InferenceSessionDescriptor,
    session: Session,
}

impl CaptureParserOrtSession {
    pub fn descriptor(&self) -> &InferenceSessionDescriptor {
        &self.descriptor
    }

    pub fn input_names(&self) -> Vec<String> {
        self.session
            .inputs()
            .iter()
            .map(|input| input.name().to_string())
            .collect()
    }

    pub fn output_names(&self) -> Vec<String> {
        self.session
            .outputs()
            .iter()
            .map(|output| output.name().to_string())
            .collect()
    }

    pub fn run(
        &mut self,
        request: &CaptureParserTensorRequest,
    ) -> Result<InferenceOutput, CaptureParserError> {
        if self.descriptor != request.descriptor {
            return Err(CaptureParserError::ManifestInvalid);
        }

        let input_ids = tensor_to_array2(
            request
                .inputs
                .get(INPUT_IDS_TENSOR_NAME)
                .ok_or(CaptureParserError::ManifestInvalid)?,
        )?;
        let attention_mask = tensor_to_array2(
            request
                .inputs
                .get(ATTENTION_MASK_TENSOR_NAME)
                .ok_or(CaptureParserError::ManifestInvalid)?,
        )?;

        let outputs = self
            .session
            .run(inputs![
                INPUT_IDS_TENSOR_NAME => TensorRef::from_array_view(&input_ids)
                    .map_err(|_| CaptureParserError::InferenceFailed)?,
                ATTENTION_MASK_TENSOR_NAME => TensorRef::from_array_view(&attention_mask)
                    .map_err(|_| CaptureParserError::InferenceFailed)?,
            ])
            .map_err(|_| CaptureParserError::InferenceFailed)?;

        let (shape, data) = outputs
            .get(LOGITS_OUTPUT_TENSOR_NAME)
            .ok_or(CaptureParserError::InferenceFailed)?
            .try_extract_tensor::<f32>()
            .map_err(|_| CaptureParserError::InferenceFailed)?;

        let shape = shape.iter().map(|dim| *dim as usize).collect();
        let data = data.to_vec();

        Ok(InferenceOutput {
            outputs: BTreeMap::from([(
                LOGITS_OUTPUT_TENSOR_NAME.to_string(),
                RuntimeTensor {
                    dtype: "float32",
                    shape,
                    data: RuntimeTensorData::Float32(data),
                },
            )]),
        })
    }
}

pub fn load_ort_session(
    manifest: &CaptureParserArtifactManifest,
    bundle: &CaptureParserArtifactBundle,
) -> Result<CaptureParserOrtSession, CaptureParserError> {
    if bundle.runtime_kind != BERT_TOKEN_CLASSIFIER_RUNTIME_KIND {
        return Err(CaptureParserError::ManifestInvalid);
    }

    let model = bundle
        .files
        .iter()
        .find(|file| file.logical_name == "model")
        .ok_or(CaptureParserError::ManifestInvalid)?;
    let model_path = manifest.resolve_file_path(model);
    let descriptor = InferenceSessionDescriptor {
        parser_family: bundle.parser.clone(),
        artifact_id: bundle.artifact_id.clone(),
        suite_version: manifest.suite_version.clone(),
        tensor_contract_version: bundle
            .tensor_contract_version
            .clone()
            .ok_or(CaptureParserError::ManifestInvalid)?,
    };
    let session = Session::builder()
        .map_err(|_| CaptureParserError::SessionLoadFailed)?
        .commit_from_file(&model_path)
        .map_err(|_| CaptureParserError::SessionLoadFailed)?;

    Ok(CaptureParserOrtSession {
        descriptor,
        session,
    })
}

fn tensor_to_array2(tensor: &RuntimeTensor) -> Result<Array2<i64>, CaptureParserError> {
    if tensor.dtype != "int64" || tensor.shape.len() != 2 {
        return Err(CaptureParserError::ManifestInvalid);
    }
    let RuntimeTensorData::Int64(values) = &tensor.data else {
        return Err(CaptureParserError::ManifestInvalid);
    };
    Array2::from_shape_vec((tensor.shape[0], tensor.shape[1]), values.clone())
        .map_err(|_| CaptureParserError::ManifestInvalid)
}
