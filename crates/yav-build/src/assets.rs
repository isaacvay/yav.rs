//! Static asset pipeline for copying, fingerprinting, and manifest generation.
//!
//! Manages static files (e.g. images, fonts, compiled CSS), computes content hashes
//! for cache busting, and emits an asset manifest for mapping original URLs to fingerprinted paths.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Error types related to asset processing.
#[derive(Debug)]
pub enum AssetError {
    /// An I/O error occurred while reading or writing asset files.
    Io(std::io::Error),
    /// A path conversion error occurred.
    InvalidPath(String),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "Asset I/O error: {err}"),
            Self::InvalidPath(msg) => write!(f, "Invalid asset path: {msg}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AssetError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// An asset manifest mapping original relative paths to fingerprinted paths.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct AssetManifest {
    entries: BTreeMap<String, String>,
}

impl AssetManifest {
    /// Creates an empty manifest.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Inserts an asset mapping into the manifest.
    pub fn insert(&mut self, original: impl Into<String>, fingerprinted: impl Into<String>) {
        self.entries.insert(original.into(), fingerprinted.into());
    }

    /// Looks up the fingerprinted path for an original asset path.
    pub fn get(&self, original: &str) -> Option<&str> {
        self.entries.get(original).map(|s| s.as_str())
    }

    /// Returns a reference to all manifest entries.
    pub fn entries(&self) -> &BTreeMap<String, String> {
        &self.entries
    }

    /// Returns true if the manifest has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries in the manifest.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Serializes the manifest into a formatted JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.entries).unwrap_or_else(|_| "{}".to_string())
    }

    /// Writes the manifest JSON to a destination file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), AssetError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, self.to_json())?;
        Ok(())
    }

    /// Loads an `AssetManifest` from a JSON file on disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let entries: BTreeMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| AssetError::InvalidPath(format!("Invalid manifest JSON: {e}")))?;
        Ok(Self { entries })
    }

    /// Resolves a public URL (such as `"/public/styles.css"` or `"/styles.css"`) to its
    /// fingerprinted counterpart if present in the manifest, or returns the original URL.
    pub fn resolve_url(&self, url: &str) -> String {
        if let Some(rel) = url.strip_prefix("/public/") {
            if let Some(fp) = self.get(rel) {
                return format!("/public/{fp}");
            }
        } else if let Some(rel) = url.strip_prefix('/') {
            if let Some(fp) = self.get(rel) {
                return format!("/{fp}");
            }
        } else if let Some(fp) = self.get(url) {
            return fp.to_string();
        }
        url.to_string()
    }
}

/// Computes an 8-character hex SHA256 digest of binary content for fingerprinting.
pub fn hash_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let full = format!("{:x}", hasher.finalize());
    full[..8].to_string()
}

/// Computes a fingerprinted filename by inserting the hash before the extension.
///
/// Example: `styles.css` -> `styles.a1b2c3d4.css`.
/// Example: `favicon.ico` -> `favicon.a1b2c3d4.ico`.
/// Example: `README` -> `README.a1b2c3d4`.
pub fn fingerprint_filename(filename: &str, hash: &str) -> String {
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let ext = path.extension().and_then(|e| e.to_str());

    match ext {
        Some(extension) => format!("{stem}.{hash}.{extension}"),
        None => format!("{stem}.{hash}"),
    }
}

/// Converts a path to standard forward slashes for URL and manifest keys.
pub fn to_forward_slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Check if a relative path corresponds to a well-known root asset that must maintain
/// its canonical un-fingerprinted filename for browsers and web crawlers.
pub fn is_well_known_root_asset(rel_path: &Path) -> bool {
    let name = rel_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    matches!(
        name,
        "favicon.ico"
            | "favicon.svg"
            | "favicon-flat.svg"
            | "robots.txt"
            | "sitemap.xml"
            | "sw.js"
            | "manifest.json"
            | "manifest.webmanifest"
            | "CNAME"
    )
}

/// Pipeline for processing static project assets.
pub struct AssetPipeline;

impl AssetPipeline {
    /// Copies all assets from `src_dir` to `dst_dir`, optionally calculating content
    /// fingerprints and returning an `AssetManifest`.
    pub fn copy_dir(
        src_dir: impl AsRef<Path>,
        dst_dir: impl AsRef<Path>,
        fingerprint: bool,
    ) -> Result<AssetManifest, AssetError> {
        let src = src_dir.as_ref();
        let dst = dst_dir.as_ref();

        let mut manifest = AssetManifest::new();

        if !src.exists() {
            tracing::debug!(
                "Source directory '{}' does not exist; skipping asset copy",
                src.display()
            );
            return Ok(manifest);
        }

        Self::walk_and_copy(src, src, dst, fingerprint, &mut manifest)?;
        Ok(manifest)
    }

