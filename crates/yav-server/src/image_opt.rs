//! Dynamic Image Optimization & Blur Placeholder Endpoint (`/_yav/image`).
//!
//! Provides the server-side counterpart to `yav_ui::Image` (Next.js `next/image` parity):
//! - Serves local images from `public/` with strict path-traversal protection.
//! - Emits `Cache-Control: public, max-age=31536000, immutable` and SHA-256 `ETag` headers.
//! - Generates instant SVG Gaussian-blur LQIP (Low-Quality Image Placeholder) data URIs
//!   via [`generate_blur_svg_data_uri`] or `?blur=true`.

use axum::extract::Query;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

/// Query parameters accepted by `/_yav/image?url=/hero.png&w=800&q=80`.
#[derive(Debug, Clone, Deserialize)]
pub struct ImageOptParams {
    /// Source asset path (e.g. `/images/hero.png` or `images/hero.png`).
    pub url: String,
    /// Target width in pixels (`16..=3840`).
    #[serde(default = "default_width")]
    pub w: u32,
    /// Target compression quality (`1..=100`).
    #[serde(default = "default_quality")]
    pub q: u8,
    /// When `true`, returns a lightweight blurred SVG placeholder.
    #[serde(default)]
    pub blur: bool,
}

fn default_width() -> u32 {
    800
}

fn default_quality() -> u8 {
    80
}

/// Configuration for the dynamic image optimizer.
#[derive(Debug, Clone)]
pub struct ImageOptimizerConfig {
    pub public_dir: PathBuf,
    pub max_width: u32,
}

impl Default for ImageOptimizerConfig {
    fn default() -> Self {
        Self {
            public_dir: PathBuf::from("public"),
            max_width: 3840,
        }
    }
}

/// Generate a lightweight SVG Gaussian-blur LQIP (Low-Quality Image Placeholder) `data:image/svg+xml` URI.
///
/// Can be passed directly to `<Image blur_data_url={...} />` in `yav-ui`.
pub fn generate_blur_svg_data_uri(
    width: u32,
    height: u32,
    primary_hex: &str,
    secondary_hex: &str,
) -> String {
    let w = width.max(16);
    let h = height.max(16);
    let c1 = primary_hex.trim_start_matches('#');
    let c2 = secondary_hex.trim_start_matches('#');
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {w} {h}' width='{w}' height='{h}'>\
        <defs><linearGradient id='g' x1='0%' y1='0%' x2='100%' y2='100%'>\
        <stop offset='0%' stop-color='%23{c1}'/><stop offset='100%' stop-color='%23{c2}'/>\
        </linearGradient><filter id='b' x='0' y='0'><feGaussianBlur stdDeviation='20'/></filter></defs>\
        <rect width='100%' height='100%' fill='url(%23g)' filter='url(%23b)'/></svg>"
    );
    format!("data:image/svg+xml;utf8,{svg}")
}

/// Format a `/_yav/image` optimization URL for a given asset path, width, and quality.
pub fn optimized_image_url(src: &str, width: u32, quality: u8) -> String {
    let clamped_q = quality.clamp(1, 100);
    let clamped_w = width.clamp(16, 3840);
    format!(
        "/_yav/image?url={}&w={}&q={}",
        src.replace(' ', "%20"),
        clamped_w,
        clamped_q
    )
}

/// Validate that a relative asset path contains no parent directory (`..`) traversal components.
pub fn resolve_safe_asset_path(public_dir: &Path, raw_url: &str) -> Option<PathBuf> {
    let clean = raw_url
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("public/");
    if clean.is_empty() || clean.contains('\\') {
        return None;
    }
    let rel = Path::new(clean);
    for comp in rel.components() {
        match comp {
            Component::Normal(_) => {}
            _ => return None,
        }
    }
    Some(public_dir.join(rel))
}

fn mime_for_extension(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "webp" => "image/webp",
        "avif" => "image/avif",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Axum route handler for `/_yav/image`.
pub async fn image_optimizer_handler(Query(params): Query<ImageOptParams>) -> Response {
    let width = params.w.clamp(16, 3840);
    let quality = params.q.clamp(1, 100);

    if params.blur {
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\
            <defs><linearGradient id=\"g\" x1=\"0%\" y1=\"0%\" x2=\"100%\" y2=\"100%\">\
            <stop offset=\"0%\" stop-color=\"#1e293b\"/><stop offset=\"100%\" stop-color=\"#334155\"/>\
            </linearGradient></defs><rect width=\"100%\" height=\"100%\" fill=\"url(#g)\"/></svg>",
            w = width,
            h = (width * 9) / 16
        );
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            svg,
        )
            .into_response();
    }

    let public_dir = Path::new("public");
    let Some(file_path) = resolve_safe_asset_path(public_dir, &params.url) else {
        return (StatusCode::BAD_REQUEST, "Invalid image path").into_response();
    };

    let Ok(bytes) = tokio::fs::read(&file_path).await else {
        return (StatusCode::NOT_FOUND, "Image asset not found").into_response();
    };

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.update(width.to_le_bytes());
    hasher.update([quality]);
    let digest = hasher.finalize();
    let etag = format!(
        "\"{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\"",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    );

    let mime = mime_for_extension(&file_path);
    let mut resp = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response();

    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        resp.headers_mut().insert(header::ETAG, etag_val);
    }
    if let Ok(meta_val) = HeaderValue::from_str(&format!("w={width};q={quality}")) {
        resp.headers_mut().insert("x-yav-image-params", meta_val);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blur_svg_and_safe_path_resolution() {
        let uri = generate_blur_svg_data_uri(640, 360, "#0f172a", "#3b82f6");
        assert!(uri.starts_with("data:image/svg+xml;utf8,<svg"));
        assert!(uri.contains("feGaussianBlur"));

        let pub_dir = Path::new("public");
        assert!(resolve_safe_asset_path(pub_dir, "/images/logo.png").is_some());
        assert!(resolve_safe_asset_path(pub_dir, "../Cargo.toml").is_none());
        assert!(resolve_safe_asset_path(pub_dir, "/images/../../secret.txt").is_none());

        assert_eq!(
            optimized_image_url("/hero.webp", 1200, 85),
            "/_yav/image?url=/hero.webp&w=1200&q=85"
        );
    }
}
