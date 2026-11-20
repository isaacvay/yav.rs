//! Self-updater and project dependency updater (`yav update`).
//!
//! Checks `crates.io` for the latest versions of `yav-cli` and the `yav` framework,
//! builds the CLI into a temporary root, atomically replaces the running executable,
//! updates project manifests (`package.yavml`, `package.yav`, `yav.toml` and
//! `Cargo.toml`), refreshes only the changed `yav` crates in `Cargo.lock`, and
//! verifies the cached standalone Tailwind CSS compiler.

use clap::Args;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Stdio;
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
    #[arg(long = "self", alias = "cli", conflicts_with = "project")]
    pub self_only: bool,

    /// Only update the current Yav project dependencies (`package.yavml`, `Cargo.toml`, `Cargo.lock`)
    #[arg(long, conflicts_with = "self_only")]
    pub project: bool,

    /// Update `yav-cli` directly from the official GitHub repository (`main` branch)
    #[arg(long, conflicts_with = "project")]
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
/// Unparsable versions are never treated as updates: a malformed registry
/// response must not trigger a reinstall or a manifest rewrite.
pub fn is_newer_version(current: &str, latest: &str) -> bool {
    match (parse_semver(current), parse_semver(latest)) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => false,
    }
}

/// A manifest is only rewritten when the remote version is a real semver and newer.
/// `--force` reinstalls the CLI, it must never downgrade project dependencies.
fn should_update_version(current: &str, latest: &str) -> bool {
    parse_semver(latest).is_some() && is_newer_version(current, latest)
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

/// Classifies a TOML table header and, for `[dependencies.<name>]` style
/// sections, returns the dependency it describes.
fn section_context(section: &str) -> (bool, Option<String>) {
    const DEP_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

    if DEP_TABLES.contains(&section) || section == "workspace.dependencies" {
        return (true, None);
    }
    for prefix in [
        "dependencies.",
        "dev-dependencies.",
        "build-dependencies.",
        "workspace.dependencies.",
    ] {
        if let Some(name) = section.strip_prefix(prefix) {
            let name = name.trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty() && !name.contains('.') {
                return (true, Some(name.to_string()));
            }
        }
    }
    // `[target.'cfg(unix)'.dependencies]` and `[target.'cfg(unix)'.dependencies.yav]`
    if let Some(index) = section.rfind(".dependencies") {
        let after = &section[index + ".dependencies".len()..];
        if after.is_empty() {
            return (true, None);
        }
        if let Some(name) = after.strip_prefix('.') {
            let name = name.trim_matches(|c| c == '"' || c == '\'');
            if !name.is_empty() && !name.contains('.') {
                return (true, Some(name.to_string()));
            }
        }
    }
    (false, None)
}

fn unquote_key(key: &str) -> &str {
    key.trim().trim_matches(|c| c == '"' || c == '\'')
}

/// Reads a double- or single-quoted TOML string (both are valid TOML).
fn quoted_value(rhs: &str) -> Option<&str> {
    let value = rhs.trim();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &value[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

fn version_from_inline_table(rhs: &str) -> Option<&str> {
    let inner = rhs.trim().strip_prefix('{')?.strip_suffix('}')?;
    for token in inner.split(',') {
        if let Some((key, value)) = token.split_once('=') {
            if unquote_key(key) == "version" {
                return quoted_value(value);
            }
        }
    }
    None
}

fn rewrite_version(rhs: &str, old: &str, latest: &str) -> String {
    for quote in ['"', '\''] {
        let needle = format!("{quote}{old}{quote}");
        if rhs.contains(&needle) {
            return rhs.replacen(&needle, &format!("{quote}{latest}{quote}"), 1);
        }
    }
    rhs.to_string()
}

fn has_local_source(rhs: &str) -> bool {
    let trimmed = rhs.trim();
    let Some(inner) = trimmed.strip_prefix('{').and_then(|v| v.strip_suffix('}')) else {
        return false;
    };
    inner.split(',').any(|token| {
        token
            .split_once('=')
            .is_some_and(|(key, _)| matches!(unquote_key(key), "path" | "git"))
    })
}

/// Rewrites `name = "x"` or `name = { version = "x", ... }` dependency lines.
fn replace_version_in_dep_line(line: &str, latest: &str) -> Option<(String, String)> {
    let (lhs, rhs) = line.split_once('=')?;
    let trimmed = rhs.trim();
    if has_local_source(trimmed) {
        return None;
    }
    let old = if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        quoted_value(trimmed)?
    } else if trimmed.starts_with('{') {
        version_from_inline_table(trimmed)?
    } else {
        return None;
    };
    if !should_update_version(old, latest) {
        return None;
    }
    Some((
        format!("{lhs}={}", rewrite_version(rhs, old, latest)),
        old.to_string(),
    ))
}

/// Rewrites the `version = "x"` key inside `[dependencies.<name>]` sections.
fn replace_version_value_line(line: &str, latest: &str) -> Option<(String, String)> {
    let (lhs, rhs) = line.split_once('=')?;
    if lhs.trim() != "version" {
        return None;
    }
    let old = quoted_value(rhs)?;
    if !should_update_version(old, latest) {
        return None;
    }
    Some((
        format!("{lhs}={}", rewrite_version(rhs, old, latest)),
        old.to_string(),
    ))
}

/// Update `yav` and `yav-*` version strings inside a TOML manifest (`package.yavml`
/// or `Cargo.toml`) while preserving comments and formatting. Path/git dependencies
/// are left alone and a newer local version is never downgraded.
///
/// Returns `(updated_content, Vec<(crate_name, old_version, new_version)>)`.
pub fn update_manifest_yav_versions(
    content: &str,
    latest_version: &str,
) -> (String, Vec<(String, String, String)>) {
    if parse_semver(latest_version).is_none() {
        return (content.to_string(), Vec::new());
    }

    let mut updated_lines = Vec::new();
    let mut changes = Vec::new();
    let mut in_deps_section = false;
    let mut section_crate: Option<String> = None;
    let mut section_is_local = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let section = trimmed.trim_matches(|c| c == '[' || c == ']').trim();
            let (is_deps, krate) = section_context(section);
            in_deps_section = is_deps;
            section_crate = krate;
            section_is_local = false;
            updated_lines.push(line.to_string());
            continue;
        }

        if in_deps_section && !trimmed.starts_with('#') {
            let (key, _) = line.split_once('=').unwrap_or((trimmed, ""));
            if key.trim() == "path" || key.trim() == "git" {
                section_is_local = true;
            }
            let candidate = if let Some(krate) = &section_crate {
                if section_is_local || !YAV_CRATES.contains(&krate.as_str()) {
                    None
                } else {
                    replace_version_value_line(line, latest_version)
                        .map(|(new_line, old)| (krate.clone(), new_line, old))
                }
            } else {
                let dep_name = key.trim().trim_matches('"');
                if YAV_CRATES.contains(&dep_name) {
                    replace_version_in_dep_line(line, latest_version)
                        .map(|(new_line, old)| (dep_name.to_string(), new_line, old))
                } else {
                    None
                }
            };
            if let Some((name, new_line, old_ver)) = candidate {
                changes.push((name, old_ver, latest_version.to_string()));
                updated_lines.push(new_line);
                continue;
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

/// Checks a parsed `Cargo.toml` for an actual Yav dependency instead of relying
/// on a substring search that would also match comments or unrelated names.
fn cargo_toml_declares_yav(content: &str) -> bool {
    let Ok(value) = content.parse::<toml::Value>() else {
        return false;
    };
    fn has_yav(table: &toml::Table) -> bool {
        table.keys().any(|key| YAV_CRATES.contains(&key.as_str()))
    }
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if value
            .get(section)
            .and_then(|v| v.as_table())
            .is_some_and(has_yav)
        {
            return true;
        }
    }
    if value
        .get("workspace")
        .and_then(|v| v.get("dependencies"))
        .and_then(|v| v.as_table())
        .is_some_and(has_yav)
    {
        return true;
    }
    if let Some(targets) = value.get("target").and_then(|v| v.as_table()) {
        for platform in targets.values() {
            for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if platform
                    .get(section)
                    .and_then(|v| v.as_table())
                    .is_some_and(has_yav)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn is_yav_project_dir() -> bool {
    if ["package.yavml", "package.yav", "yav.toml"]
        .iter()
        .any(|name| Path::new(name).is_file())
    {
        return true;
    }
    fs::read_to_string("Cargo.toml")
        .map(|content| cargo_toml_declares_yav(&content))
        .unwrap_or(false)
}

/// Build `yav-cli` into a throwaway root, then atomically replace the running
/// executable. Overwriting `yav.exe` in place is impossible on Windows while it
/// is mapped, so `self_replace` renames the old binary instead.
async fn install_cli_update(from_git: bool) -> Result<(), Box<dyn std::error::Error>> {
    let source = if from_git {
        format!("Git repository ({YAV_REPO_URL})")
    } else {
        "crates.io".to_string()
    };
    println!("Building yav-cli from {source} into a temporary directory...");
    println!("This is a full source build and can take several minutes. Cargo output follows:");

    let temp_root = tempfile::TempDir::new()?;
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("install")
        .arg("--root")
        .arg(temp_root.path())
        .arg("yav-cli")
        .arg("--force");
    if from_git {
        cmd.args(["--git", YAV_REPO_URL]);
    }
    let status = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;
    if !status.success() {
        return Err(format!("cargo install exited with {status}").into());
    }

    let built = temp_root
        .path()
        .join("bin")
        .join(format!("yav{}", std::env::consts::EXE_SUFFIX));
    if !built.is_file() {
        return Err(format!("cargo install did not produce {}", built.display()).into());
    }
    self_replace::self_replace(&built)?;
    println!("yav-cli replaced on disk. Restart your shell to use the new version.");
    Ok(())
}

/// Executes the `yav update` command.
pub async fn run(args: &UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║                  YAV UPDATER                      ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    let mut errors: Vec<String> = Vec::new();
    let update_cli = !args.project;
    let update_project = !args.self_only && (args.project || is_yav_project_dir());

    // 1. CLI self-update
    if update_cli {
        if args.git {
            if args.check {
                println!(
                    "CLI update mode: Git repository ({YAV_REPO_URL}) [current: v{CURRENT_CLI_VERSION}]"
                );
            } else if let Err(err) = install_cli_update(true).await {
                errors.push(format!("CLI update failed: {err}"));
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
                            println!("Run 'yav update' to install yav-cli v{latest_cli}");
                        } else if let Err(err) = install_cli_update(false).await {
                            errors.push(format!("CLI update failed: {err}"));
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
                    // Only try Git when the registry itself was unreachable. A
                    // failed build with a working registry is not fixed by
                    // compiling the same tree from GitHub.
                    if args.force && !args.check {
                        if let Err(err) = install_cli_update(true).await {
                            errors.push(format!("CLI update from Git failed: {err}"));
                        }
                    }
                }
            }
        }
    }

    // 2. Project dependencies update
    if update_project {
        println!();
        let spinner =
            crate::spinner::Spinner::start("Checking crates.io for latest yav framework version");
        match fetch_latest_crate_version("yav").await {
            Some(latest_yav) => {
                spinner.finish_with_success(format!(
                    "Latest yav framework version: \x1b[1;32mv{latest_yav}\x1b[0m"
                ));

                let mut changed_crates = BTreeSet::new();
                for manifest_path in ["package.yavml", "package.yav", "yav.toml", "Cargo.toml"] {
                    let path = Path::new(manifest_path);
                    if !path.is_file() {
                        continue;
                    }
                    if let Ok(content) = fs::read_to_string(path) {
                        let (new_content, changes) =
                            update_manifest_yav_versions(&content, &latest_yav);
                        if !changes.is_empty() {
                            for (krate, old_v, new_v) in &changes {
                                println!(
                                    "  \x1b[36m•\x1b[0m {manifest_path}: \x1b[1m{krate}\x1b[0m \x1b[33mv{old_v}\x1b[0m → \x1b[1;32mv{new_v}\x1b[0m"
                                );
                                changed_crates.insert(krate.clone());
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

                // Only refresh the crates that actually changed, and only in
                // a real Cargo manifest. A blanket `cargo update` would bump
                // every unrelated dependency in the project.
                if !args.check && !changed_crates.is_empty() && Path::new("Cargo.toml").is_file() {
                    // A project that has never been built has no lockfile
                    // entries yet; `cargo update -p` would fail even though
                    // the manifest is correct. The next build resolves them.
                    let lockfile = fs::read_to_string("Cargo.lock").unwrap_or_default();
                    let locked: Vec<&String> = changed_crates
                        .iter()
                        .filter(|name| lockfile.contains(&format!("name = \"{name}\"")))
                        .collect();
                    if locked.is_empty() {
                        println!("Cargo.lock has no Yav entries yet; they will be resolved on the next build");
                    } else {
                        let cargo_spinner = crate::spinner::Spinner::start(
                            "Refreshing Cargo.lock for updated yav crates",
                        );
                        let mut cargo_error = None;
                        for krate in locked {
                            match tokio::process::Command::new("cargo")
                                .args(["update", "-p", krate.as_str()])
                                .output()
                                .await
                            {
                                Ok(out) if out.status.success() => {}
                                Ok(out) => {
                                    let stderr = String::from_utf8_lossy(&out.stderr);
                                    cargo_error = Some(format!(
                                        "cargo update -p {krate} failed: {}",
                                        stderr.lines().next().unwrap_or("unknown error")
                                    ));
                                    break;
                                }
                                Err(err) => {
                                    cargo_error =
                                        Some(format!("Failed to run cargo update: {err}"));
                                    break;
                                }
                            }
                        }
                        if let Some(msg) = cargo_error {
                            cargo_spinner.finish_with_error(&msg);
                            errors.push(msg);
                        } else {
                            cargo_spinner.finish_with_success("Cargo.lock dependencies refreshed");
                        }
                    }
                }

                // Verify the standalone Tailwind compiler is present. Its
                // version is pinned by package.yavml; this is a cache check,
                // not a Tailwind version upgrade.
                if !args.check {
                    if let Ok(manifest) = PackageYav::discover() {
                        let cfg = manifest.to_config();
                        if cfg.tailwind.enabled {
                            let mgr = TailwindManager::new(&cfg.tailwind.version);
                            if !mgr.is_cached() || args.force {
                                if let Ok(bin_path) = mgr.binary_path() {
                                    match crate::tailwind_setup::download_with_progress(
                                        &mgr, &bin_path,
                                    )
                                    .await
                                    {
                                        Ok(()) => println!(
                                            "\x1b[32m✔\x1b[0m Tailwind CSS v{} downloaded successfully",
                                            cfg.tailwind.version
                                        ),
                                        Err(err) => {
                                            let msg = format!(
                                                "Failed to download Tailwind CSS v{}: {err}",
                                                cfg.tailwind.version
                                            );
                                            eprintln!("\x1b[31m{msg}\x1b[0m");
                                            errors.push(msg);
                                        }
                                    }
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
            None => {
                spinner.finish_with_warning(
                    "Could not reach crates.io to check yav framework version. Skipping project dependency update.",
                );
                errors.push("Could not reach crates.io to check latest yav version".to_string());
            }
        }
    }

    if errors.is_empty() {
        println!("\n\x1b[1;32mUpdate complete!\x1b[0m");
        Ok(())
    } else {
        eprintln!(
            "\n\x1b[1;31mUpdate completed with {} error(s):\x1b[0m",
            errors.len()
        );
        for err in &errors {
            eprintln!("  • {err}");
        }
        Err(format!("Update failed with {} error(s)", errors.len()).into())
    }
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
    fn test_unparsable_versions_are_never_updates() {
        assert!(!is_newer_version("0.1.6", "not-a-version"));
        assert!(!is_newer_version("not-a-version", "0.1.7"));
        assert!(!should_update_version("0.1.6", "unknown"));
    }

    #[test]
    fn test_manifest_update_handles_single_quoted_toml_strings() {
        let input = "[dependencies]\nyav = '0.1.0'\nyav-ui = { version = '0.1.0', features = ['digest'] }\n[dependencies.'yav-server']\nversion = '0.1.1'\n";
        let (updated, changes) = update_manifest_yav_versions(input, "0.1.6");
        assert_eq!(changes.len(), 3, "unexpected changes: {changes:?}");
        assert!(updated.contains("yav = '0.1.6'"));
        assert!(updated.contains("version = '0.1.6', features = ['digest']"));
        assert!(updated.ends_with("version = '0.1.6'\n"));
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

        let (updated, changes) = update_manifest_yav_versions(input, "0.1.2");
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

    #[test]
    fn test_manifest_update_ignores_feature_names_containing_git() {
        let input = "[dependencies]\nyav = { version = \"0.1.0\", features = [\"digest\"] }\n";
        let (updated, changes) = update_manifest_yav_versions(input, "0.1.6");
        assert_eq!(changes.len(), 1);
        assert!(updated.contains("version = \"0.1.6\""));
    }

    #[test]
    fn test_manifest_update_never_downgrades() {
        let input = "[dependencies]\nyav = \"0.1.7\"\nyav-server = { version = \"0.2.0\" }\n";
        let (updated, changes) = update_manifest_yav_versions(input, "0.1.6");
        assert!(changes.is_empty());
        assert_eq!(updated, input);
    }

    #[test]
    fn test_manifest_update_rejects_invalid_remote_version() {
        let input = "[dependencies]\nyav = \"0.1.0\"\n";
        let (updated, changes) = update_manifest_yav_versions(input, "unknown");
        assert!(changes.is_empty());
        assert_eq!(updated, input);
    }

    #[test]
    fn test_manifest_update_handles_dependency_subtables_and_targets() {
        let input = r#"[dependencies.yav]
version = "0.1.0"
features = ["full-icons"]

[target.'cfg(unix)'.dependencies]
yav-icons = "0.1.1"

[target.'cfg(windows)'.dependencies.yav-server]
version = "0.1.2"

[dev-dependencies.yav-core]
path = "../yav-core"
"#;
        let (updated, changes) = update_manifest_yav_versions(input, "0.1.6");
        assert_eq!(changes.len(), 3);
        assert!(updated.contains("version = \"0.1.6\"\nfeatures"));
        assert!(updated.contains("yav-icons = \"0.1.6\""));
        assert!(updated.contains("version = \"0.1.6\"\n\n[dev-dependencies.yav-core]"));
        assert!(updated.contains("path = \"../yav-core\""));
    }

    #[test]
    fn test_cargo_toml_declares_yav_ignores_unrelated_mentions() {
        assert!(!cargo_toml_declares_yav(
            "[package]\ndescription = \"yav yav yav\"\n[dependencies]\nserde = \"1\"\n"
        ));
        assert!(cargo_toml_declares_yav("[dependencies]\nyav = \"0.1.6\"\n"));
        assert!(cargo_toml_declares_yav(
            "[target.'cfg(unix)'.dependencies]\nyav-router = \"0.1.6\"\n"
        ));
    }
}
