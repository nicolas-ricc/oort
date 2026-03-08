use crate::error::ApiError;
use crate::models::concepts::nlp::CandidateKeyword;
use crate::models::concepts::validation;
use crate::models::inference::{EmbeddingBackend, GenerationParams, LlmBackend};
use log::{debug, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Max concepts to return from the hybrid pipeline.
const MAX_CONCEPTS: usize = 15;

/// Minimum cosine similarity between a candidate embedding and the source text
/// embedding for the candidate to be kept.
const MIN_EMBEDDING_SIMILARITY: f32 = 0.3;

/// Max chars of source text to include in the LLM excerpt.
const LLM_EXCERPT_CHARS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept: String,
    pub importance: f32,
}

pub struct ConceptsModel {
    backend: Arc<dyn LlmBackend>,
    embedding_backend: Arc<dyn EmbeddingBackend>,
    llm_enrichment: bool,
}

impl ConceptsModel {
    pub fn new(
        backend: Arc<dyn LlmBackend>,
        embedding_backend: Arc<dyn EmbeddingBackend>,
        llm_enrichment: bool,
    ) -> Self {
        Self {
            backend,
            embedding_backend,
            llm_enrichment,
        }
    }

    pub fn clean_text(&self, text: &str) -> String {
        let re_punct: Regex = Regex::new(r"[^\w\s']").unwrap();
        let text = re_punct.replace_all(text, " ");

        let re_apos = Regex::new(r"\s'|'\s").unwrap();
        let text = re_apos.replace_all(&text, " ");

        let re_spaces = Regex::new(r"\s+").unwrap();
        let text = re_spaces.replace_all(&text, " ");

        text.trim().to_string()
    }

    pub fn lemmatize_concept(&self, concept: &str) -> String {
        let concept = self.clean_text(concept);

        let words: Vec<&str> = concept.split_whitespace().collect();

        let lemmatized_words: Vec<String> = words.iter().map(|word| word.to_string()).collect();

        lemmatized_words.join(" ")
    }

    fn build_candidate_hints(nlp_candidates: &[CandidateKeyword]) -> String {
        if nlp_candidates.is_empty() {
            return String::new();
        }

        let mut hints = String::new();
        for candidate in nlp_candidates.iter().take(20) {
            hints.push_str(&format!(
                "- \"{}\" (score: {:.2})\n",
                candidate.phrase, candidate.score
            ));
        }
        hints
    }

    /// Validates NLP candidates against the source text using embedding similarity.
    /// Returns candidates with similarity >= MIN_EMBEDDING_SIMILARITY, scored as
    /// a blend of NLP score and embedding similarity.
    async fn validate_candidates_with_embeddings(
        &self,
        candidates: &[CandidateKeyword],
        text: &str,
    ) -> Result<Vec<Concept>, ApiError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Embed source text (Qwen3-Embedding-0.6B handles ~8192 tokens)
        let text_embedding = self
            .embedding_backend
            .embed(text)
            .await
            .map_err(|e| ApiError::InternalError(format!("Embedding source text failed: {}", e)))?;

