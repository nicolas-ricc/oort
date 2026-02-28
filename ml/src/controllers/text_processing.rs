use actix_web::{web, HttpResponse, Responder};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::data::cdn::github::GitHubCDN;
use crate::data::client::{DatabaseClient, TextReference};
use crate::data::scraper::{ArticleScraper, derive_filename};
use crate::dimensionality::{self, ConceptGroup};
use crate::error::ApiError;
use crate::models::concepts::KeywordExtractor;

#[derive(Debug, Deserialize)]
pub struct TextInput {
    pub text: Option<String>,
    pub url: Option<String>,
    pub user_id: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConceptQuery {
    pub concept: String,
    pub user_id: String,
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
    pub scraper: Arc<ArticleScraper>,
}

pub async fn process_concepts_and_embeddings(
    text: &str,
    user_id: Option<&str>,
    state: &web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    info!("Processing text of length: {}", text.len());

    let extractor = KeywordExtractor::new();
    let nlp_candidates = extractor.extract_candidates(text, 20);

    // Compute user UUID upfront so DB query can run in parallel with LLM
    let uuid_str = user_id.map(|uid| {
        let user_uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, uid.as_bytes());
        user_uuid.to_string()
    });

    // Run LLM concept extraction and DB load concurrently
    let concepts_future = state.concepts_model.generate_concepts(text, &nlp_candidates);
    let db_future = async {
        if let Some(ref uuid_str) = uuid_str {
            info!("Loading existing concepts for user: {}", uuid_str);
            state.db_client.get_user_concepts(uuid_str).await
        } else {
            Ok(Vec::new())
        }
    };

    let (new_concepts, user_concepts) = tokio::try_join!(concepts_future, db_future)?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts = new_concepts.clone();
    let mut existing_embeddings = Vec::new();

    for (concept, embedding) in user_concepts {
        all_concepts.push(concept);
        existing_embeddings.push(embedding);
    }

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

    let mut mind_map = dimensionality::MindMapProcessor::new(None);
    let clustered_results = mind_map.process_concepts(&all_concepts, &all_embeddings)?;

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
    // Resolve text content: either from direct text or by scraping a URL
    let (text, filename, source_url) = match (&data.text, &data.url) {
        (Some(text), None) => {
            let filename = data.filename.clone().unwrap_or_else(|| {
                format!("processed_text_{}.txt", Uuid::new_v4())
            });
            (text.clone(), filename, None)
        }
        (None, Some(url)) => {
            let article = state.scraper.scrape_url(url).await?;
            let filename = data.filename.clone().unwrap_or_else(|| {
                derive_filename(&article.title, url)
            });
            (article.text_content, filename, Some(url.clone()))
        }
        (Some(_), Some(_)) => {
            return Err(ApiError::InternalError(
                "Provide either 'text' or 'url', not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(ApiError::InternalError(
                "Provide either 'text' or 'url'".to_string(),
            ));
        }
    };

    let extractor = KeywordExtractor::new();
    let nlp_candidates = extractor.extract_candidates(&text, 20);

    // Compute user UUID upfront so DB query can run in parallel with LLM
    let uuid_str = data.user_id.as_deref().map(|user_id| {
        if user_id == "default" {
            "550e8400-e29b-41d4-a716-446655440000".to_string()
        } else {
            user_id.to_string()
        }
    });

    // Run LLM concept extraction and DB load concurrently
    let concepts_future = state.concepts_model.generate_concepts(&text, &nlp_candidates);
    let db_future = async {
        if let Some(ref uuid_str) = uuid_str {
            info!("Loading existing concepts for user: {}", uuid_str);
            state.db_client.get_user_concepts(uuid_str).await
        } else {
            Ok(Vec::new())
        }
    };

    let (new_concepts, user_concepts) = tokio::try_join!(concepts_future, db_future)?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts = new_concepts.clone();
    let mut existing_embeddings = Vec::new();

