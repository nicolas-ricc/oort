# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Python notebooks for model fine-tuning experiments. Contains tooling to fine-tune models for use in the Oort concept extraction pipeline.

## Structure

```
notebooks/
└── fine-tuning/
    ├── pyproject.toml    # Poetry project config
    └── data/             # Training data directory
```

## Development Commands

```bash
cd fine-tuning
poetry install              # Install dependencies
poetry run python <script>  # Run scripts
poetry shell                # Activate virtual environment
```

## Dependencies

- Python ^3.10
- transformers 4.46.2

## Purpose

Fine-tune models to potentially replace or augment the Ollama models used in the ML backend:
- Concept extraction (currently phi3.5)
- Embedding generation (currently snowflake-arctic-embed2)