    fn walk_and_copy(
        current_dir: &Path,
        base_src: &Path,
        base_dst: &Path,
        fingerprint: bool,
        manifest: &mut AssetManifest,
    ) -> Result<(), AssetError> {
        for entry in std::fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::walk_and_copy(&path, base_src, base_dst, fingerprint, manifest)?;
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(base_src)
                    .map_err(|e| AssetError::InvalidPath(e.to_string()))?;
                let original_key = to_forward_slash(rel);

                let data = std::fs::read(&path)?;

                if fingerprint {
                    let hash = hash_bytes(&data);
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("asset");
                    let fp_name = fingerprint_filename(file_name, &hash);

                    let target_rel = if let Some(parent) = rel.parent() {
                        parent.join(&fp_name)
                    } else {
                        PathBuf::from(&fp_name)
                    };
                    let key = to_forward_slash(&target_rel);
                    let dest_fp_path = base_dst.join(&target_rel);
                    if let Some(parent) = dest_fp_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&dest_fp_path, &data)?;
                    let _ = precompress_asset(&dest_fp_path);
                    manifest.insert(original_key.clone(), key);

                    // Always also preserve the canonical un-fingerprinted file at destination
                    // so that pages referencing the original paths do not receive 404s.
                    let canonical_path = base_dst.join(rel);
                    if let Some(parent) = canonical_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&canonical_path, &data)?;
                    let _ = precompress_asset(&canonical_path);
                } else {
                    let dest_full_path = base_dst.join(rel);
                    if let Some(parent) = dest_full_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&dest_full_path, &data)?;
                    let _ = precompress_asset(&dest_full_path);
                    manifest.insert(original_key.clone(), original_key);
                }
            }
        }

        Ok(())
    }
}

/// Pre-compresses text files (.css, .js, .html, .svg, .json, .xml) generating a companion `.gz` file.
pub fn precompress_asset(path: &Path) -> Result<Option<PathBuf>, AssetError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if matches!(ext.as_str(), "css" | "js" | "html" | "svg" | "json" | "xml") {
        let content = std::fs::read(path)?;
        let mut gz_os = path.as_os_str().to_os_string();
        gz_os.push(".gz");
        let gz_path = PathBuf::from(gz_os);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&content).map_err(AssetError::Io)?;
        let gz_data = encoder.finish().map_err(AssetError::Io)?;

        std::fs::write(&gz_path, gz_data)?;
        Ok(Some(gz_path))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes() {
        let data = b"hello world";
        let hash = hash_bytes(data);
        assert_eq!(hash.len(), 8);
        assert_eq!(hash, "b94d27b9");
    }

    #[test]
    fn test_fingerprint_filename() {
        assert_eq!(
            fingerprint_filename("styles.css", "12345678"),
            "styles.12345678.css"
        );
        assert_eq!(
            fingerprint_filename("logo.png", "abcdef01"),
            "logo.abcdef01.png"
        );
        assert_eq!(
            fingerprint_filename("LICENSE", "99887766"),
            "LICENSE.99887766"
        );
    }

    #[test]
    fn test_manifest_json_format() {
        let mut manifest = AssetManifest::new();
        manifest.insert("styles.css", "styles.a1b2c3d4.css");
        manifest.insert("img/logo.png", "img/logo.e5f60718.png");

        let json = manifest.to_json();
        assert!(json.contains("\"styles.css\": \"styles.a1b2c3d4.css\""));
        assert!(json.contains("\"img/logo.png\": \"img/logo.e5f60718.png\""));
        assert_eq!(manifest.get("styles.css"), Some("styles.a1b2c3d4.css"));
    }

    #[test]
    fn test_copy_dir_pipeline() {
        let temp_dir = std::env::temp_dir().join("yav_test_assets");
        let src_dir = temp_dir.join("src");
        let dst_dir = temp_dir.join("dst");

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(src_dir.join("subdir")).unwrap();

        std::fs::write(src_dir.join("root.txt"), b"root file").unwrap();
        std::fs::write(src_dir.join("subdir/nested.css"), b"body { margin: 0; }").unwrap();

        // Test with fingerprinting
        let manifest = AssetPipeline::copy_dir(&src_dir, &dst_dir, true).unwrap();
        assert_eq!(manifest.len(), 2);

        let root_fp = manifest.get("root.txt").expect("root.txt mapping exists");
        assert!(root_fp.starts_with("root.") && root_fp.ends_with(".txt"));
        assert!(dst_dir.join(root_fp).exists());

        let nested_fp = manifest
            .get("subdir/nested.css")
            .expect("subdir/nested.css mapping exists");
        assert!(nested_fp.starts_with("subdir/nested.") && nested_fp.ends_with(".css"));
        assert!(dst_dir.join(nested_fp).exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_well_known_root_asset_preservation() {
        let temp_dir = std::env::temp_dir().join("yav_test_well_known");
        let src_dir = temp_dir.join("src");
        let dst_dir = temp_dir.join("dst");

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&src_dir).unwrap();

        std::fs::write(src_dir.join("favicon.ico"), b"binary_favicon").unwrap();
        std::fs::write(src_dir.join("robots.txt"), b"User-agent: *").unwrap();

        let manifest = AssetPipeline::copy_dir(&src_dir, &dst_dir, true).unwrap();
        assert_eq!(manifest.len(), 2);

        // Canonical un-fingerprinted files must exist at destination root!
        assert!(dst_dir.join("favicon.ico").exists());
        assert!(dst_dir.join("robots.txt").exists());

        // And fingerprinted copies must also exist according to manifest!
        let fav_fp = manifest.get("favicon.ico").unwrap();
        assert!(dst_dir.join(fav_fp).exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
