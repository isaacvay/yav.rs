//! # YAV i18n Static Extractor & Translation Linter
//!
//! Scans Rust source code for `t!("...")` and `t_strict!("...")` macros,
//! audits translation catalog coverage, and flags missing or dead translation keys.

use std::collections::{BTreeSet, HashSet};

/// Location of an internationalized translation key detected in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I18nKeyOccurrence {
    pub key: String,
    pub file_path: String,
    pub line_number: usize,
}

/// Comprehensive audit report summarizing translation completeness.
#[derive(Debug, Clone, PartialEq)]
pub struct I18nReport {
    pub total_keys_used: usize,
    pub missing_keys: Vec<String>,
    pub unused_keys: Vec<String>,
    pub coverage_pct: f64,
}

impl I18nReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.missing_keys.is_empty()
    }
}

/// Scan a Rust source string for `t!("...")` and `t_strict!("...")` calls.
#[must_use]
pub fn scan_source_code(file_path: &str, content: &str) -> Vec<I18nKeyOccurrence> {
    let mut occurrences = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        extract_macro_keys(line, "t!(", file_path, line_num, &mut occurrences);
        extract_macro_keys(line, "t_strict!(", file_path, line_num, &mut occurrences);
    }

    occurrences
}

fn extract_macro_keys(
    line: &str,
    macro_prefix: &str,
    file_path: &str,
    line_num: usize,
    out: &mut Vec<I18nKeyOccurrence>,
) {
    let mut remaining = line;
    while let Some(pos) = remaining.find(macro_prefix) {
        // Boundary check: preceding character must not be alphanumeric or underscore
        let valid_start = if pos == 0 {
            true
        } else {
            let prev_char = remaining[..pos].chars().next_back().unwrap_or(' ');
            !prev_char.is_alphanumeric() && prev_char != '_'
        };

        let after_prefix = &remaining[pos + macro_prefix.len()..];
        if valid_start {
            let trimmed = after_prefix.trim_start();
            if let Some(inside_quote) = trimmed.strip_prefix('"') {
                if let Some(end_quote) = inside_quote.find('"') {
                    let key = &inside_quote[..end_quote];
                    out.push(I18nKeyOccurrence {
                        key: key.to_string(),
                        file_path: file_path.to_string(),
                        line_number: line_num,
                    });
                    remaining = &inside_quote[end_quote + 1..];
                    continue;
                }
            }
        }
        remaining = after_prefix;
    }
}

/// Audit detected source occurrences against known translation catalog keys.
#[must_use]
pub fn lint_catalog(occurrences: &[I18nKeyOccurrence], catalog_keys: &[&str]) -> I18nReport {
    let mut used_keys: BTreeSet<String> = BTreeSet::new();
    for occ in occurrences {
        used_keys.insert(occ.key.clone());
    }

    let catalog_set: HashSet<&str> = catalog_keys.iter().copied().collect();

    let mut missing_keys = Vec::new();
    for key in &used_keys {
        if !catalog_set.contains(key.as_str()) {
            missing_keys.push(key.clone());
        }
    }

    let mut unused_keys = Vec::new();
    for &cat_key in catalog_keys {
        if !used_keys.contains(cat_key) {
            unused_keys.push(cat_key.to_string());
        }
    }

    let total = used_keys.len();
    let found = total.saturating_sub(missing_keys.len());
    let coverage_pct = if total == 0 {
        100.0
    } else {
        (found as f64 / total as f64) * 100.0
    };

    I18nReport {
        total_keys_used: total,
        missing_keys,
        unused_keys,
        coverage_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_source_code_extracts_keys() {
        let rust_code = r#"
            let title = t!("home.welcome_message");
            let err = t_strict!("errors.not_found");
            let note = "Just a string t!(\"not_real\")";
            println!("{}", t!("actions.submit"));
        "#;

        let results = scan_source_code("src/views/home.rs", rust_code);
        let keys: Vec<&str> = results.iter().map(|o| o.key.as_str()).collect();

        assert_eq!(
            keys,
            vec!["home.welcome_message", "errors.not_found", "actions.submit"]
        );
        assert_eq!(results[0].line_number, 2);
    }

    #[test]
    fn test_lint_catalog_reports_missing_and_unused() {
        let occurrences = vec![
            I18nKeyOccurrence {
                key: "nav.home".to_string(),
                file_path: "app.rs".to_string(),
                line_number: 10,
            },
            I18nKeyOccurrence {
                key: "nav.about".to_string(),
                file_path: "app.rs".to_string(),
                line_number: 12,
            },
            I18nKeyOccurrence {
                key: "nav.contact".to_string(),
                file_path: "app.rs".to_string(),
                line_number: 14,
            },
        ];

        let catalog = &["nav.home", "nav.about", "legacy.unused_key"];
        let report = lint_catalog(&occurrences, catalog);

        assert_eq!(report.total_keys_used, 3);
        assert_eq!(report.missing_keys, vec!["nav.contact"]);
        assert_eq!(report.unused_keys, vec!["legacy.unused_key"]);
        assert!((report.coverage_pct - 66.66).abs() < 0.1);
        assert!(!report.is_clean());
    }
}
