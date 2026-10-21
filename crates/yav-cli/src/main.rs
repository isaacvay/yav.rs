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
mod ext;
mod generate;
mod icon_cmd;
mod new;
mod pwa;
mod reload;
mod run_cmd;
mod sitemap_cmd;
mod test_cmd;

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
    New { name: String },
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
    /// Run tests
    Test,
    /// Run a named script defined in package.yavml (like npm run <script>)
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
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
        Commands::New { name } => {
            new::run(&name)?;
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
            ts: _,
            openapi,
            output,
        } => {
            let root = std::path::PathBuf::from(".");
            if openapi {
                generate::generate_openapi(&root, output)?;
            } else {
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
        Commands::Test => {
            test_cmd::run().await?;
        }
        Commands::Run { script, args } => {
            run_cmd::run(&script, &args)?;
        }
        Commands::Ext { action } => {
            ext::run(&action)?;
        }
    }

    Ok(())
}
