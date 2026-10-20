//! Build pipeline orchestration.
//!
//! Coordinates Cargo compilation, Tailwind CSS processing, static asset copying,
//! and asset manifest generation for both development and production builds.

use crate::assets::{AssetError, AssetPipeline};
use crate::config::{ConfigError, YavConfig};
use crate::tailwind::{TailwindError, TailwindManager};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Build target mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    /// Development build (unminified CSS, unhashed assets, debug binary).
    Dev,
    /// Production build (minified CSS, hashed assets, release binary).
    Prod,
}

/// Summary results after executing a build pipeline run.
#[derive(Debug, Clone)]
pub struct BuildSummary {
    /// The mode in which the build was executed.
    pub mode: BuildMode,
    /// Total duration elapsed during the build.
    pub duration: Duration,
    /// Whether Tailwind CSS compilation was executed.
    pub tailwind_built: bool,
    /// Number of static assets processed and copied.
    pub assets_copied: usize,
    /// Path to the generated asset manifest (if created).
    pub manifest_path: Option<PathBuf>,
}

/// Errors that can occur during pipeline execution.
#[derive(Debug)]
pub enum BuildError {
    /// Cargo compilation failed.
    CargoFailed(String),
    /// Tailwind compilation or download failed.
    Tailwind(TailwindError),
    /// Asset copying or manifest generation failed.
    Asset(AssetError),
    /// Configuration loading or parsing failed.
    Config(ConfigError),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CargoFailed(msg) => write!(f, "Cargo build failed: {msg}"),
            Self::Tailwind(err) => write!(f, "Tailwind error: {err}"),
            Self::Asset(err) => write!(f, "Asset pipeline error: {err}"),
            Self::Config(err) => write!(f, "Config error: {err}"),
            Self::Io(err) => write!(f, "I/O error during build: {err}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tailwind(err) => Some(err),
            Self::Asset(err) => Some(err),
            Self::Config(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<TailwindError> for BuildError {
    fn from(err: TailwindError) -> Self {
        Self::Tailwind(err)
    }
}

impl From<AssetError> for BuildError {
    fn from(err: AssetError) -> Self {
        Self::Asset(err)
    }
}

impl From<ConfigError> for BuildError {
    fn from(err: ConfigError) -> Self {
        Self::Config(err)
    }
}

impl From<std::io::Error> for BuildError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Orchestrator for the complete project build process.
pub struct BuildPipeline {
    config: YavConfig,
    root_dir: PathBuf,
}

impl BuildPipeline {
    /// Creates a new `BuildPipeline` with the given configuration and project root directory.
    pub fn new(config: YavConfig, root_dir: impl Into<PathBuf>) -> Self {
        Self {
            config,
            root_dir: root_dir.into(),
        }
    }

    /// Loads project configuration from `package.yavml` (or `package.yav` / `yav.toml`) in `root_dir`
    /// (falling back to defaults if missing) and returns an initialized `BuildPipeline`.
    pub fn from_root(root_dir: impl Into<PathBuf>) -> Result<Self, BuildError> {
        let root = root_dir.into();
        let manifest =
            crate::manifest::PackageYav::discover_in(&root).map_err(|e| ConfigError::Io {
                path: root.clone(),
                source: std::io::Error::other(e.to_string()),
            })?;
        let config = manifest.to_config();
        Ok(Self::new(config, root))
    }

    /// Returns the project configuration.
    pub fn config(&self) -> &YavConfig {
        &self.config
    }

    /// Returns the root directory of the project.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Executes a development build.
    pub async fn run_dev_build(&self) -> Result<BuildSummary, BuildError> {
        self.execute_build(BuildMode::Dev).await
    }

    /// Executes a production build.
    pub async fn run_prod_build(&self) -> Result<BuildSummary, BuildError> {
        self.execute_build(BuildMode::Prod).await
    }

    async fn execute_build(&self, mode: BuildMode) -> Result<BuildSummary, BuildError> {
        let start_time = std::time::Instant::now();
        let is_prod = mode == BuildMode::Prod;

        // 1. Cargo compilation
        tracing::info!("Executing Cargo build ({:?})...", mode);
        let mut cargo_cmd = tokio::process::Command::new("cargo");
        cargo_cmd.arg("build");
        if is_prod {
            cargo_cmd.arg("--release");
        }
        cargo_cmd.current_dir(&self.root_dir);

        let cargo_status = cargo_cmd.status().await?;
        if !cargo_status.success() {
            return Err(BuildError::CargoFailed(format!(
                "cargo build exited with status: {cargo_status}"
            )));
        }

        // 2. Tailwind compilation
        let mut tailwind_built = false;
        if self.config.tailwind.enabled {
            let input_css = self.root_dir.join(&self.config.tailwind.input_css);
            let output_css = self.root_dir.join(&self.config.tailwind.output_css);

            if input_css.is_file() {
                tracing::info!("Compiling Tailwind CSS from '{}'", input_css.display());
                let tailwind = TailwindManager::new(&self.config.tailwind.version)
                    .with_cache_dir(self.root_dir.join(".yav/cache/tailwind"));

                let content_paths = vec![self.root_dir.join("src"), self.root_dir.join("public")];

                let minify = is_prod && self.config.build.minify;
                tailwind
                    .run_build(&input_css, &output_css, &content_paths, minify)
                    .await?;
                tailwind_built = true;
            } else {
                tracing::warn!(
                    "Tailwind is enabled, but input CSS '{}' was not found",
                    input_css.display()
                );
            }
        }

        // 3. Asset copy
        let public_dir = self.root_dir.join(&self.config.build.public_dir);
        let out_dir = self.root_dir.join(&self.config.build.out_dir);

        let fingerprint = is_prod && self.config.build.fingerprint;
        let manifest = AssetPipeline::copy_dir(&public_dir, &out_dir, fingerprint)?;
        let assets_copied = manifest.len();

        // 4. Manifest generation
        let manifest_path = if is_prod && !manifest.is_empty() {
            let path = out_dir.join("asset-manifest.json");
            manifest.save(&path)?;
            tracing::info!("Saved asset manifest to {}", path.display());
            Some(path)
        } else {
            None
        };

        let duration = start_time.elapsed();
        Ok(BuildSummary {
            mode,
            duration,
            tailwind_built,
            assets_copied,
            manifest_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_new() {
        let config = YavConfig::default();
        let pipeline = BuildPipeline::new(config, "/tmp/test");
        assert_eq!(pipeline.root_dir(), Path::new("/tmp/test"));
        assert_eq!(pipeline.config().project.name, "my-app");
    }

    #[test]
    fn test_pipeline_from_root_default() {
        let temp_dir = std::env::temp_dir().join("yav_test_pipeline_root");
        let _ = std::fs::create_dir_all(&temp_dir);

        let pipeline = BuildPipeline::from_root(&temp_dir).unwrap();
        assert_eq!(pipeline.config().project.name, "my-app");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
