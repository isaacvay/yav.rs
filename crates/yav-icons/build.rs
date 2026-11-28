//! Build script for `yav-icons` (real per-icon compile-time tree-shaking via `YAV_ICON_ALLOWLIST`).
//!
//! When `YAV_ICON_ALLOWLIST` is set (e.g. `YAV_ICON_ALLOWLIST=home,user,settings`)
//! and the `icon-minify` feature is active, this script filters the catalog
//! source files to emit `$OUT_DIR/pruned_catalog.rs` containing only the
//! allowlisted icons and emits `cargo:rustc-cfg=yav_icon_pruned`.
//! When `YAV_ICON_ALLOWLIST` is unset or empty, the full enabled icon families
//! are compiled normally.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn normalize_allowlist(raw: &str) -> Vec<String> {
    let mut names: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| {
            let trimmed = n
                .rsplit(':')
                .next()
                .unwrap_or(n)
                .trim()
                .to_ascii_lowercase();
            if trimmed == "close" {
                "x".to_string()
            } else {
                trimmed
            }
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn filter_catalog_file(src_path: &Path, allowed: &HashSet<&str>) -> String {
    let Ok(content) = fs::read_to_string(src_path) else {
        return String::new();
    };

    let mut out = String::from("use super::{IconData, IconFamily, IconStyle};\n\n");
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("pub static ") && trimmed.contains(": &[IconData] = &[") {
            out.push_str(line);
            out.push('\n');

            // Read each `IconData { ... },` entry until the closing `];`
            while let Some(entry_line) = lines.next() {
                if entry_line.trim() == "];" {
                    out.push_str("];\n\n");
                    break;
                }
                if entry_line.trim_start().starts_with("IconData {") {
                    let mut block = String::from(entry_line);
                    block.push('\n');
                    let mut icon_name = None;
                    for inner in lines.by_ref() {
                        block.push_str(inner);
                        block.push('\n');
                        let t = inner.trim();
                        if let Some(rest) = t.strip_prefix("name: \"") {
                            if let Some(end_q) = rest.find('"') {
                                icon_name = Some(rest[..end_q].to_string());
                            }
                        }
                        if t == "}," {
                            break;
                        }
                    }
                    if let Some(ref name) = icon_name {
                        if allowed.contains(name.as_str()) {
                            out.push_str(&block);
                        }
                    }
                }
            }
        }
    }
    out
}

fn main() {
    println!("cargo:rerun-if-env-changed=YAV_ICON_ALLOWLIST");
    println!("cargo::rustc-check-cfg=cfg(yav_icon_pruned)");

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let allowlist = env::var("YAV_ICON_ALLOWLIST").unwrap_or_default();
    let names = normalize_allowlist(&allowlist);

    let out_path = PathBuf::from(&out_dir);
    if let Err(err) = fs::write(out_path.join("allowed_icons.txt"), names.join("\n")) {
        panic!("failed to write icon allowlist: {err}");
    }

    let minify_enabled = env::var_os("CARGO_FEATURE_ICON_MINIFY").is_some();
    if minify_enabled && !names.is_empty() {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let catalog_dir = PathBuf::from(manifest_dir).join("src").join("catalog");
        let allowed_set: HashSet<&str> = names.iter().map(String::as_str).collect();

        let classic_code = if env::var_os("CARGO_FEATURE_CLASSIC").is_some() {
            filter_catalog_file(&catalog_dir.join("classic.rs"), &allowed_set)
        } else {
            "pub static CLASSIC_REGULAR: &[super::IconData] = &[];\n".to_string()
        };
        let minimal_code = if env::var_os("CARGO_FEATURE_MINIMAL").is_some() {
            filter_catalog_file(&catalog_dir.join("minimal.rs"), &allowed_set)
        } else {
            "pub static MINIMAL_REGULAR: &[super::IconData] = &[];\npub static MINIMAL_SOLID: &[super::IconData] = &[];\n".to_string()
        };
        let system_code = if env::var_os("CARGO_FEATURE_SYSTEM").is_some() {
            filter_catalog_file(&catalog_dir.join("system.rs"), &allowed_set)
        } else {
            "pub static SYSTEM_REGULAR: &[super::IconData] = &[];\npub static SYSTEM_BOLD: &[super::IconData] = &[];\npub static SYSTEM_SOLID: &[super::IconData] = &[];\npub static SYSTEM_DUOTONE: &[super::IconData] = &[];\npub static SYSTEM_LIGHT: &[super::IconData] = &[];\npub static SYSTEM_THIN: &[super::IconData] = &[];\n".to_string()
        };

        let _ = fs::write(out_path.join("pruned_classic.rs"), classic_code);
        let _ = fs::write(out_path.join("pruned_minimal.rs"), minimal_code);
        let _ = fs::write(out_path.join("pruned_system.rs"), system_code);
        println!("cargo:rustc-cfg=yav_icon_pruned");
    }
}
