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
Text Input (raw text or URL)
    ↓
ArticleScraper::scrape_url()              # data/scraper.rs (URL input only)
    ↓ (dom_smoothie Readability + noise filtering)
KeywordExtractor::extract_candidates()    # models/concepts/nlp.rs
    ↓ (RAKE + TF-IDF on full text, top 20 candidates)
ConceptsModel::generate_concepts()        # models/concepts/model.rs
    ↓ Adaptive strategy:
    │   < 6000 chars: single LLM call with full text
    │   ≥ 6000 chars: MapReduce (chunk_text → parallel LLM calls → merge)
    ↓ (Ollama phi3.5, num_ctx scaled to text length, NLP candidates as hints)
EmbeddingModel::get_batch_embeddings()    # models/embeddings/model.rs
    ↓ (Ollama snowflake-arctic-embed2, concurrent via join_all)
MindMapProcessor::process_concepts()      # dimensionality.rs
    ├── merge_similar_concepts()          # Union-Find, cosine similarity > 0.7
    ├── build_similarity_matrix()         # Continuous (no threshold), all positive similarities
    ├── initialize_pca_positions()        # PCA to 3D via linfa (deterministic init)
    └── run_force_directed_layout()       # Universal repulsion + continuous attraction, convergence detection
    ↓
ConceptGroup[] with 3D positions + group_id
```

### NLP Pre-processing (`models/concepts/nlp.rs`)
`KeywordExtractor` runs RAKE + TF-IDF on the **full text** before the LLM call, providing statistical keyword candidates as hints in the phi3.5 system prompt.

### Adaptive Concept Extraction (`models/concepts/model.rs`)
- **Short texts (< 6000 chars):** Full text sent directly to LLM in a single call. `num_ctx` scaled dynamically: `max(4096, text_len/3 + 1024)`.
- **Long texts (≥ 6000 chars):** MapReduce strategy — `chunk_text()` splits into ~2000-char overlapping chunks at sentence boundaries, each chunk processed in parallel via `join_all`, results deduplicated by normalized name (highest importance wins). Downstream Union-Find merge handles semantic deduplication.

### Text Chunking (`models/concepts/truncation.rs`)
`chunk_text(text, chunk_size, overlap)` splits text into overlapping chunks at natural boundaries. Uses `find_last_boundary()` with tiered fallback: sentence end > paragraph break > markdown heading > newline > word boundary. `truncate_at_sentence_boundary()` also uses the same boundary detection. Filters abbreviations (Dr., U.S., etc.) and is UTF-8 safe.

### Parallelization
- **Embeddings:** `get_batch_embeddings()` fires all Ollama calls concurrently via `join_all`.
- **LLM + DB:** `process_text()` and `process_concepts_and_embeddings()` run LLM concept extraction and DB user concept loading concurrently via `tokio::try_join!`.

### URL Scraping (`data/scraper.rs`)
`ArticleScraper` fetches web pages and extracts article content using `dom_smoothie` (Rust Readability.js port). Two-layer noise filtering: `pre_clean_dom()` removes 30+ CSS selector patterns (reading time, author blocks, related posts, share buttons, etc.) before Readability, then `clean_extracted_text()` regex-cleans residual metadata patterns. JS-heavy sites may fail — the error messaging suggests manual upload as fallback.

## Key Types

```rust
// models/concepts/model.rs
pub struct Concept {
    pub concept: String,
    pub importance: f32,    // From LLM, blended with NLP score
}

// models/embeddings/model.rs
pub type Embedding = Array1<f32>;

// dimensionality.rs
pub struct ConceptGroup {
    pub concepts: Vec<String>,
    pub reduced_embedding: Vec<f32>,  // 3D coordinates (PCA-initialized, force-refined)
    pub connections: Vec<usize>,
    pub importance_score: f32,        // 40% NLP + 40% connections + 20% concept count
    pub group_id: usize,             // Semantic cluster ID from Union-Find merge groups
}
```

## Force Layout Parameters

`dimensionality.rs:30` - `ForceParams::default()`:
| Parameter | Value | Purpose |
|-----------|-------|---------|
| attraction_strength | 2.0 | Pull similar nodes together (weighted by continuous similarity) |
| repulsion_strength | 10.0 | Universal inverse-square repulsion between all pairs |
| center_gravity | 0.1 | Weak pull toward origin (prevents drift) |
| damping | 0.9 | Velocity decay |
| min_distance | 3.0 | (Legacy, unused — repulsion is now universal) |
| max_velocity | 2.0 | Velocity clamp |
| iterations | 150 | Max iterations (early exit on convergence < 0.001) |
| similarity_threshold | 0.7 | Merge concepts above this (Union-Find only, not force layout) |

## API Endpoints

### POST /api/vectorize
```json
// Request — provide either text or url (not both)
{ "text": "...", "url": "https://...", "user_id": "uuid", "filename": "optional.txt" }

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
- `ApiError::UrlFetchError` → 422 (URL fetch failed or unreachable)
- `ApiError::ContentExtractionError` → 422 (Readability couldn't extract article or content too short)
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
| `test_concept_group_serialization` | ConceptGroup has all fields (concepts, reduced_embedding, connections, importance_score, group_id) |
| `test_text_reference_serialization` | TextReference has all fields (text_id, user_id, filename, url, concepts, upload_timestamp, file_size) |

### Endpoint Contract Tests (`tests::endpoint_tests`)

Uses mock handlers to verify API contracts without external services.

| Test | Coverage |
|------|----------|
| `test_vectorize_success_response` | POST /api/vectorize returns correct JSON structure |
| `test_vectorize_content_type` | Response has application/json content-type |
| `test_texts_by_concept_success_response` | GET /api/texts-by-concept returns correct JSON structure |
| `test_texts_by_concept_empty_result` | Returns empty array (not error) for no matches |
