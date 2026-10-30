//! IDE Editor Extension management (`yav ext`).
//!
//! Provides automated installation and status checking for the Yav VS Code & Cursor extension,
//! embedding syntax highlighting (.yav, .rsx, .yavml), file icons, schema validation, and snippets.

use clap::Subcommand;
use std::io::Write;
use std::path::{Path, PathBuf};

const EXTENSION_DIR_NAME: &str = "yav-framework.yav-vscode-0.1.0";

// Embedded extension files from editors/vscode
const PKG_JSON: &str = include_str!("../editors/vscode/package.json");
const README_MD: &str = include_str!("../editors/vscode/README.md");
const LANG_CONFIG_YAV: &str = include_str!("../editors/vscode/language-configuration.json");
const LANG_CONFIG_YAVML: &str = include_str!("../editors/vscode/language-configuration-yavml.json");
const SYNTAX_YAV: &str = include_str!("../editors/vscode/syntaxes/yav.tmLanguage.json");
const SYNTAX_YAVML: &str = include_str!("../editors/vscode/syntaxes/yavml.tmLanguage.json");
const SCHEMA_YAVML: &str = include_str!("../editors/vscode/schemas/package.yavml.json");
const SNIPPETS_YAV: &str = include_str!("../editors/vscode/snippets/yav.json");
const SNIPPETS_YAVML: &str = include_str!("../editors/vscode/snippets/yavml.json");

const ICON_SVG: &[u8] = include_bytes!("../editors/vscode/icons/icon.svg");
const ICON_LIGHT_SVG: &[u8] = include_bytes!("../editors/vscode/icons/icon-light.svg");
const ICON_PNG: &[u8] = include_bytes!("../editors/vscode/icons/icon.png");
const YAVML_SVG: &[u8] = include_bytes!("../editors/vscode/icons/yavml.svg");
const YAVML_PNG: &[u8] = include_bytes!("../editors/vscode/icons/yavml.png");

/// Subcommands for `yav ext`.
#[derive(Subcommand, Debug)]
pub enum ExtAction {
    /// Install or update the Yav extension for VS Code, Cursor, and compatible editors
    Install {
        /// Force reinstallation even if already present
        #[arg(short, long)]
        force: bool,
    },
    /// Check the installation status of the Yav extension across installed editors
    Status,
}

/// Known editor targets and their extension folders.
#[derive(Debug, Clone)]
pub struct EditorTarget {
    pub name: &'static str,
    pub extensions_dir: PathBuf,
}

/// Returns the user's home directory across OSes (Windows, Linux, macOS).
pub fn get_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}

/// Discovers detected or candidate editor extension directories.
pub fn get_editor_targets() -> Vec<EditorTarget> {
    let mut targets = Vec::new();
    let home = match get_home_dir() {
        Some(h) => h,
        None => return targets,
    };

    // 1. VS Code (.vscode/extensions)
    let vscode_ext = home.join(".vscode").join("extensions");
    targets.push(EditorTarget {
        name: "Visual Studio Code",
        extensions_dir: vscode_ext,
    });

    // 2. Cursor (.cursor/extensions)
    let cursor_ext = home.join(".cursor").join("extensions");
    if cursor_ext.parent().map(|p| p.exists()).unwrap_or(false) || cursor_ext.exists() {
        targets.push(EditorTarget {
            name: "Cursor",
            extensions_dir: cursor_ext,
        });
    }

    // 3. VS Code Insiders (.vscode-insiders/extensions)
    let insiders_ext = home.join(".vscode-insiders").join("extensions");
    if insiders_ext.parent().map(|p| p.exists()).unwrap_or(false) || insiders_ext.exists() {
        targets.push(EditorTarget {
            name: "VS Code Insiders",
            extensions_dir: insiders_ext,
        });
    }

    // 4. VSCodium (.vscode-oss/extensions)
    let codium_ext = home.join(".vscode-oss").join("extensions");
    if codium_ext.parent().map(|p| p.exists()).unwrap_or(false) || codium_ext.exists() {
        targets.push(EditorTarget {
            name: "VSCodium",
            extensions_dir: codium_ext,
        });
    }

    // 5. Windsurf (.windsurf/extensions or .codeium/windsurf/extensions)
    let windsurf_ext = home.join(".windsurf").join("extensions");
    let codeium_windsurf_ext = home.join(".codeium").join("windsurf").join("extensions");
    if windsurf_ext.parent().map(|p| p.exists()).unwrap_or(false) || windsurf_ext.exists() {
        targets.push(EditorTarget {
            name: "Windsurf",
            extensions_dir: windsurf_ext,
        });
    } else if codeium_windsurf_ext
        .parent()
        .map(|p| p.exists())
        .unwrap_or(false)
        || codeium_windsurf_ext.exists()
    {
        targets.push(EditorTarget {
            name: "Windsurf",
            extensions_dir: codeium_windsurf_ext,
        });
    }

    targets
}

/// Checks if the Yav extension is installed in at least one editor.
pub fn is_extension_installed() -> bool {
    let targets = get_editor_targets();
    for target in targets {
        let dest = target.extensions_dir.join(EXTENSION_DIR_NAME);
        if dest.join("package.json").is_file() {
            return true;
        }
    }
    false
}

