//! Self-updater and project dependency updater (`yav update`).
//!
//! Checks `crates.io` for the latest versions of `yav-cli` and the `yav` framework,
//! upgrades the installed CLI binary, updates project manifests (`package.yavml` and `Cargo.toml`),
//! refreshes `Cargo.lock` via `cargo update`, and ensures the standalone Tailwind CSS compiler is up to date.

use clap::Args;
use std::fs;
use std::path::Path;
use std::time::Duration;
use yav_build::{PackageYav, TailwindManager};

/// Current version of the `yav-cli` binary compiled at build time.
pub const CURRENT_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Official GitHub repository URL for source fallback/git updates.
pub const YAV_REPO_URL: &str = "https://github.com/isaacvay/yav.rs";

/// Known Yav workspace crate names that may appear in user manifests.
const YAV_CRATES: &[&str] = &[
    "yav",
    "yav-core",
    "yav-server",
    "yav-ui",
    "yav-router",
    "yav-macros",
    "yav-icons",
    "yav-build",
];

/// Command-line flags for `yav update`.
#[derive(Args, Debug, Clone, Default)]
pub struct UpdateArgs {
    /// Only check for available updates without installing or modifying files
    #[arg(short, long)]
    pub check: bool,

    /// Only update the `yav` CLI binary itself
    #[arg(long = "self", alias = "cli")]
    pub self_only: bool,

    /// Only update the current Yav project dependencies (`package.yavml`, `Cargo.toml`, `Cargo.lock`)
    #[arg(long)]
    pub project: bool,

    /// Update `yav-cli` directly from the official GitHub repository (`main` branch)
    #[arg(long)]
    pub git: bool,

    /// Force reinstallation or update even if already on the latest version
    #[arg(short, long)]
    pub force: bool,
}

/// Parse a `MAJOR.MINOR.PATCH` semantic version string, ignoring leading `v` and pre-release suffixes.
pub fn parse_semver(version: &str) -> Option<(u64, u64, u64)> {
    let trimmed = version.trim().trim_start_matches('v');
    let core = trimmed.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    Some((major, minor, patch))
}

/// Returns `true` if `latest` is strictly newer than `current`.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => current.trim() != latest.trim(),
    }
}

/// Extract the latest version string from a crates.io `/api/v1/crates/<name>` JSON response.
pub fn extract_crates_io_version(json_str: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let krate = val.get("crate")?;
    if let Some(stable) = krate.get("max_stable_version").and_then(|v| v.as_str()) {
        if !stable.is_empty() && stable != "0.0.0" {
            return Some(stable.to_string());
        }
    }
    krate
        .get("max_version")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
}

/// Fetch the latest published version of a crate from `crates.io`.
pub async fn fetch_latest_crate_version(crate_name: &str) -> Option<String> {
    let url = format!("https://crates.io/api/v1/crates/{crate_name}");
    let client = reqwest::Client::builder()
        .user_agent(format!("yav-cli/{CURRENT_CLI_VERSION} ({YAV_REPO_URL})"))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let text = resp.text().await.ok()?;
    extract_crates_io_version(&text)
}

/// Update `yav` and `yav-*` version strings inside a TOML manifest (`package.yavml` or `Cargo.toml`)
/// while preserving comments, formatting, and `path` / `git` dependencies.
///
/// Returns `(updated_content, Vec<(crate_name, old_version, new_version)>)`.
pub fn update_manifest_yav_versions(
    content: &str,
    latest_version: &str,
    force: bool,
) -> (String, Vec<(String, String, String)>) {
    let mut updated_lines = Vec::new();
    let mut changes = Vec::new();
    let mut in_deps_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|c| c == '[' || c == ']').trim();
            in_deps_section = section == "dependencies"
                || section == "dev-dependencies"
                || section == "workspace.dependencies";
            updated_lines.push(line.to_string());
            continue;
        }

        if in_deps_section && !trimmed.starts_with('#') {
            if let Some((lhs, rhs)) = line.split_once('=') {
                let dep_name = lhs.trim();
                if YAV_CRATES.contains(&dep_name) {
                    let rhs_trimmed = rhs.trim();
                    // Skip path or git dependencies unless they don't specify path/git
                    let is_local_or_git =
                        rhs_trimmed.contains("path") || rhs_trimmed.contains("git");
                    if !is_local_or_git {
                        if let Some((new_line, old_ver)) =
                            replace_version_in_dep_line(line, latest_version, force)
                        {
                            changes.push((
                                dep_name.to_string(),
                                old_ver,
                                latest_version.to_string(),
                            ));
                            updated_lines.push(new_line);
                            continue;
                        }
                    }
                }
            }
        }

        updated_lines.push(line.to_string());
    }

    let mut result = updated_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    (result, changes)
}

