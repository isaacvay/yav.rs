//! Dynamic OpenGraph Social Image Generator (`/_yav/og`).
//!
//! Provides `@vercel/og` / Next.js `ImageResponse` parity in pure Rust:
//! - Dynamically renders beautiful 1200x630 social share cards for Twitter, LinkedIn, and Facebook.
//! - Includes gradient backgrounds, decorative grid meshes, automatic multi-line title wrapping,
//!   brand badges, and metadata readouts.
//! - Emits `image/svg+xml; charset=utf-8` with `Cache-Control: public, max-age=31536000, immutable`
//!   and SHA-256 `ETag` headers for instant edge/CDN caching.
//!
//! ## Crawler Compatibility & Rasterization
//!
//! *Note:* Modern platforms (such as Telegram, modern web clients, and feed readers) render
//! vector SVG cards directly. However, certain legacy crawlers (e.g. older WhatsApp or Facebook bot
//! versions) mandate PNG/JPEG images. For applications requiring 100% legacy crawler compatibility,
//! pair this endpoint with an edge renderer (e.g. `resvg` or headless Chromium) or pre-rasterize
//! SVG cards into PNG.

use axum::extract::Query;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Query parameters accepted by `/_yav/og?title=...&subtitle=...&badge=...&theme=...`.
#[derive(Debug, Clone, Deserialize)]
pub struct OgImageParams {
    /// Main headline text (e.g. article title or product name).
    pub title: String,
    /// Optional subtitle or brief description.
    #[serde(default)]
    pub subtitle: Option<String>,
    /// Optional category or organization badge (e.g. "Blog", "Documentation", "v0.1.5").
    #[serde(default)]
    pub badge: Option<String>,
    /// Theme name (`"dark"`, `"indigo"`, `"emerald"`, `"sunset"`, `"minimal"`).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Width in pixels (default 1200).
    #[serde(default = "default_width")]
    pub width: u32,
    /// Height in pixels (default 630).
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_width() -> u32 {
    1200
}

fn default_height() -> u32 {
    630
}

/// Helper function to build an OpenGraph image URL for a given title, subtitle, and badge.
pub fn og_image_url(title: &str, subtitle: Option<&str>, badge: Option<&str>) -> String {
    let mut url = format!("/_yav/og?title={}", urlencoding(title));
    if let Some(sub) = subtitle {
        url.push_str(&format!("&subtitle={}", urlencoding(sub)));
    }
    if let Some(b) = badge {
        url.push_str(&format!("&badge={}", urlencoding(b)));
    }
    url
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Render a 1200x630 SVG social card based on the given parameters.
pub fn generate_og_svg(params: &OgImageParams) -> String {
    let w = params.width.clamp(600, 2400);
    let h = params.height.clamp(315, 1260);

    let (bg_gradient, accent_color, badge_bg, text_color, subtext_color) =
        match params.theme.as_str() {
            "indigo" => (
                (
                    "rgba(15, 23, 42, 1)",
                    "rgba(30, 27, 75, 1)",
                    "rgba(99, 102, 241, 0.25)",
                ),
                "#818cf8",
                "rgba(99, 102, 241, 0.2)",
                "#f8fafc",
                "#cbd5e1",
            ),
            "emerald" => (
                (
                    "rgba(6, 78, 59, 1)",
                    "rgba(2, 44, 34, 1)",
                    "rgba(16, 185, 129, 0.25)",
                ),
                "#34d399",
                "rgba(16, 185, 129, 0.2)",
                "#f0fdf4",
                "#a7f3d0",
            ),
            "sunset" => (
                (
                    "rgba(30, 27, 75, 1)",
                    "rgba(67, 20, 7, 1)",
                    "rgba(244, 63, 94, 0.25)",
                ),
                "#fb7185",
                "rgba(244, 63, 94, 0.2)",
                "#fff1f2",
                "#fecdd3",
            ),
            "minimal" => (
                (
                    "rgba(248, 250, 252, 1)",
                    "rgba(241, 245, 249, 1)",
                    "rgba(148, 163, 184, 0.2)",
                ),
                "#0f172a",
                "rgba(15, 23, 42, 0.08)",
                "#0f172a",
                "#475569",
            ),
            _ => (
                // Default dark theme
                (
                    "rgba(9, 9, 11, 1)",
                    "rgba(24, 24, 27, 1)",
                    "rgba(99, 102, 241, 0.18)",
                ),
                "#a5b4fc",
                "rgba(255, 255, 255, 0.1)",
                "#ffffff",
                "#a1a1aa",
            ),
        };

    // Automatic word wrapping for the title (approx 36 characters per line at font-size 56)
    let words: Vec<&str> = params.title.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();
    for word in words {
        if current_line.len() + word.len() + 1 > 38 && !current_line.is_empty() {
            lines.push(current_line);
            current_line = word.to_string();
        } else if current_line.is_empty() {
            current_line = word.to_string();
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push("Yav.rs Application".to_string());
    }

    let title_start_y = if lines.len() == 1 {
        300
    } else if lines.len() == 2 {
        260
    } else {
        220
    };
    let mut title_tspans = String::new();
    for (idx, line) in lines.iter().take(3).enumerate() {
        let y = title_start_y + (idx as i32 * 68);
        title_tspans.push_str(&format!(
            r#"<tspan x="96" y="{y}">{}</tspan>"#,
            yav_core::escape_html(line)
        ));
    }

    let subtitle_y = title_start_y + (lines.len().min(3) as i32 * 68) + 24;
    let subtitle_svg = if let Some(ref sub) = params.subtitle {
        let esc_sub = yav_core::escape_html(sub);
        format!(
            r#"<text x="96" y="{subtitle_y}" fill="{subtext_color}" font-family="system-ui, -apple-system, sans-serif" font-size="28" font-weight="400">{esc_sub}</text>"#
        )
    } else {
        String::new()
    };

    let badge_svg = if let Some(ref badge_text) = params.badge {
        let esc_badge = yav_core::escape_html(badge_text);
        format!(
            r#"<g transform="translate(96, 110)">
                <rect rx="14" width="{bw}" height="38" fill="{badge_bg}" stroke="{accent_color}" stroke-opacity="0.4" stroke-width="1.5"/>
                <text x="{bx}" y="24" fill="{accent_color}" font-family="system-ui, -apple-system, sans-serif" font-size="16" font-weight="600">{esc_badge}</text>
            </g>"#,
            bw = (badge_text.len() * 10 + 28).max(80),
            bx = (badge_text.len() * 10 + 28).max(80) / 2
        )
    } else {
        String::new()
    };

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="{c0}"/>
      <stop offset="100%" stop-color="{c1}"/>
    </linearGradient>
    <radialGradient id="glow" cx="85%" cy="15%" r="65%">
      <stop offset="0%" stop-color="{c2}"/>
      <stop offset="100%" stop-color="rgba(0,0,0,0)"/>
    </radialGradient>
    <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
      <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(255,255,255,0.04)" stroke-width="1"/>
    </pattern>
  </defs>

  <!-- Background -->
  <rect width="{w}" height="{h}" fill="url(#bg)"/>
  <rect width="{w}" height="{h}" fill="url(#glow)"/>
  <rect width="{w}" height="{h}" fill="url(#grid)"/>

  <!-- Brand badge -->
  {badge_svg}

  <!-- Title -->
  <text fill="{text_color}" font-family="system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif" font-size="56" font-weight="800" letter-spacing="-0.02em">
    {title_tspans}
  </text>

  <!-- Subtitle -->
  {subtitle_svg}

  <!-- Footer branding -->
  <g transform="translate(96, 560)">
    <circle cx="12" cy="-6" r="8" fill="{accent_color}"/>
    <text x="32" y="0" fill="{subtext_color}" font-family="system-ui, -apple-system, sans-serif" font-size="20" font-weight="600" letter-spacing="0.04em">YAV.RS</text>
  </g>
</svg>"#,
        w = w,
        h = h,
        c0 = bg_gradient.0,
        c1 = bg_gradient.1,
        c2 = bg_gradient.2,
    )
}

/// Axum route handler for `/_yav/og`.
pub async fn og_image_handler(Query(params): Query<OgImageParams>) -> Response {
    let svg = generate_og_svg(&params);

    let mut hasher = Sha256::new();
    hasher.update(svg.as_bytes());
    let digest = hasher.finalize();
    let etag = format!(
        "\"{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\"",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    );

    let mut resp = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        svg,
    )
        .into_response();

    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        resp.headers_mut().insert(header::ETAG, etag_val);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_og_svg_rendering() {
        let params = OgImageParams {
            title: "Supercharging Full-Stack Rust with Yav.rs".to_string(),
            subtitle: Some("Zero JS build tools, 100% native performance".to_string()),
            badge: Some("Documentation".to_string()),
            theme: "indigo".to_string(),
            width: 1200,
            height: 630,
        };

        let svg = generate_og_svg(&params);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Supercharging Full-Stack Rust"));
        assert!(svg.contains("Documentation"));
        assert!(svg.contains("YAV.RS"));
        assert!(svg.contains("width=\"1200\""));
        assert!(svg.contains("height=\"630\""));
    }

    #[test]
    fn test_og_image_url_encoding() {
        let url = og_image_url(
            "Fast & Furious: Rust",
            Some("Next.js Parity"),
            Some("Release"),
        );
        assert!(url.starts_with("/_yav/og?title="));
        assert!(url.contains("Fast"));
        assert!(url.contains("subtitle="));
        assert!(url.contains("badge="));
    }
}
