use keyword_extraction::rake::{Rake, RakeParams};
use keyword_extraction::tf_idf::{TfIdf, TfIdfParams, TextSplit};
use std::collections::HashMap;
use stop_words::{get, LANGUAGE};

#[derive(Debug, Clone)]
pub struct CandidateKeyword {
    pub phrase: String,
    pub score: f32,
}

pub struct KeywordExtractor {
    stop_words: Vec<String>,
}

impl KeywordExtractor {
    pub fn new() -> Self {
        Self {
            stop_words: get(LANGUAGE::English)
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    pub fn extract_candidates(&self, text: &str, max_candidates: usize) -> Vec<CandidateKeyword> {
        if text.len() < 50 {
            return Vec::new();
        }

        let mut scored: HashMap<String, f32> = HashMap::new();

        // Run RAKE — extracts multi-word keyphrases with scores
        let rake = Rake::new(RakeParams::WithDefaultsAndPhraseLength(
            text,
            &self.stop_words,
            Some(3),
        ));
        let rake_results = rake.get_ranked_phrases_scores(max_candidates * 2);

        // Also get single-word RAKE keywords
        let rake_keywords = rake.get_ranked_keyword_scores(max_candidates * 2);

        // Collect all RAKE scores for normalization
        let rake_max = rake_results
            .iter()
            .map(|(_, s)| *s)
            .chain(rake_keywords.iter().map(|(_, s)| *s))
            .fold(0.0_f32, f32::max);

        if rake_max > 0.0 {
            for (phrase, score) in &rake_results {
                let key = phrase.to_lowercase();
                let normalized = score / rake_max;
                scored
                    .entry(key)
                    .and_modify(|s| *s = s.max(normalized))
                    .or_insert(normalized);
            }
            for (word, score) in &rake_keywords {
                let key = word.to_lowercase();
                let normalized = score / rake_max;
                scored
                    .entry(key)
                    .and_modify(|s| *s = s.max(normalized))
                    .or_insert(normalized);
            }
        }

        // Run TF-IDF — extracts single-term importance scores
        let tfidf = TfIdf::new(TfIdfParams::TextBlock(
            text,
            &self.stop_words,
            None,
            TextSplit::Sentences,
        ));
        let tfidf_results = tfidf.get_ranked_word_scores(max_candidates * 2);

        let tfidf_max = tfidf_results
            .iter()
            .map(|(_, s)| *s)
            .fold(0.0_f32, f32::max);

        if tfidf_max > 0.0 {
            for (word, score) in &tfidf_results {
                let key = word.to_lowercase();
                let normalized = score / tfidf_max;
                // Boost terms found by both methods
                scored
                    .entry(key)
                    .and_modify(|s| *s = (*s + normalized * 0.5).min(1.0))
                    .or_insert(normalized * 0.8);
            }
        }

        // Filter: remove phrases > 3 words, phrases < 2 chars
        let mut candidates: Vec<CandidateKeyword> = scored
            .into_iter()
            .filter(|(phrase, _)| {
                let word_count = phrase.split_whitespace().count();
                word_count <= 3 && phrase.len() >= 2
            })
            .map(|(phrase, score)| CandidateKeyword { phrase, score })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        candidates.truncate(max_candidates);
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_candidates_basic() {
        let extractor = KeywordExtractor::new();
        let text = "Machine learning is a subset of artificial intelligence. \
                    Machine learning algorithms build models based on sample data. \
                    Neural networks are a key component of deep learning. \
                    Deep learning uses neural networks with many layers.";

        let candidates = extractor.extract_candidates(text, 20);

        assert!(!candidates.is_empty());
        // Repeated terms like "machine learning" and "neural networks" should appear
        let phrases: Vec<&str> = candidates.iter().map(|c| c.phrase.as_str()).collect();
        assert!(
            phrases.iter().any(|p| p.contains("learning") || p.contains("neural")),
            "Expected common terms to appear in candidates: {:?}",
            phrases
        );
    }

    #[test]
    fn test_extract_candidates_short_text() {
        let extractor = KeywordExtractor::new();
        let candidates = extractor.extract_candidates("Too short", 20);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_extract_candidates_empty_text() {
        let extractor = KeywordExtractor::new();
        let candidates = extractor.extract_candidates("", 20);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_scores_normalized() {
        let extractor = KeywordExtractor::new();
        let text = "Rust is a systems programming language focused on safety, concurrency, \
                    and performance. Rust prevents memory errors without garbage collection. \
                    The Rust compiler enforces strict ownership rules for memory safety.";

        let candidates = extractor.extract_candidates(text, 20);

        for candidate in &candidates {
            assert!(
                candidate.score >= 0.0 && candidate.score <= 1.0,
                "Score {} for '{}' is not in 0.0-1.0 range",
                candidate.score,
                candidate.phrase
            );
        }
    }

    #[test]
    fn test_max_phrase_length() {
        let extractor = KeywordExtractor::new();
        let text = "The quick brown fox jumps over the lazy dog. \
                    Natural language processing is a field of computer science. \
                    Text analysis involves many different techniques and algorithms.";

        let candidates = extractor.extract_candidates(text, 20);

        for candidate in &candidates {
            let word_count = candidate.phrase.split_whitespace().count();
            assert!(
                word_count <= 3,
                "Phrase '{}' has {} words, expected <= 3",
                candidate.phrase,
                word_count
            );
        }
    }
}
