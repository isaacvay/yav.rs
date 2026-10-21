//! Lightweight internationalization (i18n) engine.
//!
//! Provides locale resolution, translation dictionary storage, and variable interpolation
//! with zero external dependencies.

use std::collections::HashMap;

/// An internationalization catalog and translator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct I18n {
    default_locale: String,
    supported_locales: Vec<String>,
    dictionary: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    /// Create a new i18n instance with a default fallback locale (e.g. `"en"` or `"fr"`).
    pub fn new(default_locale: impl Into<String>) -> Self {
        let def = default_locale.into();
        Self {
            supported_locales: vec![def.clone()],
            default_locale: def,
            dictionary: HashMap::new(),
        }
    }

    /// Declare supported locales (e.g. `["en", "fr", "es"]`).
    pub fn supported_locales(
        mut self,
        locales: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.supported_locales = locales.into_iter().map(Into::into).collect();
        if !self.supported_locales.contains(&self.default_locale) {
            self.supported_locales.push(self.default_locale.clone());
        }
        self
    }

    /// Add a single translation key/value for a locale.
    pub fn add_translation(
        mut self,
        locale: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let loc = locale.into();
        self.dictionary
            .entry(loc)
            .or_default()
            .insert(key.into(), value.into());
        self
    }

    /// Add multiple translation pairs for a locale.
    pub fn add_translations(
        mut self,
        locale: impl Into<String>,
        entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        let loc = locale.into();
        let map = self.dictionary.entry(loc).or_default();
        for (k, v) in entries {
            map.insert(k.into(), v.into());
        }
        self
    }

    /// Look up a translation key, falling back to the default locale, then the key itself.
    pub fn t(&self, locale: &str, key: &str) -> String {
        self.translate(locale, key, &[])
    }

    /// Look up a translation key and interpolate parameters formatted as `{param_name}`.
    ///
    /// All interpolated values are **HTML-escaped** by default to prevent XSS.
    /// Use [`Self::translate_raw`] if you need to inject pre-escaped or trusted HTML.
    pub fn translate(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        let raw = self
            .dictionary
            .get(locale)
            .and_then(|m| m.get(key))
            .or_else(|| {
                self.dictionary
                    .get(&self.default_locale)
                    .and_then(|m| m.get(key))
            })
            .map(|s| s.as_str())
            .unwrap_or(key);

        if params.is_empty() {
            return raw.to_string();
        }

        let mut out = raw.to_string();
        for (k, v) in params {
            let placeholder = format!("{{{k}}}");
            let escaped = crate::escape::escape_html(v);
            out = out.replace(&placeholder, &escaped);
        }
        out
    }

    /// Look up a translation key and interpolate parameters **without HTML escaping**.
    ///
    /// **⚠️ Security Warning**: Only use this with trusted content. Never pass user input
    /// directly — use [`Self::translate`] instead, which escapes values automatically.
    pub fn translate_raw(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        let raw = self
            .dictionary
            .get(locale)
            .and_then(|m| m.get(key))
            .or_else(|| {
                self.dictionary
                    .get(&self.default_locale)
                    .and_then(|m| m.get(key))
            })
            .map(|s| s.as_str())
            .unwrap_or(key);

        if params.is_empty() {
            return raw.to_string();
        }

        let mut out = raw.to_string();
        for (k, v) in params {
            let placeholder = format!("{{{k}}}");
            out = out.replace(&placeholder, v);
        }
        out
    }

    /// Check if a key is present in the dictionary for a locale (or default fallback).
    pub fn has_key(&self, locale: &str, key: &str) -> bool {
        self.dictionary
            .get(locale)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
            || self
                .dictionary
                .get(&self.default_locale)
                .map(|m| m.contains_key(key))
                .unwrap_or(false)
    }

    /// Translate with pluralization rules based on a numeric `count`.
    ///
    /// Evaluates `{key_prefix}_zero` (when count is 0), `{key_prefix}_one` (when count is 1),
    /// or `{key_prefix}_other`. Automatically injects `{count}` as an interpolation parameter.
    pub fn plural(
        &self,
        locale: &str,
        count: usize,
        key_prefix: &str,
        params: &[(&str, &str)],
    ) -> String {
        let zero_key = format!("{key_prefix}_zero");
        let suffix = match count {
            0 if self.has_key(locale, &zero_key) => "_zero",
            1 => "_one",
            _ => "_other",
        };
        let target_key = format!("{key_prefix}{suffix}");
        let mut all_params = params.to_vec();
        let count_str = count.to_string();
        all_params.push(("count", &count_str));
        self.translate(locale, &target_key, &all_params)
    }

    /// Load translation entries from a structured string (key = value or key: value, one per line).
    ///
    /// Ignores blank lines and comments starting with `#`.
    pub fn load_catalog_str(mut self, locale: impl Into<String>, catalog_content: &str) -> Self {
        let loc = locale.into();
        if !self.supported_locales.contains(&loc) {
            self.supported_locales.push(loc.clone());
        }
        let map = self.dictionary.entry(loc).or_default();

        for line in catalog_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((k, v)) = trimmed.split_once('=').or_else(|| trimmed.split_once(':')) {
                let key = k.trim().trim_matches('"').trim_matches('\'');
                let val = v.trim().trim_matches('"').trim_matches('\'');
                map.insert(key.to_string(), val.to_string());
            }
        }
        self
    }

    /// Resolve the most appropriate locale given an optional user preference or `Accept-Language` header.
    pub fn resolve_locale(&self, accept_language: Option<&str>, preferred: Option<&str>) -> String {
        // 1. Direct explicit preference
        if let Some(pref) = preferred {
            if self.is_supported(pref) {
                return pref.to_string();
            }
            let lang = pref.split('-').next().unwrap_or(pref);
            if self.is_supported(lang) {
                return lang.to_string();
            }
        }

        // 2. HTTP Accept-Language parsing
        if let Some(header_val) = accept_language {
            for entry in header_val.split(',') {
                let tag = entry.split(';').next().unwrap_or("").trim();
                if self.is_supported(tag) {
                    return tag.to_string();
                }
                let base = tag.split('-').next().unwrap_or(tag);
                if self.is_supported(base) {
                    return base.to_string();
                }
            }
        }

        self.default_locale.clone()
    }

    fn is_supported(&self, loc: &str) -> bool {
        self.supported_locales
            .iter()
            .any(|s| s.eq_ignore_ascii_case(loc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i18n_translation_and_fallback() {
        let i18n = I18n::new("en")
            .supported_locales(["en", "fr"])
            .add_translation("en", "welcome", "Welcome, {name}!")
            .add_translation("fr", "welcome", "Bienvenue, {name} !")
            .add_translation("en", "only_in_en", "Only English");

        // French translation with interpolation
        let fr_msg = i18n.translate("fr", "welcome", &[("name", "Alice")]);
        assert_eq!(fr_msg, "Bienvenue, Alice !");

        // Fallback to default English
        let fallback_msg = i18n.t("fr", "only_in_en");
        assert_eq!(fallback_msg, "Only English");

        // Missing key returns the key itself
        let missing = i18n.t("fr", "unknown_key");
        assert_eq!(missing, "unknown_key");
    }

    #[test]
    fn test_locale_resolution() {
        let i18n = I18n::new("en").supported_locales(["en", "fr", "es"]);

        let resolved = i18n.resolve_locale(Some("fr-FR,fr;q=0.9,en;q=0.8"), None);
        assert_eq!(resolved, "fr");

        let pref = i18n.resolve_locale(Some("fr-FR,fr;q=0.9"), Some("es"));
        assert_eq!(pref, "es");

        let default_res = i18n.resolve_locale(Some("zh-CN,zh;q=0.9"), None);
        assert_eq!(default_res, "en");
    }

    #[test]
    fn test_pluralization() {
        let i18n = I18n::new("en")
            .supported_locales(["en"])
            .add_translation("en", "items_zero", "No items")
            .add_translation("en", "items_one", "1 item")
            .add_translation("en", "items_other", "{count} items");

        assert_eq!(i18n.plural("en", 0, "items", &[]), "No items");
        assert_eq!(i18n.plural("en", 1, "items", &[]), "1 item");
        assert_eq!(i18n.plural("en", 5, "items", &[]), "5 items");
    }

    #[test]
    fn test_load_catalog_str() {
        let catalog = r#"
        # Greeting translations
        app.title = Yav Web Framework
        app.description : Build modern apps with Rust
        user.greeting = Hello, {name}!
        "#;

        let i18n = I18n::new("en").load_catalog_str("en", catalog);

        assert_eq!(i18n.t("en", "app.title"), "Yav Web Framework");
        assert_eq!(
            i18n.t("en", "app.description"),
            "Build modern apps with Rust"
        );
        assert_eq!(
            i18n.translate("en", "user.greeting", &[("name", "Bob")]),
            "Hello, Bob!"
        );
    }
}
