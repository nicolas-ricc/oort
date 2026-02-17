use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("No concepts could be extracted from the provided text")]
    NoConceptsExtracted,
    
    #[error("Error generating embeddings")]
    EmbeddingGenerationError,
    
    #[error("The file could not be decoded as text")]
    FileDecodeError,
    
    #[error("Error in HTTP request: {0}")]
    RequestError(#[from] reqwest::Error),
    
    #[error("Error reading payload: {0}")]
    PayloadError(#[from] actix_web::error::PayloadError),
    
    #[error("Error in dimensionality reduction: {0}")]
    DimensionalityError(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Failed to fetch URL: {0}")]
    UrlFetchError(String),

    #[error("Failed to extract content: {0}")]
    ContentExtractionError(String),

    #[error("Scene not found: {0}")]
    SceneNotFound(String),
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    success: bool,
    detail: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let status_code = match self {
            ApiError::NoConceptsExtracted => actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::FileDecodeError => actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::UrlFetchError(_) => actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::ContentExtractionError(_) => actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::SceneNotFound(_) => actix_web::http::StatusCode::NOT_FOUND,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        HttpResponse::build(status_code).json(ErrorResponse {
            success: false,
            detail: self.to_string(),
        })
    }
}