use crate::error::ApiError;
use crate::models::inference::EmbeddingBackend;
use log::info;
use ndarray::Array1;
use std::sync::Arc;

pub type Embedding = Array1<f32>;

pub struct EmbeddingModel {
    backend: Arc<dyn EmbeddingBackend>,
}

impl EmbeddingModel {
    pub fn new(backend: Arc<dyn EmbeddingBackend>) -> Self {
        Self { backend }
    }

    pub async fn get_batch_embeddings(&self, texts: &[String]) -> Result<Vec<Embedding>, ApiError> {
        let valid_texts: Vec<String> = texts
            .iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        info!(
            "Generating embeddings for {} concepts",
            valid_texts.len()
        );

        let embeddings = self.backend.embed_batch(&valid_texts).await?;

        Ok(embeddings
            .into_iter()
            .map(|v| Array1::from(v))
            .collect())
    }

    pub async fn get_contextual_embeddings(&self, text: &str) -> Result<Embedding, ApiError> {
        if text.trim().is_empty() {
            return Err(ApiError::InternalError("Empty text provided".to_string()));
        }

        let embedding = self.backend.embed(text).await?;
        Ok(Array1::from(embedding))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::inference::test_helpers::MockEmbeddingBackend;

    fn make_model(dim: usize) -> EmbeddingModel {
        EmbeddingModel::new(Arc::new(MockEmbeddingBackend {
            embedding: vec![0.1; dim],
            dim,
            should_fail: false,
        }))
    }

    fn make_failing_model() -> EmbeddingModel {
        EmbeddingModel::new(Arc::new(MockEmbeddingBackend {
            embedding: vec![],
            dim: 0,
            should_fail: true,
        }))
    }

    #[tokio::test]
    async fn test_get_contextual_embeddings_success() {
        let model = make_model(128);
        let result = model.get_contextual_embeddings("test text").await;
        assert!(result.is_ok());
        let embedding = result.unwrap();
        assert_eq!(embedding.len(), 128);
    }

    #[tokio::test]
    async fn test_get_contextual_embeddings_empty_text() {
        let model = make_model(128);
        let result = model.get_contextual_embeddings("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_batch_embeddings_success() {
        let model = make_model(128);
        let texts = vec![
            "first text".to_string(),
            "second text".to_string(),
            "third text".to_string(),
        ];
        let result = model.get_batch_embeddings(&texts).await;
        assert!(result.is_ok());
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 3);
        assert!(embeddings.iter().all(|e| e.len() == 128));
    }

    #[tokio::test]
    async fn test_get_batch_embeddings_filters_empty() {
        let model = make_model(128);
        let texts = vec![
            "valid text".to_string(),
            "".to_string(),
            "  ".to_string(),
            "another valid".to_string(),
        ];
        let result = model.get_batch_embeddings(&texts).await;
        assert!(result.is_ok());
        let embeddings = result.unwrap();
        // Empty and whitespace-only strings should be filtered out
        assert_eq!(embeddings.len(), 2);
    }

    #[tokio::test]
    async fn test_get_batch_embeddings_backend_failure() {
        let model = make_failing_model();
        let texts = vec!["text".to_string()];
        let result = model.get_batch_embeddings(&texts).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_embedding_dimension_consistency() {
        let model = make_model(256);
        let texts = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let result = model.get_batch_embeddings(&texts).await;
        assert!(result.is_ok());
        let embeddings = result.unwrap();
        // All embeddings should have the same dimension
        let dim = embeddings[0].len();
        assert!(embeddings.iter().all(|e| e.len() == dim));
        assert_eq!(dim, 256);
    }
}
