//! # YAV Build
//!
//! Build pipeline, static asset processing, and standalone Tailwind CSS integration
//! for the Yav web framework.
//!
//! This crate provides:
//! - **`package.yavml`** manifest loading — the central project descriptor ([`PackageYav`])
//! - Configuration loading and validation (`yav.toml`) via [`YavConfig`]
//! - Standalone Tailwind CSS binary management and compilation via [`TailwindManager`]
//! - Static asset processing, content fingerprinting, and manifest generation via [`AssetPipeline`]
//! - High-level build pipeline orchestration for dev and prod via [`BuildPipeline`]

pub mod assets;
pub mod config;
pub mod manifest;
pub mod pipeline;
pub mod tailwind;

pub use assets::{
    fingerprint_filename, hash_bytes, to_forward_slash, AssetError, AssetManifest, AssetPipeline,
};
pub use config::{
    BuildConfig, ConfigError, ProjectConfig, ServerConfig, TailwindConfig, YavConfig,
};
pub use manifest::{
    AssetsSection, BuildSection, DependencySpec, DetailedDep, ManifestError, PackageMeta,
    PackageYav, ServerSection, TailwindSection, LEGACY_PACKAGE_YAV, MANIFEST_FILE,
};
pub use pipeline::{BuildError, BuildMode, BuildPipeline, BuildSummary};
pub use tailwind::{
    construct_download_url, current_platform, detect_platform, TailwindError, TailwindManager,
};
