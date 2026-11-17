//! # YAV Build
//!
//! Build pipeline, static asset processing, and standalone Tailwind CSS integration
//! for the Yav web framework.
//!
//! This crate provides:
//! - **`package.yavml`** manifest loading — the central project descriptor ([`PackageYav`])
//! - Configuration loading and validation (`yav.toml`) via [`YavConfig`]
//! - Standalone Tailwind CSS binary management and compilation via [`TailwindManager`]
//! - Static asset processing, content fingerprinting, and pre-compression (`.br` / `.gz`) via [`AssetPipeline`]
//! - Technical Markdown parsing engine with YAML frontmatter, slug generation, and headings via [`parse_markdown`]
//! - High-level build pipeline orchestration for dev, SSG, and production via [`BuildPipeline`]

pub mod assets;
pub mod config;
pub mod fuzzy_search;
pub mod i18n_lint;
pub mod manifest;
pub mod markdown;
pub mod minifier;
pub mod mock_server;
pub mod pipeline;
pub mod pwa;
pub mod search;
pub mod tailwind;

#[cfg(feature = "a11y")]
pub mod a11y;
#[cfg(feature = "a11y")]
pub use a11y::{lint_html, lint_report_html, A11yIssue};

#[cfg(feature = "markdown")]
pub mod markdown_advanced;
#[cfg(feature = "markdown")]
pub use markdown_advanced::{highlight_code, render_markdown};

pub use assets::{
    fingerprint_filename, hash_bytes, to_forward_slash, AssetError, AssetManifest, AssetPipeline,
};
pub use config::{
    BuildConfig, ConfigError, ProjectConfig, ServerConfig, TailwindConfig, YavConfig,
};
pub use fuzzy_search::{
    damerau_levenshtein, soundex, suggest_did_you_mean, FuzzyMatch, FuzzySearchIndex,
};
pub use i18n_lint::{lint_catalog, scan_source_code, I18nKeyOccurrence, I18nReport};
pub use manifest::{
    AssetsSection, BuildSection, DependencySpec, DetailedDep, ManifestError, PackageMeta,
    PackageYav, ServerSection, TailwindSection, LEGACY_PACKAGE_YAV, MANIFEST_FILE,
};
pub use markdown::{
    highlight_code_html, load_markdown_dir, parse_markdown, slugify, MarkdownDoc, TocHeading,
};
pub use minifier::{minify_css, minify_html};
pub use mock_server::{MockEndpoint, MockFaker, MockServer};
pub use pipeline::{BuildError, BuildMode, BuildPipeline, BuildSummary};
pub use pwa::{generate_service_worker, generate_webmanifest, pwa_register_script, PwaConfig};
pub use search::{SearchDoc, SearchHit, SearchIndex};
pub use tailwind::{
    construct_download_url, current_platform, detect_platform, TailwindError, TailwindManager,
};
