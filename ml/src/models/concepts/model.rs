use crate::error::ApiError;
use crate::models::concepts::nlp::CandidateKeyword;
use log::{debug, info};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
            .timeout(Duration::from_secs(60))
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

    pub async fn generate_concepts(
        &self,
        text: &str,
        nlp_candidates: &[CandidateKeyword],
    ) -> Result<Vec<Concept>, ApiError> {
        let candidate_hints = Self::build_candidate_hints(nlp_candidates);

        let system_prompt = format!(
            r#"You are a concept extractor. Given a text and statistically-identified candidate keywords:
1. Validate which candidates are meaningful concepts in context
2. Add important concepts the statistics missed
3. Rate each concept's importance from 0.0 to 1.0 (1.0 = central theme, 0.0 = barely relevant)
4. Return 5-10 concepts total
5. Each concept should be a simple word or short phrase (1-3 words)
{candidate_hints}
Output ONLY valid JSON matching the required schema."#
        );

        let truncated_text = super::truncation::truncate_at_sentence_boundary(text, 500);

        let template = format!(
            "Extract 5-10 key concepts from this text. Rate each concept's importance from 0.0 to 1.0:\n\n{}",
            truncated_text
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

        info!("Requesting concepts using model: {}", self.model);
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: template,
            system: system_prompt,
            options: OllamaOptions { temperature: 0.0 },
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
            serde_json::from_str(&ollama_response.response).map_err(|e| {
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
}
