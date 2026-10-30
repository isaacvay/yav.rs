//! Configuration management for Yav projects.
//!
//! Handles loading and parsing of `yav.toml` project files.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root configuration structure corresponding to `yav.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct YavConfig {
    /// General project metadata.
    pub project: ProjectConfig,
    /// HTTP server configuration.
    #[serde(default)]
    pub server: ServerConfig,
    /// Tailwind CSS standalone compiler options.
    #[serde(default)]
    pub tailwind: TailwindConfig,
    /// Static build and output settings.
    #[serde(default)]
    pub build: BuildConfig,
}

/// Project metadata configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProjectConfig {
    /// Name of the project.
    pub name: String,
    /// Project version string.
    #[serde(default = "default_version")]
    pub version: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "my-app".to_string(),
            version: default_version(),
        }
    }
}

/// Server runtime configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ServerConfig {
    /// Port number to listen on.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Host address to bind to.
    #[serde(default = "default_host")]
    pub host: String,
    /// Enable response compression (gzip, brotli).
    #[serde(default)]
    pub compression: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            compression: false,
        }
    }
}

/// Tailwind CSS standalone CLI configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TailwindConfig {
    /// Whether Tailwind CSS compilation is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Target Tailwind standalone CLI version (e.g. "3.4.17").
    #[serde(default = "default_tw_version")]
    pub version: String,
    /// Path to the input CSS file.
    #[serde(default = "default_input_css")]
    pub input_css: String,
    /// Path to the generated output CSS file.
    #[serde(default = "default_output_css")]
    pub output_css: String,
}

impl Default for TailwindConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            version: default_tw_version(),
            input_css: default_input_css(),
            output_css: default_output_css(),
        }
    }
}

/// Static build configuration.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildConfig {
    /// Output directory for production build assets.
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

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            out_dir: default_out_dir(),
            public_dir: default_public_dir(),
            minify: true,
            fingerprint: true,
        }
    }
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_true() -> bool {
    true
}

fn default_tw_version() -> String {
    "3.4.17".to_string()
}

fn default_out_dir() -> String {
    "dist".to_string()
}

fn default_public_dir() -> String {
    "public".to_string()
}

fn default_input_css() -> String {
    "src/styles/app.css".to_string()
}

fn default_output_css() -> String {
    "public/styles.css".to_string()
}

impl YavConfig {
    /// Loads a `yav.toml` file from the specified path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_toml(&content)
    }

    /// Parses configuration from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, ConfigError> {
        toml::from_str(content).map_err(ConfigError::Parse)
    }

    /// Serializes this configuration to a pretty TOML string.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(ConfigError::Serialize)
    }

    /// Saves this configuration to the specified path.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let content = self.to_toml()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        std::fs::write(path, content).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    }
}

/// Errors related to configuration loading, parsing, and saving.
#[derive(Debug)]
pub enum ConfigError {
    /// An I/O error occurred when reading or writing configuration.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// An error occurred while parsing the TOML content.
    Parse(toml::de::Error),
    /// An error occurred while serializing the configuration.
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "I/O error at path '{}': {}", path.display(), source)
            }
            Self::Parse(err) => write!(f, "Failed to parse TOML configuration: {err}"),
            Self::Serialize(err) => write!(f, "Failed to serialize TOML configuration: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse(err) => Some(err),
            Self::Serialize(err) => Some(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = YavConfig::default();
        assert_eq!(config.project.name, "my-app");
        assert_eq!(config.project.version, "0.1.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.host, "127.0.0.1");
        assert!(config.tailwind.enabled);
        assert_eq!(config.tailwind.version, "3.4.17");
        assert_eq!(config.tailwind.input_css, "src/styles/app.css");
        assert_eq!(config.tailwind.output_css, "public/styles.css");
        assert_eq!(config.build.out_dir, "dist");
        assert_eq!(config.build.public_dir, "public");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
            [project]
            name = "awesome-site"
        "#;
        let config = YavConfig::from_toml(toml_str).expect("Should parse minimal config");
        assert_eq!(config.project.name, "awesome-site");
        assert_eq!(config.project.version, "0.1.0");
        assert_eq!(config.server.port, 3000);
        assert!(config.tailwind.enabled);
        assert_eq!(config.build.out_dir, "dist");
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
            [project]
            name = "custom-app"
            version = "1.2.3"

            [server]
            port = 8080
            host = "0.0.0.0"

            [tailwind]
            enabled = false
            version = "3.4.0"
            input_css = "assets/main.css"
            output_css = "public/bundle.css"

            [build]
            out_dir = "build_output"
            public_dir = "static"
        "#;
        let config = YavConfig::from_toml(toml_str).expect("Should parse full config");
        assert_eq!(config.project.name, "custom-app");
        assert_eq!(config.project.version, "1.2.3");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
        assert!(!config.tailwind.enabled);
        assert_eq!(config.tailwind.version, "3.4.0");
        assert_eq!(config.tailwind.input_css, "assets/main.css");
        assert_eq!(config.tailwind.output_css, "public/bundle.css");
        assert_eq!(config.build.out_dir, "build_output");
        assert_eq!(config.build.public_dir, "static");
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = YavConfig::default();
        let serialized = original.to_toml().expect("Serialization should succeed");
        let parsed = YavConfig::from_toml(&serialized).expect("Deserialization should succeed");
        assert_eq!(original, parsed);
    }
}
