pub mod config;
pub mod embedding;
pub mod llm;
pub mod traits;

pub use config::InferenceConfig;
pub use embedding::MistralRsEmbedding;
pub use llm::MistralRsLlm;
pub use traits::{EmbeddingBackend, GenerationParams, InferenceError, LlmBackend};

#[cfg(test)]
pub mod test_helpers {
    use async_trait::async_trait;

    use super::traits::{EmbeddingBackend, GenerationParams, InferenceError, LlmBackend};

    pub struct MockLlmBackend {
        pub response: String,
        pub should_fail: bool,
    }

    #[async_trait]
    impl LlmBackend for MockLlmBackend {
        async fn generate(
            &self,
            _system: &str,
            _user: &str,
            _params: &GenerationParams,
        ) -> Result<String, InferenceError> {
            if self.should_fail {
                return Err(InferenceError::InferenceFailed("mock error".into()));
            }
            Ok(self.response.clone())
        }

        async fn warmup(&self) -> Result<(), InferenceError> {
            Ok(())
        }

        fn model_id(&self) -> &str {
            "mock-llm"
        }
    }

    pub struct MockEmbeddingBackend {
        pub embedding: Vec<f32>,
        pub dim: usize,
        pub should_fail: bool,
    }

    #[async_trait]
    impl EmbeddingBackend for MockEmbeddingBackend {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, InferenceError> {
            if self.should_fail || text.trim().is_empty() {
                return Err(InferenceError::InferenceFailed("mock error".into()));
            }
            Ok(self.embedding.clone())
        }

        async fn embed_batch(
            &self,
            texts: &[String],
        ) -> Result<Vec<Vec<f32>>, InferenceError> {
            if self.should_fail {
                return Err(InferenceError::InferenceFailed("mock error".into()));
            }
            let mut results = Vec::new();
            for text in texts {
                if !text.trim().is_empty() {
                    results.push(self.embedding.clone());
                }
            }
            Ok(results)
        }

        async fn warmup(&self) -> Result<(), InferenceError> {
            Ok(())
        }

        fn model_id(&self) -> &str {
            "mock-embedding"
        }

        fn embedding_dim(&self) -> usize {
            self.dim
        }
    }
}
