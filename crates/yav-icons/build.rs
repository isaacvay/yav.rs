//! Build script for the opt-in `icon-minify` feature.
//!
//! It records a caller-provided icon allowlist in `OUT_DIR`. The catalog is
//! generated source, so this script intentionally does **not** rewrite it:
//! doing so would change the public icon set based on a build-time environment
//! variable and break deterministic library builds. The allowlist is exposed
//! through `yav_icons::minify` for generators and audits.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=YAV_ICON_ALLOWLIST");

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let allowlist = env::var("YAV_ICON_ALLOWLIST").unwrap_or_default();
    let mut names: Vec<&str> = allowlist
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    names.sort_unstable();
    names.dedup();

    let output = PathBuf::from(out_dir).join("allowed_icons.txt");
    if let Err(err) = fs::write(output, names.join("\n")) {
        panic!("failed to write icon allowlist: {err}");
    }
}