/// Writes all embedded extension assets into the destination folder.
fn write_extension_bundle(dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    std::fs::create_dir_all(dest.join("syntaxes"))?;
    std::fs::create_dir_all(dest.join("schemas"))?;
    std::fs::create_dir_all(dest.join("snippets"))?;
    std::fs::create_dir_all(dest.join("icons"))?;

    std::fs::write(dest.join("package.json"), PKG_JSON)?;
    std::fs::write(dest.join("README.md"), README_MD)?;
    std::fs::write(dest.join("language-configuration.json"), LANG_CONFIG_YAV)?;
    std::fs::write(
        dest.join("language-configuration-yavml.json"),
        LANG_CONFIG_YAVML,
    )?;

    std::fs::write(dest.join("syntaxes/yav.tmLanguage.json"), SYNTAX_YAV)?;
    std::fs::write(dest.join("syntaxes/yavml.tmLanguage.json"), SYNTAX_YAVML)?;
    std::fs::write(dest.join("schemas/package.yavml.json"), SCHEMA_YAVML)?;
    std::fs::write(dest.join("snippets/yav.json"), SNIPPETS_YAV)?;
    std::fs::write(dest.join("snippets/yavml.json"), SNIPPETS_YAVML)?;

    std::fs::write(dest.join("icons/icon.svg"), ICON_SVG)?;
    std::fs::write(dest.join("icons/icon-light.svg"), ICON_LIGHT_SVG)?;
    std::fs::write(dest.join("icons/icon.png"), ICON_PNG)?;
    std::fs::write(dest.join("icons/yavml.svg"), YAVML_SVG)?;
    std::fs::write(dest.join("icons/yavml.png"), YAVML_PNG)?;

    Ok(())
}

/// Installs the Yav extension into all detected editors.
pub fn install_extension(force: bool) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let targets = get_editor_targets();
    let mut installed_editors = Vec::new();

    if targets.is_empty() {
        return Err("Could not locate user home directory".into());
    }

    for target in &targets {
        let dest = target.extensions_dir.join(EXTENSION_DIR_NAME);
        if dest.join("package.json").is_file() && !force {
            installed_editors.push(format!("{} (already up to date)", target.name));
            continue;
        }

        write_extension_bundle(&dest)?;
        installed_editors.push(format!("{} -> {}", target.name, dest.display()));
    }

    Ok(installed_editors)
}

/// Interactively or automatically prompts and installs the extension if not installed yet.
pub fn prompt_and_install_if_needed() {
    if is_extension_installed() {
        println!("\x1b[32m✔ Yav extension is installed in your editor (syntax highlighting, icons & snippets ready)\x1b[0m");
        return;
    }

    println!("\n\x1b[1;36mℹ Yav VS Code / Cursor extension is not installed yet.\x1b[0m");
    println!("   It provides syntax coloring (.yav, .rsx, .yavml), icons, snippets, and schema validation.");
    print!("› Install the Yav extension now? [Y/n]: ");
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if trimmed.is_empty()
            || trimmed == "y"
            || trimmed == "yes"
            || trimmed == "o"
            || trimmed == "oui"
        {
            match install_extension(true) {
                Ok(installed) => {
                    println!("\x1b[1;32m✔ Yav extension installed successfully!\x1b[0m");
                    for item in installed {
                        println!("   • {item}");
                    }
                    println!("\x1b[90m(Your editor will automatically recognize it on next reload)\x1b[0m\n");
                }
                Err(e) => {
                    eprintln!("\x1b[33m! Could not auto-install extension: {e}\x1b[0m");
                    eprintln!(
                        "   You can install it anytime with: \x1b[1myav ext install\x1b[0m\n"
                    );
                }
            }
            return;
        }
    }

    println!("\x1b[90mℹ You can install it anytime by running: yav ext install\x1b[0m\n");
}

/// CLI runner for `yav ext`.
pub fn run(action: &ExtAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        ExtAction::Install { force } => {
            println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
            println!("\x1b[1;36m║          YAV EDITOR EXTENSION INSTALLER           ║\x1b[0m");
            println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

            let results = install_extension(*force)?;
            println!("\x1b[1;32m✔ Yav extension setup complete:\x1b[0m");
            for res in results {
                println!("  • {res}");
            }
            println!("\n\x1b[35m● Supported features:\x1b[0m");
            println!("  • .yav & .rsx RSX / JSX syntax highlighting & Emmet expansion");
            println!("  • .yavml TOML manifest syntax highlighting & JSON schema validation");
            println!("  • Custom Yav & YavML file icons");
            println!("  • Auto-completion snippets (sec-motion, sec-pwa, sec-server, etc.)");
        }
        ExtAction::Status => {
            println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
            println!("\x1b[1;36m║           YAV EXTENSION STATUS                    ║\x1b[0m");
            println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

            let targets = get_editor_targets();
            for target in targets {
                let dest = target.extensions_dir.join(EXTENSION_DIR_NAME);
                if dest.join("package.json").is_file() {
                    println!(
                        "  \x1b[32m✔ {}\x1b[0m: Installed at {}",
                        target.name,
                        dest.display()
                    );
                } else {
                    println!(
                        "  \x1b[90m○ {}\x1b[0m: Not installed (dir: {})",
                        target.name,
                        target.extensions_dir.display()
                    );
                }
            }
            println!("\n  Run \x1b[1myav ext install\x1b[0m to install or update in all editors.");
        }
    }
    Ok(())
}
