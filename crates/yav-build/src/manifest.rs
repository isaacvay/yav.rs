//! `package.yavml` manifest — the central project descriptor for Yav applications.
//!
//! Equivalent to `package.json` (Node/React) or `pubspec.yaml` (Dart/Flutter).
//! Supersedes the simpler `yav.toml` while remaining backward-compatible with it
//! and legacy `package.yav`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ── Root ──────────────────────────────────────────────────────────────────

/// Root structure of a `package.yavml` manifest.
///
/// # Example (`package.yavml`)
/// ```toml
/// [package]
/// name = "my-app"
/// version = "0.1.0"
/// description = "A blazing-fast web app built with Yav"
/// authors = ["Alice <alice@example.com>"]
/// license = "MIT"
///
/// [server]
/// port = 3000
///
/// [dependencies]
/// yav-auth = "0.1"
///
/// [scripts]
/// dev = "yav dev"
/// build = "yav build --release"
/// test = "cargo test"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PackageYav {
    /// Project identity and metadata.
    pub package: PackageMeta,
    /// HTTP server configuration.
    #[serde(default)]
    pub server: ServerSection,
    /// Tailwind CSS standalone compiler options.
    #[serde(default)]
    pub tailwind: TailwindSection,
    /// Build & output settings.
    #[serde(default)]
    pub build: BuildSection,
    /// Yav plugin / extension dependencies.
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    /// Named scripts (like `npm run <script>`).
    #[serde(default = "default_scripts")]
    pub scripts: BTreeMap<String, String>,
    /// Static assets configuration.
    #[serde(default)]
    pub assets: AssetsSection,
    /// Motion and animation configuration.
    #[serde(default)]
    pub motion: MotionSection,
    /// Progressive Web App (PWA) configuration.
    #[serde(default)]
    pub pwa: PwaSection,
}

impl Default for PackageYav {
    fn default() -> Self {
        Self {
            package: PackageMeta::default(),
            server: ServerSection::default(),
            tailwind: TailwindSection::default(),
            build: BuildSection::default(),
            dependencies: BTreeMap::new(),
            scripts: default_scripts(),
            assets: AssetsSection::default(),
            motion: MotionSection::default(),
            pwa: PwaSection::default(),
        }
    }
}

// ── [package] ─────────────────────────────────────────────────────────────

/// Project identity and metadata — equivalent to the `name` / `version` /
/// `description` fields in `package.json`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PackageMeta {
    /// Crate / project name.
    pub name: String,
    /// Semver version string.
    #[serde(default = "default_version")]
    pub version: String,
    /// One-line project description.
    #[serde(default)]
    pub description: String,
    /// List of authors (`"Name <email>"` format).
    #[serde(default)]
    pub authors: Vec<String>,
    /// SPDX license identifier.
    #[serde(default)]
    pub license: String,
    /// Repository URL.
    #[serde(default)]
    pub repository: String,
    /// Homepage URL.
    #[serde(default)]
    pub homepage: String,
    /// Keywords for discovery.
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl Default for PackageMeta {
    fn default() -> Self {
        Self {
            name: "my-app".into(),
            version: default_version(),
            description: String::new(),
            authors: Vec::new(),
            license: "MIT".into(),
            repository: String::new(),
            homepage: String::new(),
            keywords: Vec::new(),
        }
    }
}

// ── [server] ──────────────────────────────────────────────────────────────

/// HTTP server runtime configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ServerSection {
    /// Port number to listen on.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Host address to bind to.
    #[serde(default = "default_host")]
    pub host: String,
    /// Enable CORS (default: `false`).
    #[serde(default)]
    pub cors: bool,
    /// Enable HTTPS (requires `tls_cert` and `tls_key`).
    #[serde(default)]
    pub https: bool,
    /// Path to TLS certificate file.
    #[serde(default)]
    pub tls_cert: String,
    /// Path to TLS private key file.
    #[serde(default)]
    pub tls_key: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            cors: false,
            https: false,
            tls_cert: String::new(),
            tls_key: String::new(),
        }
    }
}

// ── [tailwind] ────────────────────────────────────────────────────────────

/// Tailwind CSS standalone CLI configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TailwindSection {
    /// Whether Tailwind CSS compilation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Target Tailwind standalone CLI version.
    #[serde(default = "default_tw_version")]
    pub version: String,
    /// Path to the input CSS file.
    #[serde(default = "default_input_css")]
    pub input: String,
    /// Path to the generated output CSS file.
    #[serde(default = "default_output_css")]
    pub output: String,
}

