use crate::models::concepts::Concept;
use crate::models::embeddings::Embedding;
use crate::error::ApiError;
use cdrs_tokio::cluster::session::SessionBuilder;
use cdrs_tokio::cluster::session::{Session, TcpSessionBuilder};
use cdrs_tokio::cluster::{NodeTcpConfigBuilder, TcpConnectionManager};
use cdrs_tokio::load_balancing::RoundRobinLoadBalancingStrategy;
use cdrs_tokio::query_values;
use cdrs_tokio::transport::TransportTcp;
use cdrs_tokio::types::IntoRustByName;
use chrono::{DateTime, Utc};
use log::{error, info};
use ndarray::{Array1, ArrayBase, Dim, OwnedRepr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

type CurrentSession = Session<
    TransportTcp,
    TcpConnectionManager,
    RoundRobinLoadBalancingStrategy<TransportTcp, TcpConnectionManager>,
>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TextReference {
    pub text_id: Uuid,
    pub user_id: Uuid,
    pub filename: String,
    pub url: String,
    pub concepts: Vec<String>,
    pub upload_timestamp: DateTime<Utc>,
    pub file_size: Option<i32>,
}

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

        info!("Processing {} rows from database", rows.len());

        for (row_index, row) in rows.iter().enumerate() {
            // Extract concept text (works with get_r_by_name)
            let concept_text: String = row.get_r_by_name("concept_text").map_err(|e| {
                ApiError::InternalError(format!("Concept text extraction error for row {}: {}", row_index, e))
            })?;

            info!("Processing concept {} (row {}): '{}'", row_index, row_index, concept_text);

            // For the embedding vector, we need to use a different strategy
            // Let's try to deserialize it manually using the serde functionality

            // First, get the raw bytes from the column - using CQL binary protocol
            let result: Result<String, _> = row.get_r_by_name("embedding_vector");

            // If this works, try to parse it as a comma-separated string of floats
            let embedding_vec: Vec<f32> = match result {
                Ok(string_vec) => {
                    // Parse as comma-separated values
                    let parsed_vec: Vec<f32> = string_vec
                        .split(',')
                        .filter_map(|s| s.trim().parse::<f32>().ok())
                        .collect();
                    
                    // Check if parsing resulted in empty vector
                    if parsed_vec.is_empty() {
                        log::error!("Embedding parsing resulted in empty vector for concept '{}', skipping", concept_text);
                        continue;
                    }
                    parsed_vec
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
                            let concept_id: Uuid =
                                row.get_r_by_name("concept_id").map_err(|e| {
                                    ApiError::InternalError(format!(
                                        "Concept ID extraction error: {}",
                                        e
                                    ))
                                })?;

                            log::error!("Corrupted embedding data for concept '{}' (ID: {}), skipping this concept", 
                                       concept_text, concept_id);
                            
                            // Skip this concept rather than failing the entire request
                            continue;
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
        // Validate embedding is not empty
        if embedding.is_empty() {
            return Err(ApiError::InternalError(format!(
                "Cannot save concept '{}' with zero-dimensional embedding", 
                concept.concept
            )));
        }

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

    pub async fn save_text_reference(
        &self,
        user_id: &str,
        filename: &str,
        url: &str,
        concepts: &[String],
        file_size: Option<i32>,
    ) -> Result<Uuid, ApiError> {
        let text_id = Uuid::new_v4();
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;
        let now = Utc::now();
        
        let concepts_vec: Vec<String> = concepts.iter().cloned().collect();

        // Insert into text_references table
        let query = "INSERT INTO store.text_references \
                    (text_id, user_id, filename, url, concepts, upload_timestamp, file_size) \
                    VALUES (?, ?, ?, ?, ?, ?, ?)";

        self.session
            .query_with_values(
                query,
                query_values!(
                    text_id,
                    user_uuid,
                    filename,
                    url,
                    concepts_vec,
                    now,
                    file_size
                ),
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Save text reference error: {}", e)))?;

        // Insert into concept_text_mapping table for each concept
        for concept in concepts {
            let mapping_query = "INSERT INTO store.concept_text_mapping \
                               (concept_text, user_id, text_id, filename, url, upload_timestamp) \
                               VALUES (?, ?, ?, ?, ?, ?)";

            self.session
                .query_with_values(
                    mapping_query,
                    query_values!(
                        concept.clone(),
                        user_uuid,
                        text_id,
                        filename,
                        url,
                        now
                    ),
                )
                .await
                .map_err(|e| ApiError::InternalError(format!("Save concept mapping error: {}", e)))?;
        }

        Ok(text_id)
    }

    pub async fn get_texts_by_concept(
        &self,
        user_id: &str,
        concept: &str,
    ) -> Result<Vec<TextReference>, ApiError> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {}", e)))?;

        let query = "SELECT text_id, filename, url, upload_timestamp \
                    FROM store.concept_text_mapping \
                    WHERE concept_text = ? AND user_id = ?";

        let rows = self
            .session
            .query_with_values(query, query_values!(concept, user_uuid))
            .await
            .map_err(|e| ApiError::InternalError(format!("Query error: {}", e)))?
            .response_body()
            .map_err(|e| ApiError::InternalError(format!("Response error: {}", e)))?
            .into_rows()
            .unwrap_or_default();

        let mut results = Vec::new();

        for row in rows.iter() {
            let text_id: Uuid = row.get_r_by_name("text_id").map_err(|e| {
                ApiError::InternalError(format!("Text ID extraction error: {}", e))
            })?;

            let filename: String = row.get_r_by_name("filename").map_err(|e| {
                ApiError::InternalError(format!("Filename extraction error: {}", e))
            })?;

            let url: String = row.get_r_by_name("url").map_err(|e| {
                ApiError::InternalError(format!("URL extraction error: {}", e))
            })?;

            let upload_timestamp: DateTime<Utc> = row.get_r_by_name("upload_timestamp").map_err(|e| {
                ApiError::InternalError(format!("Timestamp extraction error: {}", e))
            })?;

            // For now, we'll just include the queried concept in the concepts list
            // In a full implementation, you might want to fetch all concepts for this text
            let concepts = vec![concept.to_string()];

            results.push(TextReference {
                text_id,
                user_id: user_uuid,
                filename,
                url,
                concepts,
                upload_timestamp,
                file_size: None, // Not stored in mapping table
            });
        }

        Ok(results)
    }

}
