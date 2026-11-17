//! Advanced internationalization based on Fluent, behind `i18n-advanced`.
//!
//! Where [`crate::i18n::I18n`] is a dependency-free key/value catalog, this
//! module uses `fluent-bundle` for real Fluent syntax (variables, attributes,
//! functions, selections) and `unic-langid` for standards-compliant language
//! tag parsing.
//!
//! Scope is deliberately limited: a single language identifier and a single
//! FTL resource per catalog. Fallback chains and multiple bundle locales are
//! left to the caller.

#![cfg(feature = "i18n-advanced")]

use fluent_bundle::FluentBundle;
pub use fluent_bundle::{FluentArgs, FluentResource};
use std::str::FromStr;
use unic_langid::LanguageIdentifier;

/// A Fluent catalog for one language.
pub struct FluentCatalog {
    /// Parsed BCP 47 language identifier of this catalog.
    pub lang: LanguageIdentifier,
    /// The Fluent bundle holding the parsed FTL resource.
    pub bundle: FluentBundle<FluentResource>,
}

impl FluentCatalog {
    /// Parse a language tag and an FTL resource into a catalog.
    ///
    /// Returns a descriptive error for an invalid language tag or malformed
    /// FTL source instead of panicking.
    pub fn new(lang: &str, ftl: &str) -> Result<Self, String> {
        let lang_id = LanguageIdentifier::from_str(lang)
            .map_err(|err| format!("invalid language identifier `{lang}`: {err:?}"))?;
        let resource = FluentResource::try_new(ftl.to_string())
            .map_err(|err| format!("invalid Fluent resource for `{lang}`: {err:?}"))?;

        let mut bundle = FluentBundle::new(vec![lang_id.clone()]);
        bundle
            .add_resource(resource)
            .map_err(|err| format!("could not add Fluent resource for `{lang}`: {err:?}"))?;

        Ok(Self {
            lang: lang_id,
            bundle,
        })
    }

    /// Format a message by id, optionally with Fluent arguments.
    ///
    /// Returns an error for a missing message, a value-less message, or when
    /// Fluent reports resolution errors. Note that Fluent wraps substituted
    /// variables in Unicode isolate marks (U+2068 / U+2069) by default.
    pub fn format(&self, id: &str, args: Option<&FluentArgs<'_>>) -> Result<String, String> {
        let message = self
            .bundle
            .get_message(id)
            .ok_or_else(|| format!("missing Fluent message: `{id}`"))?;
        let pattern = message
            .value()
            .ok_or_else(|| format!("Fluent message `{id}` has no value"))?;

        let mut errors = Vec::new();
        let formatted = self.bundle.format_pattern(pattern, args, &mut errors);
        if !errors.is_empty() {
            return Err(format!("errors while formatting `{id}`: {errors:?}"));
        }
        Ok(formatted.into_owned())
    }
}

/// Return a simplified CLDR-style plural category for `n` in a few languages.
///
/// Only `en`, `fr`, `de` and `es` are covered (region subtags such as `fr-CA`
/// are reduced to their base language). Unsupported languages fall back to
/// `"other"`.
///
/// This is intentionally minimal and honest about its limits: a complete
/// implementation needs the full CLDR rules (categories `zero`, `one`, `two`,
/// `few`, `many`, `other`), ordinal vs cardinal rules, and every locale.
/// Here `"many"` is only produced by the French rule for exact multiples of
/// one million, which is the one CLDR `many` case among these four languages.
pub fn choose_plural(lang: &str, n: u64) -> &'static str {
    let base = lang
        .split(['-', '_'])
        .next()
        .unwrap_or(lang)
        .to_ascii_lowercase();

    match base.as_str() {
        "en" | "de" | "es" => {
            if n == 1 {
                "one"
            } else {
                "other"
            }
        }
        "fr" => match n {
            0 | 1 => "one",
            value if value % 1_000_000 == 0 => "many",
            _ => "other",
        },
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_simple_message_and_variable() {
        let catalog = FluentCatalog::new("en", "welcome = Hello, { $name }!\nplain = Hi.\n")
            .expect("valid catalog");

        assert_eq!(catalog.format("plain", None).unwrap(), "Hi.");

        let mut args = FluentArgs::new();
        args.set("name", "Alice");
        let formatted = catalog.format("welcome", Some(&args)).unwrap();
        assert!(formatted.contains("Alice"));
        assert!(!formatted.contains("{ $name }"));
    }

    #[test]
    fn plural_categories_for_supported_languages() {
        assert_eq!(choose_plural("en", 1), "one");
        assert_eq!(choose_plural("en-US", 0), "other");
        assert_eq!(choose_plural("fr", 0), "one");
        assert_eq!(choose_plural("fr", 2), "other");
        assert_eq!(choose_plural("fr-CA", 1_000_000), "many");
        assert_eq!(choose_plural("de", 1), "one");
        assert_eq!(choose_plural("es", 3), "other");
        assert_eq!(choose_plural("zh", 1), "other");
    }

    #[test]
    fn invalid_locale_and_missing_message_are_errors() {
        assert!(FluentCatalog::new("not a valid tag!!", "hello = Hi").is_err());

        let catalog = FluentCatalog::new("en", "hello = Hi").expect("valid catalog");
        assert!(catalog.format("missing", None).is_err());
    }
}
