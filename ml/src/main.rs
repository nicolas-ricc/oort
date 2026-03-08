use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use log::info;
use std::sync::Arc;

mod controllers;
mod models;
mod data;
mod dimensionality;
mod error;

use crate::controllers::text_processing::{process_text, get_texts_by_concept, save_scene, get_scene, AppState};
use crate::models::concepts::ConceptsModel;
use crate::models::embeddings::EmbeddingModel;
use crate::models::inference::{InferenceConfig, MistralRsLlm, MistralRsEmbedding};
use crate::data::client::DatabaseClient;
use crate::data::scraper::ArticleScraper;

async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

async fn preload_models(concepts_model: &ConceptsModel, embedding_model: &EmbeddingModel) {
    info!("Preloading models...");
    if let Err(e) = concepts_model.generate_concepts("Preloading...", &[]).await {
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

    eprintln!("=== oort-ml starting ===");

    let host = "0.0.0.0";
    let port = 8000;
    let db_nodes_str = std::env::var("DB_NODES").unwrap_or_else(|_| "oort-db:9042".to_string());

    info!("Starting server at {}:{}", host, port);
    info!("Using Database nodes: {}", db_nodes_str);

    // Load inference backends
    let config = InferenceConfig::from_env();
    info!("Loading LLM model: {} ({:?})", config.llm_model, config.llm_gguf_files);
    info!("Loading embedding model: {}", config.embedding_model);

    // Load embedding model FIRST so the LLM's Utilization-based KV cache sizing
    // accounts for the embedding model's ~0.6GB already on GPU.
    let embedding_backend = match MistralRsEmbedding::new(&config).await {
        Ok(backend) => {
            info!("Embedding model loaded successfully");
            Arc::new(backend)
        }
        Err(e) => {
            eprintln!("FATAL: Failed to load embedding model: {}", e);
            std::process::exit(1);
        }
    };

    let llm_backend = match MistralRsLlm::new(&config).await {
        Ok(backend) => {
            info!("LLM model loaded successfully");
            Arc::new(backend)
        }
        Err(e) => {
            eprintln!("FATAL: Failed to load LLM model: {}", e);
            std::process::exit(1);
        }
    };

    let concepts_model = Arc::new(ConceptsModel::new(
        llm_backend,
        Arc::clone(&embedding_backend) as Arc<dyn crate::models::inference::EmbeddingBackend>,
        config.llm_enrichment,
    ));
    let embedding_model = Arc::new(EmbeddingModel::new(embedding_backend));

    let db_nodes: Vec<&str> = db_nodes_str.split(',').collect();
    let db_client = Arc::new(
        DatabaseClient::new(&db_nodes)
            .await
            .expect("Failed to connect to database"),
    );

    let scraper = Arc::new(ArticleScraper::new());

    preload_models(&concepts_model, &embedding_model).await;

    let app_state = web::Data::new(AppState {
        concepts_model,
        embedding_model,
        db_client,
        scraper,
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
            .route("/api/health", web::get().to(health))
            .route("/api/vectorize", web::post().to(process_text))
            .route("/api/texts-by-concept", web::get().to(get_texts_by_concept))
            .route("/api/scenes", web::post().to(save_scene))
            .route("/api/scenes/{scene_id}", web::get().to(get_scene))
    })
    .bind((host, port))?
    .run()
    .await
}
