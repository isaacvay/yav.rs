//! Development server (`yav dev`).
//!
//! Runs the application with live reload, watches source code and static assets,
//! and orchestrates incremental Tailwind CSS and Cargo recompilations.

use crate::reload::ReloadServer;

use std::path::{Path, PathBuf};
use yav_build::{PackageYav, TailwindManager};

/// Classified file system change types.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ChangeSet {
    /// Rust source code or build configuration changed.
    pub rust_changed: bool,
    /// CSS stylesheets changed.
    pub css_changed: bool,
    /// Public static assets changed.
    pub public_changed: bool,
}

/// Classifies a list of changed file paths into reload/rebuild actions.
pub fn classify_changes<'a>(paths: impl IntoIterator<Item = &'a Path>) -> ChangeSet {
    let mut set = ChangeSet::default();
    for path in paths {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let in_public = path.components().any(|c| c.as_os_str() == "public");

        if ext == "rs"
            || ext == "yavml"
            || ext == "yav"
            || ext == "rsx"
            || file_name == "Cargo.toml"
            || file_name == "package.yavml"
            || file_name == "package.yav"
            || file_name == "yav.toml"
        {
            set.rust_changed = true;
        } else if ext == "css" && (!in_public || file_name == "input.css") {
            set.css_changed = true;
        } else if in_public && ext != "css" {
            set.public_changed = true;
        }
    }
    set
}

fn spawn_server(port: u16, reload_port: u16) -> std::io::Result<tokio::process::Child> {
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.arg("run")
        .env("PORT", port.to_string())
        .env("YAV_DEV", "1")
        .env("YAV_RELOAD_PORT", reload_port.to_string());

    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    cmd.spawn()
}

async fn kill_server_child(mut child: tokio::process::Child) {
    #[cfg(windows)]
    {
        if let Some(pid) = child.id() {
            // Under Windows, cargo run spawns target\debug\app.exe as a child process.
            // Using taskkill /PID {pid} /T /F terminates the entire process tree so port 3000 is cleanly released.
            let _ = tokio::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output()
                .await;
        }
    }

    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            // On Unix (Linux and macOS), cargo run spawns target/debug/app as a child process.
            // Sending signals to the negative PID (-pid) signals the entire process group,
            // ensuring both cargo and the compiled server process terminate cleanly so port 3000 is released.
            let _ = tokio::process::Command::new("kill")
                .args(["-TERM", &format!("-{}", pid)])
                .output()
                .await;
            let _ = tokio::process::Command::new("pkill")
                .args(["-TERM", "-P", &pid.to_string()])
                .output()
                .await;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let _ = tokio::process::Command::new("kill")
                .args(["-9", &format!("-{}", pid)])
                .output()
                .await;
            let _ = tokio::process::Command::new("pkill")
                .args(["-9", "-P", &pid.to_string()])
                .output()
                .await;
        }
    }

    let _ = child.kill().await;
}

