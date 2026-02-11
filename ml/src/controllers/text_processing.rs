use actix_web::{web, HttpResponse, Responder};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::data::cdn::github::GitHubCDN;
use crate::data::client::{DatabaseClient, TextReference};
use crate::dimensionality;
use crate::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct TextInput {
    pub text: String,
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
    // Process concepts and embeddings, and get the clustered results directly
    let new_concepts = state.concepts_model.generate_concepts(&data.text).await?;

    if new_concepts.is_empty() {
        return Err(ApiError::NoConceptsExtracted);
    }

    let mut all_concepts = new_concepts.clone();
    let mut existing_embeddings = Vec::new();

    let uuid_str = if let Some(user_id) = data.user_id.as_deref() {
        info!("Loading existing concepts for user: {}", user_id);
        
        // Convert "default" to a proper UUID format
        let normalized_user_id = if user_id == "default" {
            "550e8400-e29b-41d4-a716-446655440000"
        } else {
            user_id
        };

        let uuid_str = normalized_user_id.to_string();

        let user_concepts = state.db_client.get_user_concepts(&uuid_str).await?;

        for (concept, embedding) in user_concepts {
            all_concepts.push(concept);
            existing_embeddings.push(embedding);
        }

        Some(uuid_str)
    } else {
        None
    };

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

    // Spawn CDN upload + text reference saving as background task
    let text_for_cdn = data.text.clone();
    let filename_for_cdn = data.filename.clone().unwrap_or_else(|| {
        format!("processed_text_{}.txt", Uuid::new_v4())
    });
    let user_id_for_cdn = data.user_id.clone();
    let all_concept_strings: Vec<String> = all_concepts.iter().map(|c| c.concept.clone()).collect();
    let db_client_cdn = Arc::clone(&state.db_client);

    tokio::spawn(async move {
        match GitHubCDN::new().upload_text(&text_for_cdn, &filename_for_cdn).await {
            Ok(cdn_url) => {
                if let Some(user_id) = &user_id_for_cdn {
                    let normalized_user_id = if user_id == "default" {
                        "550e8400-e29b-41d4-a716-446655440000".to_string()
                    } else {
                        user_id.clone()
                    };
                    let file_size = text_for_cdn.len() as i32;
                    if let Err(e) = db_client_cdn.save_text_reference(
                        &normalized_user_id,
                        &filename_for_cdn,
                        &cdn_url,
                        &all_concept_strings,
                        Some(file_size),
                    ).await {
                        error!("Failed to save text reference: {:?}", e);
                    }
                }
            }
            Err(e) => {
                error!("CDN upload failed (non-fatal): {:?}", e);
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

            assert_eq!(input.text, "Hello world");
            assert_eq!(input.user_id, Some("123".to_string()));
            assert_eq!(input.filename, Some("test.txt".to_string()));
        }

        #[test]
        fn test_text_input_deserialization_minimal() {
            let json = r#"{"text": "Just text"}"#;
            let input: TextInput = serde_json::from_str(json).unwrap();

            assert_eq!(input.text, "Just text");
            assert!(input.user_id.is_none());
            assert!(input.filename.is_none());
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
                cluster: 0,
                connections: vec![1, 2],
                importance_score: 0.85,
            };

            let json = serde_json::to_string(&group).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

            assert!(parsed["concepts"].is_array());
            assert_eq!(parsed["concepts"][0], "concept1");
            assert!(parsed["reduced_embedding"].is_array());
            assert_eq!(parsed["reduced_embedding"].as_array().unwrap().len(), 3);
            assert_eq!(parsed["cluster"], 0);
            assert!(parsed["connections"].is_array());
            assert!(parsed["importance_score"].is_f64());
        }

        #[test]
        fn test_text_reference_serialization() {
            let text_ref = TextReference {
                text_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
                user_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
                filename: "test.txt".to_string(),
                url: "https://example.com/test.txt".to_string(),
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
            assert!(parsed["concepts"].is_array());
            assert!(parsed["upload_timestamp"].is_string());
            assert_eq!(parsed["file_size"], 1024);
        }
    }

    // ==================== Endpoint Contract Tests (Mock Handlers) ====================

    mod endpoint_tests {
        use super::*;
        use actix_web::{test, web, App, HttpResponse};

        async fn mock_vectorize_handler(
            _data: web::Json<TextInput>,
        ) -> HttpResponse {
            let mock_groups = vec![
                ConceptGroup {
                    concepts: vec!["artificial intelligence".to_string()],
                    reduced_embedding: vec![0.5, -0.3, 0.8],
                    cluster: 0,
                    connections: vec![1],
                    importance_score: 1.5,
                },
                ConceptGroup {
                    concepts: vec!["machine learning".to_string()],
                    reduced_embedding: vec![-0.2, 0.7, 0.1],
                    cluster: 0,
                    connections: vec![0],
                    importance_score: 1.2,
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
            assert!(first_group["cluster"].is_number());
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

