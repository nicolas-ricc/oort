use actix_web::{web, HttpResponse, Responder};
use bytes::BytesMut;
use futures::StreamExt;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::data::client::DatabaseClient;
use crate::dimensionality;
use crate::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct TextInput {
    pub text: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
}

pub struct AppState {
    pub concepts_model: Arc<crate::models::concepts::ConceptsModel>,
    pub embedding_model: Arc<crate::models::embeddings::EmbeddingModel>,
    pub db_client: Arc<DatabaseClient>,
}

pub async fn process_concepts_and_embeddings(
    text: &str,
    user_id: Option<&str>,
    state: &web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    info!("Processing text of length: {}", text.len());

    let new_concepts = state.concepts_model.generate_concepts(text).await?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts = new_concepts.clone();
    let mut existing_embeddings = Vec::new();

    let uuid_str = if let Some(uid) = user_id {
        info!("Loading existing concepts for user: {}", uid);

        let namespace = Uuid::NAMESPACE_DNS;

        let user_uuid = Uuid::new_v5(&namespace, uid.as_bytes());
        let uuid_str = user_uuid.to_string();

        let user_concepts = state.db_client.get_user_concepts(&uuid_str).await?;

        for (concept, embedding) in user_concepts {
            all_concepts.push(concept);
            existing_embeddings.push(embedding);
        }

        Some(uuid_str)
    } else {
        None
    };

    let concept_strings: Vec<String> = all_concepts.iter().map(|c| c.concept.clone()).collect();
    let new_concept_strings: Vec<String> = new_concepts.iter().map(|c| c.concept.clone()).collect();

    let new_embeddings = state
        .embedding_model
        .get_batch_embeddings(&new_concept_strings)
        .await?;

    if new_embeddings.len() != new_concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }

    if let Some(uuid_str) = uuid_str {
        let db_client = Arc::clone(&state.db_client);
        let user_id_owned = uuid_str;
        let new_concepts_clone = new_concepts.clone();
        let new_embeddings_clone = new_embeddings.clone();

        tokio::spawn(async move {
            for (concept, embedding) in new_concepts_clone.iter().zip(new_embeddings_clone.iter()) {
                if let Err(e) = db_client
                    .save_concept(&user_id_owned, concept, embedding)
                    .await
                {
                    error!("Failed to save concept: {:?}", e);
                }
            }
        });
    }

    let mut all_embeddings = new_embeddings;
    all_embeddings.extend(existing_embeddings);

    let clustered_results = dimensionality::cluster_concepts(&all_concepts, &all_embeddings)?;

    let response = ApiResponse {
        success: true,
        data: clustered_results,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn process_text(
    data: web::Json<TextInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    process_concepts_and_embeddings(&data.text, data.user_id.as_deref(), &state).await
}

pub async fn upload_file(
    payload: web::Payload,
    query: web::Query<TextInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let mut body: BytesMut = BytesMut::new();
    let mut payload: actix_web::dev::Payload = payload.into_inner();

    while let Some(chunk) = payload.next().await {
        let chunk: web::Bytes = chunk?;
        body.extend_from_slice(&chunk);
    }

    let text: String = std::str::from_utf8(&body)
        .map_err(|_| ApiError::FileDecodeError)?
        .to_string();

    info!("Processing uploaded file of length: {}", text.len());

    process_concepts_and_embeddings(&text, query.user_id.as_deref(), &state).await
}
