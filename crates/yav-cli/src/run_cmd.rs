//! `yav run <script>` — Execute custom named scripts defined in `package.yavml`.
//!
//! Provides the equivalent of `npm run <script>` or `cargo run --bin` for Yav projects.

use std::process::{Command, ExitStatus};
use yav_build::manifest::PackageYav;

/// Execute a named script defined in `package.yavml`.
pub fn run(script: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PackageYav::discover().unwrap_or_default();

    if let Some(cmd_template) = manifest.scripts.get(script) {
        println!("\x1b[36m> {} {}\x1b[0m", cmd_template, args.join(" "));

        let full_cmd = if args.is_empty() {
            cmd_template.clone()
        } else {
            format!("{} {}", cmd_template, args.join(" "))
        };

        #[cfg(target_os = "windows")]
        let status = Command::new("cmd").args(["/C", &full_cmd]).status()?;

        #[cfg(not(target_os = "windows"))]
        let status = Command::new("sh").args(["-c", &full_cmd]).status()?;

        handle_exit_status(status);
        Ok(())
    } else {
        eprintln!("\n\x1b[1;31m❌ Script not found: \"{}\"\x1b[0m\n", script);
        if manifest.scripts.is_empty() {
            eprintln!("No scripts configured in [scripts] section of package.yavml.");
        } else {
            eprintln!("Available scripts in package.yavml:");
            for (name, command) in &manifest.scripts {
                eprintln!("  \x1b[1;32m{:<12}\x1b[0m \x1b[90m{}\x1b[0m", name, command);
            }
            eprintln!();
        }
        std::process::exit(1);
    }
}

fn handle_exit_status(status: ExitStatus) {
    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        } else {
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_resolution_from_manifest() {
        let manifest = PackageYav::default();
        assert!(manifest.scripts.contains_key("dev"));
        assert!(manifest.scripts.contains_key("build"));
        assert!(manifest.scripts.contains_key("test"));
    }
}
