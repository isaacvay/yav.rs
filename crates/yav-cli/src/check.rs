//! Project validation runner (`yav check`).
//!
//! Validates project configuration, required directories and files,
//! and runs `cargo check` to verify code correctness.

use std::path::Path;
use yav_build::PackageYav;

/// Result of project structure and configuration validation.
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Errors found during checks.
    pub errors: Vec<String>,
    /// Warnings found during checks.
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Returns true if no errors were encountered.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validates project directories and manifest configuration without invoking external tools.
pub fn validate_project_structure(root: &Path) -> ValidationResult {
    let mut result = ValidationResult::default();

    if !root.join("Cargo.toml").is_file() {
        result
            .errors
            .push("Missing Cargo.toml in project root".to_string());
    }

    match PackageYav::discover_in(root) {
        Ok(manifest) => {
            if manifest.package.name.trim().is_empty() {
                result
                    .errors
                    .push("Project name in manifest cannot be empty".to_string());
            }
            if manifest.server.port == 0 {
                result
                    .errors
                    .push("Server port in manifest cannot be 0".to_string());
            }
            if manifest.tailwind.enabled {
                let input = root.join(&manifest.tailwind.input);
                if !input.is_file() {
                    result.warnings.push(format!(
                        "Tailwind is enabled, but input CSS '{}' was not found",
                        input.display()
                    ));
                }
            }
        }
        Err(e) => {
            result
                .errors
                .push(format!("Failed to parse project manifest: {e}"));
        }
    }

    if !root.join("src").is_dir() {
        result
            .errors
            .push("Missing required directory: src/".to_string());
    }

    if !root.join("src/main.rs").is_file() {
        result
            .errors
            .push("Missing required file: src/main.rs".to_string());
    }

    result
}

/// Executes the `yav check` command.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║               🔍 YAV PROJECT CHECK                ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    let root = Path::new(".");
    let val = validate_project_structure(root);

    for warn in &val.warnings {
        println!("\x1b[33m⚠️ Warning: {warn}\x1b[0m");
    }

    for err in &val.errors {
        eprintln!("\x1b[31m❌ Error: {err}\x1b[0m");
    }

    if !val.is_ok() {
        return Err("Project validation failed with errors".into());
    }

    println!("\x1b[32m✔ Project structure and configuration are valid\x1b[0m");
    println!("\x1b[34mRunning cargo check...\x1b[0m\n");

    let status = tokio::process::Command::new("cargo")
        .arg("check")
        .status()
        .await?;

    if status.success() {
        println!("\n\x1b[1;32m✨ All project checks passed successfully!\x1b[0m");
        Ok(())
    } else {
        Err(format!(
            "cargo check failed with exit code: {}",
            status.code().unwrap_or(-1)
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_dir() {
        let temp_dir = std::env::temp_dir().join("yav_test_check_empty");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let res = validate_project_structure(&temp_dir);
        assert!(!res.is_ok());
        assert!(res.errors.iter().any(|e| e.contains("Cargo.toml")));
        assert!(res.errors.iter().any(|e| e.contains("src/")));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_valid_dir() {
        let temp_dir = std::env::temp_dir().join("yav_test_check_valid");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(temp_dir.join("src"));

        std::fs::write(temp_dir.join("Cargo.toml"), "[package]\nname = \"foo\"").unwrap();
        std::fs::write(temp_dir.join("src/main.rs"), "fn main() {}").unwrap();

        let res = validate_project_structure(&temp_dir);
        assert!(res.is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
