use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Model loading failed: {0}")]
    ModelLoadError(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Output parsing failed: {0}")]
    OutputParsingError(String),
    #[error("Model not ready: {0}")]
    NotReady(String),
    #[error("GPU/device error: {0}")]
    DeviceError(String),
}

#[derive(Debug, Clone)]
pub struct GenerationParams {
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub json_schema: Option<serde_json::Value>,
    pub frequency_penalty: Option<f32>,
    pub dry_multiplier: Option<f32>,
}

#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        params: &GenerationParams,
    ) -> Result<String, InferenceError>;

    async fn warmup(&self) -> Result<(), InferenceError>;

    fn model_id(&self) -> &str;
}

#[async_trait]
pub trait EmbeddingBackend: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, InferenceError>;

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, InferenceError>;

    async fn warmup(&self) -> Result<(), InferenceError>;

    fn model_id(&self) -> &str;

    fn embedding_dim(&self) -> usize;
}
