//! Opt-in integrations from the competitive roadmap.
//!
//! Every submodule is feature-gated so the default build keeps the small
//! dependency surface of the framework. These modules contain real adapters;
//! they never fabricate success when a service is not configured.

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "http-llm",
    feature = "smtp",
    feature = "object-storage"
))]
use std::sync::OnceLock;

#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "http-llm",
    feature = "smtp",
    feature = "object-storage"
))]
fn shared_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    static RT: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();
    match RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("yav-roadmap-io")
            .build()
            .map_err(|e| format!("failed to build shared roadmap Tokio runtime: {e}"))
    }) {
        Ok(rt) => Ok(rt),
        Err(err) => Err(err.clone()),
    }
}

/// Drive an async future from synchronous adapter trait methods without panicking
/// when invoked inside an active Tokio runtime (`Cannot start a runtime from within a runtime`).
#[cfg(any(
    feature = "sqlite",
    feature = "postgres",
    feature = "http-llm",
    feature = "smtp",
    feature = "object-storage"
))]
pub(crate) fn block_on_shared<F>(fut: F) -> Result<F::Output, String>
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    let rt = shared_runtime()?;
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            let exec = || {
                std::thread::scope(|s| {
                    s.spawn(|| rt.block_on(fut))
                        .join()
                        .map_err(|_| "roadmap background task panicked".to_string())
                })
            };
            if matches!(
                handle.runtime_flavor(),
                tokio::runtime::RuntimeFlavor::MultiThread
            ) {
                tokio::task::block_in_place(exec)
            } else {
                exec()
            }
        }
        Err(_) => Ok(rt.block_on(fut)),
    }
}

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
