use crate::concepts::Concept;
use crate::embeddings::Embedding;
use crate::error::ApiError;
use cdrs_tokio::cluster::session::SessionBuilder;
use cdrs_tokio::cluster::session::{Session, TcpSessionBuilder};
use cdrs_tokio::cluster::{NodeTcpConfigBuilder, TcpConnectionManager};
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::query_values;
use cdrs_tokio::transport::TransportTcp;
use cdrs_tokio::types::IntoRustByName;
use chrono::Utc;
use futures::future::join_all;
use log::{error, info};
use ndarray::{Array1, ArrayBase, Dim, OwnedRepr};
use uuid::Uuid;

type CurrentSession = Session<
    TransportTcp,
    TcpConnectionManager,
    RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>,
>;

pub struct DatabaseClient {
    session: CurrentSession,
}

impl DatabaseClient {
    pub async fn new(nodes: &[&str]) -> Result<Self, ApiError> {
        let node = nodes[0]; // For simplicity, use first node

        let config = NodeTcpConfigBuilder::new()
            .with_contact_point(cdrs_tokio::cluster::NodeAddress::Hostname(node.to_string()))
            .build()
            .await
            .map_err(|e| ApiError::InternalError(format!("DB connection error: {}", e)))?;

        let session = TcpSessionBuilder::new(RoundRobinLoadBalancingStrategy::new(), config)
            .build()
            .await
            .map_err(|e| ApiError::InternalError(format!("Session build error: {}", e)))?;

        Ok(Self { session })
    }

    pub async fn get_user_concepts(
        &self,
        user_id: &str,
    ) -> Result<Vec<(Concept, Embedding)>, ApiError> {
        let query = "SELECT concept_id, concept_text, embedding_vector FROM store.user_concepts WHERE user_id = ?";

        let uuid = Uuid::parse_str(user_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;

        let rows = self
            .session
            .query_with_values(query, query_values!(uuid))
            .await
            .map_err(|e| ApiError::InternalError(format!("Query error: {}", e)))?
            .response_body()
            .map_err(|e| ApiError::InternalError(format!("Response error: {}", e)))?
            .into_rows()
            .unwrap_or_default();

        let mut results = Vec::new();

        for row in rows {
            // Extract concept text (works with get_r_by_name)
            let concept_text: String = row.get_r_by_name("concept_text").map_err(|e| {
                ApiError::InternalError(format!("Concept text extraction error: {}", e))
            })?;

            // For the embedding vector, we need to use a different strategy
            // Let's try to deserialize it manually using the serde functionality

            // First, get the raw bytes from the column - using CQL binary protocol
            let result: Result<String, _> = row.get_r_by_name("embedding_vector");

            // If this works, try to parse it as a comma-separated string of floats
            let embedding_vec: Vec<f32> = match result {
                Ok(string_vec) => {
                    // Parse as comma-separated values
                    string_vec
                        .split(',')
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .collect()
                }
                Err(_) => {
                    // If string doesn't work, try to parse it as a JSON array
                    let result: Result<String, _> = row.get_r_by_name("embedding_vector");
                    match result {
                        Ok(json_str) => {
                            // Parse JSON array
                            serde_json::from_str::<Vec<f32>>(&json_str).map_err(|e| {
                                ApiError::InternalError(format!("JSON parsing error: {}", e))
                            })?
                        }
                        Err(_) => {
                            // Last resort - let's try an alternative approach
                            // Query the column separately with a different approach
                            let single_query = "SELECT embedding_vector FROM store.user_concepts WHERE user_id = ? AND concept_id = ?";
                            let concept_id: Uuid =
                                row.get_r_by_name("concept_id").map_err(|e| {
                                    ApiError::InternalError(format!(
                                        "Concept ID extraction error: {}",
                                        e
                                    ))
                                })?;

                            // Re-query to get the vector in a raw format
                            let raw_result = self
                                .session
                                .query_with_values(single_query, query_values!(uuid, concept_id))
                                .await
                                .map_err(|e| {
                                    ApiError::InternalError(format!("Second query error: {}", e))
                                })?;

                            // Process to extract the vector based on your actual storage format
                            // This is a placeholder - you'll need to adapt this to how your data is actually stored
                            Vec::new()
                        }
                    }
                }
            };

            let concept = Concept {
                concept: concept_text,
            };
            let embedding: ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>> = Array1::from(embedding_vec);

            results.push((concept, embedding));
        }

        info!("Retrieved {} concepts for user {}", results.len(), user_id);
        Ok(results)
    }

    pub async fn save_concept(
        &self,
        user_id: &str,
        concept: &Concept,
        embedding: &Embedding,
    ) -> Result<(), ApiError> {
        let concept_id = Uuid::new_v4();
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;
        let now = Utc::now();

        // Convert embedding to Vec<f64> for Cassandra compatibility
        let embedding_vec: Vec<f64> = embedding.iter().map(|&x| x as f64).collect();

        // Insert into user_concepts table
        let query = "INSERT INTO store.user_concepts \
                    (user_id, concept_id, concept_text, embedding_vector, created_at) \
                    VALUES (?, ?, ?, ?, ?)";

        self.session
            .query_with_values(
                query,
                query_values!(
                    user_uuid,
                    concept_id,
                    concept.concept.clone(),
                    embedding_vec,
                    now
                ),
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Save concept error: {}", e)))?;

        // Insert source information
        let source_query = "INSERT INTO store.concept_sources \
                           (concept_id, user_id, source_type, source_text, created_at) \
                           VALUES (?, ?, ?, ?, ?)";

        self.session
            .query_with_values(
                source_query,
                query_values!(
                    concept_id,
                    user_uuid,
                    "text_upload",
                    "User uploaded text",
                    now
                ),
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Save source error: {}", e)))?;

        Ok(())
    }

    pub async fn save_concepts_batch(
        &self,
        user_id: &str,
        concepts: &[Concept],
        embeddings: &[Embedding],
    ) -> Result<(), ApiError> {
        if concepts.len() != embeddings.len() {
            return Err(ApiError::InternalError(
                "Concept and embedding count mismatch".to_string(),
            ));
        }

        let mut futures = Vec::new();

        for (concept, embedding) in concepts.iter().zip(embeddings.iter()) {
            let future = self.save_concept(user_id, concept, embedding);
            futures.push(future);
        }

        // Execute all futures concurrently
        let results = join_all(futures).await;

        // Check for errors
        for result in results {
            if let Err(e) = result {
                error!("Error saving concept: {}", e);
                // Continue saving other concepts even if one fails
            }
        }

        Ok(())
    }
}
