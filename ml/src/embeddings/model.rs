use crate::error::ApiError;
use log::{debug, info};
use ndarray::{Array1, ArrayBase, Dim, OwnedRepr};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub type Embedding = Array1<f32>;

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

pub struct EmbeddingModel {
    base_url: String,
    client: Client,
    model_name: String,
}

impl EmbeddingModel {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
            model_name: "snowflake-arctic-embed2".to_string(),
        }
    }

    pub async fn get_batch_embeddings(&self, texts: &[String]) -> Result<Vec<Embedding>, ApiError> {
        let mut embeddings = Vec::new();

        for text in texts {
            let text = text.trim();
            if !text.is_empty() {
                debug!(
                    "Processing: '{}'",
                    &text.chars().take(50).collect::<String>()
                );

                let embedding: ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>> = self.get_contextual_embeddings(text).await?;
                embeddings.push(embedding);
            }
        }

        Ok(embeddings)
    }

    pub async fn get_contextual_embeddings(&self, text: &str) -> Result<Embedding, ApiError> {
        if text.is_empty() {
            return Err(ApiError::InternalError("Empty text provided".to_string()));
        }

        let request: EmbeddingRequest = EmbeddingRequest {
            model: self.model_name.clone(),
            prompt: text.to_string(),
        };

        let url: String = format!("{}/api/embeddings", self.base_url);
        debug!("Sending request to: {}", url);

        let response: reqwest::Response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                info!("Error requesting embeddings: {}", e);
                ApiError::RequestError(e)
            })?;

        if !response.status().is_success() {
            let status: reqwest::StatusCode = response.status();
            let body: String = response.text().await.unwrap_or_default();
            return Err(ApiError::InternalError(format!(
                "Error {}: {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response.json().await.map_err(|e| {
            info!("Error parsing embedding response: {}", e);
            ApiError::RequestError(e)
        })?;

        let embedding: ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>> =
            Array1::from(embedding_response.embedding);
        Ok(embedding)
    }

    /*     pub fn get_similarity(&self, embedding1: ArrayView1<f32>, embedding2: ArrayView1<f32>) -> f32 {
        // Use BLAS-accelerated operations
        let norm1 = embedding1.norm_l2();
        let norm2 = embedding2.norm_l2();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        // Calculate cosine similarity using BLAS dot product
        embedding1.dot(&embedding2) / (norm1 * norm2)
    }

    pub fn find_most_similar(
        &self,
        query_embedding: ArrayView1<f32>,
        comparison_embeddings: &[Embedding],
        top_k: usize,
    ) -> Vec<(usize, f32)> {
        let mut similarities = Vec::with_capacity(comparison_embeddings.len());

        for (i, emb) in comparison_embeddings.iter().enumerate() {
            let similarity = self.get_similarity(query_embedding, emb.view());
            similarities.push((i, similarity));
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        similarities.truncate(top_k);
        similarities
    }*/
}
