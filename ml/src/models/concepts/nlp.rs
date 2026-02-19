use keyword_extraction::rake::{Rake, RakeParams};
use keyword_extraction::tf_idf::{TfIdf, TfIdfParams, TextSplit};
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashMap;
use stop_words::{get, LANGUAGE};

#[derive(Debug, Clone)]
pub struct CandidateKeyword {
    pub phrase: String,
    pub score: f32,
}

pub struct KeywordExtractor {
    stop_words: Vec<String>,
    stemmer: Stemmer,
}

impl KeywordExtractor {
    pub fn new() -> Self {
        Self {
            stop_words: get(LANGUAGE::English)
                .iter()
                .map(|s| s.to_string())
                .collect(),
            stemmer: Stemmer::create(Algorithm::English),
        }
    }

    /// Slide a 2-word window over the text (skipping stop words) and score
    /// each bigram using the geometric mean of its constituent TF-IDF scores.
    /// Only bigrams appearing 2+ times are returned.
    fn extract_tfidf_bigrams(
        &self,
        text: &str,
        tfidf: &TfIdf,
        tfidf_max: f32,
    ) -> Vec<(String, f32)> {
        if tfidf_max <= 0.0 {
            return Vec::new();
        }

        // Collect content words (lowercased, non-stop)
        let content_words: Vec<&str> = text
            .split_whitespace()
            .filter(|w| {
                let lower = w.to_lowercase();
                lower.len() >= 2 && !self.stop_words.contains(&lower)
            })
            .collect();

        // Count bigram occurrences and track surface forms
        let mut bigram_counts: HashMap<String, usize> = HashMap::new();
        for window in content_words.windows(2) {
            let key = format!(
                "{} {}",
                window[0].to_lowercase(),
                window[1].to_lowercase()
            );
            *bigram_counts.entry(key).or_insert(0) += 1;
        }

        // Score bigrams that appear 2+ times
        bigram_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .filter_map(|(bigram, _)| {
                let parts: Vec<&str> = bigram.split_whitespace().collect();
                let s1 = tfidf.get_score(parts[0]);
                let s2 = tfidf.get_score(parts[1]);
                if s1 > 0.0 && s2 > 0.0 {
                    let geo_mean = (s1 * s2).sqrt();
                    let normalized = geo_mean / tfidf_max;
                    Some((bigram, normalized))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Stem each word of a phrase and join back together.
    fn stem_phrase(&self, phrase: &str) -> String {
        phrase
            .split_whitespace()
            .map(|w| self.stemmer.stem(w).to_string())
            .collect::<Vec<_>>()
            .join(" ")
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

            // TF-IDF bigrams: surface multi-word concepts independently of RAKE
            let bigrams = self.extract_tfidf_bigrams(text, &tfidf, tfidf_max);
            for (bigram, normalized) in bigrams {
                scored
                    .entry(bigram)
                    .and_modify(|s| *s = (*s + normalized * 0.5).min(1.0))
                    .or_insert(normalized * 0.8);
            }
        }

        // Filter: remove phrases > 3 words, phrases < 2 chars
        let filtered: Vec<(String, f32)> = scored
            .into_iter()
            .filter(|(phrase, _)| {
                let word_count = phrase.split_whitespace().count();
                word_count <= 3 && phrase.len() >= 2
            })
            .collect();

        // Stem-based deduplication: group by stemmed form, keep best surface form
        // (phrase, score) for each stem key
        let mut stem_groups: HashMap<String, (String, f32, usize)> = HashMap::new();
        for (phrase, score) in filtered {
            let stem_key = self.stem_phrase(&phrase);
            stem_groups
                .entry(stem_key)
                .and_modify(|(best_phrase, best_score, count)| {
                    *count += 1;
                    if score > *best_score {
                        *best_phrase = phrase.clone();
                        *best_score = score;
                    }
                })
                .or_insert((phrase, score, 1));
        }

        let mut candidates: Vec<CandidateKeyword> = stem_groups
            .into_values()
            .map(|(phrase, score, count)| {
                // Slight boost when multiple variants merged (confirms importance)
                let boosted = if count > 1 {
                    (score + 0.1).min(1.0)
                } else {
                    score
                };
                CandidateKeyword {
                    phrase,
                    score: boosted,
                }
            })
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
    fn test_stem_dedup_merges_variants() {
        let extractor = KeywordExtractor::new();
        // Text with morphological variants that share a stem
        // "learning", "learns", "learned" all stem to "learn"
        // ("learner" stems differently, so it's excluded)
        let text = "Learning is important for students who learn new skills. \
                    People who learns by studying become learned scholars. \
                    Teaching learning methods helps those who have learned before. \
                    Networks of neural networks process networked data through networking.";

        let candidates = extractor.extract_candidates(text, 20);
        let phrases: Vec<&str> = candidates.iter().map(|c| c.phrase.as_str()).collect();

        // "learning", "learns", "learned" all stem to "learn" — should collapse
        let learn_variants: Vec<&&str> = phrases
            .iter()
            .filter(|p| {
                let lower = p.to_lowercase();
                lower == "learning" || lower == "learns" || lower == "learned" || lower == "learn"
            })
            .collect();

        assert!(
            learn_variants.len() <= 1,
            "Expected stem dedup to merge learn/learning/learns/learned into at most 1, got {:?}",
            learn_variants
        );
    }

    #[test]
    fn test_bigrams_detected() {
        let extractor = KeywordExtractor::new();
        // Text with repeated bigrams that TF-IDF should catch
        let text = "Machine learning is transforming the technology industry. \
                    Machine learning models are used in many applications. \
                    Deep learning extends machine learning with neural networks. \
                    Neural networks power modern deep learning systems. \
                    The field of machine learning continues to grow rapidly.";

        let candidates = extractor.extract_candidates(text, 30);
        let phrases: Vec<&str> = candidates.iter().map(|c| c.phrase.as_str()).collect();

        assert!(
            phrases.iter().any(|p| p.contains("machine") && p.contains("learning")),
            "Expected 'machine learning' bigram in candidates: {:?}",
            phrases
        );
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
