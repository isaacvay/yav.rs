//! Cloud and server deployment scaffolding (`yav deploy`).
//!
//! Provides ready-to-run deployment templates for Fly.io, Shuttle.rs, Railway, and Systemd VPS.

use clap::Subcommand;
use std::fs;
use std::path::Path;

#[derive(Subcommand, Debug, Clone)]
pub enum DeployTarget {
    /// Scaffold Fly.io configuration (fly.toml)
    Fly {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold Shuttle.rs configuration (Shuttle.toml)
    Shuttle {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold Railway deployment configuration (railway.json)
    Railway {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold Linux systemd service unit file
    Systemd {
        #[arg(short, long)]
        force: bool,
    },
}

pub fn run(target: &DeployTarget) -> Result<(), Box<dyn std::error::Error>> {
    match target {
        DeployTarget::Fly { force } => scaffold_fly(*force),
        DeployTarget::Shuttle { force } => scaffold_shuttle(*force),
        DeployTarget::Railway { force } => scaffold_railway(*force),
        DeployTarget::Systemd { force } => scaffold_systemd(*force),
    }
}

fn detect_app_name() -> String {
    if let Ok(content) = fs::read_to_string("package.yavml") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name =") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    return val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
    }
    if let Ok(content) = fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name =") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    return val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
    }
    "yav-app".to_string()
}

fn scaffold_fly(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let fly_path = Path::new("fly.toml");
    if fly_path.exists() && !force {
        println!("\x1b[33m⚠️ fly.toml already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }

    let app_name = detect_app_name();
    let content = format!(
        r#"app = "{app_name}"
primary_region = "cdg"

[build]
  dockerfile = "Dockerfile"

[env]
  YAV_ENV = "production"

[http_service]
  internal_port = 3000
  force_https = true
  auto_stop_machines = 'stop'
  auto_start_machines = true
  min_machines_running = 1
  processes = ['app']

[[http_service.checks]]
  interval = '15s'
  timeout = '2s'
  grace_period = '5s'
  method = 'get'
  path = '/healthz'

[[vm]]
  memory = '512mb'
  cpu_kind = 'shared'
  cpus = 1
"#
    );

    fs::write(fly_path, content)?;
    println!("\x1b[32m✔ Created fly.toml for Fly.io deployment\x1b[0m");
    println!("  Run: fly launch && fly deploy");
    Ok(())
}

fn scaffold_shuttle(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let shuttle_path = Path::new("Shuttle.toml");
    if shuttle_path.exists() && !force {
        println!("\x1b[33m⚠️ Shuttle.toml already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }

    let app_name = detect_app_name();
    let content = format!(
        r#"name = "{app_name}"
"#
    );

    fs::write(shuttle_path, content)?;
    println!("\x1b[32m✔ Created Shuttle.toml for Shuttle.rs deployment\x1b[0m");
    println!("  Run: cargo shuttle deploy");
    Ok(())
}

fn scaffold_railway(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let railway_path = Path::new("railway.json");
    if railway_path.exists() && !force {
        println!("\x1b[33m⚠️ railway.json already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }

    let content = r#"{
  "$schema": "https://railway.app/railway.schema.json",
  "build": {
    "builder": "DOCKERFILE",
    "dockerfilePath": "Dockerfile"
  },
  "deploy": {
    "numReplicas": 1,
    "restartPolicyType": "ON_FAILURE",
    "restartPolicyMaxRetries": 10
  }
}
"#;

    fs::write(railway_path, content)?;
    println!("\x1b[32m✔ Created railway.json for Railway.app deployment\x1b[0m");
    println!("  Run: railway up");
    Ok(())
}

fn scaffold_systemd(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let app_name = detect_app_name();
    let service_filename = format!("{app_name}.service");
    let service_path = Path::new(&service_filename);
    if service_path.exists() && !force {
        println!("\x1b[33m⚠️ {service_filename} already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }

    let content = format!(
        r#"[Unit]
Description={app_name} web service (Yav.rs)
After=network.target

[Service]
Type=simple
User=www-data
Group=www-data
WorkingDirectory=/var/www/{app_name}
ExecStart=/var/www/{app_name}/{app_name}
Restart=always
RestartSec=5s
Environment=YAV_ENV=production
Environment=PORT=3000
Environment=RUST_LOG=info
Environment=SESSION_SECRET=change_me_in_production_at_least_32_bytes_long

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true

[Install]
WantedBy=multi-user.target
"#
    );

    fs::write(service_path, content)?;
    println!("\x1b[32m✔ Created {service_filename} systemd service unit file\x1b[0m");
    println!("  Install on Linux: sudo cp {service_filename} /etc/systemd/system/ && sudo systemctl enable --now {app_name}");
    Ok(())
}
