//! End-to-end CLI tests: real process exit codes and real files on disk.
//!
//! These deliberately use `std::process::Command` plus the compile-time
//! `CARGO_BIN_EXE_yav` path instead of an extra assertion crate, so the suite
//! works offline and still catches the "fake success" class of regressions
//! (a command printing ✔ while returning success despite a failed operation).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique, self-cleaning working directory for one test.
struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "yav_cli_it_{}_{}_{}",
            std::process::id(),
            label,
            seq
        ));
        // Only ever touches a directory this test itself owns.
        if dir.starts_with(std::env::temp_dir()) {
            let _ = fs::remove_dir_all(&dir);
        }
        fs::create_dir_all(&dir).expect("create temporary project directory");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, contents: &str) {
        let target = self.0.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(target, contents).expect("write fixture file");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        if self.0.starts_with(std::env::temp_dir()) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn yav() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yav"))
}

fn run(mut cmd: Command) -> Output {
    cmd.output().expect("spawn the yav binary")
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn version_and_bare_invocation_succeed() {
    let version = run({
        let mut c = yav();
        c.arg("--version");
        c
    });
    assert!(version.status.success(), "{}", combined(&version));
    assert!(
        combined(&version).contains(env!("CARGO_PKG_VERSION")),
        "version output should contain the crate version: {}",
        combined(&version)
    );

    let help = run(yav());
    assert!(help.status.success(), "{}", combined(&help));
    assert!(
        combined(&help).to_ascii_lowercase().contains("usage"),
        "bare invocation should print help: {}",
        combined(&help)
    );
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    let out = run({
        let mut c = yav();
        c.arg("definitely-not-a-command");
        c
    });
    assert!(
        !out.status.success(),
        "clap must reject unknown subcommands: {}",
        combined(&out)
    );
}

#[test]
fn doctor_exits_nonzero_when_manifest_is_missing() {
    let project = TempProject::new("doctor");
    let out = run({
        let mut c = yav();
        c.current_dir(project.path())
            .env_remove("DATABASE_URL")
            .arg("doctor");
        c
    });
    assert!(
        !out.status.success(),
        "doctor must fail when no project manifest exists: {}",
        combined(&out)
    );
    assert!(
        combined(&out).contains("check(s) failed"),
        "doctor must report its failures: {}",
        combined(&out)
    );
}

#[test]
fn db_seed_fails_loudly_without_database_url() {
    let project = TempProject::new("dbseed");
    let out = run({
        let mut c = yav();
        c.current_dir(project.path())
            .env_remove("DATABASE_URL")
            .args(["db", "seed"]);
        c
    });
    assert!(
        !out.status.success(),
        "seed must not report success without a database: {}",
        combined(&out)
    );
    assert!(
        combined(&out).contains("DATABASE_URL"),
        "error must name the missing configuration: {}",
        combined(&out)
    );
    // The seed file is still scaffolded, but no fake "applied" message.
    assert!(project.path().join("migrations/seed.sql").is_file());
    assert!(
        !combined(&out).contains("Seed data applied"),
        "must not claim the seed ran: {}",
        combined(&out)
    );
}

#[test]
fn codegen_ts_and_openapi_write_two_distinct_files() {
    let project = TempProject::new("codegen");
    project.write("src/routes/page.rs", "// index route\n");
    project.write("src/routes/about.rs", "// about route\n");

    let out = run({
        let mut c = yav();
        c.current_dir(project.path())
            .args(["codegen", "--ts", "--openapi", "--output", "api.gen"]);
        c
    });
    assert!(out.status.success(), "{}", combined(&out));

    let ts_path = project.path().join("api.gen");
    let spec_path = project.path().join("api.openapi.json");
    assert!(
        ts_path.is_file(),
        "TypeScript output missing: {}",
        combined(&out)
    );
    assert!(
        spec_path.is_file(),
        "OpenAPI output must not overwrite the TypeScript file: {}",
        combined(&out)
    );

    let ts = fs::read_to_string(ts_path).expect("read TypeScript output");
    assert!(ts.contains("export type YavRoute"));
    let spec = fs::read_to_string(spec_path).expect("read OpenAPI output");
    assert!(
        spec.contains("\"openapi\": \"3.1.0\""),
        "specification must be OpenAPI 3.1: {spec}"
    );
    assert!(spec.contains("/about"), "routes must be discovered: {spec}");
}

#[test]
fn icon_info_uses_a_populated_catalog() {
    let out = run({
        let mut c = yav();
        c.args(["icon", "info", "home"]);
        c
    });
    assert!(out.status.success(), "{}", combined(&out));
    assert!(
        combined(&out).contains("Icon: home"),
        "`yav icon` must ship with at least the Classic catalog compiled: {}",
        combined(&out)
    );
}

#[test]
fn generate_route_rejects_path_traversal() {
    let project = TempProject::new("traversal");
    let out = run({
        let mut c = yav();
        c.current_dir(project.path())
            .args(["generate", "route", "../escape"]);
        c
    });
    assert!(
        !out.status.success(),
        "path traversal must be rejected: {}",
        combined(&out)
    );
    assert!(
        !project.path().join("src/routes/escape.rs").exists(),
        "no file may be written outside the routes directory"
    );
}
