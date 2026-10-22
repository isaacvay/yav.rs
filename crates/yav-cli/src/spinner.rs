//! Animated terminal spinner and progress indicators for long-running CLI operations.

use std::io::{IsTerminal, Write};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// An animated terminal spinner that displays real-time feedback during async tasks.
pub struct Spinner {
    stop_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
    is_tty: bool,
}

impl Spinner {
    /// Starts an animated spinner with the given message.
    pub fn start(message: impl Into<String>) -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let message = message.into();

        if !is_tty {
            println!("\x1b[36m⏳\x1b[0m {message}...");
            return Self {
                stop_tx: None,
                handle: None,
                is_tty: false,
            };
        }

        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            let mut interval = tokio::time::interval(Duration::from_millis(80));

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        break;
                    }
                    _ = interval.tick() => {
                        let frame = frames[i % frames.len()];
                        print!("\r\x1b[2K\x1b[36m{frame}\x1b[0m {message}");
                        let _ = std::io::stdout().flush();
                        i += 1;
                    }
                }
            }
        });

        Self {
            stop_tx: Some(stop_tx),
            handle: Some(handle),
            is_tty: true,
        }
    }

    /// Stops the spinner and clears the current terminal line.
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        if self.is_tty {
            print!("\r\x1b[2K");
            let _ = std::io::stdout().flush();
        }
    }

    /// Finishes the spinner with a green success checkmark.
    pub fn finish_with_success(mut self, message: impl std::fmt::Display) {
        self.stop();
        println!("\x1b[32m✔\x1b[0m {message}");
    }

    /// Finishes the spinner with a yellow warning indicator.
    pub fn finish_with_warning(mut self, message: impl std::fmt::Display) {
        self.stop();
        println!("\x1b[33m⚠️\x1b[0m {message}");
    }

    /// Finishes the spinner with a red error cross.
    pub fn finish_with_error(mut self, message: impl std::fmt::Display) {
        self.stop();
        println!("\x1b[31m❌\x1b[0m {message}");
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}
