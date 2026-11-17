//! CLI contract tests.
//!
//! Pure process-level checks against the real `yav` binary. These commands
//! only print information and exit: no network access and no writes inside
//! the repository (or anywhere else).

use assert_cmd::Command;

/// Version declared once in the workspace `[workspace.package]` manifest.
const WORKSPACE_VERSION: &str = "0.1.5";

#[test]
fn version_flag_reports_workspace_version() {
    let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");

    let assert = cmd.arg("--version").assert().success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("UTF-8 stdout");
    assert!(
        stdout.contains(WORKSPACE_VERSION),
        "expected `{WORKSPACE_VERSION}` in `yav --version` output, got: {stdout:?}"
    );
}

#[test]
fn new_help_lists_the_check_flag() {
    let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");

    let assert = cmd.args(["new", "--help"]).assert().success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("UTF-8 stdout");
    assert!(
        stdout.contains("--check"),
        "expected `--check` in `yav new --help`, got: {stdout:?}"
    );
}
