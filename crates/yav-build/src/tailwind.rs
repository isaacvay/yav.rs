//! Tailwind CSS standalone CLI binary management and compilation.
//!
//! Downloads and executes the standalone Tailwind CLI without requiring Node.js or npm.

use std::path::{Path, PathBuf};

/// Error types related to Tailwind CLI management and execution.
#[derive(Debug)]
pub enum TailwindError {
    /// Failed to download the binary.
    DownloadFailed(String),
    /// The standalone binary was not found locally.
    BinaryNotFound(String),
    /// Compilation exited with a non-zero exit code.
    BuildFailed(String),
    /// The current OS or architecture is not supported by standalone Tailwind CLI.
    UnsupportedPlatform(String),
    /// Downloaded binary SHA256 checksum did not match the expected value.
    ChecksumMismatch {
        /// Expected SHA256 hex string.
        expected: String,
        /// Actual computed SHA256 hex string.
        actual: String,
    },
    /// An I/O error occurred.
    Io(std::io::Error),
    /// A network error occurred while downloading the binary.
    Network(reqwest::Error),
}

impl std::fmt::Display for TailwindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DownloadFailed(msg) => write!(f, "Tailwind CLI download failed: {msg}"),
            Self::BinaryNotFound(msg) => write!(f, "Tailwind binary not found: {msg}"),
            Self::BuildFailed(msg) => write!(f, "Tailwind build failed: {msg}"),
            Self::UnsupportedPlatform(msg) => write!(f, "Unsupported platform for Tailwind: {msg}"),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Tailwind binary SHA256 mismatch: expected {expected}, got {actual}"
                )
            }
            Self::Io(err) => write!(f, "I/O error during Tailwind operation: {err}"),
            Self::Network(err) => write!(f, "Network error during Tailwind operation: {err}"),
        }
    }
}

impl std::error::Error for TailwindError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Network(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TailwindError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<reqwest::Error> for TailwindError {
    fn from(err: reqwest::Error) -> Self {
        Self::Network(err)
    }
}

/// Detects the Tailwind standalone release platform target for a given OS and architecture.
pub fn detect_platform(os: &str, arch: &str) -> Result<String, TailwindError> {
    let os_name = match os {
        "windows" => "windows",
        "macos" | "darwin" => "macos",
        "linux" => "linux",
        other => {
            return Err(TailwindError::UnsupportedPlatform(format!(
                "Unsupported operating system: {other}"
            )))
        }
    };

    let arch_name = match arch {
        "x86_64" | "x64" | "amd64" => "x64",
        "aarch64" | "arm64" => "arm64",
        other => {
            return Err(TailwindError::UnsupportedPlatform(format!(
                "Unsupported CPU architecture: {other}"
            )))
        }
    };

    if os_name == "windows" {
        Ok(format!("{os_name}-{arch_name}.exe"))
    } else {
        Ok(format!("{os_name}-{arch_name}"))
    }
}

/// Detects the platform identifier for the current runtime host.
pub fn current_platform() -> Result<String, TailwindError> {
    detect_platform(std::env::consts::OS, std::env::consts::ARCH)
}

/// Constructs the GitHub release download URL for the standalone Tailwind CLI.
pub fn construct_download_url(version: &str, platform: &str) -> String {
    let clean_version = version.trim_start_matches('v');
    format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v{clean_version}/tailwindcss-{platform}"
    )
}

/// Manages fetching, caching, and running the standalone Tailwind CSS CLI.
#[derive(Debug, Clone)]
pub struct TailwindManager {
    version: String,
    cache_dir: PathBuf,
}

