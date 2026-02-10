# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Rust backend service for concept extraction, embedding generation, and dimensionality reduction. Processes text into 3D-positioned concept clusters for visualization.

## Development Commands

```bash
cargo run                    # Start server at :8000
cargo watch -x run           # Hot-reload development
cargo check                  # Type check without building
cargo test                   # Run unit tests
cargo build --release        # Production build
```

**Note**: Requires `libopenblas-dev` and `liblapack-dev` for ndarray-linalg.

## Processing Pipeline

```
Text Input
    ↓
ConceptsModel::generate_concepts()     # controllers/text_processing.rs:120
    ↓ (Ollama phi3.5)
EmbeddingModel::get_batch_embeddings() # controllers/text_processing.rs:155
    ↓ (Ollama snowflake-arctic-embed2)
MindMapProcessor::process_concepts()   # dimensionality.rs:66
    ├── merge_similar_concepts()       # Cosine similarity > 0.7
    ├── build_similarity_matrix()
    ├── run_force_directed_layout()    # 200 iterations
    ├── apply_clustering()             # K-means via linfa
    └── PCA reduction to 3D
    ↓
ConceptGroup[] with 3D positions
```

## Key Types

```rust
// models/embeddings/model.rs:8
pub type Embedding = Array1<f32>;

// dimensionality.rs:13
pub struct ConceptGroup {
    pub concepts: Vec<String>,
    pub reduced_embedding: Vec<f32>,  // 3D coordinates
    pub cluster: usize,
    pub connections: Vec<usize>,
    pub importance_score: f32,
}
```

## Force Layout Parameters

`dimensionality.rs:34` - `ForceParams::default()`:
| Parameter | Value | Purpose |
|-----------|-------|---------|
| attraction_strength | 2.0 | Pull connected nodes together |
| repulsion_strength | 100.0 | Push all nodes apart |
| center_gravity | 0.2 | Pull toward origin |
| damping | 0.9 | Velocity decay |
| min_distance | 3.0 | Minimum node separation |
| similarity_threshold | 0.7 | Merge concepts above this |

## API Endpoints

### POST /api/vectorize
```json
// Request
{ "text": "...", "user_id": "uuid", "filename": "optional.txt" }

// Response
{ "success": true, "data": [ConceptGroup, ...] }
```

### GET /api/texts-by-concept
```
?concept=<text>&user_id=<uuid>
```
Returns text references containing the concept.

## Data Storage

- **Cassandra** (`data/client.rs`) - User concepts and text references
- **GitHub CDN** (`data/cdn/github.rs`) - Uploaded text files served via jsDelivr

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| OLLAMA_URL | http://ollama:11434 | Ollama API endpoint |
| DB_NODES | oort-db:9042 | Cassandra nodes (comma-separated) |
| GITHUB_TOKEN_FILE | - | Path to GitHub token for CDN |
| GITHUB_OWNER | - | GitHub username for CDN repo |
| RUST_LOG | info | Log level |

## Error Handling

- `ApiError::NoConceptsExtracted` → 422 (no concepts found in text)
- `ApiError::EmbeddingGenerationError` → 422 (embedding count mismatch)
- `ApiError::RequestError` → 500 (Ollama/external service failure)
- `ApiError::DatabaseError` → 500 (Cassandra failure)

## Unit Tests

Run tests with `cargo test`. Tests are located in `controllers/text_processing.rs`.

### Serialization Tests (`tests::serialization_tests`)

| Test | Coverage |
|------|----------|
| `test_text_input_deserialization` | TextInput JSON parsing with all fields |
| `test_text_input_deserialization_minimal` | TextInput with only required `text` field |
| `test_concept_query_deserialization` | ConceptQuery query params parsing |
| `test_api_response_serialization` | ApiResponse<T> serializes with `success` and `data` |
| `test_concept_group_serialization` | ConceptGroup has all fields (concepts, reduced_embedding, cluster, connections, importance_score) |
| `test_text_reference_serialization` | TextReference has all fields (text_id, user_id, filename, url, concepts, upload_timestamp, file_size) |

### Endpoint Contract Tests (`tests::endpoint_tests`)

Uses mock handlers to verify API contracts without external services.

| Test | Coverage |
|------|----------|
| `test_vectorize_success_response` | POST /api/vectorize returns correct JSON structure |
| `test_vectorize_content_type` | Response has application/json content-type |
| `test_texts_by_concept_success_response` | GET /api/texts-by-concept returns correct JSON structure |
| `test_texts_by_concept_empty_result` | Returns empty array (not error) for no matches |
