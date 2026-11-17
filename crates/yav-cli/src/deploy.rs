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
    /// Scaffold Render.com Blueprint configuration (render.yaml)
    Render {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold Cloudflare Containers / Workers configuration (wrangler.toml)
    Cloudflare {
        #[arg(short, long)]
        force: bool,
    },
    /// Scaffold Caddy reverse proxy configuration (Caddyfile)
    Caddy {
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
        DeployTarget::Render { force } => scaffold_render(*force),
        DeployTarget::Cloudflare { force } => scaffold_cloudflare(*force),
        DeployTarget::Caddy { force } => scaffold_caddy(*force),
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
Environment=SESSION_SECRET=${{SESSION_SECRET:?SESSION_SECRET environment variable is required}}

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

pub fn generate_render_yaml(app_name: &str) -> String {
    format!(
        r#"services:
  - type: web
    name: {app_name}
    runtime: docker
    dockerfilePath: ./Dockerfile
    healthCheckPath: /healthz
    envVars:
      - key: YAV_ENV
        value: production
      - key: PORT
        value: "3000"
      - key: SESSION_SECRET
        generateValue: true
"#
    )
}

fn scaffold_render(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let render_path = Path::new("render.yaml");
    if render_path.exists() && !force {
        println!("\x1b[33m⚠️ render.yaml already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }
    let app_name = detect_app_name();
    fs::write(render_path, generate_render_yaml(&app_name))?;
    println!("\x1b[32m✔ Created render.yaml for Render.com deployment\x1b[0m");
    Ok(())
}

pub fn generate_wrangler_toml(app_name: &str) -> String {
    format!(
        r#"name = "{app_name}"
compatibility_date = "2026-01-01"

[vars]
YAV_ENV = "production"

[[containers]]
max_instances = 3
name = "{app_name}-container"
image = "./Dockerfile"
"#
    )
}

fn scaffold_cloudflare(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let wrangler_path = Path::new("wrangler.toml");
    if wrangler_path.exists() && !force {
        println!("\x1b[33m⚠️ wrangler.toml already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }
    let app_name = detect_app_name();
    fs::write(wrangler_path, generate_wrangler_toml(&app_name))?;
    println!("\x1b[32m✔ Created wrangler.toml for Cloudflare Containers deployment\x1b[0m");
    Ok(())
}

pub fn generate_caddyfile(app_name: &str) -> String {
    format!(
        r#"# Caddyfile for {app_name} (Auto-HTTPS & HTTP/3)
{app_name}.example.com {{
    encode zstd gzip
    handle_path /public/* {{
        root * ./public
        header Cache-Control "public, max-age=31536000, immutable"
        file_server
    }}
    reverse_proxy 127.0.0.1:3000 {{
        health_uri /healthz
        health_interval 15s
    }}
}}
"#
    )
}

fn scaffold_caddy(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let caddy_path = Path::new("Caddyfile");
    if caddy_path.exists() && !force {
        println!("\x1b[33m⚠️ Caddyfile already exists. Use --force to overwrite.\x1b[0m");
        return Ok(());
    }
    let app_name = detect_app_name();
    fs::write(caddy_path, generate_caddyfile(&app_name))?;
    println!("\x1b[32m✔ Created Caddyfile for Caddy reverse proxy deployment\x1b[0m");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_generators() {
        let render = generate_render_yaml("my-saas");
        assert!(render.contains("name: my-saas"));
        assert!(render.contains("healthCheckPath: /healthz"));

        let wrangler = generate_wrangler_toml("my-saas");
        assert!(wrangler.contains("name = \"my-saas\""));
        assert!(wrangler.contains("[[containers]]"));

        let caddy = generate_caddyfile("my-saas");
        assert!(caddy.contains("reverse_proxy 127.0.0.1:3000"));
        assert!(caddy.contains("encode zstd gzip"));
    }
}
