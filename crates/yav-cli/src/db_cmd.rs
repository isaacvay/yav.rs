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
    /// Apply all pending SQL migrations and record checksums in the journal
    Migrate,
    /// Rollback the most recently applied SQL migration
    Rollback,
    /// Run or scaffold database seed data (`migrations/seed.sql`)
    Seed,
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

            let migrator = yav_server::db::Migrator::new(migrations_dir);
            let statuses = migrator.status()?;
            println!("Database Migrations ({} found):", statuses.len());
            for st in statuses {
                let badge = if st.applied {
                    if st.checksum_valid {
                        "✅ applied"
                    } else {
                        "⚠️ checksum mismatch"
                    }
                } else {
                    "⏳ pending"
                };
                println!("  • {} [{}]", st.version, badge);
            }
        }

        DbAction::Migrate => {
            if !migrations_dir.exists() {
                println!("⚠️  No migrations/ directory found. Run `yav db init` first.");
                return Ok(());
            }
            let migrator = yav_server::db::Migrator::new(migrations_dir);
            let applied = migrator.migrate_up_with(|sql| {
                println!("  → Migrating SQL ({} bytes)", sql.len());
                execute_sql_against_database(sql)
            })?;
            if applied.is_empty() {
                println!("✨ Database is already up to date.");
            } else {
                for v in &applied {
                    println!("  ✅ Applied migration: {v}");
                }
                println!("✨ Applied {} migration(s) successfully.", applied.len());
            }
        }

        DbAction::Rollback => {
            let migrator = yav_server::db::Migrator::new(migrations_dir);
            match migrator.rollback_last_with(|sql| {
                println!("  ↩ Rolling back SQL ({} bytes)", sql.len());
                execute_sql_against_database(sql)
            })? {
                Some(v) => println!("✨ Rolled back migration: {v}"),
                None => println!("ℹ️  No applied migrations to roll back."),
            }
        }

        DbAction::Seed => {
            if !migrations_dir.exists() {
                fs::create_dir_all(migrations_dir)?;
            }
            let seed_path = migrations_dir.join("seed.sql");
            if !seed_path.exists() {
                fs::write(
                    &seed_path,
                    "-- Seed data for local development\nINSERT INTO users (email, name) VALUES ('admin@yav.rs', 'Admin');\n",
                )?;
                println!("  created {}", seed_path.display());
            }

            // Actually execute the seed SQL against the configured database
            let seed_sql = fs::read_to_string(&seed_path)
                .map_err(|e| format!("Failed to read seed file: {e}"))?;
            match execute_sql_against_database(&seed_sql) {
                Ok(()) => println!("🌱 Seed data applied from {}", seed_path.display()),
                Err(e) => {
                    eprintln!("✗ Failed to execute seed SQL: {e}");
                    eprintln!(
                        "  Seed file is available at {}. Run it manually with your SQL client.",
                        seed_path.display()
                    );
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}

fn read_database_url() -> Option<String> {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        if !url.trim().is_empty() {
            return Some(url.trim().to_string());
        }
    }
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("DATABASE_URL=") {
                let clean = rest.trim().trim_matches('"').trim_matches('\'');
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

fn execute_sql_against_database(sql: &str) -> Result<(), String> {
    let Some(db_url) = read_database_url() else {
        return Err(
            "DATABASE_URL is not set. Configure it in .env or as an environment variable to run migrations."
                .to_string(),
        );
    };

    if db_url.starts_with("sqlite:") || db_url.ends_with(".db") || db_url.ends_with(".sqlite") {
        let db_path = db_url
            .strip_prefix("sqlite://")
            .or_else(|| db_url.strip_prefix("sqlite:"))
            .unwrap_or(&db_url);
        let status = std::process::Command::new("sqlite3")
            .arg(db_path)
            .arg(sql)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("  🎯 Executed SQL via sqlite3 on '{db_path}'");
                Ok(())
            }
            Ok(s) => Err(format!(
                "sqlite3 returned non-zero exit code: {:?}",
                s.code()
            )),
            Err(_) => Err(
                "sqlite3 binary not found in PATH. Install sqlite3 to run migrations against SQLite."
                    .to_string(),
            ),
        }
    } else if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        let status = std::process::Command::new("psql")
            .arg(&db_url)
            .arg("-c")
            .arg(sql)
            .status();
        match status {
            Ok(s) if s.success() => {
                println!("  🎯 Executed SQL via psql on database");
                Ok(())
            }
            Ok(s) => Err(format!("psql returned non-zero exit code: {:?}", s.code())),
            Err(_) => Err(
                "psql binary not found in PATH. Install PostgreSQL client tools to run migrations."
                    .to_string(),
            ),
        }
    } else {
        Err(format!(
            "Unsupported DATABASE_URL format: '{db_url}'. Supported formats: sqlite:path, postgres://..., postgresql://..."
        ))
    }
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