impl TailwindManager {
    /// Creates a new `TailwindManager` with the specified version string.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            cache_dir: PathBuf::from(".yav/cache/tailwind"),
        }
    }

    /// Overrides the default cache directory (`.yav/cache/tailwind`).
    pub fn with_cache_dir(mut self, cache_dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = cache_dir.into();
        self
    }

    /// Returns the configured version string.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Computes the expected binary file name for the current platform.
    pub fn binary_filename(&self) -> Result<String, TailwindError> {
        let platform = current_platform()?;
        Ok(format!("tailwindcss-{platform}"))
    }

    /// Returns the full cached path for the current platform's binary.
    pub fn binary_path(&self) -> Result<PathBuf, TailwindError> {
        let filename = self.binary_filename()?;
        let clean_version = self.version.trim_start_matches('v');
        Ok(self.cache_dir.join(clean_version).join(filename))
    }

    /// Returns true if the binary already exists in the local cache.
    pub fn is_cached(&self) -> bool {
        match self.binary_path() {
            Ok(p) => p.is_file(),
            Err(_) => false,
        }
    }

    /// Ensures the Tailwind CLI binary is available locally, downloading if necessary.
    pub async fn ensure_binary(&self) -> Result<PathBuf, TailwindError> {
        let bin_path = self.binary_path()?;
        if bin_path.is_file() {
            tracing::debug!("Using cached Tailwind CLI binary at {}", bin_path.display());
            return Ok(bin_path);
        }

        self.download_binary(&bin_path, None).await?;
        Ok(bin_path)
    }

    /// Downloads the standalone Tailwind CLI binary from GitHub releases.
    pub async fn download_binary(
        &self,
        dest_path: &Path,
        expected_sha256: Option<&str>,
    ) -> Result<(), TailwindError> {
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let platform = current_platform()?;
        let url = construct_download_url(&self.version, &platform);
        tracing::info!("Downloading Tailwind standalone CLI from {}", url);

        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            return Err(TailwindError::DownloadFailed(format!(
                "Failed to download Tailwind CLI from {url}: HTTP status {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;

        // SHA256 verification placeholder
        if let Some(expected) = expected_sha256 {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = format!("{:x}", hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err(TailwindError::ChecksumMismatch {
                    expected: expected.to_string(),
                    actual,
                });
            }
        } else {
            tracing::warn!("No SHA256 checksum available for Tailwind binary verification");
        }

        tokio::fs::write(dest_path, &bytes).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(dest_path, perms)?;
        }

        tracing::info!("Tailwind standalone CLI saved to {}", dest_path.display());
        Ok(())
    }

    /// Builds the command line argument list for running Tailwind CLI.
    pub fn build_args(
        &self,
        input_css: &Path,
        output_css: &Path,
        content_paths: &[impl AsRef<Path>],
        minify: bool,
    ) -> Vec<String> {
        let mut args = vec![
            "-i".to_string(),
            input_css.to_string_lossy().to_string(),
            "-o".to_string(),
            output_css.to_string_lossy().to_string(),
        ];

        if !content_paths.is_empty() {
            let patterns: Vec<String> = content_paths
                .iter()
                .map(|p| {
                    let s = p.as_ref().to_string_lossy().replace('\\', "/");
                    if s == "src" || s == "./src" {
                        "./src/**/*.{rs,html,yav,rsx,yavml}".to_string()
                    } else if s == "public" || s == "./public" {
                        "./public/**/*.{html,js}".to_string()
                    } else if s.ends_with("/src") {
                        format!("{s}/**/*.{{rs,html,yav,rsx,yavml}}")
                    } else if s.ends_with("/public") {
                        format!("{s}/**/*.{{html,js}}")
                    } else {
                        s
                    }
                })
                .collect();
            args.push("--content".to_string());
            args.push(patterns.join(","));
        }

        if minify {
            args.push("--minify".to_string());
        }

        args
    }

    /// Executes a Tailwind CSS build.
    pub async fn run_build(
        &self,
        input_css: impl AsRef<Path>,
        output_css: impl AsRef<Path>,
        content_paths: &[impl AsRef<Path>],
        minify: bool,
    ) -> Result<(), TailwindError> {
        let bin_path = self.ensure_binary().await?;
        let args = self.build_args(
            input_css.as_ref(),
            output_css.as_ref(),
            content_paths,
            minify,
        );

        tracing::info!("Compiling Tailwind CSS with binary {}", bin_path.display());
        let status = tokio::process::Command::new(&bin_path)
            .args(&args)
            .status()
            .await?;

        if !status.success() {
            return Err(TailwindError::BuildFailed(format!(
                "Tailwind build process exited with status: {}",
                status
            )));
        }

        Ok(())
    }

    /// Starts Tailwind CSS in watch mode, returning the spawned child process.
    pub fn run_watch(
        &self,
        input_css: impl AsRef<Path>,
        output_css: impl AsRef<Path>,
        content_paths: &[impl AsRef<Path>],
    ) -> Result<tokio::process::Child, TailwindError> {
        let bin_path = self.binary_path()?;
        if !bin_path.is_file() {
            return Err(TailwindError::BinaryNotFound(format!(
                "Tailwind binary not found at '{}'. Call ensure_binary() first.",
                bin_path.display()
            )));
        }

        let mut args = self.build_args(
            input_css.as_ref(),
            output_css.as_ref(),
            content_paths,
            false,
        );
        args.push("--watch".to_string());

        let child = tokio::process::Command::new(&bin_path)
            .args(&args)
            .spawn()?;

        Ok(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform_known() {
        assert_eq!(detect_platform("linux", "x86_64").unwrap(), "linux-x64");
        assert_eq!(detect_platform("linux", "aarch64").unwrap(), "linux-arm64");
        assert_eq!(detect_platform("macos", "arm64").unwrap(), "macos-arm64");
        assert_eq!(detect_platform("darwin", "x86_64").unwrap(), "macos-x64");
        assert_eq!(
            detect_platform("windows", "x86_64").unwrap(),
            "windows-x64.exe"
        );
        assert_eq!(
            detect_platform("windows", "arm64").unwrap(),
            "windows-arm64.exe"
        );
    }

    #[test]
    fn test_detect_platform_unsupported() {
        assert!(matches!(
            detect_platform("freebsd", "x86_64"),
            Err(TailwindError::UnsupportedPlatform(_))
        ));
        assert!(matches!(
            detect_platform("linux", "mips"),
            Err(TailwindError::UnsupportedPlatform(_))
        ));
    }

    #[test]
    fn test_construct_download_url() {
        let url1 = construct_download_url("3.4.17", "linux-x64");
        assert_eq!(
            url1,
            "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.17/tailwindcss-linux-x64"
        );

        let url2 = construct_download_url("v3.4.17", "windows-x64.exe");
        assert_eq!(
            url2,
            "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.17/tailwindcss-windows-x64.exe"
        );
    }

    #[test]
    fn test_build_args() {
        let manager = TailwindManager::new("3.4.17");
        let input = Path::new("src/app.css");
        let output = Path::new("dist/app.css");
        let content = vec![Path::new("src/**/*.rs")];

        let args = manager.build_args(input, output, &content, true);
        assert_eq!(
            args,
            vec![
                "-i",
                "src/app.css",
                "-o",
                "dist/app.css",
                "--content",
                "src/**/*.rs",
                "--minify"
            ]
        );

        let dev_args = manager.build_args(input, output, &content, false);
        assert!(!dev_args.contains(&"--minify".to_string()));
    }
}
