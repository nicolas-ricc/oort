use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use log::info;
use std::sync::Arc;

mod controllers;
mod models;
mod data;
mod dimensionality;
mod error;

use crate::controllers::text_processing::{process_text, get_texts_by_concept, AppState};
use crate::models::concepts::ConceptsModel;
use crate::models::embeddings::EmbeddingModel;
use crate::data::client::DatabaseClient;

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
            .route("/api/texts-by-concept", web::get().to(get_texts_by_concept))
    })
    .bind((host, port))?
    .run()
    .await
}