impl Default for TailwindSection {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            version: default_tw_version(),
            input: default_input_css(),
            output: default_output_css(),
        }
    }
}

// ── [build] ───────────────────────────────────────────────────────────────

/// Static build & output configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildSection {
    /// Output directory for production build.
    #[serde(default = "default_out_dir")]
    pub out_dir: String,
    /// Directory containing public static assets.
    #[serde(default = "default_public_dir")]
    pub public_dir: String,
    /// Minify CSS / HTML in production builds.
    #[serde(default = "default_true")]
    pub minify: bool,
    /// Generate content-hash filenames for cache busting.
    #[serde(default = "default_true")]
    pub fingerprint: bool,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            out_dir: default_out_dir(),
            public_dir: default_public_dir(),
            minify: true,
            fingerprint: true,
        }
    }
}

// ── [dependencies] ────────────────────────────────────────────────────────

/// A dependency specification — version string or detailed table.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DependencySpec {
    /// Simple version string: `"0.1"`.
    Version(String),
    /// Detailed spec with optional path, git, features.
    Detailed(DetailedDep),
}

/// Detailed dependency with path / git / features support.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DetailedDep {
    /// Version requirement.
    #[serde(default)]
    pub version: Option<String>,
    /// Local path to the crate.
    #[serde(default)]
    pub path: Option<String>,
    /// Git repository URL.
    #[serde(default)]
    pub git: Option<String>,
    /// Git branch name.
    #[serde(default)]
    pub branch: Option<String>,
    /// Enabled features.
    #[serde(default)]
    pub features: Vec<String>,
}

// ── [assets] ──────────────────────────────────────────────────────────────

/// Static assets configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AssetsSection {
    /// Path to favicon file.
    #[serde(default = "default_favicon")]
    pub favicon: String,
    /// Path to document / project icon file (e.g. `"public/yav-document.png"`).
    #[serde(default)]
    pub icon: Option<String>,
    /// Additional font files or URLs.
    #[serde(default)]
    pub fonts: Vec<String>,
    /// Additional static directories to copy into the build.
    #[serde(default)]
    pub static_dirs: Vec<String>,
}

fn default_favicon() -> String {
    "public/favicon.svg".into()
}

impl Default for AssetsSection {
    fn default() -> Self {
        Self {
            favicon: default_favicon(),
            icon: Some("public/yav-document.png".into()),
            fonts: Vec::new(),
            static_dirs: Vec::new(),
        }
    }
}

// ── [motion] ──────────────────────────────────────────────────────────────

/// Motion and animation configuration (`[motion]`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MotionSection {
    /// Enable the Yav motion runtime (default: `true`).
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Respect user's prefers-reduced-motion (`"user"`, `"always"`, or `"never"`).
    #[serde(default = "default_reduced_motion")]
    pub reduced_motion: String,
}

impl Default for MotionSection {
    fn default() -> Self {
        Self {
            enabled: true,
            reduced_motion: "user".into(),
        }
    }
}

// ── [pwa] ─────────────────────────────────────────────────────────────────

/// Progressive Web App (PWA) configuration (`[pwa]`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PwaSection {
    /// Enable PWA features (default: `false`).
    #[serde(default)]
    pub enabled: bool,
    /// PWA application name (defaults to package.name if empty).
    #[serde(default)]
    pub name: String,
    /// PWA short name for mobile app tiles.
    #[serde(default)]
    pub short_name: String,
    /// PWA browser theme color (e.g. `"#0f172a"`).
    #[serde(default = "default_pwa_theme_color")]
    pub theme_color: String,
    /// PWA splash/background color (e.g. `"#ffffff"`).
    #[serde(default = "default_pwa_background_color")]
    pub background_color: String,
    /// PWA display mode (`"standalone"`, `"fullscreen"`, `"minimal-ui"`, `"browser"`).
    #[serde(default = "default_pwa_display")]
    pub display: String,
    /// Start URL when launched from home screen.
    #[serde(default = "default_pwa_start_url")]
    pub start_url: String,
}

impl Default for PwaSection {
    fn default() -> Self {
        Self {
            enabled: false,
            name: String::new(),
            short_name: String::new(),
            theme_color: "#0f172a".into(),
            background_color: "#0f172a".into(),
            display: "standalone".into(),
            start_url: "/".into(),
        }
    }
}

