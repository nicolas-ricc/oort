use crate::error::ApiError;
use dom_query::Document;
use dom_smoothie::Readability;
use log::info;
use regex::Regex;
use std::time::Duration;

const NOISE_SELECTORS: &[&str] = &[
    // Reading time
    "[class*='reading-time']",
    "[class*='read-time']",
    "[class*='readtime']",
    "[class*='min-read']",
    "[class*='time-to-read']",
    // Author/byline blocks
    "[class*='author-info']",
    "[class*='author-bio']",
    "[class*='author-box']",
    "[class*='byline']",
    // Post metadata containers
    "[class*='post-meta']",
    "[class*='entry-meta']",
    "[class*='article-meta']",
    "[class*='article-info']",
    "[class*='post-info']",
    "[class*='meta-info']",
    // Related/recommended posts
    "[class*='related-posts']",
    "[class*='related-articles']",
    "[class*='recommended']",
    "[class*='more-stories']",
    "[class*='read-next']",
    // Share/social
    "[class*='share-buttons']",
    "[class*='social-share']",
    "[class*='sharing']",
    // Newsletter/subscribe
    "[class*='newsletter']",
    "[class*='subscribe']",
    // Navigation
    "nav",
    "[role='navigation']",
    "[class*='breadcrumb']",
    // Tags/categories
    "[class*='tag-list']",
    "[class*='category-list']",
    "[class*='post-tags']",
    "[class*='article-tags']",
    // Comments
    "[class*='comments']",
    "[id*='comments']",
];

fn pre_clean_dom(doc: &Document) {
    for selector in NOISE_SELECTORS {
        let selection = doc.select(selector);
        selection.remove();
    }
}

fn clean_extracted_text(text: &str) -> String {
    let patterns: Vec<Regex> = vec![
        // Reading time variants
        Regex::new(r"(?i)\d+\s*min(ute)?s?\s+read").unwrap(),
        Regex::new(r"(?i)reading\s+time:\s*\d+\s*min(ute)?s?").unwrap(),
        // Standalone bylines (line-anchored)
        Regex::new(r"(?m)^\s*By\s+\p{Lu}\p{L}+(?:\s+\p{Lu}\p{L}+){1,3}\s*$").unwrap(),
        // Standalone date lines
        Regex::new(
            r"(?m)^\s*Published\s+on\s+(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}\s*$",
        )
        .unwrap(),
        // Share CTAs
        Regex::new(r"(?i)Share\s+this\b").unwrap(),
        Regex::new(r"(?i)Share\s+on\s+\w+").unwrap(),
        // Related section headers
        Regex::new(r"(?mi)^\s*Related\s+posts?\s*$").unwrap(),
        Regex::new(r"(?mi)^\s*You\s+may\s+also\s+like\s*$").unwrap(),
        Regex::new(r"(?mi)^\s*Read\s+next\s*$").unwrap(),
        // Newsletter CTAs
        Regex::new(r"(?mi)^\s*Subscribe\b.*$").unwrap(),
        Regex::new(r"(?mi)^\s*Sign\s+up\b.*$").unwrap(),
    ];

    let mut result = text.to_string();
    for pattern in &patterns {
        result = pattern.replace_all(&result, "").to_string();
    }

    // Collapse excessive blank lines
    let collapse = Regex::new(r"\n{3,}").unwrap();
    result = collapse.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}

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

        let doc = Document::from(html.as_str());
        pre_clean_dom(&doc);

        let mut readability =
            Readability::with_document(doc, Some(url), None).map_err(|e| {
                ApiError::ContentExtractionError(format!("Failed to parse HTML: {}", e))
            })?;

        let article = readability.parse().map_err(|e| {
            ApiError::ContentExtractionError(format!("Failed to extract article: {}", e))
        })?;

        let title = article.title;
        let text_content = clean_extracted_text(&article.text_content.to_string());

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

    #[test]
    fn test_clean_extracted_text_reading_time() {
        assert_eq!(
            clean_extracted_text("Some intro\n8 min read\nArticle body here"),
            "Some intro\n\nArticle body here"
        );
        assert_eq!(
            clean_extracted_text("5 minutes read"),
            ""
        );
        assert_eq!(
            clean_extracted_text("Reading time: 12 min"),
            ""
        );
    }

    #[test]
    fn test_clean_extracted_text_byline() {
        assert_eq!(
            clean_extracted_text("By John Smith\nThe article begins here."),
            "The article begins here."
        );
        assert_eq!(
            clean_extracted_text("By María García López\nContent follows."),
            "Content follows."
        );
    }

    #[test]
    fn test_clean_extracted_text_preserves_body() {
        let text = "The theory proposed by McLuhan describes how media affects cognition.";
        assert_eq!(clean_extracted_text(text), text);
    }

    #[test]
    fn test_clean_extracted_text_related_posts() {
        assert_eq!(
            clean_extracted_text("End of article.\nRelated posts\nSome other title"),
            "End of article.\n\nSome other title"
        );
    }

    #[test]
    fn test_clean_extracted_text_collapses_blank_lines() {
        assert_eq!(
            clean_extracted_text("Paragraph one.\n\n\n\n\nParagraph two."),
            "Paragraph one.\n\nParagraph two."
        );
    }

    #[test]
    fn test_pre_clean_dom() {
        let html = r#"<html><body>
            <div class="reading-time">5 min read</div>
            <div class="author-bio">About the author...</div>
            <article><p>Real article content here.</p></article>
            <div class="related-posts"><a href="/other">Other post</a></div>
        </body></html>"#;

        let doc = Document::from(html);
        pre_clean_dom(&doc);

        let cleaned_html = doc.html().to_string();
        assert!(!cleaned_html.contains("5 min read"));
        assert!(!cleaned_html.contains("About the author"));
        assert!(!cleaned_html.contains("Other post"));
        assert!(cleaned_html.contains("Real article content here."));
    }
}
