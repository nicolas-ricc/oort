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
    ↓ Hybrid NLP + guarded LLM pipeline:
    │   1. validate_candidates_with_embeddings() — embed candidates + source text,
    │      keep candidates with cosine similarity ≥ 0.3 (NLP baseline, always works)
    │   2. try_llm_enrichment() — send NLP candidates + 500-char excerpt to LLM
    │      for 3-5 overarching themes (optional, with degenerate output detection)
    │   3. merge_nlp_and_llm() — deduplicate, NLP takes priority, cap at 15
    ↓ (LLM failure is non-fatal → falls back to NLP-only concepts)
EmbeddingModel::get_batch_embeddings()    # models/embeddings/model.rs
    ↓ (concurrent via join_all)
MindMapProcessor::process_concepts()      # dimensionality.rs
    ├── merge_similar_concepts()          # Union-Find, cosine similarity > 0.7
    ├── build_similarity_matrix()         # Continuous (no threshold), all positive similarities
    ├── initialize_pca_positions()        # PCA to 3D via linfa (deterministic init)
    └── run_force_directed_layout()       # Universal repulsion + continuous attraction, convergence detection
    ↓
ConceptGroup[] with 3D positions + group_id
```

### NLP Pre-processing (`models/concepts/nlp.rs`)
`KeywordExtractor` runs RAKE + TF-IDF on the **full text**, producing statistical keyword candidates that serve as the primary concept source.

### Hybrid Concept Extraction (`models/concepts/model.rs`)
The pipeline uses NLP as the primary extractor with the LLM demoted to an optional theme enricher:
1. **NLP + Embedding validation:** NLP candidates are validated against the source text using embedding cosine similarity (≥ 0.3 threshold). Score = 50% NLP + 50% similarity. Always produces results.
2. **Guarded LLM enrichment** (optional, controlled by `LLM_ENRICHMENT` env var): NLP candidates + 500-char text excerpt sent to LLM for 3-5 overarching themes. Uses `frequency_penalty=1.5` + `dry_multiplier=0.8` to prevent repetition loops. Degenerate output detected via sliding 8-char window (>5 repeats) and missing closing brace. On degeneration, retries once with stronger penalties (`frequency_penalty=2.0`, `dry_multiplier=1.2`, `temperature=0.1`). Failure is non-fatal — falls back to NLP-only concepts.
3. **Merge:** NLP concepts + LLM themes deduplicated case-insensitively (NLP takes priority), capped at 15.

No chunking needed: NLP runs on full text, LLM receives candidates + brief excerpt (~400 tokens total).

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

## GPU Memory Management

**Critical**: Model loading order in `main.rs` matters. The embedding model (~0.6GB) must load **before** the LLM (~2.6GB). KV cache is sized via `MemoryGpuConfig::ContextSize(4096)` by default (set via `LLM_CONTEXT_SIZE`), allocating KV for 4096 tokens (~1.5 GB). Phi-3.5-mini uses standard MHA (32 KV heads), so KV cost is ~384 KB/token. Total VRAM: ~4.7 GB on RTX 3070 (8GB), leaving ~3.3 GB free. `LLM_CONCURRENCY=1` ensures only one sequence at a time. `LLM_GPU_UTILIZATION` is available as an advanced override for Utilization-based sizing. LLM and embedding inference are strictly sequential in the pipeline, so both models can coexist on GPU. See `~/.claude/projects/-home-nicolasr-Projects-oort/memory/gpu-memory.md` for full analysis.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| OLLAMA_URL | http://ollama:11434 | Ollama API endpoint |
| DB_NODES | oort-db:9042 | Cassandra nodes (comma-separated) |
| GITHUB_TOKEN_FILE | - | Path to GitHub token for CDN |
| GITHUB_OWNER | - | GitHub username for CDN repo |
| RUST_LOG | info | Log level |
| LLM_CONTEXT_SIZE | 4096 | KV cache token capacity |
| LLM_GPU_UTILIZATION | (unset) | GPU memory fraction for KV cache, overrides `LLM_CONTEXT_SIZE` when set (0.0-1.0) |
| EMBEDDING_ON_CPU | false | Force embedding model to CPU (`true` for tight GPU memory) |
| LLM_ENRICHMENT | true | Enable LLM theme enrichment (`false` for NLP-only mode) |

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