// ── Defaults ──────────────────────────────────────────────────────────────

fn default_reduced_motion() -> String {
    "user".into()
}
fn default_pwa_theme_color() -> String {
    "#0f172a".into()
}
fn default_pwa_background_color() -> String {
    "#0f172a".into()
}
fn default_pwa_display() -> String {
    "standalone".into()
}
fn default_pwa_start_url() -> String {
    "/".into()
}

fn default_version() -> String {
    "0.1.0".into()
}
fn default_port() -> u16 {
    3000
}
fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_true() -> bool {
    true
}
fn default_tw_version() -> String {
    "3.4.17".into()
}
fn default_input_css() -> String {
    "src/styles/app.css".into()
}
fn default_output_css() -> String {
    "public/styles.css".into()
}
fn default_out_dir() -> String {
    "dist".into()
}
fn default_public_dir() -> String {
    "public".into()
}

fn default_scripts() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("dev".into(), "yav dev".into());
    m.insert("build".into(), "yav build".into());
    m.insert("test".into(), "cargo test".into());
    m
}

// ── File name constants ──────────────────────────────────────────────────

/// The canonical manifest file name.
pub const MANIFEST_FILE: &str = "package.yavml";

/// Legacy manifest file name supported for backward compatibility.
pub const LEGACY_PACKAGE_YAV: &str = "package.yav";

// ── Load / Save ──────────────────────────────────────────────────────────

impl PackageYav {
    /// Load a `package.yavml` (or `package.yav`) manifest from the given path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ManifestError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_str(&content)
    }

    /// Parse from a TOML string (convenience alias for `FromStr::from_str`).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(content: &str) -> Result<Self, ManifestError> {
        <Self as std::str::FromStr>::from_str(content)
    }

    /// Serialize to a pretty TOML string.
    pub fn to_toml(&self) -> Result<String, ManifestError> {
        toml::to_string_pretty(self).map_err(ManifestError::Serialize)
    }

    /// Save to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ManifestError> {
        let path = path.as_ref();
        let content = self.to_toml()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ManifestError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::write(path, content).map_err(|e| ManifestError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    }

    /// Discover and load the manifest from a given root directory.
    ///
    /// Searches for `package.yavml` first, then `package.yav`, then falls back to `yav.toml`
    /// (converting it to a [`PackageYav`] for backward compatibility).
    pub fn discover_in(root: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let root = root.as_ref();
        let pkg_yavml = root.join(MANIFEST_FILE);
        if pkg_yavml.exists() {
            return Self::load(&pkg_yavml);
        }

        let pkg_yav = root.join(LEGACY_PACKAGE_YAV);
        if pkg_yav.exists() {
            return Self::load(&pkg_yav);
        }

        // Fallback: convert legacy yav.toml
        let legacy_path = root.join("yav.toml");
        if legacy_path.exists() {
            let cfg = super::config::YavConfig::load(&legacy_path)
                .map_err(|e| ManifestError::Legacy(e.to_string()))?;
            return Ok(Self::from_legacy(&cfg));
        }

        // No manifest found — return defaults
        Ok(Self::default())
    }

    /// Discover and load the manifest from the current directory.
    pub fn discover() -> Result<Self, ManifestError> {
        Self::discover_in(".")
    }

    /// Convert a legacy [`YavConfig`] (`yav.toml`) into a [`PackageYav`].
    pub fn from_legacy(cfg: &super::config::YavConfig) -> Self {
        Self {
            package: PackageMeta {
                name: cfg.project.name.clone(),
                version: cfg.project.version.clone(),
                ..PackageMeta::default()
            },
            server: ServerSection {
                port: cfg.server.port,
                host: cfg.server.host.clone(),
                ..ServerSection::default()
            },
            tailwind: TailwindSection {
                enabled: cfg.tailwind.enabled,
                version: cfg.tailwind.version.clone(),
                input: cfg.tailwind.input_css.clone(),
                output: cfg.tailwind.output_css.clone(),
            },
            build: BuildSection {
                out_dir: cfg.build.out_dir.clone(),
                public_dir: cfg.build.public_dir.clone(),
                minify: cfg.build.minify,
                fingerprint: cfg.build.fingerprint,
            },
            dependencies: BTreeMap::new(),
            scripts: default_scripts(),
            assets: AssetsSection::default(),
            motion: MotionSection::default(),
            pwa: PwaSection::default(),
        }
    }

    /// Convert this [`PackageYav`] into a [`YavConfig`] for backwards compatibility.
    pub fn to_config(&self) -> super::config::YavConfig {
        super::config::YavConfig {
            project: super::config::ProjectConfig {
                name: self.package.name.clone(),
                version: self.package.version.clone(),
            },
            server: super::config::ServerConfig {
                port: self.server.port,
                host: self.server.host.clone(),
            },
            tailwind: super::config::TailwindConfig {
                enabled: self.tailwind.enabled,
                version: self.tailwind.version.clone(),
                input_css: self.tailwind.input.clone(),
                output_css: self.tailwind.output.clone(),
            },
            build: super::config::BuildConfig {
                out_dir: self.build.out_dir.clone(),
                public_dir: self.build.public_dir.clone(),
                minify: self.build.minify,
                fingerprint: self.build.fingerprint,
            },
        }
    }
}

