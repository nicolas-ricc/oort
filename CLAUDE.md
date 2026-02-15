# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Oort is a concept visualization system that processes text to extract concepts, generates embeddings via Ollama, and displays them as an interactive 3D mind-map. Users upload text files or paste URLs, concepts are extracted using NLP pre-processing (RAKE + TF-IDF) followed by an LLM, embedded, grouped via Union-Find similarity merging, positioned via PCA + force-directed layout, and rendered as planets in 3D space.

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Frontend      │───▶│   ML Backend    │───▶│   Ollama        │
│   (React/R3F)   │    │   (Rust/Actix)  │    │   (LLM/Embed)   │
│   :5173         │    │   :8000         │    │   :11434        │
└─────────────────┘    └────────┬────────┘    └─────────────────┘
                                │
                       ┌────────▼────────┐
                       │   Cassandra DB  │
                       │   :9042         │
                       └─────────────────┘
```

### ML Backend (Rust - `ml/`)
- **Actix-web** server exposing `/api/vectorize` and `/api/texts-by-concept`
- **ArticleScraper** (`data/scraper.rs`) - Fetches and extracts article content from URLs using `dom_smoothie` (Readability.js port) with two-layer noise filtering
- **KeywordExtractor** (`models/concepts/nlp.rs`) - RAKE + TF-IDF pre-processing on full text to generate candidate keywords for the LLM
- **ConceptsModel** (`models/concepts/`) - Extracts concepts from text using Ollama (phi3.5), with NLP candidate hints and sentence-boundary truncation at 500 chars
- **EmbeddingModel** (`models/embeddings/`) - Generates embeddings using Ollama (snowflake-arctic-embed2)
- **MindMapProcessor** (`dimensionality.rs`) - Merges similar concepts (Union-Find), builds continuous similarity matrix, initializes 3D positions via PCA (linfa), refines with force-directed layout (universal repulsion + convergence detection)
- **DatabaseClient** (`data/client.rs`) - Cassandra operations for concepts and text references
- **GitHubCDN** (`data/cdn/`) - Uploads processed text to GitHub as CDN storage

### Frontend (React - `apps/frontend/`)
- **React Three Fiber** with drei for 3D visualization
- `cloud/Scene.tsx` - Main 3D scene rendering planets from concept clusters
- `cloud/Planet.tsx` - Individual concept planet with texture and label
- `cloud/Render.tsx` - Canvas setup and camera controls
- `layout/Menu.tsx` - Concept search, file upload, and URL input interface
- Uses TanStack Query for API calls

### Database (Cassandra - `db/`)
- Keyspace: `store`
- Tables: `user_concepts`, `concept_sources`, `text_references`, `concept_text_mapping`

## Subproject Documentation

This repository contains detailed CLAUDE.md files for each major component. **IMPORTANT**: When analyzing or modifying code in these projects, you MUST read the corresponding CLAUDE.md file first for project-specific context, patterns, and critical information.

### Frontend (`apps/frontend/CLAUDE.md`)
Read this file when working with:
- React components, 3D visualization, or UI code
- React Three Fiber (R3F) scenes, planets, or rendering
- TanStack Query API integration
- Component testing with Vitest

**Contains**: Component hierarchy, SCENE_SCALE synchronization rules, 3D coordinate system, post-processing setup, unit test specifications.

### ML Backend (`ml/CLAUDE.md`)
Read this file when working with:
- Rust backend code, API endpoints, or processing pipeline
- Concept extraction, embedding generation, or clustering
- Force-directed layout or dimensionality reduction
- Database operations or CDN integration

**Contains**: Processing pipeline details, force layout parameters, API contracts, data types, error handling, unit test specifications.

## Development Commands

### Run entire stack with Docker
```bash
docker-compose up --build
```

### Frontend only
```bash
cd apps/frontend
pnpm install
pnpm dev           # Dev server at :5173
pnpm build         # Production build
pnpm lint          # ESLint
```

### ML Backend only
```bash
cd ml
cargo build
cargo run          # Server at :8000
cargo watch -x run # Hot-reload development
cargo check        # Type check without building
```

### Database
```bash
docker-compose up oort-db    # Start Cassandra
# Init script runs automatically via cassandra-init service
```

## Key Constants

- `SCENE_SCALE = 2` - Defined in both `App.tsx` and `Scene.tsx`, controls all 3D positioning
- Ollama models: `phi3.5` (concept extraction), `snowflake-arctic-embed2` (embeddings)
- Default user UUID: `550e8400-e29b-41d4-a716-446655440000`

## Environment Variables

- `OLLAMA_URL` - Ollama service URL (default: `http://ollama:11434`)
- `DB_NODES` - Cassandra nodes (default: `oort-db:9042`)
- `GITHUB_TOKEN_FILE` - Path to GitHub token for CDN uploads
- `GITHUB_OWNER` - GitHub username for CDN repository
- `RUST_LOG` - Logging level (default: `info`)

## Ports

- 5173: Frontend (Vite)
- 8000: ML Backend (Actix)
- 9042: Cassandra
- 11434: Ollama
- 3000: Grafana