        // Embed all candidate phrases
        let candidate_texts: Vec<String> =
            candidates.iter().map(|c| c.phrase.clone()).collect();
        let candidate_embeddings = self
            .embedding_backend
            .embed_batch(&candidate_texts)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Embedding candidates failed: {}", e))
            })?;

        let mut concepts = Vec::new();
        for (i, candidate) in candidates.iter().enumerate() {
            if i >= candidate_embeddings.len() {
                break;
            }
            let similarity = cosine_similarity(&text_embedding, &candidate_embeddings[i]);
            if similarity >= MIN_EMBEDDING_SIMILARITY {
                let blended_score = candidate.score * 0.5 + similarity * 0.5;
                concepts.push(Concept {
                    concept: self.lemmatize_concept(&candidate.phrase),
                    importance: blended_score.clamp(0.0, 1.0),
                });
            }
        }

        // Sort by importance descending
        concepts.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!(
            "Embedding validation: {}/{} candidates passed (>= {:.1} similarity)",
            concepts.len(),
            candidates.len(),
            MIN_EMBEDDING_SIMILARITY
        );

        Ok(concepts)
    }

    /// Attempts LLM theme enrichment using NLP candidates + brief text excerpt.
    /// Returns Err on degenerate output or LLM failure (caller should fall back to NLP-only).
    async fn try_llm_enrichment(
        &self,
        text: &str,
        candidates: &[CandidateKeyword],
    ) -> Result<Vec<Concept>, String> {
        let candidate_list = Self::build_candidate_hints(candidates);
        let excerpt = truncate_to_char_boundary(text, LLM_EXCERPT_CHARS);

        let system_prompt = r#"You are a theme extractor. Given candidate keywords extracted from a text and a brief excerpt, identify 3-5 overarching intellectual themes that connect these concepts. Each theme should be 1-3 words. Focus on abstract ideas, not specific terms already listed.
Output ONLY valid JSON matching the required schema."#;

        let user_prompt = format!(
            "Candidate keywords:\n{}\nText excerpt:\n{}",
            candidate_list, excerpt
        );

        let json_schema: serde_json::Value = serde_json::json!({
            "type": "object",
            "properties": {
                "concepts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "importance": { "type": "number" }
                        },
                        "required": ["name", "importance"]
                    }
                }
            },
            "required": ["concepts"]
        });

        // First attempt: deterministic with moderate penalties
        let params = GenerationParams {
            temperature: 0.0,
            max_tokens: Some(256),
            json_schema: Some(json_schema.clone()),
            frequency_penalty: Some(1.5),
            dry_multiplier: Some(0.8),
        };

        info!(
            "LLM theme enrichment: sending {} candidates + {} char excerpt",
            candidates.len().min(20),
            excerpt.len()
        );

        match self
            .backend
            .generate(system_prompt, &user_prompt, &params)
            .await
        {
            Ok(response) => {
                if !validation::detect_degenerate_output(&response) {
                    return self
                        .parse_and_validate_llm_response(&response)
                        .map_err(|e| format!("LLM parse error: {}", e));
                }
                warn!("LLM produced degenerate output on first attempt, retrying");
            }
            Err(e) => {
                warn!("LLM enrichment failed on first attempt: {}", e);
            }
        }

        // Retry with stronger penalties and slight temperature
        let retry_params = GenerationParams {
            temperature: 0.1,
            max_tokens: Some(256),
            json_schema: Some(json_schema),
            frequency_penalty: Some(2.0),
            dry_multiplier: Some(1.2),
        };

        match self
            .backend
            .generate(system_prompt, &user_prompt, &retry_params)
            .await
        {
            Ok(response) => {
                if validation::detect_degenerate_output(&response) {
                    return Err("LLM produced degenerate output on retry".into());
                }
                self.parse_and_validate_llm_response(&response)
                    .map_err(|e| format!("LLM parse error on retry: {}", e))
            }
            Err(e) => Err(format!("LLM enrichment failed on retry: {}", e)),
        }
    }

    /// Parses LLM JSON response and applies concept validation filters.
    fn parse_and_validate_llm_response(
        &self,
        response: &str,
    ) -> Result<Vec<Concept>, ApiError> {
        let concepts = self.parse_concepts_response(response)?;
        Ok(validation::validate_concepts(concepts))
    }

    fn parse_concepts_response(&self, response: &str) -> Result<Vec<Concept>, ApiError> {
        #[derive(Debug, Deserialize)]
        struct ConceptEntry {
            name: String,
            importance: Option<f64>,
        }

        #[derive(Debug, Deserialize)]
        struct ConceptsResponse {
            concepts: Vec<serde_json::Value>,
        }

        let concepts_response: ConceptsResponse =
            serde_json::from_str(response).map_err(|e| {
                info!("Error parsing nested JSON: {}", e);
                ApiError::InternalError(format!("Failed to parse concepts JSON: {}", e))
            })?;

        let mut concepts: Vec<Concept> = Vec::new();
        for value in concepts_response.concepts {
            match value {
                // New format: { "name": "...", "importance": 0.8 }
                serde_json::Value::Object(_) => {
                    if let Ok(entry) = serde_json::from_value::<ConceptEntry>(value) {
                        let name = entry.name.trim().to_string();
                        if !name.is_empty() && name.split_whitespace().count() <= 3 {
                            let lemmatized = self.lemmatize_concept(&name);
                            let importance = entry
                                .importance
                                .map(|i| (i as f32).clamp(0.0, 1.0))
                                .unwrap_or(0.5);
                            concepts.push(Concept {
                                concept: lemmatized,
                                importance,
                            });
                        }
                    }
                }
                // Backward compat: plain string
                serde_json::Value::String(s) => {
                    let s = s.trim();
                    if !s.is_empty() && s.split_whitespace().count() <= 3 {
                        let lemmatized = self.lemmatize_concept(s);
                        concepts.push(Concept {
                            concept: lemmatized,
                            importance: 0.5,
                        });
                    }
                }
                _ => {}
            }
        }

        debug!("Lemmatized concepts: {:?}", concepts);
        Ok(concepts)
    }

    /// Merges NLP concepts with LLM themes, deduplicating by name (case-insensitive).
    /// NLP concepts take priority. Capped at MAX_CONCEPTS total.
    fn merge_nlp_and_llm(nlp: Vec<Concept>, llm: Vec<Concept>) -> Vec<Concept> {
        let mut seen: HashMap<String, Concept> = HashMap::new();

        // NLP concepts first (priority)
        for concept in nlp {
            let key = concept.concept.to_lowercase();
            seen.entry(key).or_insert(concept);
        }

        // Add LLM themes that don't duplicate NLP concepts
        for concept in llm {
            let key = concept.concept.to_lowercase();
            seen.entry(key).or_insert(concept);
        }

        let mut merged: Vec<Concept> = seen.into_values().collect();
        merged.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        merged.truncate(MAX_CONCEPTS);
        merged
    }

    pub async fn generate_concepts(
        &self,
        text: &str,
        nlp_candidates: &[CandidateKeyword],
    ) -> Result<Vec<Concept>, ApiError> {
        // Step 1: Validate NLP candidates with embedding similarity (always works)
        let nlp_concepts = self
            .validate_candidates_with_embeddings(nlp_candidates, text)
            .await?;

        info!(
            "NLP baseline: {} concepts from {} candidates",
            nlp_concepts.len(),
            nlp_candidates.len()
        );

        // Step 2: Optionally enrich with LLM themes
        let final_concepts = if self.llm_enrichment {
            match self.try_llm_enrichment(text, nlp_candidates).await {
                Ok(llm_themes) if !llm_themes.is_empty() => {
                    info!(
                        "LLM enrichment added {} themes, merging with {} NLP concepts",
                        llm_themes.len(),
                        nlp_concepts.len()
                    );
                    Self::merge_nlp_and_llm(nlp_concepts, llm_themes)
                }
                Ok(_) => {
                    warn!("LLM returned no themes, using NLP concepts only");
                    nlp_concepts
                }
                Err(e) => {
                    warn!("LLM enrichment failed (non-fatal): {}", e);
                    nlp_concepts
                }
            }
        } else {
            debug!("LLM enrichment disabled, using NLP concepts only");
            nlp_concepts
        };

        if final_concepts.is_empty() {
            return Err(ApiError::NoConceptsExtracted);
        }

        Ok(final_concepts)
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Truncates text to at most `max_chars`, respecting UTF-8 char boundaries.
fn truncate_to_char_boundary(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        return text;
    }
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::inference::test_helpers::{MockEmbeddingBackend, MockLlmBackend};

    fn mock_embedding_backend() -> Arc<dyn EmbeddingBackend> {
        Arc::new(MockEmbeddingBackend {
            embedding: vec![0.5; 8],
            dim: 8,
            should_fail: false,
        })
    }

    fn make_model(response: &str) -> ConceptsModel {
        ConceptsModel::new(
            Arc::new(MockLlmBackend {
                response: response.to_string(),
                should_fail: false,
            }),
            mock_embedding_backend(),
            true,
        )
    }

    fn make_model_no_llm(response: &str) -> ConceptsModel {
        ConceptsModel::new(
            Arc::new(MockLlmBackend {
                response: response.to_string(),
                should_fail: false,
            }),
            mock_embedding_backend(),
            false,
        )
    }

    fn make_failing_model() -> ConceptsModel {
        ConceptsModel::new(
            Arc::new(MockLlmBackend {
                response: String::new(),
                should_fail: true,
            }),
            mock_embedding_backend(),
            true,
        )
    }

    fn sample_candidates() -> Vec<CandidateKeyword> {
        vec![
            CandidateKeyword {
                phrase: "machine learning".to_string(),
                score: 0.95,
            },
            CandidateKeyword {
                phrase: "neural networks".to_string(),
                score: 0.82,
            },
        ]
    }

    #[tokio::test]
    async fn test_generate_concepts_with_candidates() {
        let model = make_model(
            r#"{"concepts": [{"name": "artificial intelligence", "importance": 0.9}]}"#,
        );

        let candidates = sample_candidates();
        let result = model
            .generate_concepts("Machine learning uses neural networks for AI.", &candidates)
            .await;
        assert!(result.is_ok());
        let concepts = result.unwrap();
        // Should have NLP concepts (from embedding validation) + LLM themes
        assert!(!concepts.is_empty());
    }

    #[tokio::test]
    async fn test_generate_concepts_empty_candidates_llm_only() {
        // With no NLP candidates, NLP path produces nothing.
        // LLM enrichment should still produce themes (it gets empty candidate list + excerpt).
        let model = make_model(
            r#"{"concepts": [{"name": "machine learning", "importance": 0.9}]}"#,
        );

        let result = model
            .generate_concepts("Machine learning uses neural networks.", &[])
            .await;
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert!(!concepts.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_llm_fails_nlp_fallback() {
        let model = make_failing_model();
        let candidates = sample_candidates();

        let result = model
            .generate_concepts("Machine learning uses neural networks for AI.", &candidates)
            .await;
        // LLM fails but NLP concepts should still come through
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert!(!concepts.is_empty());
        // Should contain NLP candidates that passed embedding validation
        assert!(concepts
            .iter()
            .any(|c| c.concept == "machine learning" || c.concept == "neural networks"));
    }

    #[tokio::test]
    async fn test_hybrid_llm_disabled_nlp_only() {
        let model = make_model_no_llm(
            r#"{"concepts": [{"name": "should not appear", "importance": 0.9}]}"#,
        );

        let candidates = sample_candidates();
        let result = model
            .generate_concepts("Machine learning uses neural networks for AI.", &candidates)
            .await;
        assert!(result.is_ok());
        let concepts = result.unwrap();
        // LLM themes should not appear
        assert!(!concepts.iter().any(|c| c.concept == "should not appear"));
    }

    #[tokio::test]
    async fn test_hybrid_deduplication() {
        // LLM returns a concept that already exists in NLP candidates
        let model = make_model(
            r#"{"concepts": [{"name": "machine learning", "importance": 0.5}, {"name": "deep learning", "importance": 0.8}]}"#,
        );

        let candidates = sample_candidates();
        let result = model
            .generate_concepts("Machine learning uses neural networks.", &candidates)
            .await;
        assert!(result.is_ok());
        let concepts = result.unwrap();

        // "machine learning" should appear only once (NLP version preferred)
        let ml_count = concepts
            .iter()
            .filter(|c| c.concept.to_lowercase() == "machine learning")
            .count();
        assert_eq!(ml_count, 1);
    }

    #[tokio::test]
    async fn test_hybrid_degenerate_nlp_fallback() {
        // LLM returns degenerate output (missing closing brace)
        let model = ConceptsModel::new(
            Arc::new(MockLlmBackend {
                response: r#"{"concepts": [{"name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaa"#.to_string(),
                should_fail: false,
            }),
            mock_embedding_backend(),
            true,
        );

        let candidates = sample_candidates();
        let result = model
            .generate_concepts("Machine learning uses neural networks.", &candidates)
            .await;
        // Should fall back to NLP concepts
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert!(!concepts.is_empty());
    }

    #[test]
    fn test_parse_concepts_response_valid() {
        let model = make_model("");
        let response = r#"{"concepts": [{"name": "AI", "importance": 0.9}, {"name": "ML", "importance": 0.7}]}"#;
        let result = model.parse_concepts_response(response);
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert_eq!(concepts.len(), 2);
        assert_eq!(concepts[0].concept, "AI");
        assert!((concepts[0].importance - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_parse_concepts_response_mixed_formats() {
        let model = make_model("");
        let response = r#"{"concepts": [{"name": "AI", "importance": 0.9}, "plain string concept"]}"#;
        let result = model.parse_concepts_response(response);
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert_eq!(concepts.len(), 2);
        assert!(concepts.iter().any(|c| c.concept == "AI"));
        assert!(concepts
            .iter()
            .any(|c| c.concept == "plain string concept" && c.importance == 0.5));
    }

    #[test]
    fn test_parse_concepts_filters_long_phrases() {
        let model = make_model("");
        let response = r#"{"concepts": [{"name": "this is a very long phrase", "importance": 0.5}, {"name": "short", "importance": 0.8}]}"#;
        let result = model.parse_concepts_response(response);
        assert!(result.is_ok());
        let concepts = result.unwrap();
        // "this is a very long phrase" has 6 words, should be filtered
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].concept, "short");
    }

    #[test]
    fn test_build_candidate_hints_empty() {
        let hints = ConceptsModel::build_candidate_hints(&[]);
        assert!(hints.is_empty());
    }

    #[test]
    fn test_build_candidate_hints_truncates_to_20() {
        let candidates: Vec<CandidateKeyword> = (0..30)
            .map(|i| CandidateKeyword {
                phrase: format!("keyword_{}", i),
                score: 1.0 - (i as f32 * 0.03),
            })
            .collect();

        let hints = ConceptsModel::build_candidate_hints(&candidates);
        // Should only include first 20
        assert!(hints.contains("keyword_19"));
        assert!(!hints.contains("keyword_20"));
    }

    #[test]
    fn test_merge_nlp_and_llm_deduplicates() {
        let nlp = vec![
            Concept {
                concept: "AI".to_string(),
                importance: 0.8,
            },
            Concept {
                concept: "ML".to_string(),
                importance: 0.6,
            },
        ];
        let llm = vec![
            Concept {
                concept: "ai".to_string(),
                importance: 0.9,
            }, // same as AI (case-insensitive), NLP version kept
            Concept {
                concept: "deep learning".to_string(),
                importance: 0.7,
            },
        ];

        let merged = ConceptsModel::merge_nlp_and_llm(nlp, llm);
        assert_eq!(merged.len(), 3); // AI, ML, deep learning
        let ai = merged
            .iter()
            .find(|c| c.concept.to_lowercase() == "ai")
            .unwrap();
        // NLP version kept (importance 0.8, not LLM's 0.9)
        assert!((ai.importance - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_merge_caps_at_max() {
        let nlp: Vec<Concept> = (0..10)
            .map(|i| Concept {
                concept: format!("nlp_{}", i),
                importance: 0.5,
            })
            .collect();
        let llm: Vec<Concept> = (0..10)
            .map(|i| Concept {
                concept: format!("llm_{}", i),
                importance: 0.5,
            })
            .collect();

        let merged = ConceptsModel::merge_nlp_and_llm(nlp, llm);
        assert!(merged.len() <= MAX_CONCEPTS);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);
    }

    #[test]
    fn test_truncate_to_char_boundary() {
        assert_eq!(truncate_to_char_boundary("hello", 10), "hello");
        assert_eq!(truncate_to_char_boundary("hello world", 5), "hello");
        // UTF-8 multi-byte: "é" is 2 bytes
        let text = "café";
        let truncated = truncate_to_char_boundary(text, 4);
        assert!(truncated == "caf" || truncated == "café");
    }
}