fn replace_version_in_dep_line(
    line: &str,
    latest_version: &str,
    force: bool,
) -> Option<(String, String)> {
    let (lhs, rhs) = line.split_once('=')?;
    let rhs_trimmed = rhs.trim();

    // Case 1: Simple string version `yav = "0.1.0"`
    if let Some(stripped) = rhs_trimmed.strip_prefix('"') {
        let end_quote = stripped.find('"')?;
        let old_ver = &stripped[..end_quote];
        if is_newer_version(old_ver, latest_version) || (force && old_ver != latest_version) {
            let new_rhs = rhs.replacen(
                &format!("\"{old_ver}\""),
                &format!("\"{latest_version}\""),
                1,
            );
            return Some((format!("{lhs}={new_rhs}"), old_ver.to_string()));
        }
        return None;
    }

    // Case 2: Inline table `yav = { version = "0.1.0", features = [...] }`
    if rhs_trimmed.starts_with('{') {
        if let Some(ver_idx) = rhs.find("version") {
            let after_ver = &rhs[ver_idx + "version".len()..];
            let eq_idx = after_ver.find('=')?;
            let after_eq = &after_ver[eq_idx + 1..];
            let first_quote = after_eq.find('"')?;
            let rest = &after_eq[first_quote + 1..];
            let second_quote = rest.find('"')?;
            let old_ver = &rest[..second_quote];
            if is_newer_version(old_ver, latest_version) || (force && old_ver != latest_version) {
                let new_rhs = rhs.replacen(
                    &format!("\"{old_ver}\""),
                    &format!("\"{latest_version}\""),
                    1,
                );
                return Some((format!("{lhs}={new_rhs}"), old_ver.to_string()));
            }
        }
    }

    None
}

async fn install_cli_update(from_git: bool) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = crate::spinner::Spinner::start(if from_git {
        format!("Installing latest yav-cli from Git ({YAV_REPO_URL})")
    } else {
        "Installing latest yav-cli from crates.io".to_string()
    });

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("install");
    if from_git {
        cmd.args(["--git", YAV_REPO_URL, "yav-cli", "--force"]);
    } else {
        cmd.args(["yav-cli", "--force"]);
    }

    let output = cmd.output().await?;
    if output.status.success() {
        spinner.finish_with_success("Successfully updated yav-cli binary!");
        return Ok(());
    }

    if !from_git {
        spinner.finish_with_warning(
            "crates.io install failed; falling back to official GitHub repository...",
        );
        let fallback_spinner =
            crate::spinner::Spinner::start(format!("Installing yav-cli from Git ({YAV_REPO_URL})"));
        let git_output = tokio::process::Command::new("cargo")
            .args(["install", "--git", YAV_REPO_URL, "yav-cli", "--force"])
            .output()
            .await?;
        if git_output.status.success() {
            fallback_spinner
                .finish_with_success("Successfully updated yav-cli binary from GitHub!");
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&git_output.stderr);
        fallback_spinner.finish_with_error("Failed to update yav-cli");
        return Err(format!("cargo install failed:\n{}", stderr.trim()).into());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    spinner.finish_with_error("Failed to update yav-cli");
    Err(format!("cargo install failed:\n{}", stderr.trim()).into())
}

fn is_yav_project_dir() -> bool {
    if Path::new("package.yavml").is_file()
        || Path::new("package.yav").is_file()
        || Path::new("yav.toml").is_file()
    {
        return true;
    }
    if let Ok(cargo_toml) = fs::read_to_string("Cargo.toml") {
        return cargo_toml.contains("yav");
    }
    false
}

