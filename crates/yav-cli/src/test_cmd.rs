//! Test runner (`yav test`).
//!
//! Invokes `cargo test` with project configuration and displays test outcomes.

/// Executes the `yav test` command.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║               🧪 RUNNING YAV TESTS                ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    let status = tokio::process::Command::new("cargo")
        .arg("test")
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
