//! `yav run <script>` — Execute custom named scripts defined in `package.yavml`.
//!
//! Provides the equivalent of `npm run <script>` or `cargo run --bin` for Yav projects.

use std::process::{Command, ExitStatus};
use yav_build::manifest::PackageYav;

/// Execute a named script defined in `package.yavml`.
///
/// **Security:** The script template from `package.yavml` is executed via the
/// system shell (`sh -c` on Unix, `cmd /C` on Windows). User-provided `args`
/// are shell-escaped before being appended to prevent command injection.
/// Only the script template itself (which is trusted, being part of the
/// project manifest) is passed unescaped to the shell.
pub fn run(script: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PackageYav::discover().unwrap_or_default();

    if let Some(cmd_template) = manifest.scripts.get(script) {
        // Shell-escape user-provided args to prevent injection.
        // Each arg is quoted so shell metacharacters are treated literally.
        let escaped_args: Vec<String> = args.iter().map(|a| shell_escape_arg(a)).collect();
        let display_args = escaped_args.join(" ");

        println!("\x1b[36m> {} {}\x1b[0m", cmd_template, display_args);

        let full_cmd = if escaped_args.is_empty() {
            cmd_template.clone()
        } else {
            format!("{} {}", cmd_template, display_args)
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

/// Shell-escape a single argument so it is treated literally by `sh -c` / `cmd /C`.
///
/// On Unix: wraps in single quotes and escapes any embedded single quotes
/// via the standard `'\''` sequence.
///
/// On Windows: wraps in double quotes and escapes embedded double quotes
/// with `^"`. This is sufficient for `cmd /C` interpretation.
fn shell_escape_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    // If the arg contains no shell metacharacters, it's safe unquoted.
    let safe_chars = arg
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == '\\');
    if safe_chars {
        return arg.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        // Windows cmd.exe: wrap in double quotes, escape embedded double quotes.
        let escaped = arg.replace('"', "^\"");
        format!("\"{escaped}\"")
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix sh: wrap in single quotes, escape embedded single quotes.
        let escaped = arg.replace('\'', "'\\''");
        format!("'{escaped}'")
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

    #[test]
    fn test_shell_escape_arg_safe_chars() {
        assert_eq!(shell_escape_arg("hello"), "hello");
        assert_eq!(shell_escape_arg("file.txt"), "file.txt");
        assert_eq!(shell_escape_arg("src/main.rs"), "src/main.rs");
    }

    #[test]
    fn test_shell_escape_arg_empty() {
        assert_eq!(shell_escape_arg(""), "\"\"");
    }

    #[test]
    fn test_shell_escape_arg_injection_attempt() {
        // This payload would break out of the command if not properly escaped.
        let payload = "foo; rm -rf /";
        let escaped = shell_escape_arg(payload);
        // On Unix, the entire payload should be inside single quotes.
        #[cfg(not(target_os = "windows"))]
        {
            assert!(escaped.starts_with('\''));
            assert!(escaped.ends_with('\''));
            assert!(!escaped.contains("; rm"));
        }
        // On Windows, it should be inside double quotes.
        #[cfg(target_os = "windows")]
        {
            assert!(escaped.starts_with('"'));
            assert!(escaped.ends_with('"'));
        }
    }

    #[test]
    fn test_shell_escape_arg_backticks() {
        let payload = "`whoami`";
        let escaped = shell_escape_arg(payload);
        // The backticks must be inside quotes, not bare.
        assert!(
            escaped.starts_with('\'') || escaped.starts_with('"'),
            "Backticks should be quoted: got {escaped}"
        );
    }
}
