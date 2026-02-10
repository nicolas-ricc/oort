use crate::error::ApiError;
use log::info;

const TEXTS_REPO: &str = "oort-cdn";
const MAIN_REPO: &str = "oort";

pub struct GitHubCDN {
    token: String,
    repo: String,
    owner: String,
    client: reqwest::Client,
}

impl GitHubCDN {
    pub fn new() -> Self {
        let token = if let Ok(token_file) = std::env::var("GITHUB_TOKEN_FILE") {
            std::fs::read_to_string(token_file)
                .unwrap_or_else(|_| std::env::var("GITHUB_TOKEN").unwrap_or_default())
                .trim()
                .to_string()
        } else {
            std::env::var("GITHUB_TOKEN").unwrap_or_default()
        };
        
        Self {
            token,
            repo: TEXTS_REPO.to_string(),
            owner: std::env::var("GITHUB_OWNER").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn upload_text(&self, content: &str, filename: &str) -> Result<String, ApiError> {
        info!("GitHub upload config - owner: '{}', repo: '{}', token_present: {}", 
              self.owner, self.repo, !self.token.is_empty());
        
        if self.owner.is_empty() || self.token.is_empty() {
            return Err(ApiError::InternalError("GitHub owner or token not configured".to_string()));
        }
        
        let encoded_content = base64::encode(content);
        
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/texts/{}",
            self.owner, self.repo, filename
        );
        
        info!("GitHub upload URL: {}", url);

        // First, check if the file already exists to get its SHA
        let existing_response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "OortML")
            .send()
            .await?;

        let mut payload = serde_json::json!({
            "message": format!("Add text: {}", filename),
            "content": encoded_content,
            "branch": "main"
        });

        // If file exists, add the SHA to the payload for updating
        if existing_response.status().is_success() {
            if let Ok(existing_content) = existing_response.json::<serde_json::Value>().await {
                if let Some(sha) = existing_content.get("sha").and_then(|s| s.as_str()) {
                    payload["sha"] = serde_json::Value::String(sha.to_string());
                    payload["message"] = serde_json::Value::String(format!("Update text: {}", filename));
                    info!("File exists, updating with SHA: {}", sha);
                }
            }
        } else {
            info!("File doesn't exist, creating new file");
        }

        let response = self.client
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "OortML")
            .json(&payload)
            .send()
            .await?;
        
        info!("GitHub upload response status: {}", response.status());
        if response.status().is_success() {
            Ok(format!(
                "https://cdn.jsdelivr.net/gh/{}/{}@main/texts/{}",
                self.owner, self.repo, filename
            ))
        } else {
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error".to_string());
            info!("GitHub upload failed with body: {}", error_body);
            Err(ApiError::InternalError(format!("GitHub upload failed: {}", error_body)))
        }
    }
}