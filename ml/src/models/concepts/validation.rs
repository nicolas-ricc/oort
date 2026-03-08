use super::model::Concept;

/// Detects degenerate LLM output (repetition loops, truncated responses).
pub fn detect_degenerate_output(raw: &str) -> bool {
    let trimmed = raw.trim();

    // Truncated mid-generation: JSON should end with '}'
    if !trimmed.ends_with('}') {
        return true;
    }

    // Sliding 8-char window: any substring repeating >5 times → degenerate
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 8 {
        let mut window_counts = std::collections::HashMap::new();
        for window in bytes.windows(8) {
            let count = window_counts.entry(window.to_vec()).or_insert(0usize);
            *count += 1;
            if *count > 5 {
                return true;
            }
        }
    }

    false
}

/// Filters out garbage concept names.
pub fn validate_concepts(concepts: Vec<Concept>) -> Vec<Concept> {
    concepts
        .into_iter()
        .filter(|c| {
            let name = &c.concept;

            // Filter concept names > 50 chars
            if name.len() > 50 {
                return false;
            }

            // Filter names with >30% non-alphabetic characters
            if !name.is_empty() {
                let alpha_count = name.chars().filter(|ch| ch.is_alphabetic() || ch.is_whitespace()).count();
                let total = name.chars().count();
                if (alpha_count as f32 / total as f32) < 0.7 {
                    return false;
                }
            }

            // Filter names where any single char is >50% of name length
            if name.len() >= 4 {
                let mut char_counts = std::collections::HashMap::new();
                let total = name.chars().count();
                for ch in name.chars() {
                    if ch.is_whitespace() {
                        continue;
                    }
                    let count = char_counts.entry(ch.to_lowercase().next().unwrap_or(ch)).or_insert(0usize);
                    *count += 1;
                    if *count * 2 > total {
                        return false;
                    }
                }
            }

            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_degenerate_repetitive() {
        let degenerate = r#"{"concepts": [{"name": "vancedonomyvancedonomyvancedonomyvancedonomyvancedonomyvancedonomyvancedonomyvancedonomy"}"#;
        // doesn't end with } either, but the repetition check should catch it
        assert!(detect_degenerate_output(degenerate));
    }

    #[test]
    fn test_detect_degenerate_truncated() {
        let truncated = r#"{"concepts": [{"name": "machine learning", "importance": 0.9"#;
        assert!(detect_degenerate_output(truncated));
    }

    #[test]
    fn test_detect_clean_output() {
        let clean = r#"{"concepts": [{"name": "machine learning", "importance": 0.9}]}"#;
        assert!(!detect_degenerate_output(clean));
    }

    #[test]
    fn test_validate_filters_garbage() {
        let concepts = vec![
            Concept { concept: "a]]]]]]]]]]]".to_string(), importance: 0.5 },
            Concept { concept: "this is a really really really really really really long concept name that exceeds fifty characters".to_string(), importance: 0.5 },
            Concept { concept: "aaaaaaaaa".to_string(), importance: 0.5 },
            Concept { concept: "valid concept".to_string(), importance: 0.8 },
        ];

        let filtered = validate_concepts(concepts);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].concept, "valid concept");
    }

    #[test]
    fn test_validate_keeps_valid() {
        let concepts = vec![
            Concept { concept: "machine learning".to_string(), importance: 0.9 },
            Concept { concept: "AI".to_string(), importance: 0.8 },
            Concept { concept: "neural networks".to_string(), importance: 0.7 },
        ];

        let filtered = validate_concepts(concepts);
        assert_eq!(filtered.len(), 3);
    }
}