    for (concept, embedding) in user_concepts {
        all_concepts.push(concept);
        existing_embeddings.push(embedding);
    }

    let new_concept_strings: Vec<String> = new_concepts.iter().map(|c| c.concept.clone()).collect();

    let new_embeddings = state
        .embedding_model
        .get_batch_embeddings(&new_concept_strings)
        .await?;

    if new_embeddings.len() != new_concepts.len() {
        return Err(ApiError::EmbeddingGenerationError);
    }

    if let Some(uuid_str) = &uuid_str {
        let db_client = Arc::clone(&state.db_client);
        let user_id_owned = uuid_str.clone();
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

    let mut mind_map = dimensionality::MindMapProcessor::new(None);
    let clustered_results = mind_map.process_concepts(&all_concepts, &all_embeddings)?;

    // Spawn text reference saving + CDN upload as background task
    let is_uploaded_text = source_url.is_none();
    let text_for_cdn = text.clone();
    let filename_for_cdn = filename.clone();
    let user_id_for_cdn = data.user_id.clone();
    let source_url_for_cdn = source_url.unwrap_or_default();
    let all_concept_strings: Vec<String> = all_concepts.iter().map(|c| c.concept.clone()).collect();
    let db_client_cdn = Arc::clone(&state.db_client);

    tokio::spawn(async move {
        // Save text reference immediately (with empty URL) so it's available for queries
        let saved = if let Some(user_id) = &user_id_for_cdn {
            let normalized_user_id = if user_id == "default" {
                "550e8400-e29b-41d4-a716-446655440000".to_string()
            } else {
                user_id.clone()
            };
            let file_size = text_for_cdn.len() as i32;
            match db_client_cdn.save_text_reference(
                &normalized_user_id,
                &filename_for_cdn,
                "",
                &source_url_for_cdn,
                &all_concept_strings,
                Some(file_size),
            ).await {
                Ok(text_id) => Some((text_id, normalized_user_id)),
                Err(e) => { error!("Failed to save text reference: {:?}", e); None }
            }
        } else {
            None
        };

        // CDN upload only for user-uploaded texts (URL-sourced texts already have their original URL)
        if is_uploaded_text {
            match GitHubCDN::new().upload_text(&text_for_cdn, &filename_for_cdn).await {
                Ok(cdn_url) => {
                    info!("CDN upload succeeded: {}", cdn_url);
                    // Update the URL in DB now that we have it
                    if let Some((text_id, ref user_id)) = saved {
                        if let Err(e) = db_client_cdn.update_text_url(
                            text_id, user_id, &cdn_url, &all_concept_strings,
                        ).await {
                            error!("Failed to update CDN URL in DB: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("CDN upload failed (non-fatal): {:?}", e);
                }
            }
        }
    });

    let response = ApiResponse {
        success: true,
        data: clustered_results,
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_texts_by_concept(
    query: web::Query<ConceptQuery>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    info!("Fetching texts for concept: {} (user: {})", query.concept, query.user_id);

    // Convert "default" to a proper UUID format
    let normalized_user_id = if query.user_id == "default" {
        "550e8400-e29b-41d4-a716-446655440000"
    } else {
        &query.user_id
    };

    let text_references = state
        .db_client
        .get_texts_by_concept(normalized_user_id, &query.concept)
        .await?;

    let response = ApiResponse {
        success: true,
        data: text_references,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[derive(Debug, Deserialize)]
pub struct SaveSceneInput {
    pub scene_data: Vec<ConceptGroup>,
    pub scene_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SaveSceneResponse {
    pub scene_id: String,
}

pub async fn save_scene(
    data: web::Json<SaveSceneInput>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let scene_data_json = serde_json::to_string(&data.scene_data)
        .map_err(|e| ApiError::InternalError(format!("JSON serialization error: {}", e)))?;

    let scene_id = if let Some(existing_id) = &data.scene_id {
        info!("Updating existing scene: {}", existing_id);
        state.db_client.update_scene(existing_id, &scene_data_json).await?;
        existing_id.clone()
    } else {
        let new_id = nanoid::nanoid!(10);
        info!("Creating new scene: {}", new_id);
        state.db_client.save_scene(&new_id, &scene_data_json).await?;
        new_id
    };

    let response = ApiResponse {
        success: true,
        data: SaveSceneResponse { scene_id },
    };

    Ok(HttpResponse::Ok().json(response))
}

pub async fn get_scene(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let scene_id = path.into_inner();
    info!("Loading scene: {}", scene_id);

    let scene_data_json = state.db_client.get_scene(&scene_id).await?;

    let scene_data: Vec<ConceptGroup> = serde_json::from_str(&scene_data_json)
        .map_err(|e| ApiError::InternalError(format!("JSON deserialization error: {}", e)))?;

    let response = ApiResponse {
        success: true,
        data: scene_data,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimensionality::ConceptGroup;
    use crate::data::client::TextReference;
    use chrono::Utc;
    use uuid::Uuid;

    // ==================== Request Deserialization Tests ====================

    mod serialization_tests {
        use super::*;

        #[test]
        fn test_text_input_deserialization() {
            let json = r#"{"text": "Hello world", "user_id": "123", "filename": "test.txt"}"#;
            let input: TextInput = serde_json::from_str(json).unwrap();

            assert_eq!(input.text, Some("Hello world".to_string()));
            assert_eq!(input.user_id, Some("123".to_string()));
            assert_eq!(input.filename, Some("test.txt".to_string()));
            assert!(input.url.is_none());
        }

        #[test]
        fn test_text_input_deserialization_minimal() {
            let json = r#"{"text": "Just text"}"#;
            let input: TextInput = serde_json::from_str(json).unwrap();

            assert_eq!(input.text, Some("Just text".to_string()));
            assert!(input.user_id.is_none());
            assert!(input.filename.is_none());
            assert!(input.url.is_none());
        }

        #[test]
        fn test_text_input_deserialization_with_url() {
            let json = r#"{"url": "https://example.com/article", "user_id": "123"}"#;
            let input: TextInput = serde_json::from_str(json).unwrap();

            assert!(input.text.is_none());
            assert_eq!(input.url, Some("https://example.com/article".to_string()));
            assert_eq!(input.user_id, Some("123".to_string()));
        }

        #[test]
        fn test_text_input_rejects_missing_both() {
            let json = r#"{"user_id": "123"}"#;
            let input: TextInput = serde_json::from_str(json).unwrap();

            assert!(input.text.is_none());
            assert!(input.url.is_none());
        }

        #[test]
        fn test_concept_query_deserialization() {
            let json = r#"{"concept": "machine learning", "user_id": "550e8400-e29b-41d4-a716-446655440000"}"#;
            let query: ConceptQuery = serde_json::from_str(json).unwrap();

            assert_eq!(query.concept, "machine learning");
            assert_eq!(query.user_id, "550e8400-e29b-41d4-a716-446655440000");
        }

        #[test]
        fn test_api_response_serialization() {
            let response = ApiResponse {
                success: true,
                data: vec!["item1", "item2"],
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed["success"], true);
            assert!(parsed["data"].is_array());
            assert_eq!(parsed["data"][0], "item1");
            assert_eq!(parsed["data"][1], "item2");
        }

        #[test]
        fn test_concept_group_serialization() {
            let group = ConceptGroup {
                concepts: vec!["concept1".to_string(), "concept2".to_string()],
                reduced_embedding: vec![1.0, 2.0, 3.0],
                connections: vec![1, 2],
                importance_score: 0.85,
                group_id: 0,
            };

            let json = serde_json::to_string(&group).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert!(parsed["concepts"].is_array());
            assert_eq!(parsed["concepts"][0], "concept1");
            assert!(parsed["reduced_embedding"].is_array());
            assert_eq!(parsed["reduced_embedding"].as_array().unwrap().len(), 3);
            assert!(parsed["connections"].is_array());
            assert!(parsed["importance_score"].is_f64());
        }

        #[test]
        fn test_save_scene_input_deserialization_new() {
            let json = r#"{"scene_data": [{"concepts": ["ai"], "reduced_embedding": [1.0, 2.0, 3.0], "connections": [1], "importance_score": 0.5, "group_id": 0}]}"#;
            let input: SaveSceneInput = serde_json::from_str(json).unwrap();

            assert!(input.scene_id.is_none());
            assert_eq!(input.scene_data.len(), 1);
            assert_eq!(input.scene_data[0].concepts[0], "ai");
        }

        #[test]
        fn test_save_scene_input_deserialization_update() {
            let json = r#"{"scene_data": [{"concepts": ["ai"], "reduced_embedding": [1.0, 2.0, 3.0], "connections": [], "importance_score": 0.5, "group_id": 0}], "scene_id": "xK9bQ4mR2p"}"#;
            let input: SaveSceneInput = serde_json::from_str(json).unwrap();

            assert_eq!(input.scene_id, Some("xK9bQ4mR2p".to_string()));
            assert_eq!(input.scene_data.len(), 1);
        }

        #[test]
        fn test_save_scene_response_serialization() {
            let response = ApiResponse {
                success: true,
                data: SaveSceneResponse {
                    scene_id: "xK9bQ4mR2p".to_string(),
                },
            };

            let json = serde_json::to_string(&response).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed["success"], true);
            assert_eq!(parsed["data"]["scene_id"], "xK9bQ4mR2p");
        }

        #[test]
        fn test_scene_data_roundtrip() {
            let groups = vec![
                ConceptGroup {
                    concepts: vec!["artificial intelligence".to_string(), "AI".to_string()],
                    reduced_embedding: vec![0.5, -0.3, 0.8],
                    connections: vec![1],
                    importance_score: 1.5,
                    group_id: 0,
                },
                ConceptGroup {
                    concepts: vec!["machine learning".to_string()],
                    reduced_embedding: vec![-0.2, 0.7, 0.1],
                    connections: vec![0],
                    importance_score: 1.2,
                    group_id: 1,
                },
            ];

            let json = serde_json::to_string(&groups).unwrap();
            let deserialized: Vec<ConceptGroup> = serde_json::from_str(&json).unwrap();

            assert_eq!(deserialized.len(), 2);
            assert_eq!(deserialized[0].concepts, groups[0].concepts);
            assert_eq!(deserialized[0].reduced_embedding, groups[0].reduced_embedding);
            assert_eq!(deserialized[1].group_id, 1);
        }

        #[test]
        fn test_text_reference_serialization() {
            let text_ref = TextReference {
                text_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
                user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                filename: "test.txt".to_string(),
                url: "https://example.com/test.txt".to_string(),
                source_url: "https://original-article.com/post".to_string(),
                concepts: vec!["concept1".to_string()],
                upload_timestamp: Utc::now(),
                file_size: Some(1024),
            };

            let json = serde_json::to_string(&text_ref).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert!(parsed["text_id"].is_string());
            assert!(parsed["user_id"].is_string());
            assert_eq!(parsed["filename"], "test.txt");
            assert_eq!(parsed["url"], "https://example.com/test.txt");
            assert_eq!(parsed["source_url"], "https://original-article.com/post");
            assert!(parsed["concepts"].is_array());
            assert!(parsed["upload_timestamp"].is_string());
            assert_eq!(parsed["file_size"], 1024);
        }
    }

    // ==================== Endpoint Contract Tests (Mock Handlers) ====================

    mod endpoint_tests {
        use super::*;
        use actix_web::{test, web, App, HttpResponse, ResponseError};

        async fn mock_vectorize_handler(
            _data: web::Json<TextInput>,
        ) -> HttpResponse {
            let mock_groups = vec![
                ConceptGroup {
                    concepts: vec!["artificial intelligence".to_string()],
                    reduced_embedding: vec![0.5, -0.3, 0.8],
                    connections: vec![1],
                    importance_score: 1.5,
                    group_id: 0,
                },
                ConceptGroup {
                    concepts: vec!["machine learning".to_string()],
                    reduced_embedding: vec![-0.2, 0.7, 0.1],
                    connections: vec![0],
                    importance_score: 1.2,
                    group_id: 0,
                },
            ];

            let response = ApiResponse {
                success: true,
                data: mock_groups,
            };

            HttpResponse::Ok().json(response)
        }

        async fn mock_texts_by_concept_handler(
            _query: web::Query<ConceptQuery>,
        ) -> HttpResponse {
            let mock_refs = vec![
                TextReference {
                    text_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
                    user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                    filename: "document.txt".to_string(),
                    url: "https://cdn.example.com/document.txt".to_string(),
                    source_url: "https://example.com/article".to_string(),
                    concepts: vec!["machine learning".to_string()],
                    upload_timestamp: Utc::now(),
                    file_size: Some(2048),
                },
            ];

            let response = ApiResponse {
                success: true,
                data: mock_refs,
            };

            HttpResponse::Ok().json(response)
        }

        async fn mock_texts_by_concept_empty_handler(
            _query: web::Query<ConceptQuery>,
        ) -> HttpResponse {
            let response: ApiResponse<Vec<TextReference>> = ApiResponse {
                success: true,
                data: vec![],
            };

            HttpResponse::Ok().json(response)
        }

        #[actix_web::test]
        async fn test_vectorize_success_response() {
            let app = test::init_service(
                App::new().route("/api/vectorize", web::post().to(mock_vectorize_handler))
            ).await;

            let req = test::TestRequest::post()
                .uri("/api/vectorize")
                .set_json(serde_json::json!({
                    "text": "Artificial intelligence and machine learning",
                    "user_id": "default"
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;

            assert_eq!(body["success"], true);
            assert!(body["data"].is_array());

            let data = body["data"].as_array().unwrap();
            assert!(!data.is_empty());

            let first_group = &data[0];
            assert!(first_group["concepts"].is_array());
            assert!(first_group["reduced_embedding"].is_array());
            assert_eq!(first_group["reduced_embedding"].as_array().unwrap().len(), 3);
            assert!(first_group["connections"].is_array());
            assert!(first_group["importance_score"].is_number());
        }

        #[actix_web::test]
        async fn test_vectorize_content_type() {
            let app = test::init_service(
                App::new().route("/api/vectorize", web::post().to(mock_vectorize_handler))
            ).await;

            let req = test::TestRequest::post()
                .uri("/api/vectorize")
                .set_json(serde_json::json!({"text": "Test text"}))
                .to_request();

            let resp = test::call_service(&app, req).await;

            let content_type = resp.headers().get("content-type").unwrap();
            assert!(content_type.to_str().unwrap().contains("application/json"));
        }

        #[actix_web::test]
        async fn test_texts_by_concept_success_response() {
            let app = test::init_service(
                App::new().route("/api/texts-by-concept", web::get().to(mock_texts_by_concept_handler))
            ).await;

            let req = test::TestRequest::get()
                .uri("/api/texts-by-concept?concept=machine%20learning&user_id=550e8400-e29b-41d4-a716-446655440000")
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;

            assert_eq!(body["success"], true);
            assert!(body["data"].is_array());

            let data = body["data"].as_array().unwrap();
            assert!(!data.is_empty());

            let first_ref = &data[0];
            assert!(first_ref["text_id"].is_string());
            assert!(first_ref["user_id"].is_string());
            assert!(first_ref["filename"].is_string());
            assert!(first_ref["url"].is_string());
            assert!(first_ref["concepts"].is_array());
            assert!(first_ref["upload_timestamp"].is_string());
        }

        async fn mock_save_scene_handler(
            data: web::Json<SaveSceneInput>,
        ) -> HttpResponse {
            let scene_id = data.scene_id.clone().unwrap_or_else(|| "newScene123".to_string());

            let response = ApiResponse {
                success: true,
                data: SaveSceneResponse { scene_id },
            };

            HttpResponse::Ok().json(response)
        }

        async fn mock_get_scene_handler(
            _path: web::Path<String>,
        ) -> HttpResponse {
            let mock_groups = vec![
                ConceptGroup {
                    concepts: vec!["artificial intelligence".to_string()],
                    reduced_embedding: vec![0.5, -0.3, 0.8],
                    connections: vec![1],
                    importance_score: 1.5,
                    group_id: 0,
                },
            ];

            let response = ApiResponse {
                success: true,
                data: mock_groups,
            };

            HttpResponse::Ok().json(response)
        }

        async fn mock_get_scene_not_found_handler(
            path: web::Path<String>,
        ) -> HttpResponse {
            let scene_id = path.into_inner();
            ApiError::SceneNotFound(scene_id).error_response()
        }

        #[actix_web::test]
        async fn test_save_scene_new() {
            let app = test::init_service(
                App::new().route("/api/scenes", web::post().to(mock_save_scene_handler))
            ).await;

            let req = test::TestRequest::post()
                .uri("/api/scenes")
                .set_json(serde_json::json!({
                    "scene_data": [{
                        "concepts": ["ai"],
                        "reduced_embedding": [1.0, 2.0, 3.0],
                        "connections": [1],
                        "importance_score": 0.5,
                        "group_id": 0
                    }]
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
            assert!(body["data"]["scene_id"].is_string());
        }

        #[actix_web::test]
        async fn test_save_scene_update() {
            let app = test::init_service(
                App::new().route("/api/scenes", web::post().to(mock_save_scene_handler))
            ).await;

            let req = test::TestRequest::post()
                .uri("/api/scenes")
                .set_json(serde_json::json!({
                    "scene_data": [{
                        "concepts": ["ai"],
                        "reduced_embedding": [1.0, 2.0, 3.0],
                        "connections": [],
                        "importance_score": 0.5,
                        "group_id": 0
                    }],
                    "scene_id": "existingId1"
                }))
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["data"]["scene_id"], "existingId1");
        }

        #[actix_web::test]
        async fn test_get_scene_success() {
            let app = test::init_service(
                App::new().route("/api/scenes/{scene_id}", web::get().to(mock_get_scene_handler))
            ).await;

            let req = test::TestRequest::get()
                .uri("/api/scenes/testScene1")
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
            assert!(body["data"].is_array());
            assert!(!body["data"].as_array().unwrap().is_empty());

            let first_group = &body["data"][0];
            assert!(first_group["concepts"].is_array());
            assert!(first_group["reduced_embedding"].is_array());
        }

        #[actix_web::test]
        async fn test_get_scene_not_found() {
            let app = test::init_service(
                App::new().route("/api/scenes/{scene_id}", web::get().to(mock_get_scene_not_found_handler))
            ).await;

            let req = test::TestRequest::get()
                .uri("/api/scenes/nonexistent")
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), 404);

            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], false);
        }

        #[actix_web::test]
        async fn test_texts_by_concept_empty_result() {
            let app = test::init_service(
                App::new().route("/api/texts-by-concept", web::get().to(mock_texts_by_concept_empty_handler))
            ).await;

            let req = test::TestRequest::get()
                .uri("/api/texts-by-concept?concept=nonexistent&user_id=550e8400-e29b-41d4-a716-446655440000")
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success());

            let body: serde_json::Value = test::read_body_json(resp).await;

            assert_eq!(body["success"], true);
            assert!(body["data"].is_array());
            assert!(body["data"].as_array().unwrap().is_empty());
        }
    }
}

