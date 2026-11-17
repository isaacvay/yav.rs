//! Opt-in integrations from the competitive roadmap.
//!
//! Every submodule is feature-gated so the default build keeps the small
//! dependency surface of the framework. These modules contain real adapters;
//! they never fabricate success when a service is not configured.

#[cfg(any(feature = "sqlite", feature = "postgres", feature = "tagged-cache"))]
pub mod persistence;

#[cfg(feature = "durable-jobs")]
pub mod jobs;

#[cfg(feature = "smtp")]
pub mod email;

#[cfg(feature = "object-storage")]
pub mod storage;

#[cfg(feature = "http-llm")]
pub mod llm;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "schema-codegen")]
pub mod schema;

#[cfg(feature = "oauth")]
pub mod oauth;

#[cfg(feature = "argon2")]
pub mod security;

#[cfg(feature = "otel-sdk")]
pub mod otel;
