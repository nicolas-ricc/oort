use crate::error::ApiError;
use crate::models::concepts::nlp::CandidateKeyword;
use futures::future::join_all;
use log::{debug, info};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept: String,
    pub importance: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    options: OllamaOptions,
    format: serde_json::Value,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaOptions {
    temperature: f32,
    num_ctx: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct ConceptsModel {
    base_url: String,
    client: Client,
    model: String,
}

impl ConceptsModel {
    pub fn new(base_url: &str) -> Self {
        let client: Client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
            model: "phi3.5".to_string(),
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

        let mut hints = String::from("\n\nCandidate keywords (from statistical analysis of full text):\n");
        for candidate in nlp_candidates.iter().take(20) {
            hints.push_str(&format!("- \"{}\" (score: {:.2})\n", candidate.phrase, candidate.score));
        }
        hints
    }

    fn compute_num_ctx(text_len: usize) -> u32 {
        let estimated_tokens = text_len / 3;
        std::cmp::max(4096, (estimated_tokens + 1024) as u32)
    }

    /// Core LLM call: sends text to Ollama and returns extracted concepts.
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

        let template = format!(
            "Extract the key concepts from this text. Rate each concept's importance from 0.0 to 1.0:\n\n{}",
            text
        );

        // JSON schema for structured output: { concepts: [{ name: string, importance: number }] }
        let format: serde_json::Value = serde_json::json!({
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

        let num_ctx = Self::compute_num_ctx(text.len());
        info!("Requesting concepts using model: {} (num_ctx: {})", self.model, num_ctx);

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: template,
            system: system_prompt,
            options: OllamaOptions { temperature: 0.0, num_ctx },
            format,
            stream: false,
        };

        let url = format!("{}/api/generate", self.base_url);
        debug!("Sending request to: {}", url);
        info!("Request body: {:?}", request);

        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                info!("Error requesting concepts: {}", e);
                ApiError::RequestError(e)
            })?;

        let body: String = response.text().await.map_err(|e| {
            info!("Error extracting response text: {}", e);
            ApiError::RequestError(e)
        })?;

        info!("Raw response: {}", body);

        let ollama_response: OllamaResponse = serde_json::from_str(&body).map_err(|e| {
            info!("Error parsing response JSON: {}", e);
            ApiError::InternalError(format!("JSON parse error: {}", e))
        })?;

        self.parse_concepts_response(&ollama_response.response)
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

        if text.len() < 6000 {
            // Short text: single LLM call with full text (no truncation)
            self.call_llm(text, &candidate_hints).await
        } else {
            // Long text: MapReduce — split into chunks, extract from each, merge
            info!("Text length {} exceeds 6000 chars, using MapReduce chunking", text.len());
            let chunks = super::truncation::chunk_text(text, 2000, 200);
            info!("Split into {} chunks", chunks.len());

            let futures: Vec<_> = chunks
                .iter()
                .map(|chunk| self.call_llm(chunk, &candidate_hints))
                .collect();
            let results = join_all(futures).await;
            let chunk_concepts: Result<Vec<Vec<Concept>>, ApiError> =
                results.into_iter().collect();
            let merged = Self::merge_chunk_concepts(chunk_concepts?);

            debug!("Merged {} unique concepts from {} chunks", merged.len(), chunks.len());
            Ok(merged)
        }
    }
}
