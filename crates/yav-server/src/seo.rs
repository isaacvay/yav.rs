//! Sitemap & robots.txt generator for search engine crawlers.
//!
//! Provides builders and Axum response handlers to serve compliant XML sitemaps
//! and robots.txt rules.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

/// A single URL entry in an XML sitemap.
#[derive(Debug, Clone, PartialEq)]
pub struct SitemapEntry {
    /// Full URL location (e.g., `"https://example.com/about"`).
    pub loc: String,
    /// Last modification date (YYYY-MM-DD or ISO 8601).
    pub lastmod: Option<String>,
    /// Expected change frequency (`"always"`, `"hourly"`, `"daily"`, `"weekly"`, `"monthly"`, `"yearly"`, `"never"`).
    pub changefreq: Option<String>,
    /// Priority relative to other URLs on your site (0.0 to 1.0).
    pub priority: Option<f32>,
}

impl SitemapEntry {
    /// Create a new sitemap entry for the given location.
    pub fn new(loc: impl Into<String>) -> Self {
        Self {
            loc: loc.into(),
            lastmod: None,
            changefreq: None,
            priority: None,
        }
    }

    /// Set last modified date.
    pub fn lastmod(mut self, lastmod: impl Into<String>) -> Self {
        self.lastmod = Some(lastmod.into());
        self
    }

    /// Set change frequency.
    pub fn changefreq(mut self, freq: impl Into<String>) -> Self {
        self.changefreq = Some(freq.into());
        self
    }

    /// Set priority between 0.0 and 1.0.
    pub fn priority(mut self, priority: f32) -> Self {
        self.priority = Some(priority.clamp(0.0, 1.0));
        self
    }
}

/// An XML Sitemap collection.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sitemap {
    entries: Vec<SitemapEntry>,
}

impl Sitemap {
    /// Create an empty sitemap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entry to the sitemap.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, entry: SitemapEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add an entry to the sitemap (alias for `add`).
    pub fn add_entry(self, entry: SitemapEntry) -> Self {
        self.add(entry)
    }

    /// Add multiple entries.
    pub fn extend(mut self, entries: impl IntoIterator<Item = SitemapEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Render the XML document conforming to the sitemaps.org standard.
    pub fn to_xml(&self) -> String {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for entry in &self.entries {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", xml_escape(&entry.loc)));
            if let Some(ref lm) = entry.lastmod {
                xml.push_str(&format!("    <lastmod>{}</lastmod>\n", xml_escape(lm)));
            }
            if let Some(ref cf) = entry.changefreq {
                xml.push_str(&format!(
                    "    <changefreq>{}</changefreq>\n",
                    xml_escape(cf)
                ));
            }
            if let Some(p) = entry.priority {
                xml.push_str(&format!("    <priority>{:.1}</priority>\n", p));
            }
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");
        xml
    }

    /// Convert into an Axum [`Response`] with `Content-Type: application/xml`.
    pub fn into_response(&self) -> Response {
        let body = self.to_xml();
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            body,
        )
            .into_response()
    }
}

impl IntoResponse for Sitemap {
    fn into_response(self) -> Response {
        Sitemap::into_response(&self)
    }
}

/// Builder for `robots.txt` configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RobotsTxt {
    user_agents: Vec<RobotsRule>,
    sitemaps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct RobotsRule {
    user_agent: String,
    allow: Vec<String>,
    disallow: Vec<String>,
}

impl RobotsTxt {
    /// Create a new robots.txt configuration with standard `User-agent: *` allowing all.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add directives for a specific User-Agent.
    pub fn user_agent(
        mut self,
        user_agent: impl Into<String>,
        allow: impl IntoIterator<Item = impl Into<String>>,
        disallow: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.user_agents.push(RobotsRule {
            user_agent: user_agent.into(),
            allow: allow.into_iter().map(Into::into).collect(),
            disallow: disallow.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Allow all user-agents on all paths.
    pub fn allow_all(mut self) -> Self {
        self.user_agents.push(RobotsRule {
            user_agent: "*".into(),
            allow: vec!["/".into()],
            disallow: vec![],
        });
        self
    }

    /// Disallow all robots everywhere (e.g. for staging/preview environments).
    pub fn disallow_all(mut self) -> Self {
        self.user_agents.push(RobotsRule {
            user_agent: "*".into(),
            allow: vec![],
            disallow: vec!["/".into()],
        });
        self
    }

    /// Add a Sitemap index/URL declaration.
    pub fn sitemap(mut self, url: impl Into<String>) -> Self {
        self.sitemaps.push(url.into());
        self
    }

    /// Render standard robots.txt text format.
    pub fn to_txt(&self) -> String {
        let mut out = String::new();

        if self.user_agents.is_empty() {
            out.push_str("User-agent: *\nAllow: /\n\n");
        } else {
            for rule in &self.user_agents {
                out.push_str(&format!("User-agent: {}\n", rule.user_agent));
                for allow in &rule.allow {
                    out.push_str(&format!("Allow: {}\n", allow));
                }
                for disallow in &rule.disallow {
                    out.push_str(&format!("Disallow: {}\n", disallow));
                }
                out.push('\n');
            }
        }

        for sitemap in &self.sitemaps {
            out.push_str(&format!("Sitemap: {}\n", sitemap));
        }

        out
    }

    /// Convert into an Axum [`Response`] with `Content-Type: text/plain`.
    pub fn into_response(&self) -> Response {
        let body = self.to_txt();
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            body,
        )
            .into_response()
    }
}

impl IntoResponse for RobotsTxt {
    fn into_response(self) -> Response {
        RobotsTxt::into_response(&self)
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sitemap_xml() {
        let sitemap = Sitemap::new()
            .add(
                SitemapEntry::new("https://example.com/")
                    .priority(1.0)
                    .changefreq("daily"),
            )
            .add(SitemapEntry::new("https://example.com/about").priority(0.8));

        let xml = sitemap.to_xml();
        assert!(xml.contains("<loc>https://example.com/</loc>"));
        assert!(xml.contains("<priority>1.0</priority>"));
        assert!(xml.contains("<changefreq>daily</changefreq>"));
        assert!(xml.contains("<loc>https://example.com/about</loc>"));
    }

    #[test]
    fn test_robots_txt() {
        let robots = RobotsTxt::new()
            .user_agent("*", vec!["/public"], vec!["/admin", "/private"])
            .sitemap("https://example.com/sitemap.xml");

        let txt = robots.to_txt();
        assert!(txt.contains("User-agent: *"));
        assert!(txt.contains("Allow: /public"));
        assert!(txt.contains("Disallow: /admin"));
        assert!(txt.contains("Sitemap: https://example.com/sitemap.xml"));
    }
}
