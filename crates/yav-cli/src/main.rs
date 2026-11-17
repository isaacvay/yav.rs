//! # YAV CLI
//!
//! Command-line interface for the Yav web framework.
//!
//! Provides project scaffolding (`yav new`), development server with live reload (`yav dev`),
//! production builds (`yav build`), project validation (`yav check`), and test execution (`yav test`).

mod add;
mod analyze;
mod build;
mod check;
mod db_cmd;
mod deploy;
mod dev;
mod docker;
mod doctor;
#[cfg(feature = "dx-inspect")]
mod dx;
mod export_cmd;
mod ext;
mod generate;
mod icon_cmd;
mod new;
mod pwa;
mod reload;
mod routes_cmd;
mod run_cmd;
mod sitemap_cmd;
mod spinner;
mod tailwind_setup;
mod test_cmd;
mod update_cmd;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "yav",
    version,
    disable_version_flag = true,
    about = "The Yav web framework CLI — High-performance full-stack web framework for Rust",
    long_about = "The Yav web framework CLI.\nFast, type-safe full-stack web development with compiled Rust, built-in Tailwind CSS, and zero Node.js dependencies."
)]
struct Cli {
    /// Print version information (-v, -V, or --version)
    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    _version: Option<bool>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Yav project
    New {
        name: String,
        /// Verify the generated project with `cargo check` before reporting success
        #[arg(long)]
        check: bool,
    },
    /// Add a styled UI component or block (e.g. auth-card, stat-grid, data-table, pricing-table, navbar)
    Add { component: String },
    /// Generate a route, component, or layout (alias: g)
    #[command(alias = "g")]
    Generate {
        #[command(subcommand)]
        target: generate::GenerateTarget,
    },
    /// Explore and search the built-in Lucide icon catalog
    Icon {
        #[command(subcommand)]
        action: icon_cmd::IconAction,
    },
    /// Manage database initialization and SQL migrations
    Db {
        #[command(subcommand)]
        action: db_cmd::DbAction,
    },
    /// Generate production Dockerfile and docker-compose.yml
    Docker {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold cloud deployment configurations (Fly.io, Shuttle, Railway, Systemd)
    Deploy {
        #[command(subcommand)]
        target: deploy::DeployTarget,
    },
    /// Generate static sitemap.xml and robots.txt in public/
    Sitemap {
        #[arg(short, long, default_value = "https://example.com")]
        base_url: String,
    },
    /// Start the development server
    Dev {
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Generate TypeScript interfaces or OpenAPI specifications
    Codegen {
        /// Generate TypeScript definitions
        #[arg(long)]
        ts: bool,
        /// Generate OpenAPI JSON specification
        #[arg(long)]
        openapi: bool,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Analyze project assets, bundle sizes, routes, and performance scorecard
    Analyze,
    /// Scaffold Progressive Web App (PWA) manifest, service worker, and offline page
    Pwa {
        /// Force overwrite existing PWA assets
        #[arg(short, long)]
        force: bool,
    },
    /// Build for production
    Build,
    /// Check the project for errors
    Check,
    /// Run tests with optional filters and cargo test arguments
    Test {
        /// Arguments and filters to pass through to cargo test (e.g. `yav test my_test -- --nocapture`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run a named script defined in package.yavml (like `npm run <script>`)
    Run {
        /// Name of the script to execute (e.g. dev, build, test)
        script: String,
        /// Additional arguments to pass through to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Manage IDE editor extensions (VS Code, Cursor)
    Ext {
        #[command(subcommand)]
        action: ext::ExtAction,
    },
    /// Export website to standalone static HTML for CDN/Pages hosting (SSG)
    Export {
        /// Target output directory
        #[arg(short, long, default_value = "dist")]
        output: String,
        /// Base canonical URL
        #[arg(short, long, default_value = "https://example.com")]
        base_url: String,
    },
    /// Update the Yav CLI binary and current project dependencies (aliases: up, upgrade)
    #[command(alias = "up", alias = "upgrade")]
    Update(update_cmd::UpdateArgs),
    /// Diagnose environment, Rust toolchain, manifest, and Tailwind CSS setup
    Doctor,
    /// Inspect discovered file-system routes, HTTP methods, layouts, and middlewares
    Routes,
}

/// Derive a separate OpenAPI output path when `--ts --openapi --output X` is used.
fn sibling_openapi_output(output: &str) -> String {
    let path = std::path::Path::new(output);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("openapi");
    path.with_file_name(format!("{stem}.openapi.json"))
        .to_string_lossy()
        .into_owned()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    #[cfg(feature = "dx-inspect")]
    if let Ok(summary) = dx::workspace_summary() {
        tracing::debug!("{summary}");
    }

    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            let _ = Cli::command().print_help();
            println!();
            return Ok(());
        }
    };

    match command {
        Commands::New { name, check } => {
            new::run(&name, check).await?;
        }
        Commands::Add { component } => {
            add::run(&component)?;
        }
        Commands::Generate { target } => {
            generate::run(&target)?;
        }
        Commands::Icon { action } => {
            icon_cmd::run(&action)?;
        }
        Commands::Db { action } => {
            db_cmd::run(&action)?;
        }
        Commands::Docker { force } => {
            docker::run(force)?;
        }
        Commands::Deploy { target } => {
            deploy::run(&target)?;
        }
        Commands::Sitemap { base_url } => {
            sitemap_cmd::run(&base_url)?;
        }
        Commands::Dev { port } => {
            dev::run(port).await?;
        }
        Commands::Codegen {
            ts,
            openapi,
            output,
        } => {
            let root = std::path::PathBuf::from(".");
            if openapi && ts {
                // Both generators must not write to the same `--output`: the
                // second one would silently overwrite the first. Keep the
                // TypeScript file at the requested path and derive a sibling
                // `<stem>.openapi.json` for the specification.
                let openapi_output = output.as_deref().map(sibling_openapi_output);
                generate::generate_typescript(&root, output)?;
                generate::generate_openapi(&root, openapi_output)?;
            } else if openapi {
                generate::generate_openapi(&root, output)?;
            } else {
                // Default or --ts: generate TypeScript
                generate::generate_typescript(&root, output)?;
            }
        }
        Commands::Analyze => {
            analyze::run()?;
        }
        Commands::Pwa { force } => {
            pwa::run(force)?;
        }
        Commands::Build => {
            build::run().await?;
        }
        Commands::Check => {
            check::run().await?;
        }
        Commands::Test { args } => {
            test_cmd::run(&args).await?;
        }
        Commands::Run { script, args } => {
            run_cmd::run(&script, &args)?;
        }
        Commands::Ext { action } => {
            ext::run(&action)?;
        }
        Commands::Export { output, base_url } => {
            export_cmd::run(&output, &base_url).await?;
        }
        Commands::Update(args) => {
            update_cmd::run(&args).await?;
        }
        Commands::Doctor => {
            doctor::run()?;
        }
        Commands::Routes => {
            routes_cmd::run()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn codegen_output_paths_never_collide() {
        assert_eq!(sibling_openapi_output("api.gen"), "api.openapi.json");
        // Compare paths, not strings: Windows renders the separator as `\`.
        assert_eq!(
            Path::new(&sibling_openapi_output("dist/types.d.ts")),
            Path::new("dist").join("types.d.openapi.json")
        );
        assert_eq!(
            Path::new(&sibling_openapi_output("out/generated")),
            Path::new("out").join("generated.openapi.json")
        );
    }

    #[test]
    fn test_command_parses_arguments() {
        let cli = Cli::try_parse_from(["yav", "test"]).unwrap();
        match cli.command {
            Some(Commands::Test { args }) => assert!(args.is_empty()),
            _ => panic!("Expected Commands::Test"),
        }

        let cli = Cli::try_parse_from(["yav", "test", "my_filter", "--", "--nocapture"]).unwrap();
        match cli.command {
            Some(Commands::Test { args }) => {
                assert_eq!(args, vec!["my_filter", "--", "--nocapture"]);
            }
            _ => panic!("Expected Commands::Test with arguments"),
        }

        let cli = Cli::try_parse_from(["yav", "test", "-p", "yav-core"]).unwrap();
        match cli.command {
            Some(Commands::Test { args }) => {
                assert_eq!(args, vec!["-p", "yav-core"]);
            }
            _ => panic!("Expected Commands::Test with package flag"),
        }
    }
}