async fn wait_for_server_ready(port: u16, max_duration: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < max_duration {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    false
}

/// Executes the `yav dev` command.
pub async fn run(cli_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load configuration from package.yavml (or package.yav / yav.toml)
    let manifest = match PackageYav::discover() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n\x1b[1;31m❌ Error in package.yavml:\x1b[0m\n  {e}\n");
            return Err(
                format!("Cannot start dev server: package.yavml has syntax errors: {e}").into(),
            );
        }
    };
    let config = manifest.to_config();

    let server_port = if cli_port != 3000 {
        cli_port
    } else {
        config.server.port
    };

    println!("\x1b[1;36m╔═══════════════════════════════════════════════════╗\x1b[0m");
    println!("\x1b[1;36m║               🚀 YAV DEV SERVER                   ║\x1b[0m");
    println!("\x1b[1;36m╚═══════════════════════════════════════════════════╝\x1b[0m\n");

    // 2. Start Live Reload SSE Server
    let (reload_server, reload_port) = ReloadServer::start(35729).await?;
    println!("\x1b[32m⚡ Live reload server listening on http://127.0.0.1:{reload_port}\x1b[0m");
    println!("\x1b[90m   Live reload enabled automatically for browser sessions\x1b[0m");

    // 3. Initial Tailwind compile if enabled
    let tailwind_mgr = if config.tailwind.enabled {
        let mgr = TailwindManager::new(&config.tailwind.version);
        let input_css = Path::new(&config.tailwind.input_css);
        let output_css = Path::new(&config.tailwind.output_css);

        if input_css.is_file() {
            let is_ready = crate::tailwind_setup::ensure_tailwind_in_dev(&mgr)
                .await
                .unwrap_or(false);
            if is_ready {
                let spinner = crate::spinner::Spinner::start("Compiling initial Tailwind CSS");
                let start = std::time::Instant::now();
                let content_paths = vec![Path::new("src"), Path::new("public")];
                if let Err(e) = mgr
                    .run_build(input_css, output_css, &content_paths, false)
                    .await
                {
                    spinner.finish_with_warning(format!("Tailwind initial build warning: {e}"));
                } else {
                    spinner.finish_with_success(format!(
                        "Initial CSS compiled to {} in {:.0?}",
                        output_css.display(),
                        start.elapsed()
                    ));
                }
            }
        }
        Some(mgr)
    } else {
        None
    };

    // 4. Start child application server
    println!("\x1b[34m📦 Starting application server...\x1b[0m");
    let mut server_child: Option<tokio::process::Child> =
        match spawn_server(server_port, reload_port) {
            Ok(child) => {
                println!(
                    "\x1b[1;32m🌐 Application running at http://{}:{}\x1b[0m\n",
                    config.server.host, server_port
                );
                Some(child)
            }
            Err(e) => {
                eprintln!("\x1b[31m❌ Failed to spawn cargo run: {e}\x1b[0m");
                None
            }
        };

    // 5. Setup file system watcher
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<PathBuf>>();
    let mut debouncer = notify_debouncer_mini::new_debouncer(
        std::time::Duration::from_millis(300),
        move |res: notify_debouncer_mini::DebounceEventResult| {
            if let Ok(events) = res {
                let paths: Vec<PathBuf> = events.into_iter().map(|e| e.path).collect();
                let _ = event_tx.send(paths);
            }
        },
    )?;

    if Path::new("src").is_dir() {
        debouncer
            .watcher()
            .watch(Path::new("src"), notify::RecursiveMode::Recursive)?;
        println!("\x1b[35m👀 Watching src/ for changes\x1b[0m");
    }
    if Path::new("public").is_dir() {
        debouncer
            .watcher()
            .watch(Path::new("public"), notify::RecursiveMode::Recursive)?;
        println!("\x1b[35m👀 Watching public/ for changes\x1b[0m");
    }
    if Path::new("package.yavml").is_file() {
        debouncer.watcher().watch(
            Path::new("package.yavml"),
            notify::RecursiveMode::NonRecursive,
        )?;
    } else if Path::new("package.yav").is_file() {
        debouncer.watcher().watch(
            Path::new("package.yav"),
            notify::RecursiveMode::NonRecursive,
        )?;
    } else if Path::new("yav.toml").is_file() {
        debouncer
            .watcher()
            .watch(Path::new("yav.toml"), notify::RecursiveMode::NonRecursive)?;
    }

    println!("\x1b[90m(Press Ctrl+C to terminate development server)\x1b[0m\n");

    // 6. Main async event loop
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n\x1b[33m🛑 Shutting down dev server...\x1b[0m");
                if let Some(child) = server_child.take() {
                    kill_server_child(child).await;
                }
                break;
            }
            changed_paths = event_rx.recv() => {
                let paths = match changed_paths {
                    Some(p) => p,
                    None => break,
                };

                let ref_paths: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
                let changes = classify_changes(ref_paths);

                if changes.rust_changed {
                    let is_manifest_change = paths.iter().any(|p| {
                        let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        fname == "package.yavml" || fname == "package.yav" || fname == "yav.toml"
                    });

                    if is_manifest_change {
                        match PackageYav::discover() {
                            Ok(_) => {
                                println!("\x1b[32m✔ package.yavml is valid\x1b[0m");
                                reload_server.notify_clear_error();
                            }
                            Err(e) => {
                                eprintln!("\n\x1b[1;31m❌ Syntax error in package.yavml:\x1b[0m\n  {e}\n");
                                reload_server.notify_error(format!("package.yavml syntax error:\n{e}"));
                                continue;
                            }
                        }
                    }

                    // Rebuild Tailwind CSS so utility classes newly added in Rust templates are extracted
                    if let Some(ref mgr) = tailwind_mgr {
                        let input_css = Path::new(&config.tailwind.input_css);
                        let output_css = Path::new(&config.tailwind.output_css);
                        let content_paths = vec![Path::new("src"), Path::new("public")];
                        if let Err(e) = mgr.run_build(input_css, output_css, &content_paths, false).await {
                            eprintln!("\x1b[33m⚠️ Tailwind compilation warning: {e}\x1b[0m");
                        }
                    }

                    println!("\x1b[33m🔄 Code modified. Rebuilding and restarting server...\x1b[0m");
                    if let Some(old) = server_child.take() {
                        kill_server_child(old).await;
                    }

                    match spawn_server(server_port, reload_port) {
                        Ok(new_child) => {
                            server_child = Some(new_child);
                            if wait_for_server_ready(server_port, std::time::Duration::from_secs(90)).await {
                                println!("\x1b[32m✔ Server ready!\x1b[0m");
                                reload_server.notify_clear_error();
                                reload_server.notify_reload();
                            } else {
                                eprintln!("\x1b[33m⚠️ Server did not respond within 90s\x1b[0m");
                            }
                        }
                        Err(e) => {
                            eprintln!("\x1b[31m❌ Failed to restart server: {e}\x1b[0m");
                            reload_server.notify_error(format!("Failed to spawn cargo run: {e}"));
                        }
                    }
                } else if changes.css_changed {
                    if let Some(ref mgr) = tailwind_mgr {
                        let spinner = crate::spinner::Spinner::start("Stylesheet modified. Rebuilding Tailwind CSS");
                        let input_css = Path::new(&config.tailwind.input_css);
                        let output_css = Path::new(&config.tailwind.output_css);
                        let content_paths = vec![Path::new("src"), Path::new("public")];
                        match mgr.run_build(input_css, output_css, &content_paths, false).await {
                            Ok(()) => {
                                spinner.finish_with_success("Tailwind CSS rebuilt! Hot swapping stylesheets...");
                                reload_server.notify_clear_error();
                                reload_server.notify_css_reload(&config.tailwind.output_css);
                            }
                            Err(e) => {
                                spinner.finish_with_error(format!("Tailwind build failed: {e}"));
                                reload_server.notify_error(format!("Tailwind build error: {e}"));
                            }
                        }
                    }
                } else if changes.public_changed {
                    println!("\x1b[35m📁 Static asset in public/ modified. Reloading browser...\x1b[0m");
                    reload_server.notify_reload();
                }
            }
            status = async {
                match server_child.as_mut() {
                    Some(child) => child.wait().await,
                    None => std::future::pending().await,
                }
            } => {
                server_child = None;
                match status {
                    Ok(exit_status) => {
                        eprintln!("\x1b[33m⚠️ Application server exited with: {exit_status}\x1b[0m");
                        if !exit_status.success() {
                            reload_server.notify_error(format!("Application process exited with status: {exit_status}"));
                        }
                        println!("\x1b[90mWaiting for file changes to restart...\x1b[0m");
                    }
                    Err(e) => {
                        eprintln!("\x1b[31m❌ Error in child server process: {e}\x1b[0m");
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_changes() {
        let rs_path = Path::new("src/main.rs");
        let toml_path = Path::new("yav.toml");
        let css_path = Path::new("src/styles/app.css");
        let public_img = Path::new("public/img/logo.png");

        let c1 = classify_changes([rs_path]);
        assert!(c1.rust_changed);
        assert!(!c1.css_changed);
        assert!(!c1.public_changed);

        let c2 = classify_changes([css_path]);
        assert!(!c2.rust_changed);
        assert!(c2.css_changed);
        assert!(!c2.public_changed);

        let c3 = classify_changes([public_img]);
        assert!(!c3.rust_changed);
        assert!(!c3.css_changed);
        assert!(c3.public_changed);

        let c4 = classify_changes([toml_path]);
        assert!(c4.rust_changed);

        // Generated Tailwind output in public/styles.css must NOT trigger css_changed loop
        let generated_css = Path::new("public/styles.css");
        let c5 = classify_changes([generated_css]);
        assert!(!c5.css_changed);
        assert!(!c5.public_changed);
    }
}
