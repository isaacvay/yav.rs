//! Docker production container scaffolding.
//!
//! Generates an ultra-optimized, multi-stage Dockerfile, docker-compose.yml,
//! and .dockerignore tailored for minimal image size (<30MB) and dependency caching.

use std::{fs, path::Path};

/// Scaffolds production Docker deployment files.
pub fn run(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let dockerfile_path = Path::new("Dockerfile");
    let compose_path = Path::new("docker-compose.yml");
    let ignore_path = Path::new(".dockerignore");

    if dockerfile_path.exists() && !force {
        println!("⚠️  Dockerfile already exists. Use --force to overwrite.");
        return Ok(());
    }

    // Detect package name from Cargo.toml or package.yavml
    let app_name = detect_app_name().unwrap_or_else(|| "yav-app".to_string());

    let has_crates = Path::new("crates").exists();
    let has_public = Path::new("public").exists();

    // 1. Dockerfile
    let dockerfile_content = generate_dockerfile(&app_name, has_crates, has_public);
    fs::write(dockerfile_path, dockerfile_content)?;
    println!("  created Dockerfile (multi-stage minimal Alpine build)");

    // 2. docker-compose.yml
    let compose_content = format!(
        r#"services:
  web:
    build: .
    container_name: {app_name}
    ports:
      - "3000:3000"
    environment:
      - YAV_ENV=production
      - HOST=0.0.0.0
      - PORT=3000
      - RUST_LOG=info
    restart: unless-stopped
"#
    );
    fs::write(compose_path, compose_content)?;
    println!("  created docker-compose.yml");

    // 3. .dockerignore
    let ignore_content = r#"target/
.git/
.gitignore
.env*
*.md
docs/
"#;
    fs::write(ignore_path, ignore_content)?;
    println!("  created .dockerignore");

    println!("\n✨ Docker configuration generated successfully!");
    println!("Run with: docker compose up --build");
    Ok(())
}

fn detect_app_name() -> Option<String> {
    if let Ok(content) = fs::read_to_string("Cargo.toml") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name =") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    return Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                }
            }
        }
    }
    None
}

/// Generate Dockerfile contents tailored to project structure.
pub fn generate_dockerfile(app_name: &str, has_crates: bool, has_public: bool) -> String {
    let copy_crates = if has_crates {
        "COPY crates ./crates/\n"
    } else {
        ""
    };
    let copy_public = if has_public {
        "COPY public ./public/\n"
    } else {
        ""
    };
    let copy_runtime_public = if has_public {
        "COPY --from=builder /app/public /app/public\n"
    } else {
        ""
    };

    format!(
        r#"# ==========================================
# Multi-stage Dockerfile for Yav Application
# Produces an ultra-lean (<30MB) Alpine runtime image
# ==========================================

# 1. Builder Stage
FROM rust:1.80-alpine3.20 AS builder
WORKDIR /app

RUN apk add --no-cache musl-dev curl build-base

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* package.yavml* ./
{copy_crates}COPY src ./src/
{copy_public}
# Build application in release mode and compile Tailwind CSS if present
ENV CARGO_INCREMENTAL=0
RUN cargo build --release && \
    if [ -f "public/input.css" ]; then \
      ARCH=$(uname -m); \
      if [ "$ARCH" = "aarch64" ]; then TW_ARCH="linux-arm64-musl"; else TW_ARCH="linux-x64-musl"; fi; \
      curl -sSL "https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.17/tailwindcss-${{TW_ARCH}}" -o /tmp/tailwindcss && \
      chmod +x /tmp/tailwindcss && \
      /tmp/tailwindcss -i public/input.css -o public/styles.css --content "./src/**/*.{{rs,yav,rsx,html}},./public/**/*.html" --minify; \
    fi

# 2. Runtime Stage
FROM alpine:3.20 AS runtime
WORKDIR /app

# Run as non-privileged system user for security
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

COPY --from=builder /app/target/release/{app_name} /app/server
{copy_runtime_public}
RUN chown -R appuser:appgroup /app
USER appuser

ENV YAV_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info
EXPOSE 3000

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://127.0.0.1:3000/healthz || exit 1

CMD ["/app/server"]
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_app_name() {
        let manifest = r#"[package]
name = "my-awesome-yav-app"
version = "0.1.0"
"#;
        let mut name = None;
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name =") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    name = Some(val.trim().trim_matches('"').to_string());
                }
            }
        }
        assert_eq!(name, Some("my-awesome-yav-app".to_string()));
    }

    #[test]
    fn test_generate_dockerfile_without_crates() {
        let df = generate_dockerfile("my-app", false, true);
        assert!(!df.contains("COPY crates ./crates/"));
        assert!(df.contains("COPY public ./public/"));
        assert!(df.contains("COPY --from=builder /app/target/release/my-app /app/server"));
    }

    #[test]
    fn test_generate_dockerfile_with_crates() {
        let df = generate_dockerfile("my-app", true, false);
        assert!(df.contains("COPY crates ./crates/"));
        assert!(!df.contains("COPY public ./public/"));
    }
}
