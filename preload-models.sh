#!/bin/bash
# preload_models.sh
curl -X POST http://ollama:11434/api/generate -d '{"model":"phi3.5","prompt":"hello"}'
curl -X POST http://ollama:11434/api/embeddings -d '{"model":"snowflake-arctic-embed2","prompt":"hello"}'