impl std::str::FromStr for PackageYav {
    type Err = ManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s).map_err(ManifestError::Parse)
    }
}

impl From<PackageYav> for super::config::YavConfig {
    fn from(pkg: PackageYav) -> Self {
        pkg.to_config()
    }
}

impl From<&PackageYav> for super::config::YavConfig {
    fn from(pkg: &PackageYav) -> Self {
        pkg.to_config()
    }
}

// ── Errors ────────────────────────────────────────────────────────────────

/// Errors related to `package.yavml` manifest operations.
#[derive(Debug)]
pub enum ManifestError {
    /// I/O error reading or writing the manifest.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// TOML parse error.
    Parse(toml::de::Error),
    /// TOML serialization error.
    Serialize(toml::ser::Error),
    /// Error converting from legacy `yav.toml`.
    Legacy(String),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "I/O error at '{}': {}", path.display(), source)
            }
            Self::Parse(e) => write!(f, "Failed to parse package.yavml: {e}"),
            Self::Serialize(e) => write!(f, "Failed to serialize package.yavml: {e}"),
            Self::Legacy(msg) => write!(f, "Legacy yav.toml conversion error: {msg}"),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse(e) => Some(e),
            Self::Serialize(e) => Some(e),
            Self::Legacy(_) => None,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
            [package]
            name = "hello-world"
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        assert_eq!(pkg.package.name, "hello-world");
        assert_eq!(pkg.package.version, "0.1.0");
        assert_eq!(pkg.server.port, 3000);
        assert!(pkg.tailwind.enabled);
        assert!(pkg.dependencies.is_empty());
        assert_eq!(pkg.scripts.len(), 3); // dev, build, test
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
            [package]
            name = "super-app"
            version = "2.0.0"
            description = "The ultimate web app"
            authors = ["Alice <a@b.com>", "Bob <b@c.com>"]
            license = "Apache-2.0"
            repository = "https://github.com/example/super-app"
            homepage = "https://super-app.dev"
            keywords = ["web", "fast", "rust"]

            [server]
            port = 8080
            host = "0.0.0.0"
            cors = true

            [tailwind]
            enabled = false
            version = "4.0.0"
            input = "assets/main.css"
            output = "public/bundle.css"

            [build]
            out_dir = "build"
            public_dir = "static"
            minify = false
            fingerprint = false

            [dependencies]
            yav-auth = "0.1"

            [dependencies.yav-db]
            version = "0.2"
            features = ["postgres", "sqlite"]

            [scripts]
            dev = "yav dev --open"
            build = "yav build --release"
            test = "cargo test --workspace"
            lint = "cargo clippy -- -D warnings"
            deploy = "fly deploy"

            [assets]
            favicon = "public/icon.svg"
            fonts = ["https://fonts.googleapis.com/css2?family=Inter"]
            static_dirs = ["images", "videos"]
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        assert_eq!(pkg.package.name, "super-app");
        assert_eq!(pkg.package.version, "2.0.0");
        assert_eq!(pkg.package.authors.len(), 2);
        assert_eq!(pkg.package.keywords, vec!["web", "fast", "rust"]);
        assert_eq!(pkg.server.port, 8080);
        assert!(pkg.server.cors);
        assert!(!pkg.tailwind.enabled);
        assert_eq!(pkg.tailwind.version, "4.0.0");
        assert!(!pkg.build.minify);
        assert!(!pkg.build.fingerprint);
        assert_eq!(pkg.dependencies.len(), 2);
        assert_eq!(pkg.scripts.len(), 5);
        assert_eq!(pkg.assets.fonts.len(), 1);
        assert_eq!(pkg.assets.static_dirs, vec!["images", "videos"]);

        // Check detailed dep
        match &pkg.dependencies["yav-db"] {
            DependencySpec::Detailed(d) => {
                assert_eq!(d.version.as_deref(), Some("0.2"));
                assert_eq!(d.features, vec!["postgres", "sqlite"]);
            }
            _ => panic!("Expected detailed dep"),
        }
    }

    #[test]
    fn roundtrip_serialization() {
        let original = PackageYav::default();
        let toml_str = original.to_toml().unwrap();
        let parsed = PackageYav::from_str(&toml_str).unwrap();
        assert_eq!(original.package, parsed.package);
        assert_eq!(original.server, parsed.server);
        assert_eq!(original.tailwind, parsed.tailwind);
        assert_eq!(original.build, parsed.build);
    }

    #[test]
    fn from_legacy_config() {
        let cfg = super::super::config::YavConfig {
            project: super::super::config::ProjectConfig {
                name: "legacy-app".into(),
                version: "1.0.0".into(),
            },
            server: super::super::config::ServerConfig {
                port: 4000,
                host: "0.0.0.0".into(),
            },
            tailwind: super::super::config::TailwindConfig::default(),
            build: super::super::config::BuildConfig::default(),
        };
        let pkg = PackageYav::from_legacy(&cfg);
        assert_eq!(pkg.package.name, "legacy-app");
        assert_eq!(pkg.server.port, 4000);
        assert_eq!(pkg.server.host, "0.0.0.0");
    }

    #[test]
    fn dependency_spec_version_string() {
        let toml = r#"
            [package]
            name = "dep-test"

            [dependencies]
            yav-auth = "0.3"
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        match &pkg.dependencies["yav-auth"] {
            DependencySpec::Version(v) => assert_eq!(v, "0.3"),
            _ => panic!("Expected Version variant"),
        }
    }

    #[test]
    fn dependency_spec_path() {
        let toml = r#"
            [package]
            name = "path-test"

            [dependencies.my-plugin]
            path = "../plugins/my-plugin"
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        match &pkg.dependencies["my-plugin"] {
            DependencySpec::Detailed(d) => {
                assert_eq!(d.path.as_deref(), Some("../plugins/my-plugin"));
            }
            _ => panic!("Expected Detailed variant"),
        }
    }

    #[test]
    fn dependency_spec_git() {
        let toml = r#"
            [package]
            name = "git-test"

            [dependencies.yav-analytics]
            git = "https://github.com/yav-rs/analytics.git"
            branch = "main"
            features = ["charts"]
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        match &pkg.dependencies["yav-analytics"] {
            DependencySpec::Detailed(d) => {
                assert_eq!(
                    d.git.as_deref(),
                    Some("https://github.com/yav-rs/analytics.git")
                );
                assert_eq!(d.branch.as_deref(), Some("main"));
                assert_eq!(d.features, vec!["charts"]);
            }
            _ => panic!("Expected Detailed variant"),
        }
    }

    #[test]
    fn scripts_custom_override() {
        let toml = r#"
            [package]
            name = "scripts-test"

            [scripts]
            dev = "yav dev --port 4000"
            deploy = "docker push myapp"
        "#;
        let pkg = PackageYav::from_str(toml).unwrap();
        assert_eq!(pkg.scripts["dev"], "yav dev --port 4000");
        assert_eq!(pkg.scripts["deploy"], "docker push myapp");
        // Note: custom scripts replace defaults entirely (serde behavior)
        assert!(!pkg.scripts.contains_key("test"));
    }

    #[test]
    fn motion_and_pwa_sections() {
        let toml = r##"
            [package]
            name = "motion-pwa-app"

            [motion]
            enabled = true
            reduced_motion = "user"

            [pwa]
            enabled = true
            name = "Motion PWA App"
            short_name = "MotionApp"
            theme_color = "#3b82f6"
            background_color = "#0f172a"
            display = "standalone"
            start_url = "/"
        "##;
        let pkg = PackageYav::from_str(toml).unwrap();
        assert!(pkg.motion.enabled);
        assert_eq!(pkg.motion.reduced_motion, "user");
        assert!(pkg.pwa.enabled);
        assert_eq!(pkg.pwa.name, "Motion PWA App");
        assert_eq!(pkg.pwa.short_name, "MotionApp");
        assert_eq!(pkg.pwa.theme_color, "#3b82f6");
    }
}
