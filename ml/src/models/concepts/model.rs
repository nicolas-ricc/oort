use crate::error::ApiError;
use crate::models::concepts::nlp::CandidateKeyword;
use crate::models::inference::{GenerationParams, LlmBackend};
use futures::future::join_all;
use log::{debug, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Max text chars in a single LLM call.
/// Conservative budget assuming 4096 context minimum:
/// (4096 - 512 output - 500 prompt overhead) * 3.5 chars/tok ≈ 10,794
const MAX_TEXT_CHARS: usize = 10_000;

/// Chunk size for MapReduce splitting.
const CHUNK_SIZE_CHARS: usize = 6000;

/// Overlap between consecutive chunks.
const CHUNK_OVERLAP_CHARS: usize = 500;

/// Max concurrent LLM calls (leverages prefix caching for shared prompts).
const LLM_CONCURRENCY: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept: String,
    pub importance: f32,
}

pub struct ConceptsModel {
    backend: Arc<dyn LlmBackend>,
}

impl ConceptsModel {
    pub fn new(backend: Arc<dyn LlmBackend>) -> Self {
        Self { backend }
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

        let mut hints = String::from("\n\nCandidate keywords (from statistical analysis of full text):\n");
        for candidate in nlp_candidates.iter().take(20) {
            hints.push_str(&format!("- \"{}\" (score: {:.2})\n", candidate.phrase, candidate.score));
        }
        hints
    }

    /// Core LLM call: sends text to the backend and returns extracted concepts.
    async fn call_llm(&self, text: &str, candidate_hints: &str) -> Result<Vec<Concept>, ApiError> {
        let system_prompt = format!(
            r#"You are a concept extractor that identifies the core intellectual themes in a text.
Given a text and statistically-identified candidate keywords:

1. Identify the central themes and ideas (not just mentioned terms)
2. Validate which candidates represent meaningful concepts in context
3. Add important conceptual themes the statistics missed
4. Prefer domain-specific concepts over generic ones (e.g., "reinforcement learning" over "method")
5. Rate each concept's importance: 1.0 = central thesis, 0.7 = major supporting theme, 0.3 = mentioned topic
6. Return 5-15 concepts total
7. Each concept should be a word or short phrase (1-3 words)
{candidate_hints}
Output ONLY valid JSON matching the required schema."#
        );

        let user_prompt = format!(
            "Extract the key concepts from this text. Rate each concept's importance from 0.0 to 1.0:\n\n{}",
            text
        );

        // JSON schema for structured output: { concepts: [{ name: string, importance: number }] }
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

        info!("Requesting concepts using model: {}", self.backend.model_id());

        let params = GenerationParams {
            temperature: 0.0,
            max_tokens: Some(512),
            json_schema: Some(json_schema),
        };

        let response = self
            .backend
            .generate(&system_prompt, &user_prompt, &params)
            .await?;

        info!("Raw response: {}", response);

        self.parse_concepts_response(&response)
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

    /// Deduplicates concepts from multiple chunks by normalized name, keeping highest importance.
    fn merge_chunk_concepts(chunk_results: Vec<Vec<Concept>>) -> Vec<Concept> {
        let mut best_by_name: HashMap<String, Concept> = HashMap::new();

        for concepts in chunk_results {
            for concept in concepts {
                let key = concept.concept.to_lowercase();
                match best_by_name.entry(key) {
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        if concept.importance > e.get().importance {
                            *e.get_mut() = concept;
                        }
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(concept);
                    }
                }
            }
        }

        best_by_name.into_values().collect()
    }

