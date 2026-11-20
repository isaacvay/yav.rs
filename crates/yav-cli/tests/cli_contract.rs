//! CLI contract tests.
//!
//! Pure process-level checks against the real `yav` binary. These commands
//! only print information or fail against an unreachable local proxy: no
//! external network access and no writes inside the repository.

use assert_cmd::Command;
use std::fs;

#[test]
fn version_flag_reports_workspace_version() {
    let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");

    let assert = cmd.arg("--version").assert().success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("UTF-8 stdout");
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "expected `{}` in `yav --version` output, got: {stdout:?}",
        env!("CARGO_PKG_VERSION")
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

#[test]
fn update_help_lists_all_flags_and_aliases_work() {
    for name in ["update", "up", "upgrade"] {
        let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");
        let assert = cmd.args([name, "--help"]).assert().success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("UTF-8 stdout");
        for flag in ["--check", "--self", "--project", "--git", "--force"] {
            assert!(
                stdout.contains(flag),
                "expected `{flag}` in `yav {name} --help`, got: {stdout:?}"
            );
        }
    }
}

#[test]
fn update_rejects_conflicting_scopes() {
    for args in [
        ["update", "--self", "--project"],
        ["update", "--git", "--project"],
    ] {
        let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");
        let assert = cmd.args(args).assert().failure();
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
        assert!(
            stderr.contains("cannot be used with"),
            "`yav {}` must be rejected instead of silently doing nothing, got: {stderr}",
            args.join(" ")
        );
    }
}

#[test]
fn update_check_offline_reports_failure_without_writing() {
    let dir = tempfile::tempdir().expect("a temporary project directory");
    let cargo_toml =
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n\n[dependencies]\nyav = \"0.1.5\"\n";
    fs::write(dir.path().join("Cargo.toml"), cargo_toml).expect("write Cargo.toml");
    fs::write(
        dir.path().join("package.yavml"),
        "[package]\nname = \"probe\"\nversion = \"0.1.0\"\n",
    )
    .expect("write package.yavml");

    let mut cmd = Command::cargo_bin("yav").expect("the yav binary must be built by cargo");
    let assert = cmd
        .current_dir(dir.path())
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("NO_PROXY", "")
        .args(["update", "--project", "--check"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("Could not reach crates.io"),
        "offline update must report the failed version check, got: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("Cargo.toml")).expect("read Cargo.toml"),
        cargo_toml,
        "--check must never modify manifests"
    );
    assert!(
        !dir.path().join("Cargo.lock").exists(),
        "--check must never run cargo update"
    );
}
