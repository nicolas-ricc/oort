use regex::Regex;

/// Finds the largest byte index ≤ `index` that is a valid UTF-8 char boundary.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    let mut i = index.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Known abbreviations that end with a period but are not sentence endings.
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "inc", "ltd", "dept",
    "approx", "fig", "eq", "vol", "no", "gen", "gov", "eg", "ie",
];

/// Common TLDs that should not be treated as sentence-ending periods.
const TLDS: &[&str] = &["com", "org", "net", "io", "edu", "gov", "co"];

/// Returns true if the regex match is actually an abbreviation or TLD, not a real sentence end.
fn is_abbreviation(text: &str, match_start: usize) -> bool {
    // Walk backward from match_start to find the beginning of the word before the period.
    let before = &text[..=match_start];
    let word_start = before.rfind(|c: char| c.is_whitespace()).map_or(0, |p| p + 1);
    let word = &text[word_start..=match_start].to_lowercase();

    // Single-character word before period → likely an initial (J., U., etc.)
    if word.len() == 1 && word.chars().next().map_or(false, |c| c.is_alphabetic()) {
        return true;
    }

    ABBREVIATIONS.iter().any(|a| word == a) || TLDS.iter().any(|t| word == t)
}

/// Truncates `text` to at most `max_bytes` bytes at the best natural boundary.
///
/// If the text fits within the limit it is returned unchanged.
/// Otherwise the text is cut at the last sentence/paragraph/line/word boundary
/// before the limit and `"..."` is appended.
pub(crate) fn truncate_at_sentence_boundary(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }

    let safe_end = floor_char_boundary(text, max_bytes);
    if safe_end == 0 {
        return String::new();
    }

    let window = &text[..safe_end];
    let min_pos = max_bytes / 5; // 20% of budget

    // --- Tier A: Sentence boundary ---
    // Matches a lowercase letter, comma, or closing paren followed by sentence-ending
    // punctuation and then whitespace (or end of string).
    let sentence_re = Regex::new(r"[a-z,)][.!?](\s|$)").unwrap();
    let mut best_sentence: Option<usize> = None;
    for m in sentence_re.find_iter(window) {
        // The cut position is right after the punctuation mark (skip the leading char).
        let cut = m.start() + 2; // 1 byte for [a-z,)] + 1 byte for [.!?]
        if cut >= min_pos {
            if !is_abbreviation(window, m.start()) {
                best_sentence = Some(cut);
            }
        }
    }
    if let Some(pos) = best_sentence {
        return format!("{}...", &text[..pos]);
    }

    // --- Tier B: Paragraph break (\n\n) ---
    if let Some(pos) = window.rfind("\n\n") {
        if pos >= min_pos {
            return format!("{}...", &text[..pos]);
        }
    }

    // --- Tier C: Markdown heading (\n#) ---
    if let Some(pos) = window.rfind("\n#") {
        if pos >= min_pos {
            return format!("{}...", &text[..pos]);
        }
    }

    // --- Tier D: Single newline ---
    if let Some(pos) = window.rfind('\n') {
        if pos >= min_pos {
            return format!("{}...", &text[..pos]);
        }
    }

    // --- Tier E: Last whitespace (word boundary, no min position) ---
    if let Some(pos) = window.rfind(|c: char| c.is_whitespace()) {
        if pos > 0 {
            return format!("{}...", &text[..pos]);
        }
    }

    // --- Tier F: Raw char-safe cut ---
    format!("{}...", window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_passthrough() {
        let text = "Hello world.";
        let result = truncate_at_sentence_boundary(text, 500);
        assert_eq!(result, "Hello world.");
    }

    #[test]
    fn test_empty_text() {
        let result = truncate_at_sentence_boundary("", 500);
        assert_eq!(result, "");
    }

    #[test]
    fn test_cuts_at_last_sentence() {
        // Build a text with two sentences, the second one pushing past the limit.
        let s1 = format!("The quick brown fox jumped over the lazy dog. {}", "a".repeat(460));
        let result = truncate_at_sentence_boundary(&s1, 500);
        assert!(result.ends_with("..."));
        assert!(result.contains("dog."));
        assert!(result.len() <= 503); // 500 + "..."
    }

    #[test]
    fn test_question_and_exclamation() {
        let text = format!("Is this a question? {} More text here.", "x".repeat(490));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        assert!(result.contains("question?"));

        let text2 = format!("What an amazing thing! {} More text here.", "x".repeat(490));
        let result2 = truncate_at_sentence_boundary(&text2, 500);
        assert!(result2.ends_with("..."));
        assert!(result2.contains("thing!"));
    }

    #[test]
    fn test_abbreviation_skip() {
        // "Dr." should not be treated as a sentence boundary; should fall through to
        // a later tier or find the real sentence end.
        let text = format!(
            "Dr. Smith went to the store and bought groceries for the week. {}",
            "a".repeat(450)
        );
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // Should cut at "week." not at "Dr."
        assert!(result.contains("week."));
    }

    #[test]
    fn test_decimal_not_boundary() {
        // "3.14" — the digit before the period doesn't match [a-z,)] so it's not
        // picked up as a sentence boundary at all.
        let text = format!(
            "The value of pi is approximately 3.14 and that is a famous constant in mathematics. {}",
            "a".repeat(430)
        );
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // Should NOT cut right after "3.14"
        assert!(result.contains("mathematics."));
    }

    #[test]
    fn test_paragraph_break_fallback() {
        // No sentence punctuation, but has a paragraph break.
        let line = "x".repeat(200);
        let text = format!("{}\n\n{}", line, "y".repeat(400));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        assert!(!result.contains("yy")); // Cut at the paragraph break
    }

    #[test]
    fn test_markdown_heading_fallback() {
        // No sentence punctuation or paragraph break, but has a markdown heading.
        let line = "x".repeat(200);
        let text = format!("{}\n# Heading\n{}", line, "y".repeat(400));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        assert!(!result.contains("Heading"));
    }

    #[test]
    fn test_word_boundary_fallback() {
        // No punctuation, no newlines — just words.
        let text = format!("{} {}", "word".repeat(100), "tail".repeat(100));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // Should cut at a space, not mid-word
        let trimmed = result.trim_end_matches("...");
        assert!(!trimmed.ends_with("wor")); // Not mid-word
    }

    #[test]
    fn test_single_long_word_raw_cut() {
        let text = "a".repeat(1000);
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        assert_eq!(result.len(), 503); // 500 chars + "..."
    }

    #[test]
    fn test_multibyte_utf8_no_panic() {
        // Each emoji is 4 bytes. 200 emojis = 800 bytes, well over 500.
        let text = "🌍".repeat(200);
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // Verify it's valid UTF-8 (would panic on access if not)
        let _ = result.len();
    }

    #[test]
    fn test_min_position_threshold() {
        // "Hi. " is at position 3 — below 20% of 500 (100). It should be skipped
        // in favor of a later tier (word boundary further in the text).
        let text = format!("Hi. {} end of text", "word ".repeat(110));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // "Hi." is too early; should fall through to word boundary much deeper in the text
        let trimmed = result.trim_end_matches("...");
        assert!(trimmed.len() > 100);
    }

    #[test]
    fn test_ellipsis_handling() {
        // Interior dots in "..." — the second and third dots are preceded by "."
        // which is not in [a-z,)], so they won't match the sentence regex.
        // Use enough filler to push past the 500 byte limit.
        let text = format!("Something happened... {} more text here.", "x".repeat(500));
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // The ellipsis "..." in "happened..." should not be treated as a sentence
        // boundary (dots preceded by dots don't match [a-z,)]). The function
        // falls through to a word/whitespace boundary.
        assert!(result.contains("happened"));
    }

    #[test]
    fn test_url_period_skip() {
        // "example.com" — "e" before "." matches [a-z] but the next char is "c"
        // (not whitespace), so the regex won't match. Sentence detection skips it.
        let text = format!(
            "Visit example.com for more info and also check out the documentation that is available online for all users. {}",
            "a".repeat(420)
        );
        let result = truncate_at_sentence_boundary(&text, 500);
        assert!(result.ends_with("..."));
        // Should NOT cut at "example.com" — no whitespace after the period there.
        // Should cut at "users." which is a real sentence end.
        assert!(result.contains("users."));
    }
}
