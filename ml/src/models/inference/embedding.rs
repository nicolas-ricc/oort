use async_trait::async_trait;
use log::info;
use mistralrs::{EmbeddingModelBuilder, EmbeddingRequest, IsqType, Model};

use super::config::InferenceConfig;
use super::traits::{EmbeddingBackend, InferenceError};

pub struct MistralRsEmbedding {
    model: Model,
    model_id: String,
    dim: usize,
}

impl MistralRsEmbedding {
    pub async fn new(config: &InferenceConfig) -> Result<Self, InferenceError> {
        let mut builder = EmbeddingModelBuilder::new(&config.embedding_model)
            .with_logging()
            .with_isq(IsqType::Q8_0);

        if !config.use_gpu || config.embedding_force_cpu {
            builder = builder.with_force_cpu();
        }

        let model = builder
            .build()
            .await
            .map_err(|e| InferenceError::ModelLoadError(e.to_string()))?;

        // Probe embedding dimension with a test input
        let probe = model
            .generate_embedding("probe")
            .await
            .map_err(|e| InferenceError::ModelLoadError(
                format!("Failed to probe embedding dimension: {}", e),
            ))?;
        let dim = probe.len();
        info!("Embedding model loaded: {} (dim={})", config.embedding_model, dim);

        Ok(Self {
            model,
            model_id: config.embedding_model.clone(),
            dim,
        })
    }
}

#[async_trait]
impl EmbeddingBackend for MistralRsEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, InferenceError> {
        if text.trim().is_empty() {
            return Err(InferenceError::InferenceFailed(
                "Empty text provided".into(),
            ));
        }

        self.model
            .generate_embedding(text)
            .await
            .map_err(|e| InferenceError::InferenceFailed(e.to_string()))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, InferenceError> {
        let valid_texts: Vec<&str> = texts
            .iter()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();

        if valid_texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut request = EmbeddingRequest::builder();
        for text in &valid_texts {
            request = request.add_prompt(*text);
        }

        self.model
            .generate_embeddings(request)
            .await
            .map_err(|e| InferenceError::InferenceFailed(e.to_string()))
    }

    async fn warmup(&self) -> Result<(), InferenceError> {
        info!("Warming up embedding model: {}", self.model_id);
        self.embed("warmup")
            .await
            .map(|_| ())
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}
