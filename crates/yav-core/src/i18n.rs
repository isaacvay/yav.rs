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

    /// Return the default fallback locale code (e.g. `"en"`).
    pub fn default_locale(&self) -> &str {
        &self.default_locale
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

    /// Look up a translation key and interpolate parameters formatted as `{param_name}`
    /// in a single pass.
    ///
    /// Returns a plain `String` suitable for rendering inside `rsx!` (where [`crate::Node::Text`]
    /// automatically HTML-escapes text nodes). If you are injecting the translated string directly
    /// into raw HTML outside `rsx!`, use [`Self::translate_escaped`].
    pub fn translate(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        let raw = self.lookup_raw(locale, key);
        interpolate_single_pass(raw, params, false)
    }

    /// Look up a translation key and interpolate parameters with HTML-escaping applied to
    /// each parameter value in a single pass.
    pub fn translate_escaped(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        let raw = self.lookup_raw(locale, key);
        interpolate_single_pass(raw, params, true)
    }

    /// Alias for [`Self::translate`] (un-escaped parameter interpolation).
    pub fn translate_raw(&self, locale: &str, key: &str, params: &[(&str, &str)]) -> String {
        self.translate(locale, key, params)
    }

    fn lookup_raw<'a>(&'a self, locale: &str, key: &'a str) -> &'a str {
        self.dictionary
            .get(locale)
            .and_then(|m| m.get(key))
            .or_else(|| {
                self.dictionary
                    .get(&self.default_locale)
                    .and_then(|m| m.get(key))
            })
            .map(|s| s.as_str())
            .unwrap_or(key)
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

            let split_pos = match (trimmed.find('='), trimmed.find(':')) {
                (Some(i), Some(j)) => Some(i.min(j)),
                (Some(i), None) => Some(i),
                (None, Some(j)) => Some(j),
                (None, None) => None,
            };
            if let Some(pos) = split_pos {
                let k = &trimmed[..pos];
                let v = &trimmed[pos + 1..];
                let key = k.trim().trim_matches('"').trim_matches('\'');
                let val = v.trim().trim_matches('"').trim_matches('\'');
                map.insert(key.to_string(), val.to_string());
            }
        }
        self
    }

    /// Resolve the most appropriate locale given an optional user preference or `Accept-Language` header.
    ///
    /// Respects RFC 9110 `;q=` quality weights (sorting higher weights first and ignoring `;q=0`).
    pub fn resolve_locale(&self, accept_language: Option<&str>, preferred: Option<&str>) -> String {
        // 1. Direct explicit preference
        if let Some(pref) = preferred {
            let trimmed = pref.trim();
            if let Some(matched) = self.match_supported(trimmed) {
                return matched;
            }
            let lang = trimmed.split(['-', '_']).next().unwrap_or(trimmed);
            if let Some(matched) = self.match_supported(lang) {
                return matched;
            }
        }

        // 2. HTTP Accept-Language parsing with RFC 9110 quality weights (;q=0.8)
        if let Some(header_val) = accept_language {
            let mut candidates: Vec<(&str, f32)> = Vec::new();
            for entry in header_val.split(',') {
                let mut parts = entry.split(';');
                let tag = parts.next().unwrap_or("").trim();
                if tag.is_empty() {
                    continue;
                }
                let mut q: f32 = 1.0;
                for param in parts {
                    let param = param.trim();
                    if let Some(val_str) = param
                        .strip_prefix("q=")
                        .or_else(|| param.strip_prefix("Q="))
                    {
                        if let Ok(parsed) = val_str.trim().parse::<f32>() {
                            q = parsed.clamp(0.0, 1.0);
                        }
                    }
                }
                // RFC 9110: q=0 means "not acceptable"
                if q > 0.0 {
                    candidates.push((tag, q));
                }
            }

            // Stable sort by quality descending
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (tag, _) in candidates {
                if tag == "*" {
                    return self.default_locale.clone();
                }
                if let Some(matched) = self.match_supported(tag) {
                    return matched;
                }
                let base = tag.split(['-', '_']).next().unwrap_or(tag);
                if let Some(matched) = self.match_supported(base) {
                    return matched;
                }
            }
        }

        self.default_locale.clone()
    }

    fn match_supported(&self, loc: &str) -> Option<String> {
        self.supported_locales
            .iter()
            .find(|s| s.eq_ignore_ascii_case(loc))
            .cloned()
    }

    /// Serialize the internal translation dictionary to a JSON string without external dependencies.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        let mut loc_iter = self.dictionary.iter().peekable();
        while let Some((loc, entries)) = loc_iter.next() {
            out.push('"');
            out.push_str(&escape_json(loc));
            out.push_str("\":{");
            let mut ent_iter = entries.iter().peekable();
            while let Some((k, v)) = ent_iter.next() {
                out.push('"');
                out.push_str(&escape_json(k));
                out.push_str("\":\"");
                out.push_str(&escape_json(v));
                out.push('"');
                if ent_iter.peek().is_some() {
                    out.push(',');
                }
            }
            out.push('}');
            if loc_iter.peek().is_some() {
                out.push(',');
            }
        }
        out.push('}');
        out
    }

    /// Generate an embedded client-side script that enables dynamic language switching in the browser.
    ///
    /// Automatically translates:
    /// - Elements with `data-i18n="key"` (updates `textContent`)
    /// - Elements with `data-i18n-attr="attr:key, attr2:key2"` (updates attributes like `placeholder`, `title`, `aria-label`)
    ///
    /// Exposes `window.yavI18n.setLocale(loc)` and synchronizes `document.documentElement.lang` and the `yav_locale` cookie.
    pub fn client_script(&self, current_locale: &str) -> String {
        let json = self.to_json();
        let safe_locale = escape_json(current_locale);
        format!(
            r##"<script id="yav-i18n-payload" type="application/json">{json}</script>
<script>(function(){{var p=document.getElementById('yav-i18n-payload');var d=p?JSON.parse(p.textContent||'{{}}'):{{}};window.yavI18n={{locale:"{safe_locale}",setLocale:function(loc){{if(!d[loc])return;this.locale=loc;document.documentElement.lang=loc;document.cookie='yav_locale='+encodeURIComponent(loc)+';path=/;max-age=31536000';var dict=d[loc];document.querySelectorAll('[data-i18n]').forEach(function(el){{var k=el.getAttribute('data-i18n');if(dict[k])el.textContent=dict[k]}});document.querySelectorAll('[data-i18n-attr]').forEach(function(el){{var s=el.getAttribute('data-i18n-attr');s.split(',').forEach(function(pair){{var pts=pair.split(':');if(pts.length===2){{var attr=pts[0].trim();var key=pts[1].trim();if(dict[key])el.setAttribute(attr,dict[key])}} }})}})}};}})();</script>"##,
            json = json,
            safe_locale = safe_locale,
        )
    }

    /// Return [`Self::client_script`] as a [`crate::Node::Raw`].
    pub fn client_script_node(&self, current_locale: &str) -> crate::Node {
        crate::Node::raw(self.client_script(current_locale))
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Interpolates `{placeholder}` tokens in `template` in a single pass so parameter values
/// containing `{other}` are never re-expanded by subsequent parameters.
fn interpolate_single_pass(template: &str, params: &[(&str, &str)], escape_values: bool) -> String {
    if params.is_empty() || !template.contains('{') {
        return template.to_string();
    }

    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;

    while let Some(open_idx) = rest.find('{') {
        out.push_str(&rest[..open_idx]);
        let after_open = &rest[open_idx + 1..];

        if let Some(close_idx) = after_open.find('}') {
            let candidate_key = &after_open[..close_idx];
            if !candidate_key.contains('{') {
                if let Some(&(_, val)) = params.iter().find(|&&(k, _)| k == candidate_key) {
                    if escape_values {
                        out.push_str(&crate::escape::escape_html(val));
                    } else {
                        out.push_str(val);
                    }
                    rest = &after_open[close_idx + 1..];
                    continue;
                }
            }
        }

        out.push('{');
        rest = after_open;
    }

    out.push_str(rest);
    out
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
