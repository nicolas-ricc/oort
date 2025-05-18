use crate::error::ApiError;
use log::{debug, info};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    options: OllamaOptions,
    format: OllamaFormat,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaFormat {
    r#type: String,
    properties: Properties,
    required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Schema {
    r#type: String,
    properties: Properties,
    required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Properties {
    concepts: ConceptsSchema,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConceptsSchema {
    r#type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Items {
    #[serde(rename = "type")]
    item_type: String,
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

        let lemmatized_words: Vec<String> = words
            .iter()
            .map(|word| word.to_string())
            .collect();

        lemmatized_words.join(" ")
    }

    pub async fn generate_concepts(&self, text: &str) -> Result<Vec<Concept>, ApiError> {
        let system_prompt = r#"You are a concept extractor that MUST:
        1. Extract key concepts from the text
        2. Output ONLY simple concepts separated by commas (NO bullet points, NO descriptions)
        4. Example output:
            Happy Prince, Golden Statue, Ruby Sword, Sapphire Eyes, Town Councillors
        
        DO NOT include:
        - Bullet points (-)
        - Descriptions or explanations
        - Newlines
        - Colons or semicolons"#;

        let truncated_text = if text.len() > 500 {
            format!("{}...", &text[..500])
        } else {
            text.to_string()
        };

        let template = format!(
            "Extract 5-10 key concepts from this text as simple words or short phrases separated by commas ONLY: {}",
            truncated_text
        );
        let format: OllamaFormat = OllamaFormat {
            r#type: "object".to_string(),
            properties: Properties {
                concepts: ConceptsSchema {
                    r#type: "array".to_string(),
                },
            },
            required: ["concepts".to_string()].to_vec(),
        };
        info!("Requesting concepts using model: {}", self.model);
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: template,
            system: system_prompt.to_string(),
            options: OllamaOptions { temperature: 0.0 },
            format: format,
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
        struct ConceptsResponse {
            concepts: Vec<String>,
        }

        let concepts_response: ConceptsResponse = serde_json::from_str(&ollama_response.response)
            .map_err(|e| {
            info!("Error parsing nested JSON: {}", e);
            ApiError::InternalError(format!("Failed to parse concepts JSON: {}", e))
        })?;

        let mut concepts: Vec<Concept> = Vec::new();
        for concept in concepts_response.concepts {
            let concept = concept.trim();
            if !concept.is_empty() && concept.split_whitespace().count() <= 3 {
                let lemmatized = self.lemmatize_concept(&concept);
                concepts.push(Concept {
                    concept: lemmatized,
                });
            }
        }

        debug!("Lemmatized concepts: {:?}", concepts);
        Ok(concepts)
    }
}
