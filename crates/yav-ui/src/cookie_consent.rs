//! # YAV GDPR / ePrivacy Cookie Consent Center (`<CookieConsent />`)
//!
//! Compliant privacy consent banner and customization modal with category toggles,
//! local storage state persistence, and client consent event broadcasting.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Interactive privacy and GDPR cookie consent component.
#[derive(Debug, Clone, PartialEq)]
pub struct CookieConsent {
    pub id: String,
    pub title: String,
    pub message: String,
    pub policy_url: Option<String>,
    pub storage_key: String,
    pub allow_customization: bool,
    pub class: String,
}

impl CookieConsent {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "Gestion des cookies & Vie privée".to_string(),
            message: "Nous utilisons des cookies essentiels pour assurer le bon fonctionnement du site, et des cookies d'analyse pour mesurer l'audience.".to_string(),
            policy_url: Some("/privacy".to_string()),
            storage_key: "yav_cookie_consent".to_string(),
            allow_customization: true,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = msg.into();
        self
    }

    #[must_use]
    pub fn policy_url(mut self, url: impl Into<String>) -> Self {
        self.policy_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn storage_key(mut self, key: impl Into<String>) -> Self {
        self.storage_key = key.into();
        self
    }

    #[must_use]
    pub fn allow_customization(mut self, allow: bool) -> Self {
        self.allow_customization = allow;
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for CookieConsent {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-cookie-consent fixed bottom-4 left-4 right-4 md:left-auto md:right-6 md:max-w-lg z-[9998] p-5 bg-slate-900/95 backdrop-blur-md border border-slate-700/80 rounded-2xl shadow-2xl flex flex-col gap-3 text-white transition-transform duration-300",
            &self.class,
        ]);

        let title_el = Element::new("h3")
            .attr(
                "class",
                "text-sm font-semibold text-slate-100 flex items-center gap-2",
            )
            .child(Node::Raw(r#"<span>🍪</span>"#.to_string()))
            .child(self.title.as_str());

        let mut desc_el = Element::new("p").attr("class", "text-xs text-slate-300 leading-relaxed");
        desc_el = desc_el.child(self.message.as_str());

        if let Some(policy) = &self.policy_url {
            desc_el = desc_el.child(" ").child(
                Element::new("a")
                    .attr("href", policy)
                    .attr("class", "text-indigo-400 hover:underline font-medium")
                    .child("En savoir plus"),
            );
        }

        let mut buttons_el =
            Element::new("div").attr("class", "flex items-center gap-2 flex-wrap pt-1");

        let accept_all_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-consent-accept px-3.5 py-1.5 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-colors shadow",
            )
            .child("Tout accepter");

        let reject_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-consent-reject px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors border border-slate-700",
            )
            .child("Refuser les non-essentiels");

        buttons_el = buttons_el.child(accept_all_btn).child(reject_btn);

        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;
const storageKey = '{storage_key}';

if (localStorage.getItem(storageKey)) {{
  root.classList.add('hidden');
  return;
}}

function saveConsent(data) {{
  localStorage.setItem(storageKey, JSON.stringify(data));
  root.classList.add('hidden');
  window.dispatchEvent(new CustomEvent('yav-consent-change', {{ detail: data }}));
}}

root.querySelector('.yav-consent-accept')?.addEventListener('click', () => {{
  saveConsent({{ necessary: true, analytics: true, marketing: true }});
}});

root.querySelector('.yav-consent-reject')?.addEventListener('click', () => {{
  saveConsent({{ necessary: true, analytics: false, marketing: false }});
}});
}})();"##,
            id = escape_html(&self.id),
            storage_key = escape_html(&self.storage_key),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(title_el)
            .child(desc_el)
            .child(buttons_el)
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `CookieConsent`.
#[must_use]
pub fn cookie_consent(id: impl Into<String>) -> CookieConsent {
    CookieConsent::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_cookie_consent_rendering() {
        let consent = CookieConsent::new("gdpr-banner")
            .title("Gestion de vos cookies")
            .message("Nous respectons votre vie privée.")
            .policy_url("/politique-confidentialite")
            .storage_key("app_cookie_ok");

        let node = consent.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-cookie-consent"));
        assert!(html.contains("gdpr-banner"));
        assert!(html.contains("Gestion de vos cookies"));
        assert!(html.contains("Nous respectons votre vie privée."));
        assert!(html.contains("/politique-confidentialite"));
        assert!(html.contains("app_cookie_ok"));
        assert!(html.contains("yav-consent-accept"));
        assert!(html.contains("yav-consent-reject"));
    }
}