    pub async fn generate_concepts(
        &self,
        text: &str,
        nlp_candidates: &[CandidateKeyword],
    ) -> Result<Vec<Concept>, ApiError> {
        let candidate_hints = Self::build_candidate_hints(nlp_candidates);

        if text.len() < MAX_TEXT_CHARS {
            // Short text: single LLM call with full text (no truncation)
            self.call_llm(text, &candidate_hints).await
        } else {
            // Long text: MapReduce — split into chunks, extract from each, merge
            info!("Text length {} exceeds {} chars, using MapReduce chunking", text.len(), MAX_TEXT_CHARS);
            let chunks = super::truncation::chunk_text(text, CHUNK_SIZE_CHARS, CHUNK_OVERLAP_CHARS);
            let total = chunks.len();
            info!("Split into {} chunks", total);

            let semaphore = Arc::new(Semaphore::new(LLM_CONCURRENCY));
            let futures: Vec<_> = chunks
                .iter()
                .enumerate()
                .map(|(i, chunk)| {
                    let sem = Arc::clone(&semaphore);
                    let chunk = chunk.clone();
                    let hints = candidate_hints.clone();
                    async move {
                        let _permit = sem.acquire().await.unwrap();
                        info!("Processing chunk {}/{} ({} chars)", i + 1, total, chunk.len());
                        self.call_llm(&chunk, &hints).await
                    }
                })
                .collect();
            let results = join_all(futures).await;
            let chunk_concepts: Result<Vec<Vec<Concept>>, ApiError> =
                results.into_iter().collect();
            let merged = Self::merge_chunk_concepts(chunk_concepts?);

            debug!("Merged {} unique concepts from {} chunks", merged.len(), total);
            Ok(merged)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::inference::test_helpers::MockLlmBackend;

    fn make_model(response: &str) -> ConceptsModel {
        ConceptsModel::new(Arc::new(MockLlmBackend {
            response: response.to_string(),
            should_fail: false,
        }))
    }

    fn make_failing_model() -> ConceptsModel {
        ConceptsModel::new(Arc::new(MockLlmBackend {
            response: String::new(),
            should_fail: true,
        }))
    }

    #[tokio::test]
    async fn test_generate_concepts_short_text() {
        let model = make_model(r#"{"concepts": [{"name": "machine learning", "importance": 0.9}, {"name": "neural networks", "importance": 0.7}]}"#);

        let result = model.generate_concepts("Machine learning uses neural networks.", &[]).await;
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert_eq!(concepts.len(), 2);
        assert!(concepts.iter().any(|c| c.concept == "machine learning"));
        assert!(concepts.iter().any(|c| c.concept == "neural networks"));
    }

    #[tokio::test]
    async fn test_generate_concepts_long_text_mapreduce() {
        // Generate text exceeding MAX_TEXT_CHARS (10,000)
        let long_text = "Machine learning is transforming the world. ".repeat(300);
        assert!(long_text.len() > 10000);

        let model = make_model(
            r#"{"concepts": [{"name": "machine learning", "importance": 0.9}]}"#,
        );

        let result = model.generate_concepts(&long_text, &[]).await;
        assert!(result.is_ok());
        let concepts = result.unwrap();
        // MapReduce should merge duplicates from chunks
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].concept, "machine learning");
    }

    #[tokio::test]
    async fn test_generate_concepts_llm_failure() {
        let model = make_failing_model();
        let result = model.generate_concepts("Some text.", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_concepts_empty_response() {
        let model = make_model(r#"{"concepts": []}"#);
        let result = model.generate_concepts("Some text.", &[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_generate_concepts_malformed_json() {
        let model = make_model("not valid json at all");
        let result = model.generate_concepts("Some text.", &[]).await;
        assert!(result.is_err());
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
        assert!(concepts.iter().any(|c| c.concept == "plain string concept" && c.importance == 0.5));
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
    fn test_merge_chunk_concepts_deduplicates() {
        let chunk1 = vec![
            Concept { concept: "AI".to_string(), importance: 0.8 },
            Concept { concept: "ML".to_string(), importance: 0.6 },
        ];
        let chunk2 = vec![
            Concept { concept: "ai".to_string(), importance: 0.9 }, // same as AI, higher importance
            Concept { concept: "robotics".to_string(), importance: 0.5 },
        ];

        let merged = ConceptsModel::merge_chunk_concepts(vec![chunk1, chunk2]);
        assert_eq!(merged.len(), 3); // AI, ML, robotics
        let ai = merged.iter().find(|c| c.concept.to_lowercase() == "ai").unwrap();
        assert!((ai.importance - 0.9).abs() < 0.01); // kept higher importance
    }
}
