//! Test runner (`yav test`).
//!
//! Invokes `cargo test` with project configuration and displays test outcomes.
//! Any extra arguments are forwarded verbatim to `cargo test`, allowing
//! filters, package selectors, and test-binary flags:
//!
//! ```text
//! yav test                       # run all tests
//! yav test my_test               # filter by test name
//! yav test -p yav-core           # test a specific package
//! yav test -- --nocapture        # pass flags to the test binary
//! yav test -p yav-core -- --nocapture
//! ```

/// Executes the `yav test` command, forwarding `args` to `cargo test`.
pub async fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║                 RUNNING YAV TESTS                 ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    if !args.is_empty() {
        let display: Vec<&str> = args.iter().map(String::as_str).collect();
        println!("\x1b[90m  cargo test {}\x1b[0m\n", display.join(" "));
    }

    let status = tokio::process::Command::new("cargo")
        .arg("test")
        .args(args)
        .status()
        .await?;

    if status.success() {
        println!("\n\x1b[1;32m✔ All tests passed successfully!\x1b[0m");
        Ok(())
    } else {
        Err(format!(
            "Tests failed with exit code: {}",
            status.code().unwrap_or(-1)
        )
        .into())
    }
}
