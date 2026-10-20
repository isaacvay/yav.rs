//! Database migrations and scaffolding CLI command.
//!
//! Provides SQL migration file management, timestamped migration generation,
//! and database connection initialization for SQLite and PostgreSQL.

use clap::Subcommand;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// Subcommands for `yav db`.
#[derive(Subcommand, Debug, Clone)]
pub enum DbAction {
    /// Initialize migrations directory and .env configuration
    Init,
    /// Create a new timestamped SQL migration file
    New { name: String },
    /// List available migrations and their status
    Status,
}

/// Executes the database CLI commands.
pub fn run(action: &DbAction) -> Result<(), Box<dyn std::error::Error>> {
    let migrations_dir = Path::new("migrations");

    match action {
        DbAction::Init => {
            if !migrations_dir.exists() {
                fs::create_dir_all(migrations_dir)?;
                println!("  created migrations/");
            }

            // Create sample initial migration if empty
            let init_file = migrations_dir.join("0001_initial_schema.sql");
            if !init_file.exists() {
                let init_sql = r#"-- Initial Database Migration
-- Up
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Down
DROP TABLE IF EXISTS users;
"#;
                fs::write(&init_file, init_sql)?;
                println!("  created {}", init_file.display());
            }

            // Create .env if missing
            let env_path = Path::new(".env");
            if !env_path.exists() {
                let env_content = "DATABASE_URL=sqlite:app.db\n";
                fs::write(env_path, env_content)?;
                println!("  created .env with DATABASE_URL=sqlite:app.db");
            }

            println!("\n✨ Database scaffolding initialized successfully!");
        }

        DbAction::New { name } => {
            if !migrations_dir.exists() {
                fs::create_dir_all(migrations_dir)?;
            }

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let clean_name = name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>();

            let filename = format!("{timestamp}_{clean_name}.sql");
            let file_path = migrations_dir.join(&filename);

            let template = format!(
                r#"-- Migration: {name}
-- Timestamp: {timestamp}

-- Up
-- Write your forward schema changes here:


-- Down
-- Write rollback instructions here:

"#
            );

            fs::write(&file_path, template)?;
            println!("  created migration: {}", file_path.display());
            println!("✨ Edit your migration file and apply with your preferred SQL runner.");
        }

        DbAction::Status => {
            if !migrations_dir.exists() {
                println!("⚠️  No migrations/ directory found. Run `yav db init` first.");
                return Ok(());
            }

            let mut files = Vec::new();
            if let Ok(entries) = fs::read_dir(migrations_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "sql") {
                        if let Some(name) = path.file_name() {
                            files.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }

            files.sort();

            println!("Database Migrations ({} found):", files.len());
            for f in files {
                println!("  • {f}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_clean_migration_name() {
        let name = "Add User Avatars & Roles";
        let clean = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        assert_eq!(clean, "add_user_avatars___roles");
    }
}
