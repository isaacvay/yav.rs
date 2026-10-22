//! Static asset serving with automatic MIME detection and path traversal protection.
//!
//! Provides utilities to safely resolve paths inside a static asset root
//! directory, detect MIME types, and serve static assets over HTTP.

use crate::error::YavError;
use axum::{
    body::Body,
    http::{header, Method, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::{Component, Path, PathBuf};

/// Determine the MIME `Content-Type` for a file based on its file extension.
///
/// Returns standard MIME types with charset attributes for text files.
/// Falls back to `application/octet-stream` for unrecognized extensions.
pub fn mime_type_for_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    match ext.as_deref() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain; charset=utf-8",
        Some("pdf") => "application/pdf",
        Some("xml") => "application/xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// Safely resolve a request path relative to a base directory, preventing path traversal attacks.
///
/// Rejects:
/// - Paths containing null bytes (`\0`)
/// - Directory traversal components (`..`)
/// - Absolute paths or drive prefixes
/// - Symlinks escaping the base directory
///
/// # Errors
/// Returns [`YavError::BadRequest`] if any traversal attempt is detected.
pub fn resolve_static_path(base_dir: &Path, request_path: &str) -> Result<PathBuf, YavError> {
    if request_path.contains('\0') {
        return Err(YavError::BadRequest(
            "Invalid static file path: contains null byte".to_string(),
        ));
    }

    // Normalize Windows backslashes and strip leading forward slashes
    let normalized = request_path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');

    let rel_path = Path::new(trimmed);

    // Inspect individual path components
    for comp in rel_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(YavError::BadRequest(
                    "Path traversal detected: parent directory references ('..') are forbidden"
                        .to_string(),
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(YavError::BadRequest(
                    "Path traversal detected: absolute paths are forbidden".to_string(),
                ));
            }
            _ => {}
        }
    }

    let full_path = base_dir.join(rel_path);

    // Canonicalization check if both base and target exist on disk
    if let (Ok(canonical_base), Ok(canonical_full)) =
        (base_dir.canonicalize(), full_path.canonicalize())
    {
        if !canonical_full.starts_with(&canonical_base) {
            return Err(YavError::BadRequest(
                "Path traversal detected: target path escapes base directory".to_string(),
            ));
        }
    }

    Ok(full_path)
}

/// Asynchronously serve a static file from `base_dir` given a relative `request_path`.
/// Parse an HTTP `Range: bytes=start-end` header.
pub fn parse_range_header(range: &str, total_len: usize) -> Option<(usize, usize)> {
    if total_len == 0 {
        return None;
    }
    let trimmed = range.trim();
    let bytes_part = trimmed.strip_prefix("bytes=")?;
    // Only support single range
    let first_range = bytes_part.split(',').next()?.trim();
    let (start_str, end_str) = first_range.split_once('-')?;
    if start_str.is_empty() {
        // Suffix range: -500 (last 500 bytes)
        let suffix_len: usize = end_str.parse().ok()?;
        if suffix_len == 0 {
            return None;
        }
        let start = total_len.saturating_sub(suffix_len);
        let end = total_len - 1;
        Some((start, end))
    } else if end_str.is_empty() {
        // Range: 500-
        let start: usize = start_str.parse().ok()?;
        if start >= total_len {
            return None;
        }
        Some((start, total_len - 1))
    } else {
        // Range: 0-499
        let start: usize = start_str.parse().ok()?;
        let end: usize = end_str.parse().ok()?;
        if start > end || start >= total_len {
            return None;
        }
        let clamped_end = end.min(total_len - 1);
        Some((start, clamped_end))
    }
}

/// Asynchronously serve a static file from disk.
///
/// Checks path traversal, verifies file existence, reads content asynchronously,
/// sets `Content-Type`, `Content-Length`, `Accept-Ranges: bytes`, and handles partial content
/// if a valid `Range` header is present.
///
/// # Errors
/// Returns [`YavError::BadRequest`] if traversal is attempted.
/// Returns [`YavError::NotFound`] if the file does not exist or is a directory.
/// Returns [`YavError::Internal`] if an I/O error occurs.
pub async fn serve_static_file(base_dir: &Path, request_path: &str) -> Result<Response, YavError> {
    serve_static_file_with_range(base_dir, request_path, None).await
}

/// Asynchronously serve a static file with optional HTTP `Range` header support.
pub async fn serve_static_file_with_range(
    base_dir: &Path,
    request_path: &str,
    range_header: Option<&str>,
) -> Result<Response, YavError> {
    serve_static_file_with_encoding(base_dir, request_path, range_header, None).await
}

fn client_accepts_encoding(accept_encoding: &str, encoding: &str) -> bool {
    for part in accept_encoding.split(',') {
        let mut subparts = part.split(';');
        let enc_name = subparts.next().unwrap_or("").trim();
        if enc_name.eq_ignore_ascii_case(encoding) || enc_name == "*" {
            if let Some(param) = subparts.next() {
                let trimmed = param.trim();
                if let Some(val) = trimmed.strip_prefix("q=") {
                    if let Ok(q) = val.trim().parse::<f32>() {
                        if q <= 0.0 {
                            return false;
                        }
                    }
                }
            }
            return true;
        }
    }
    false
}

/// Asynchronously serve a static file with optional HTTP `Range` header and `Accept-Encoding` pre-compressed support.
///
/// If the client accepts `br` or `gzip` encoding and a corresponding `.br` or `.gz` pre-compressed file exists,
/// it is served directly with `Content-Encoding`.
pub async fn serve_static_file_with_encoding(
    base_dir: &Path,
    request_path: &str,
    range_header: Option<&str>,
    accept_encoding: Option<&str>,
) -> Result<Response, YavError> {
    let file_path = resolve_static_path(base_dir, request_path)?;

    if !file_path.is_file() {
        return Err(YavError::NotFound);
    }

    let mime = mime_type_for_path(&file_path);

    // Check for pre-compressed assets (.br or .gz) if no byte-range is requested
    let (actual_file, content_encoding) = if range_header.is_none() {
        if let Some(enc) = accept_encoding {
            let mut br_os = file_path.as_os_str().to_os_string();
            br_os.push(".br");
            let br_path = PathBuf::from(br_os);

            let mut gz_os = file_path.as_os_str().to_os_string();
            gz_os.push(".gz");
            let gz_path = PathBuf::from(gz_os);

            if client_accepts_encoding(enc, "br") && br_path.is_file() {
                (br_path, Some("br"))
            } else if client_accepts_encoding(enc, "gzip") && gz_path.is_file() {
                (gz_path, Some("gzip"))
            } else {
                (file_path.clone(), None)
            }
        } else {
            (file_path.clone(), None)
        }
    } else {
        (file_path.clone(), None)
    };

    let meta = tokio::fs::metadata(&actual_file)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => YavError::NotFound,
            std::io::ErrorKind::PermissionDenied => {
                YavError::Internal("Permission denied reading static file".to_string())
            }
            _ => YavError::Internal(format!("Failed to stat static file: {}", err)),
        })?;

    /// Maximum static asset size read into RAM at once (100 MB) to prevent OOM DoS.
    const MAX_STATIC_READ_BYTES: usize = 100 * 1024 * 1024;
    let total_len = usize::try_from(meta.len()).unwrap_or(usize::MAX);

    let is_fingerprinted = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| {
            let parts: Vec<&str> = name.split('.').collect();
            parts.len() >= 3
                && parts[parts.len() - 2].len() == 8
                && parts[parts.len() - 2]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit())
        })
        .unwrap_or(false);

    let cache_control = if is_fingerprinted {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    };

    let range_bounds = range_header.and_then(|r| parse_range_header(r, total_len));

    let (status, body_bytes, content_range_val) = if let Some((start, end)) = range_bounds {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        let slice_len = (end - start + 1).min(MAX_STATIC_READ_BYTES);
        let actual_end = start + slice_len - 1;
        let mut file = tokio::fs::File::open(&actual_file)
            .await
            .map_err(|e| YavError::Internal(format!("Failed to open static file: {e}")))?;
        file.seek(std::io::SeekFrom::Start(start as u64))
            .await
            .map_err(|e| YavError::Internal(format!("Failed to seek static file: {e}")))?;
        let mut buf = vec![0u8; slice_len];
        file.read_exact(&mut buf)
            .await
            .map_err(|e| YavError::Internal(format!("Failed to read static range: {e}")))?;
        (
            StatusCode::PARTIAL_CONTENT,
            buf,
            Some(format!("bytes {start}-{actual_end}/{total_len}")),
        )
    } else {
        if total_len > MAX_STATIC_READ_BYTES {
            return Err(YavError::BadRequest(
                "Static file exceeds maximum in-memory read size; use HTTP Range requests"
                    .to_string(),
            ));
        }
        let contents = tokio::fs::read(&actual_file)
            .await
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => YavError::NotFound,
                std::io::ErrorKind::PermissionDenied => {
                    YavError::Internal("Permission denied reading static file".to_string())
                }
                _ => YavError::Internal(format!("Failed to read static file: {}", err)),
            })?;
        (StatusCode::OK, contents, None)
    };

    let content_len = body_bytes.len();
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, content_len.to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, cache_control)
        .header(header::VARY, "Accept-Encoding");

    // Prevent stored XSS in user-supplied or static SVG files when navigated to directly
    if mime == "image/svg+xml" {
        builder = builder.header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; style-src 'unsafe-inline'; sandbox",
        );
    }

    if let Some(enc) = content_encoding {
        builder = builder.header(header::CONTENT_ENCODING, enc);
    }

    if let Some(range_val) = content_range_val {
        builder = builder.header(header::CONTENT_RANGE, range_val);
    }

    let response = builder
        .body(Body::from(body_bytes))
        .map_err(|e| YavError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

/// Create an Axum [`Router`](axum::Router) that serves files from the specified directory.
pub fn static_router(base_dir: PathBuf) -> axum::Router {
    axum::Router::new().fallback(
        move |method: Method, headers: axum::http::HeaderMap, uri: axum::http::Uri| {
            let base = base_dir.clone();
            async move {
                if method == Method::OPTIONS {
                    return (
                        StatusCode::NO_CONTENT,
                        [
                            (header::ALLOW, "GET, HEAD, OPTIONS"),
                            (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, HEAD, OPTIONS"),
                            (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
                        ],
                        "",
                    )
                        .into_response();
                }
                if method != Method::GET && method != Method::HEAD {
                    return (
                        StatusCode::METHOD_NOT_ALLOWED,
                        [(header::ALLOW, "GET, HEAD")],
                        "Method Not Allowed",
                    )
                        .into_response();
                }
                let path = uri.path();
                let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
                let encoding = headers
                    .get(header::ACCEPT_ENCODING)
                    .and_then(|v| v.to_str().ok());
                match serve_static_file_with_encoding(&base, path, range, encoding).await {
                    Ok(mut resp) => {
                        if method == Method::HEAD {
                            *resp.body_mut() = Body::empty();
                        }
                        resp
                    }
                    Err(err) => err.into_response(),
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_type_resolution() {
        assert_eq!(
            mime_type_for_path(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            mime_type_for_path(Path::new("styles/app.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(
            mime_type_for_path(Path::new("scripts/bundle.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            mime_type_for_path(Path::new("data/info.json")),
            "application/json"
        );
        assert_eq!(mime_type_for_path(Path::new("logo.png")), "image/png");
        assert_eq!(mime_type_for_path(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(mime_type_for_path(Path::new("vector.svg")), "image/svg+xml");
        assert_eq!(mime_type_for_path(Path::new("icon.ico")), "image/x-icon");
        assert_eq!(mime_type_for_path(Path::new("font.woff2")), "font/woff2");
        assert_eq!(
            mime_type_for_path(Path::new("program.wasm")),
            "application/wasm"
        );
        assert_eq!(
            mime_type_for_path(Path::new("unknown.xyz123")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_static_route_resolution_valid() {
        let base = Path::new("/var/www/public");
        let resolved = resolve_static_path(base, "css/main.css").expect("valid path");
        assert_eq!(resolved, base.join("css/main.css"));

        let leading = resolve_static_path(base, "/js/app.js").expect("leading slash trimmed");
        assert_eq!(leading, base.join("js/app.js"));
    }

    #[test]
    fn test_static_route_resolution_traversal_parent() {
        let base = Path::new("/var/www/public");
        let result = resolve_static_path(base, "../secret.txt");
        assert!(matches!(result, Err(YavError::BadRequest(_))));
    }

    #[test]
    fn test_static_route_resolution_traversal_nested() {
        let base = Path::new("/var/www/public");
        let result = resolve_static_path(base, "images/../../etc/passwd");
        assert!(matches!(result, Err(YavError::BadRequest(_))));
    }

    #[test]
    fn test_static_route_resolution_traversal_windows_backslash() {
        let base = Path::new("C:\\www\\public");
        let result = resolve_static_path(base, r"images\..\..\secret.txt");
        assert!(matches!(result, Err(YavError::BadRequest(_))));
    }

    #[test]
    fn test_static_route_resolution_null_byte() {
        let base = Path::new("/var/www/public");
        let result = resolve_static_path(base, "app\0.js");
        assert!(matches!(result, Err(YavError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_serve_static_file_real_file() {
        let temp_dir = std::env::temp_dir().join(format!("yav_test_{}", std::process::id()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let test_file = temp_dir.join("test.css");
        tokio::fs::write(&test_file, b"body { color: red; }")
            .await
            .expect("write test file");

        let response = serve_static_file(&temp_dir, "test.css")
            .await
            .expect("serve static file");

        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(ct, "text/css; charset=utf-8");

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert_eq!(body_bytes.as_ref(), b"body { color: red; }");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_serve_static_file_not_found() {
        let temp_dir = std::env::temp_dir();
        let result = serve_static_file(&temp_dir, "non_existent_file_987654.txt").await;
        assert!(matches!(result, Err(YavError::NotFound)));
    }

    #[test]
    fn test_parse_range_header() {
        assert_eq!(parse_range_header("bytes=0-499", 1000), Some((0, 499)));
        assert_eq!(parse_range_header("bytes=500-", 1000), Some((500, 999)));
        assert_eq!(parse_range_header("bytes=-200", 1000), Some((800, 999)));
        assert_eq!(parse_range_header("bytes=0-1500", 1000), Some((0, 999))); // clamped
        assert_eq!(parse_range_header("bytes=1200-", 1000), None); // start >= len
        assert_eq!(parse_range_header("invalid", 1000), None);
    }

    #[tokio::test]
    async fn test_serve_static_file_range_request() {
        let temp_dir = std::env::temp_dir().join(format!("yav_range_test_{}", std::process::id()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let test_file = temp_dir.join("media.dat");
        tokio::fs::write(&test_file, b"0123456789")
            .await
            .expect("write test file");

        let response = serve_static_file_with_range(&temp_dir, "media.dat", Some("bytes=2-6"))
            .await
            .expect("serve range");

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok()),
            Some("bytes 2-6/10")
        );
        assert_eq!(
            response
                .headers()
                .get(header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok()),
            Some("bytes")
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        assert_eq!(body_bytes.as_ref(), b"23456");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_static_router_head_and_method_not_allowed() {
        use tower::ServiceExt;
        let temp_dir = std::env::temp_dir().join(format!("yav_head_test_{}", std::process::id()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let test_file = temp_dir.join("asset.txt");
        tokio::fs::write(&test_file, b"Hello Static File")
            .await
            .expect("write test file");

        let app = static_router(temp_dir.clone());

        // Test HEAD request: headers present, empty body
        let req = axum::http::Request::builder()
            .method("HEAD")
            .uri("/asset.txt")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::CONTENT_LENGTH).unwrap(), "17");
        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body_bytes.is_empty(), "HEAD request must have empty body");

        // Test POST request: 405 Method Not Allowed with Allow header
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/asset.txt")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(resp.headers().get(header::ALLOW).unwrap(), "GET, HEAD");

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
