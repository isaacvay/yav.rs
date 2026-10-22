//! Standalone Tailwind CSS compiler setup, caching, and download manager.

use std::io::{IsTerminal, Write};
use std::path::Path;
use yav_build::TailwindManager;

/// Renders a dynamic download progress bar in the terminal.
pub fn format_progress_bar(downloaded: u64, total: Option<u64>) -> String {
    let dl_mb = downloaded as f64 / (1024.0 * 1024.0);
    if let Some(tot) = total {
        let tot_mb = tot as f64 / (1024.0 * 1024.0);
        let pct = if tot > 0 {
            (downloaded as f64 / tot as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let bar_width: usize = 25;
        let filled = (pct * bar_width as f64).round() as usize;
        let empty = bar_width.saturating_sub(filled);
        let filled_bar = if filled > 0 {
            let mut s = "=".repeat(filled.saturating_sub(1));
            s.push('>');
            s
        } else {
            String::new()
        };
        let bar = format!("[{filled_bar}{}]", " ".repeat(empty));
        format!(
            "{bar} {:>5.1}% ({:.1} MB / {:.1} MB)",
            pct * 100.0,
            dl_mb,
            tot_mb
        )
    } else {
        format!("({:.1} MB downloaded)", dl_mb)
    }
}

/// Downloads the standalone Tailwind CLI with live progress in the terminal.
pub async fn download_with_progress(
    mgr: &TailwindManager,
    dest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_tty = std::io::stdout().is_terminal();
    let version = mgr.version();

    println!(
        "\x1b[36m📥 Downloading standalone Tailwind CSS compiler v{version} (~45 MB)...\x1b[0m"
    );

    let mut last_printed_pct = 0;
    mgr.download_binary_with_progress(dest_path, None, |downloaded, total| {
        if is_tty {
            let progress_str = format_progress_bar(downloaded, total);
            print!("\r\x1b[2K   {progress_str}");
            let _ = std::io::stdout().flush();
        } else if let Some(tot) = total {
            let pct = (downloaded * 100 / tot) as u32;
            if pct >= last_printed_pct + 25 {
                println!("   Download progress: {pct}%");
                last_printed_pct = pct;
            }
        }
    })
    .await?;

    if is_tty {
        print!("\r\x1b[2K");
        let _ = std::io::stdout().flush();
    }
    println!(
        "\x1b[1;32m✔ Tailwind standalone compiler cached globally at {}\x1b[0m\n",
        dest_path.display()
    );

    Ok(())
}

/// Prompts user and sets up Tailwind compiler during `yav new`.
pub async fn prompt_and_install_in_new(version: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mgr = TailwindManager::new(version);
    if mgr.is_cached() {
        println!("\x1b[32m✔ Tailwind CSS standalone compiler is ready (cached globally)\x1b[0m");
        return Ok(true);
    }

    let bin_path = mgr.binary_path()?;

    println!("\n\x1b[1;36m💡 Tailwind CSS standalone compiler is needed to build styles.\x1b[0m");
    println!("   It runs standalone (~45 MB) and does not require Node.js or npm.");
    print!("👉 Download Tailwind standalone compiler now? [Y/n]: ");
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if trimmed.is_empty()
            || trimmed == "y"
            || trimmed == "yes"
            || trimmed == "o"
            || trimmed == "oui"
        {
            match download_with_progress(&mgr, &bin_path).await {
                Ok(()) => return Ok(true),
                Err(e) => {
                    eprintln!("\x1b[33m⚠️ Could not download Tailwind compiler: {e}\x1b[0m");
                    eprintln!("   It will be retried when running 'yav dev'.\n");
                    return Ok(false);
                }
            }
        }
    }

    println!("\x1b[90mℹ Skipped Tailwind compiler download. You can install it later when running 'yav dev'.\x1b[0m\n");
    Ok(false)
}

/// Checks and prompts user if Tailwind compiler is missing during `yav dev`.
pub async fn ensure_tailwind_in_dev(
    mgr: &TailwindManager,
) -> Result<bool, Box<dyn std::error::Error>> {
    if mgr.is_cached() {
        return Ok(true);
    }

    let bin_path = mgr.binary_path()?;

    println!("\n\x1b[1;36m💡 Tailwind CSS standalone compiler is not yet installed on this machine.\x1b[0m");
    println!("   It is required to compile utility classes into styles.css.");
    print!("👉 Download Tailwind compiler now? [Y/n]: ");
    let _ = std::io::stdout().flush();

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if trimmed.is_empty()
            || trimmed == "y"
            || trimmed == "yes"
            || trimmed == "o"
            || trimmed == "oui"
        {
            match download_with_progress(mgr, &bin_path).await {
                Ok(()) => return Ok(true),
                Err(e) => {
                    eprintln!("\x1b[31m❌ Failed to download Tailwind compiler: {e}\x1b[0m");
                    return Ok(false);
                }
            }
        }
    }

    println!("\x1b[33m⚠️ Skipping Tailwind compilation. Running with existing styles...\x1b[0m\n");
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_progress_bar_with_total() {
        let total = 40 * 1024 * 1024;
        let downloaded = 20 * 1024 * 1024;
        let bar = format_progress_bar(downloaded, Some(total));
        assert!(bar.contains("50.0%"));
        assert!(bar.contains("20.0 MB / 40.0 MB"));
        assert!(bar.contains('['));
        assert!(bar.contains(']'));
    }

    #[test]
    fn test_format_progress_bar_unknown_total() {
        let downloaded = 15 * 1024 * 1024;
        let bar = format_progress_bar(downloaded, None);
        assert!(bar.contains("15.0 MB downloaded"));
    }
}
