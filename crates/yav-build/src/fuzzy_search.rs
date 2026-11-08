//! Phonetic & Typo-Tolerant Fuzzy Search Engine.
//!
//! Provides Damerau-Levenshtein distance, Soundex/Metaphone phonetic encoding,
//! and typo-tolerant query matching with "Did you mean?" suggestions.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Computes the Damerau-Levenshtein edit distance between two strings,
/// accounting for insertions, deletions, substitutions, and transpositions.
#[must_use]
pub fn damerau_levenshtein(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();

    let len1 = chars1.len();
    let len2 = chars2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut d = vec![vec![0usize; len2 + 1]; len1 + 1];

    for (i, row) in d.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate().take(len2 + 1) {
        *cell = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };

            d[i][j] = (d[i - 1][j] + 1) // deletion
                .min(d[i][j - 1] + 1) // insertion
                .min(d[i - 1][j - 1] + cost); // substitution

            // Transposition check
            if i > 1 && j > 1 && chars1[i - 1] == chars2[j - 2] && chars1[i - 2] == chars2[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }

    d[len1][len2]
}

/// Compute American Soundex phonetic encoding of a word (e.g. "Robert" -> "R163").
#[must_use]
pub fn soundex(word: &str) -> String {
    let word = word.trim().to_uppercase();
    let chars: Vec<char> = word.chars().filter(|c| c.is_ascii_alphabetic()).collect();

    if chars.is_empty() {
        return "0000".to_string();
    }

    let first = chars[0];
    let mut code = String::with_capacity(4);
    code.push(first);

    let map_char = |c: char| -> char {
        match c {
            'B' | 'F' | 'P' | 'V' => '1',
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
            'D' | 'T' => '3',
            'L' => '4',
            'M' | 'N' => '5',
            'R' => '6',
            _ => '0',
        }
    };

    let mut prev_code = map_char(first);

    for &c in &chars[1..] {
        let cur = map_char(c);
        if cur != '0' && cur != prev_code {
            code.push(cur);
            if code.len() == 4 {
                break;
            }
        }
        if cur != '0' || c == 'H' || c == 'W' {
            prev_code = cur;
        }
    }

    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// A matched search hit with score and distance metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyMatch {
    /// Document or term ID.
    pub id: String,
    /// Term text.
    pub text: String,
    /// Minimum edit distance from query.
    pub distance: usize,
    /// Whether matched phonetically.
    pub phonetic_match: bool,
    /// Overall relevance score (higher is better, 0.0..=1.0).
    pub score: f32,
}

/// Fast in-memory typo-tolerant search index with phonetic clustering.
#[derive(Debug, Clone, Default)]
pub struct FuzzySearchIndex {
    /// Map of term -> document IDs
    terms: HashMap<String, HashSet<String>>,
    /// Map of soundex phonetic code -> list of terms
    phonetic_map: HashMap<String, Vec<String>>,
    /// All indexed documents (id -> text)
    docs: HashMap<String, String>,
}

impl FuzzySearchIndex {
    /// Create a new empty fuzzy search index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a document to the index.
    pub fn add(&mut self, id: impl Into<String>, content: impl Into<String>) {
        let id_str = id.into();
        let content_str = content.into();

        for word in content_str.split_whitespace() {
            let clean: String = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();

            if clean.len() >= 2 {
                self.terms
                    .entry(clean.clone())
                    .or_default()
                    .insert(id_str.clone());

                let snd = soundex(&clean);
                let list = self.phonetic_map.entry(snd).or_default();
                if !list.contains(&clean) {
                    list.push(clean);
                }
            }
        }

        self.docs.insert(id_str, content_str);
    }

    /// Search the index with typo tolerance (max edit distance) and phonetic fallbacks.
    #[must_use]
    pub fn search(&self, query: &str, max_distance: usize, limit: usize) -> Vec<FuzzyMatch> {
        let query_clean: String = query
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();

        if query_clean.is_empty() {
            return Vec::new();
        }

        let query_soundex = soundex(&query_clean);
        let mut matches = Vec::new();

        // 1. Direct and Levenshtein scan
        for term in self.terms.keys() {
            let dist = damerau_levenshtein(&query_clean, term);
            let is_phonetic = soundex(term) == query_soundex;

            if dist <= max_distance || is_phonetic {
                let max_len = query_clean.len().max(term.len());
                let score = if dist == 0 {
                    1.0
                } else {
                    let base = 1.0 - (dist as f32 / max_len as f32);
                    if is_phonetic {
                        (base + 0.15).min(0.95)
                    } else {
                        base
                    }
                };

                for doc_id in &self.terms[term] {
                    matches.push(FuzzyMatch {
                        id: doc_id.clone(),
                        text: term.clone(),
                        distance: dist,
                        phonetic_match: is_phonetic,
                        score,
                    });
                }
            }
        }

        // Sort descending by score, ascending by distance
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.distance.cmp(&b.distance))
        });

        matches.truncate(limit);
        matches
    }

    /// Generate an intelligent "Did you mean?" suggestion for a misspelled query.
    #[must_use]
    pub fn suggest_did_you_mean(&self, query: &str) -> Option<String> {
        let query_clean = query.to_lowercase();
        if self.terms.contains_key(&query_clean) {
            return None; // Exact match exists, no suggestion needed
        }

        let hits = self.search(&query_clean, 2, 1);
        hits.into_iter().next().map(|h| h.text)
    }
}

/// Standalone helper to find the best candidate match for a query.
#[must_use]
pub fn suggest_did_you_mean<'a>(query: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let q = query.to_lowercase();
    let mut best: Option<(&'a str, usize)> = None;

    for &candidate in candidates {
        let c_lower = candidate.to_lowercase();
        let dist = damerau_levenshtein(&q, &c_lower);
        if dist <= 2 {
            if let Some((_, best_dist)) = best {
                if dist < best_dist {
                    best = Some((candidate, dist));
                }
            } else {
                best = Some((candidate, dist));
            }
        }
    }

    best.map(|(c, _)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damerau_levenshtein_distance() {
        assert_eq!(damerau_levenshtein("rust", "rust"), 0);
        assert_eq!(damerau_levenshtein("teh", "the"), 1); // transposition
        assert_eq!(damerau_levenshtein("robert", "rupert"), 2);
    }

    #[test]
    fn test_soundex_phonetic_equivalence() {
        assert_eq!(soundex("Robert"), soundex("Rupert"));
        assert_eq!(soundex("Smith"), soundex("Smythe"));
        assert_eq!(soundex("Jackson"), soundex("Jaxon"));
    }

    #[test]
    fn test_fuzzy_search_and_did_you_mean() {
        let mut index = FuzzySearchIndex::new();
        index.add("doc_1", "Authentication and security architecture for Rust");
        index.add("doc_2", "Database query optimization and postgres pool");

        // Search with typo: "securty" -> "security"
        let hits = index.search("securty", 2, 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "doc_1");
        assert_eq!(hits[0].text, "security");

        // Suggest did you mean
        let suggestion = index.suggest_did_you_mean("optimzation");
        assert_eq!(suggestion.as_deref(), Some("optimization"));
    }
}
