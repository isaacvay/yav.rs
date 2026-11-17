//! Dynamic Image Optimization & Blur Placeholder Endpoint (`/_yav/image`).
//!
//! Provides the server-side counterpart to `yav_ui::Image` (Next.js `next/image` parity):
//! - Serves local images from `public/` with strict path-traversal protection.
//! - **Really decodes, resizes and re-encodes** images (Lanczos3 + WebP / JPEG / PNG)
//!   instead of returning the original bytes.
//! - Content negotiation via `Accept: image/webp` and an explicit `?fm=` override.
//! - Emits `Cache-Control: public, max-age=31536000, immutable`, SHA-256 `ETag`
//!   and `Vary: Accept` headers.
//! - Generates a real LQIP (Low-Quality Image Placeholder) derived from the source
//!   pixels via `?blur=true`, or a colour gradient via [`generate_blur_svg_data_uri`].

use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
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
    /// When `true`, returns a lightweight blurred SVG placeholder derived from
    /// the actual source image.
    #[serde(default)]
    pub blur: bool,
    /// Output format override: `webp`, `jpeg`/`jpg`, `png` or `original`.
    /// When omitted, the format is negotiated from the `Accept` header.
    #[serde(default)]
    pub fm: Option<String>,
}

/// Single extractor combining the query string and the `Accept` header.
///
/// Keeping this to one extractor lets the handler be registered through both
/// `YavApp::get_with` (single-extractor API) and plain `axum::routing::get`,
/// while still performing real content negotiation.
#[derive(Debug, Clone)]
pub struct ImageOptRequest {
    pub params: ImageOptParams,
    pub accept: Option<String>,
}

impl<S> FromRequestParts<S> for ImageOptRequest
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<ImageOptParams>::from_request_parts(parts, state)
            .await
            .map_err(|err| err.into_response())?;
        let accept = parts
            .headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        Ok(Self { params, accept })
    }
}

/// Output format used by the optimizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Lossless WebP (pure Rust encoder from the `image` crate).
    WebP,
    /// JPEG, honouring the `q` quality parameter.
    Jpeg,
    /// PNG (lossless, preserves alpha).
    Png,
}

