//! Full-Text Search Indexer with BM25-style Scoring and Snippet Highlighting (Pagefind / Algolia DocSearch equivalent).
//!
//! Indexes [`MarkdownDoc`] or arbitrary documents into a compact,
//! serializable inverted index with title boosting, prefix matching, and `<mark>` snippet extraction.

use crate::markdown::MarkdownDoc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// A document stored in the [`SearchIndex`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchDoc {
    pub id: String,
    pub title: String,
    pub url: String,
    pub body: String,
    pub tags: Vec<String>,
}

/// A ranked search result returned by [`SearchIndex::search`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub url: String,
    pub score: f64,
    /// Contextual excerpt with matched query terms wrapped in `<mark>...</mark>`.
    pub snippet_html: String,
}

/// Inverted full-text search index supporting exact and prefix matching, title boosting,
/// and JSON export for static or server-side search endpoints.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchIndex {
    docs: Vec<SearchDoc>,
    /// Inverted index mapping lowercase token -> Vec<(doc_index, term_frequency)>.
    postings: BTreeMap<String, Vec<(usize, u32)>>,
}

impl SearchIndex {
    /// Create a new empty [`SearchIndex`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a [`SearchIndex`] directly from a slice of parsed [`MarkdownDoc`] pages.
    pub fn from_markdown_docs(docs: &[MarkdownDoc], base_url_prefix: &str) -> Self {
        let mut index = Self::new();
        let prefix = base_url_prefix.trim_end_matches('/');
        for doc in docs {
            let plain = strip_html_tags(&doc.html);
            let url = format!("{}/{}", prefix, doc.slug.trim_start_matches('/'));
            index.add_document(SearchDoc {
                id: doc.slug.clone(),
                title: doc.title.clone(),
                url,
                body: plain,
                tags: doc.headings.iter().map(|h| h.text.clone()).collect(),
            });
        }
        index
    }

    /// Add a document to the search index.
    pub fn add_document(&mut self, doc: SearchDoc) {
        let doc_idx = self.docs.len();
        let mut term_counts: HashMap<String, u32> = HashMap::new();

        // Title tokens get 3x weight
        for tok in tokenize(&doc.title) {
            *term_counts.entry(tok).or_insert(0) += 3;
        }
        // Heading/tag tokens get 2x weight
        for tag in &doc.tags {
            for tok in tokenize(tag) {
                *term_counts.entry(tok).or_insert(0) += 2;
            }
        }
        // Body tokens get 1x weight
        for tok in tokenize(&doc.body) {
            *term_counts.entry(tok).or_insert(0) += 1;
        }

        for (term, freq) in term_counts {
            self.postings.entry(term).or_default().push((doc_idx, freq));
        }

        self.docs.push(doc);
    }

    /// Query the search index and return up to `limit` ranked [`SearchHit`] results.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let q_tokens = tokenize(query);
        if q_tokens.is_empty() || self.docs.is_empty() {
            return Vec::new();
        }

        let total_docs = self.docs.len() as f64;
        let mut scores: HashMap<usize, f64> = HashMap::new();

        for q_tok in &q_tokens {
            for (indexed_term, plist) in &self.postings {
                let match_boost = if indexed_term == q_tok {
                    2.0
                } else if indexed_term.starts_with(q_tok) {
                    1.0
                } else {
                    continue;
                };

                let df = plist.len() as f64;
                let idf = ((total_docs - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.1);

                for &(doc_idx, tf) in plist {
                    let tf_score = (tf as f64) / (tf as f64 + 1.2);
                    *scores.entry(doc_idx).or_insert(0.0) += tf_score * idf * match_boost;
                }
            }
        }

        let mut ranked: Vec<(usize, f64)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit.max(1));

        ranked
            .into_iter()
            .map(|(doc_idx, score)| {
                let doc = &self.docs[doc_idx];
                SearchHit {
                    id: doc.id.clone(),
                    title: doc.title.clone(),
                    url: doc.url.clone(),
                    score,
                    snippet_html: build_highlighted_snippet(&doc.body, &q_tokens, 140),
                }
            })
            .collect()
    }

    /// Serialize the search index to JSON for static export (`search-index.json`).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize a [`SearchIndex`] from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Return the number of indexed documents.
    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_lowercase())
        .collect()
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_highlighted_snippet(body: &str, query_tokens: &[String], max_chars: usize) -> String {
    let snippet: String = body.chars().take(max_chars).collect();
    let mut words = Vec::new();
    for word in snippet.split_whitespace() {
        let lower = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if query_tokens
            .iter()
            .any(|q| !lower.is_empty() && lower.starts_with(q))
        {
            words.push(format!("<mark>{word}</mark>"));
        } else {
            words.push(word.to_string());
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse_markdown;

    #[test]
    fn test_search_index_ranking_and_highlighting() {
        let doc1 =
            parse_markdown("# Reactive Signals\nFine-grained reactivity without Virtual DOM.");
        let doc2 = parse_markdown("# Database Migrations\nRun SQL migrations and query builders.");

        let index = SearchIndex::from_markdown_docs(&[doc1, doc2], "/docs");
        assert_eq!(index.doc_count(), 2);

        let hits = index.search("signals reactivity", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Reactive Signals");
        assert!(hits[0].snippet_html.contains("<mark>reactivity</mark>"));
    }
}
