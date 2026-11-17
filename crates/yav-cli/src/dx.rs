//! Workspace inspection helpers behind the `dx-inspect` feature.
//!
//! Uses real `cargo metadata` output instead of guessing project structure.
//! The result is emitted at debug level so normal CLI output stays unchanged.

/// Summarize the current Cargo workspace.
pub fn workspace_summary() -> Result<String, String> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .map_err(|err| format!("cargo metadata failed: {err}"))?;

    let workspace_members = metadata.workspace_members.len();
    let packages = metadata.packages.len();
    let root = metadata
        .workspace_root
        .as_std_path()
        .to_string_lossy()
        .into_owned();

    Ok(format!(
        "workspace at {root}: {workspace_members} members, {packages} resolved packages"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_is_not_empty_when_metadata_is_available() {
        // Running inside this workspace, cargo metadata must succeed.
        let summary = workspace_summary().expect("cargo metadata should work in the workspace");
        assert!(summary.contains("workspace at"));
        assert!(summary.contains("members"));
    }
}
