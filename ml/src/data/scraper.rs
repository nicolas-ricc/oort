use crate::error::ApiError;
use dom_smoothie::Readability;
use log::info;
use std::time::Duration;

pub struct ScrapedArticle {
    pub title: String,
    pub text_content: String,
}

pub struct ArticleScraper {
    client: reqwest::Client,
}

impl ArticleScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("Mozilla/5.0 (compatible; OortBot/1.0)")
            .build()
            .expect("Failed to build HTTP client");

        Self { client }
    }

    pub async fn scrape_url(&self, url: &str) -> Result<ScrapedArticle, ApiError> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ApiError::UrlFetchError(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        info!("Fetching URL: {}", url);

        let response = self.client.get(url).send().await.map_err(|e| {
            ApiError::UrlFetchError(format!("Failed to fetch URL: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::UrlFetchError(format!(
                "URL returned HTTP {}",
                status.as_u16()
            )));
        }

        let html = response.text().await.map_err(|e| {
            ApiError::UrlFetchError(format!("Failed to read response body: {}", e))
        })?;

        let mut readability = Readability::new(html.as_str(), Some(url), None).map_err(|e| {
            ApiError::ContentExtractionError(format!("Failed to parse HTML: {}", e))
        })?;

        let article = readability.parse().map_err(|e| {
            ApiError::ContentExtractionError(format!("Failed to extract article: {}", e))
        })?;

        let title = article.title;
        let text_content = article.text_content.to_string();

        if text_content.len() < 50 {
            return Err(ApiError::ContentExtractionError(
                "Extracted content is too short. The page may require JavaScript to render. Try downloading the article text and uploading it as a file instead.".to_string(),
            ));
        }

        info!(
            "Extracted article: \"{}\" ({} chars)",
            title,
            text_content.len()
        );

        Ok(ScrapedArticle {
            title,
            text_content,
        })
    }
}

pub fn derive_filename(title: &str, url: &str) -> String {
    let slug = if !title.is_empty() {
        slugify(title)
    } else {
        url.split('/')
            .filter(|s| !s.is_empty())
            .last()
            .map(|s| slugify(s))
            .unwrap_or_else(|| "article".to_string())
    };

    let truncated = if slug.len() > 80 {
        slug[..80].to_string()
    } else {
        slug
    };

    format!("{}.txt", truncated)
}

fn slugify(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_filename_from_title() {
        assert_eq!(
            derive_filename("My Great Article", "https://example.com/foo"),
            "my-great-article.txt"
        );
    }

    #[test]
    fn test_derive_filename_empty_title_uses_url() {
        assert_eq!(
            derive_filename("", "https://example.com/some-article-path"),
            "some-article-path.txt"
        );
    }

    #[test]
    fn test_derive_filename_special_characters() {
        let result = derive_filename("¿Qué vuelva la colimba?", "https://example.com");
        assert!(result.ends_with(".txt"));
        assert!(result.contains("vuelva"));
        assert!(result.contains("colimba"));
    }

    #[test]
    fn test_derive_filename_truncation() {
        let long_title = "a".repeat(200);
        let result = derive_filename(&long_title, "https://example.com");
        assert!(result.len() <= 84); // 80 + ".txt"
    }

    #[test]
    fn test_derive_filename_fallback() {
        assert_eq!(
            derive_filename("", "https://example.com/"),
            "example-com.txt"
        );
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("test--multiple---dashes"), "test-multiple-dashes");
    }
}