/// Executes the `yav update` command.
pub async fn run(args: &UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║                 🔄 YAV UPDATER                    ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    let update_cli = !args.project;
    let update_project = !args.self_only && (args.project || is_yav_project_dir());

    // 1. CLI Self-Update
    if update_cli {
        if args.git {
            if args.check {
                println!(
                    "\x1b[36mℹ\x1b[0m CLI update mode: Git repository ({YAV_REPO_URL}) [current: v{CURRENT_CLI_VERSION}]"
                );
            } else {
                install_cli_update(true).await?;
            }
        } else {
            let spinner =
                crate::spinner::Spinner::start("Checking crates.io for latest yav-cli version");
            match fetch_latest_crate_version("yav-cli").await {
                Some(latest_cli) => {
                    if is_newer_version(CURRENT_CLI_VERSION, &latest_cli) || args.force {
                        spinner.finish_with_success(format!(
                            "Update available for yav-cli: \x1b[33mv{CURRENT_CLI_VERSION}\x1b[0m → \x1b[1;32mv{latest_cli}\x1b[0m"
                        ));
                        if args.check {
                            println!(
                                "  \x1b[90mRun 'yav update' to install yav-cli v{latest_cli}\x1b[0m"
                            );
                        } else {
                            install_cli_update(false).await?;
                        }
                    } else {
                        spinner.finish_with_success(format!(
                            "yav-cli \x1b[1;32mv{CURRENT_CLI_VERSION}\x1b[0m is already up to date"
                        ));
                    }
                }
                None => {
                    spinner.finish_with_warning(format!(
                        "Could not reach crates.io to check yav-cli version (current: v{CURRENT_CLI_VERSION})"
                    ));
                    if args.force && !args.check {
                        install_cli_update(true).await?;
                    }
                }
            }
        }
    }

    // 2. Project Dependencies Update
    if update_project {
        println!();
        let spinner =
            crate::spinner::Spinner::start("Checking crates.io for latest yav framework version");
        let latest_yav = fetch_latest_crate_version("yav")
            .await
            .unwrap_or_else(|| CURRENT_CLI_VERSION.to_string());
        spinner.finish_with_success(format!(
            "Latest yav framework version: \x1b[1;32mv{latest_yav}\x1b[0m"
        ));

        for manifest_path in ["package.yavml", "package.yav", "Cargo.toml"] {
            let path = Path::new(manifest_path);
            if !path.is_file() {
                continue;
            }
            if let Ok(content) = fs::read_to_string(path) {
                let (new_content, changes) =
                    update_manifest_yav_versions(&content, &latest_yav, args.force);
                if !changes.is_empty() {
                    for (krate, old_v, new_v) in &changes {
                        println!(
                            "  \x1b[36m•\x1b[0m {manifest_path}: \x1b[1m{krate}\x1b[0m \x1b[33mv{old_v}\x1b[0m → \x1b[1;32mv{new_v}\x1b[0m"
                        );
                    }
                    if !args.check {
                        fs::write(path, new_content)?;
                        println!(
                            "\x1b[32m✔\x1b[0m Updated dependency versions in \x1b[1m{manifest_path}\x1b[0m"
                        );
                    }
                } else {
                    println!(
                        "\x1b[32m✔\x1b[0m \x1b[1m{manifest_path}\x1b[0m is already up to date"
                    );
                }
            }
        }

        if !args.check && Path::new("Cargo.toml").is_file() {
            let cargo_spinner =
                crate::spinner::Spinner::start("Updating Cargo.lock dependencies (cargo update)");
            let status = tokio::process::Command::new("cargo")
                .arg("update")
                .output()
                .await;
            match status {
                Ok(out) if out.status.success() => {
                    cargo_spinner.finish_with_success("Cargo.lock dependencies updated");
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    cargo_spinner.finish_with_warning(format!(
                        "cargo update reported a warning: {}",
                        stderr.lines().next().unwrap_or("unknown error")
                    ));
                }
                Err(e) => {
                    cargo_spinner.finish_with_warning(format!("Failed to run cargo update: {e}"));
                }
            }
        }

        // Check Tailwind standalone binary if configured in project
        if !args.check {
            if let Ok(manifest) = PackageYav::discover() {
                let cfg = manifest.to_config();
                if cfg.tailwind.enabled {
                    let mgr = TailwindManager::new(&cfg.tailwind.version);
                    if !mgr.is_cached() || args.force {
                        if let Ok(bin_path) = mgr.binary_path() {
                            let _ = crate::tailwind_setup::download_with_progress(&mgr, &bin_path)
                                .await;
                        }
                    } else {
                        println!(
                            "\x1b[32m✔\x1b[0m Tailwind CSS standalone compiler (v{}) is cached and ready",
                            cfg.tailwind.version
                        );
                    }
                }
            }
        }
    }

    println!("\n\x1b[1;32m✨ Update complete!\x1b[0m");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver_and_comparison() {
        assert_eq!(parse_semver("0.1.2"), Some((0, 1, 2)));
        assert_eq!(parse_semver("v1.4.0-beta.1"), Some((1, 4, 0)));
        assert!(is_newer_version("0.1.1", "0.1.2"));
        assert!(is_newer_version("0.1.9", "0.2.0"));
        assert!(!is_newer_version("0.1.2", "0.1.2"));
        assert!(!is_newer_version("0.2.0", "0.1.9"));
    }

    #[test]
    fn test_extract_crates_io_version() {
        let sample =
            r#"{"crate":{"id":"yav-cli","max_stable_version":"0.1.3","max_version":"0.1.3"}}"#;
        assert_eq!(extract_crates_io_version(sample), Some("0.1.3".to_string()));
    }

    #[test]
    fn test_update_manifest_yav_versions_preserves_path_and_updates_semver() {
        let input = r#"[package]
name = "my-app"
version = "0.1.0"

[dependencies]
yav = "0.1.0"
yav-ui = { version = "0.1.0", features = ["islands"] }
yav-core = { path = "../crates/yav-core" }
serde = "1.0"
"#;

        let (updated, changes) = update_manifest_yav_versions(input, "0.1.2", false);
        assert_eq!(changes.len(), 2);
        assert_eq!(
            changes[0],
            ("yav".to_string(), "0.1.0".to_string(), "0.1.2".to_string())
        );
        assert_eq!(
            changes[1],
            (
                "yav-ui".to_string(),
                "0.1.0".to_string(),
                "0.1.2".to_string()
            )
        );
        assert!(updated.contains(r#"yav = "0.1.2""#));
        assert!(updated.contains(r#"yav-ui = { version = "0.1.2", features = ["islands"] }"#));
        assert!(updated.contains(r#"yav-core = { path = "../crates/yav-core" }"#));
        assert!(updated.contains(r#"serde = "1.0""#));
    }
}
