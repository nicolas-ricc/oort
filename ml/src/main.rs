use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod concepts;
mod data;
mod dimensionality;
mod embeddings;
mod error;

use crate::concepts::ConceptsModel;
use crate::data::client::DatabaseClient;
use crate::embeddings::EmbeddingModel;
use crate::error::ApiError;

struct AppState {
    concepts_model: Arc<ConceptsModel>,
    embedding_model: Arc<EmbeddingModel>,
    db_client: Arc<DatabaseClient>,
}

#[derive(Debug, Deserialize)]
struct TextInput {
    text: String,
    user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}

async fn process_text(
    data: web::Json<TextInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    info!("Processing text of length: {}", data.text.len());

    // Extract concepts from the new text
    let new_concepts = state.concepts_model.generate_concepts(&data.text).await?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts = new_concepts.clone();
    let mut existing_embeddings = Vec::new();

    // If user_id is provided, get existing concepts from database
    if let Some(user_id) = &data.user_id {
        info!("Loading existing concepts for user: {}", user_id);

        let user_concepts = state.db_client.get_user_concepts(user_id).await?;

        // Combine existing concepts with new ones
        for (concept, embedding) in user_concepts {
            all_concepts.push(concept);
            existing_embeddings.push(embedding);
        }
    }

    // Get concept strings for embeddings
    let concept_strings: Vec<String> = all_concepts.iter().map(|c| c.concept.clone()).collect();

    // Generate embeddings for new concepts only
    let new_concept_strings: Vec<String> = new_concepts.iter().map(|c| c.concept.clone()).collect();

    let new_embeddings = state
        .embedding_model
        .get_batch_embeddings(&new_concept_strings)
        .await?;

    if new_embeddings.len() != new_concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }

    // Save new concepts to database asynchronously
    if let Some(user_id) = &data.user_id {
        let db_client = Arc::clone(&state.db_client);
        let user_id = user_id.clone();
        let new_concepts_clone = new_concepts.clone();
        let new_embeddings_clone = new_embeddings.clone();

        // Spawn a task to save concepts without waiting for completion
        tokio::spawn(async move {
            for (concept, embedding) in new_concepts_clone.iter().zip(new_embeddings_clone.iter()) {
                if let Err(e) = db_client.save_concept(&user_id, concept, embedding).await {
                    error!("Failed to save concept: {:?}", e);
                }
            }
        });
    }

    // Combine new embeddings with existing ones
    let mut all_embeddings = new_embeddings;
    all_embeddings.extend(existing_embeddings);

    // Cluster concepts with embeddings
    let clustered_results = dimensionality::cluster_concepts(&all_concepts, &all_embeddings)?;

    let response = ApiResponse {
        success: true,
        data: clustered_results,
    };

    Ok(HttpResponse::Ok().json(response))
}

async fn upload_file(
    payload: web::Payload,
    query: web::Query<TextInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    use bytes::BytesMut;
    use futures::StreamExt;

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

    let new_concepts = state.concepts_model.generate_concepts(&text).await?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts: Vec<concepts::Concept> = new_concepts.clone();
    let mut existing_embeddings: Vec<ndarray::ArrayBase<ndarray::OwnedRepr<f32>, ndarray::Dim<[usize; 1]>>> = Vec::new();

    if let Some(user_id) = &query.user_id {
        info!("Loading existing concepts for user: {}", user_id);

        let user_concepts = state.db_client.get_user_concepts(user_id).await?;

        for (concept, embedding) in user_concepts {
            all_concepts.push(concept);
            existing_embeddings.push(embedding);
        }
    }

    let concept_strings: Vec<String> = all_concepts.iter().map(|c| c.concept.clone()).collect();

    let new_concept_strings: Vec<String> = new_concepts.iter().map(|c| c.concept.clone()).collect();

    let new_embeddings = state
        .embedding_model
        .get_batch_embeddings(&new_concept_strings)
        .await?;

    if new_embeddings.len() != new_concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }

    if let Some(user_id) = &query.user_id {
        let db_client = Arc::clone(&state.db_client);
        let user_id = user_id.clone();
        let new_concepts_clone = new_concepts.clone();
        let new_embeddings_clone = new_embeddings.clone();

        tokio::spawn(async move {
            for (concept, embedding) in new_concepts_clone.iter().zip(new_embeddings_clone.iter()) {
                if let Err(e) = db_client.save_concept(&user_id, concept, embedding).await {
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

async fn preload_models(concepts_model: &ConceptsModel, embedding_model: &EmbeddingModel) {
    info!("Preloading models...");
    if let Err(e) = concepts_model.generate_concepts("Preloading...").await {
        info!("Error preloading concepts model: {:?}", e);
    }
    if let Err(e) = embedding_model
        .get_contextual_embeddings("Preloading...")
        .await
    {
        info!("Error preloading embedding model: {:?}", e);
    }
    info!("Models preloaded successfully");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let host = "0.0.0.0";
    let port = 8000;
    let ollama_base_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://ollama:11434".to_string());
    let db_nodes = std::env::var("DB_NODES").unwrap_or_else(|_| "oort-db:9042".to_string());

    info!("Starting server at {}:{}", host, port);
    info!("Using Ollama at: {}", ollama_base_url);
    info!("Using Database nodes: {}", db_nodes);

    let concepts_model = Arc::new(ConceptsModel::new(&ollama_base_url));
    let embedding_model = Arc::new(EmbeddingModel::new(&ollama_base_url));

    let db_nodes: Vec<&str> = db_nodes.split(',').collect();
    let db_client = Arc::new(
        DatabaseClient::new(&db_nodes)
            .await
            .expect("Failed to connect to database"),
    );

    preload_models(&concepts_model, &embedding_model).await;

    let app_state = web::Data::new(AppState {
        concepts_model,
        embedding_model,
        db_client,
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/vectorize", web::post().to(process_text))
            .route("/api/upload", web::post().to(upload_file))
    })
    .bind((host, port))?
    .run()
    .await
}
