//! Compile-pass contract for the `rsx!` macro.
//!
//! `trybuild` copies this crate's `[dev-dependencies]` into the generated test
//! project, which is what makes `yav_core` visible to the macro expansion:
//! `codegen::yav_core_root()` then resolves to `::yav_core` instead of the
//! `yav` facade (which would be a circular dependency for this crate).

#[test]
fn valid_rsx_fixture_compiles() {
    // `trybuild` launches a nested cargo invocation. Under some Windows/Cargo
    // combinations that nested invocation waits on the parent test process,
    // causing a deadlock. Keep the fixture available but opt-in so
    // `cargo test --workspace` remains deterministic.
    if std::env::var_os("YAV_RUN_TRYBUILD").is_none() {
        eprintln!("skipping trybuild fixture; set YAV_RUN_TRYBUILD=1 to enable");
        return;
    }

    let t = trybuild::TestCases::new();
    t.pass("tests/ui/valid_rsx.rs");
}