impl OutputFormat {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "webp" => Some(Self::WebP),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            _ => None,
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Self::WebP => "image/webp",
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
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

/// Validate that a relative asset path contains no parent directory (`..`)
/// traversal, hidden files, or percent-encoded variants of either.
pub fn resolve_safe_asset_path(public_dir: &Path, raw_url: &str) -> Option<PathBuf> {
    let decoded = crate::static_files::decode_url_path(raw_url.trim())?;
    let clean = decoded
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("public/");
    if clean.is_empty() || clean.contains('\\') {
        return None;
    }
    let rel = Path::new(clean);
    for comp in rel.components() {
        match comp {
            Component::Normal(name) => {
                if name.to_str().is_some_and(|n| n.starts_with('.')) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    // Canonicalisation catches symlinks pointing outside `public/`.
    let full_path = public_dir.join(rel);
    if full_path.exists() || full_path.symlink_metadata().is_ok() {
        let base = public_dir.canonicalize().ok()?;
        let target = full_path.canonicalize().ok()?;
        if !target.starts_with(base) {
            return None;
        }
    }
    Some(full_path)
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

/// Decode an image, resize it to `target_width` (never upscaling) with a Lanczos3
/// kernel, and re-encode it in `format`.
///
/// Returns `(encoded_bytes, mime, width, height)`.
pub fn optimize_image_bytes(
    bytes: &[u8],
    target_width: u32,
    quality: u8,
    format: OutputFormat,
) -> Result<(Vec<u8>, &'static str, u32, u32), String> {
    let decoded = image::load_from_memory(bytes).map_err(|err| err.to_string())?;
    let (source_w, source_h) = decoded.dimensions();
    if source_w == 0 || source_h == 0 {
        return Err("image has zero dimensions".to_string());
    }
    let width = target_width.min(source_w).max(1);
    let height =
        ((source_h as u64 * width as u64) / source_w as u64).clamp(1, u32::MAX as u64) as u32;

    let resized: DynamicImage = if width == source_w && height == source_h {
        decoded
    } else {
        DynamicImage::ImageRgba8(image::imageops::resize(
            &decoded.to_rgba8(),
            width,
            height,
            image::imageops::FilterType::Lanczos3,
        ))
    };

    let mut out = Vec::new();
    match format {
        OutputFormat::WebP => {
            image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .write_image(
                    resized.as_bytes(),
                    resized.width(),
                    resized.height(),
                    resized.color().into(),
                )
                .map_err(|err| err.to_string())?;
        }
        OutputFormat::Jpeg => {
            let rgb = resized.to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
                .write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|err| err.to_string())?;
        }
        OutputFormat::Png => {
            image::codecs::png::PngEncoder::new(&mut out)
                .write_image(
                    resized.as_bytes(),
                    resized.width(),
                    resized.height(),
                    resized.color().into(),
                )
                .map_err(|err| err.to_string())?;
        }
    }
    Ok((out, format.mime(), width, height))
}

/// Build a genuine LQIP from the source pixels: a tiny (≤24px wide) Lanczos3
/// downscale, encoded as PNG and embedded in a blurred SVG document.
pub fn generate_blur_svg_document_from_bytes(
    bytes: &[u8],
    display_width: u32,
) -> Result<String, String> {
    use base64::Engine;

    let decoded = image::load_from_memory(bytes).map_err(|err| err.to_string())?;
    let (w, h) = decoded.dimensions();
    if w == 0 || h == 0 {
        return Err("image has zero dimensions".to_string());
    }
    let out_w = display_width.clamp(16, 3840);
    let out_h = ((h as u64 * out_w as u64) / w as u64).clamp(1, 3840) as u32;
    let thumb_w = w.clamp(1, 24);
    let thumb_h = ((h as u64 * thumb_w as u64) / w as u64).clamp(1, 100) as u32;
    let thumb = image::imageops::resize(
        &decoded.to_rgba8(),
        thumb_w,
        thumb_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(
            thumb.as_raw(),
            thumb_w,
            thumb_h,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|err| err.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&png);

    Ok(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {out_w} {out_h}' width='{out_w}' height='{out_h}'>\
         <filter id='b'><feGaussianBlur stdDeviation='8'/></filter>\
         <image width='100%' height='100%' preserveAspectRatio='none' filter='url(%23b)' href='data:image/png;base64,{encoded}'/>\
         </svg>"
    ))
}

/// Same LQIP as a `data:image/svg+xml` URI, suitable for `blurDataURL` props.
pub fn generate_blur_placeholder_from_bytes(bytes: &[u8]) -> Result<String, String> {
    let svg = generate_blur_svg_document_from_bytes(bytes, 320)?;
    let mut encoded = String::with_capacity(svg.len() + 32);
    for byte in svg.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b','
            | b':'
            | b';'
            | b'/'
            | b'='
            | b'\'' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    Ok(format!("data:image/svg+xml,{encoded}"))
}

fn negotiated_format(fm: Option<&str>, accept: Option<&str>, source_path: &Path) -> OutputFormat {
    if let Some(explicit) = fm.and_then(OutputFormat::parse) {
        return explicit;
    }
    let accepts_webp = accept
        .map(|a| a.to_ascii_lowercase().contains("image/webp"))
        .unwrap_or(false);
    if accepts_webp {
        return OutputFormat::WebP;
    }
    match image::ImageFormat::from_path(source_path) {
        Ok(ImageFormat::Jpeg) => OutputFormat::Jpeg,
        Ok(ImageFormat::Png) => OutputFormat::Png,
        _ => OutputFormat::Png,
    }
}

fn sha256_etag(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!(
        "\"{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\"",
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7]
    )
}

/// Axum route handler for `/_yav/image`.
pub async fn image_optimizer_handler(request: ImageOptRequest) -> Response {
    let params = request.params;
    let width = params.w.clamp(16, 3840);
    let quality = params.q.clamp(1, 100);

    let public_dir = Path::new("public");
    let Some(file_path) = resolve_safe_asset_path(public_dir, &params.url) else {
        return (StatusCode::BAD_REQUEST, "Invalid image path").into_response();
    };

    let Ok(bytes) = tokio::fs::read(&file_path).await else {
        return (StatusCode::NOT_FOUND, "Image asset not found").into_response();
    };

    if params.blur {
        return match generate_blur_svg_document_from_bytes(&bytes, width) {
            Ok(svg) => (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
                    (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
                ],
                svg,
            )
                .into_response(),
            Err(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                format!("Cannot decode source image for LQIP: {err}"),
            )
                .into_response(),
        };
    }

    let accept = request.accept.as_deref();
    let format = negotiated_format(params.fm.as_deref(), accept, &file_path);

    let (output, mime, out_w, out_h) = match optimize_image_bytes(&bytes, width, quality, format) {
        Ok(result) => result,
        Err(err) => {
            // Unsupported inputs (SVG, AVIF, corrupt files) are still useful:
            // serve them unchanged with the original MIME type and no-store.
            tracing::warn!(path = %file_path.display(), error = %err, "Image optimization failed; serving original bytes");
            let mut passthrough = (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime_for_extension(&file_path)),
                    (header::CACHE_CONTROL, "private, no-store"),
                ],
                bytes,
            )
                .into_response();
            passthrough
                .headers_mut()
                .insert(header::VARY, HeaderValue::from_static("Accept"));
            return passthrough;
        }
    };

    let etag = sha256_etag(&output);
    let mut resp = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        output,
    )
        .into_response();

    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        resp.headers_mut().insert(header::ETAG, etag_val);
    }
    resp.headers_mut()
        .insert(header::VARY, HeaderValue::from_static("Accept"));
    if let Ok(meta_val) =
        HeaderValue::from_str(&format!("w={out_w};h={out_h};q={quality};fm={mime}"))
    {
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

    fn sample_png(width: u32, height: u32) -> Vec<u8> {
        let mut img = image::RgbaImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]);
        }
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn test_optimizer_really_resizes_and_reencodes() {
        let src = sample_png(200, 100);
        let (out, mime, w, h) = optimize_image_bytes(&src, 50, 80, OutputFormat::WebP).unwrap();
        assert_eq!(mime, "image/webp");
        assert_eq!((w, h), (50, 25));
        assert!(out.starts_with(b"RIFF"), "must be a real WebP container");
        assert_ne!(out, src, "output must not be the original bytes");

        // Round-trip: the encoded WebP must decode to exactly the target size.
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!(decoded.dimensions(), (50, 25));
    }

    #[test]
    fn test_optimizer_jpeg_quality_and_no_upscaling() {
        let src = sample_png(40, 20);
        let low = optimize_image_bytes(&src, 40, 10, OutputFormat::Jpeg).unwrap();
        let high = optimize_image_bytes(&src, 40, 95, OutputFormat::Jpeg).unwrap();
        assert_eq!(low.1, "image/jpeg");
        assert!(low.0.len() < high.0.len(), "quality must affect JPEG size");

        // Never upscale beyond the source width.
        let capped = optimize_image_bytes(&src, 3840, 80, OutputFormat::Png).unwrap();
        assert_eq!((capped.2, capped.3), (40, 20));
    }

    #[test]
    fn test_lqip_is_derived_from_the_source_image() {
        let src = sample_png(200, 100);
        let uri = generate_blur_placeholder_from_bytes(&src).unwrap();
        assert!(uri.starts_with("data:image/svg+xml,"));
        assert!(uri.contains("data:image/png;base64,"));
        assert!(uri.contains("feGaussianBlur"));
        assert!(
            !uri.contains('#'),
            "data URI must not contain raw fragments"
        );

        let svg = generate_blur_svg_document_from_bytes(&src, 320).unwrap();
        assert!(svg.contains("width='320'"));

        let other = sample_png(200, 100);
        // Same synthetic input is deterministic; a different real image yields a
        // different placeholder.
        assert_eq!(
            generate_blur_placeholder_from_bytes(&src).unwrap(),
            generate_blur_placeholder_from_bytes(&other).unwrap()
        );
    }

    #[test]
    fn test_content_negotiation() {
        let path = Path::new("photo.jpg");
        assert_eq!(
            negotiated_format(None, Some("image/avif,image/webp,*/*"), path),
            OutputFormat::WebP
        );
        assert_eq!(negotiated_format(None, None, path), OutputFormat::Jpeg);
        assert_eq!(
            negotiated_format(Some("png"), Some("image/webp"), path),
            OutputFormat::Png
        );
    }

    #[test]
    fn test_safe_path_blocks_encoded_traversal_and_dotfiles() {
        let pub_dir = Path::new("public");
        assert!(resolve_safe_asset_path(pub_dir, "/%2e%2e/Cargo.toml").is_none());
        assert!(resolve_safe_asset_path(pub_dir, "/.env").is_none());
        assert!(resolve_safe_asset_path(pub_dir, "/images/a%20b.png").is_some());
    }
}
