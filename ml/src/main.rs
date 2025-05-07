use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Import our modules
mod concepts;
mod embeddings;
mod dimensionality;
mod error;

use crate::concepts::ConceptsModel;
use crate::embeddings::EmbeddingModel;
use crate::error::ApiError;

// Represent our application state
struct AppState {
    concepts_model: Arc<ConceptsModel>,
    embedding_model: Arc<EmbeddingModel>,
}

#[derive(Debug, Deserialize)]
struct TextInput {
    text: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
}

// Handler for the /api/vectorize endpoint
async fn process_text(
    data: web::Json<TextInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    info!("Processing text of length: {}", data.text.len());
    
    // Extract concepts
    let concepts = state.concepts_model.generate_concepts(&data.text).await?;
    
    if concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }
    
    // Get concept strings for embeddings
    let concept_strings: Vec<String> = concepts.iter()
        .map(|c| c.concept.clone())
        .collect();
    
    // Generate embeddings
    let embeddings = state.embedding_model.get_batch_embeddings(&concept_strings).await?;
    
    if embeddings.len() != concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }
    
    // Cluster concepts with embeddings
    let clustered_results = dimensionality::cluster_concepts(&concepts, &embeddings)?;
    
    let response = ApiResponse {
        success: true,
        data: clustered_results,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

// Handler for the /api/upload endpoint
async fn upload_file(
    payload: web::Payload,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    use futures::{StreamExt, TryStreamExt};
    use bytes::BytesMut;
    
    // Read the file content
    let mut body = BytesMut::new();
    let mut payload = payload.into_inner();
    
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }
    
    // Convert to text
    let text = std::str::from_utf8(&body)
        .map_err(|_| ApiError::FileDecodeError)?
        .to_string();
    
    info!("Processing uploaded file of length: {}", text.len());
    
    // Extract concepts
    let concepts = state.concepts_model.generate_concepts(&text).await?;
    
    if concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }
    
    // Get concept strings for embeddings
    let concept_strings: Vec<String> = concepts.iter()
        .map(|c| c.concept.clone())
        .collect();
    
    // Generate embeddings
    let embeddings = state.embedding_model.get_batch_embeddings(&concept_strings).await?;
    
    if embeddings.len() != concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }
    
    // Cluster concepts with embeddings
    let clustered_results = dimensionality::cluster_concepts(&concepts, &embeddings)?;
    
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
    if let Err(e) = embedding_model.get_contextual_embeddings("Preloading...").await {
        info!("Error preloading embedding model: {:?}", e);
    }
    info!("Models preloaded successfully");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // Configuration parameters
    let host = "0.0.0.0";
    let port = 8000;
    let ollama_base_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://ollama:11434".to_string());
    
    info!("Starting server at {}:{}", host, port);
    info!("Using Ollama at: {}", ollama_base_url);
    
    // Initialize models
    let concepts_model = Arc::new(ConceptsModel::new(&ollama_base_url));
    let embedding_model = Arc::new(EmbeddingModel::new(&ollama_base_url));
    
    // Preload models
    preload_models(&concepts_model, &embedding_model).await;

    // Application state
    let app_state = web::Data::new(AppState {
        concepts_model,
        embedding_model,
    });
    
    // Start HTTP server
    HttpServer::new(move || {
        // Configure CORS
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