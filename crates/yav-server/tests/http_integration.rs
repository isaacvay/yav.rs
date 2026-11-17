//! End-to-end HTTP tests against a real TCP listener.
//!
//! The unit tests inside `src/` exercise middleware through
//! `tower::ServiceExt::oneshot`. This suite goes one step further: it boots an
//! Axum server on an ephemeral loopback port and issues real network requests
//! with `reqwest`, so status codes, headers and bodies are checked exactly as a
//! browser would see them. It covers the security-sensitive paths that were
//! previously only unit-tested: security headers, CSP nonces, conditional ETag
//! requests, CSRF enforcement and the built-in health endpoints.

use axum::{
    extract::Request,
    http::header,
    middleware::{self, Next},
    response::{Html, Response},
    routing::{get, post},
    Router,
};
use std::time::Duration;
use yav_server::{
    compute_etag, csrf_middleware, etag_middleware, global_tracer, security_headers_middleware,
    CsrfConfig, Session, YavApp,
};

const CSRF_TOKEN: &str = "integration-token";

/// Start a real HTTP server and return its base URL.
async fn serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral loopback port");
    let addr = listener.local_addr().expect("read listener address");
    tokio::spawn(async move {
        // The task is dropped when the test runtime shuts down.
        let _ = axum::serve(listener, app).await;
    });
    format!("http://{addr}")
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build HTTP client")
}

/// Stand in for the session middleware with a session carrying a known token,
/// then delegate to the real CSRF middleware.
async fn csrf_guard(mut req: Request, next: Next) -> Response {
    let session = Session::new();
    session.set("_csrf_token", CSRF_TOKEN);
    req.extensions_mut().insert(session);
    req.extensions_mut().insert(CsrfConfig::enabled());
    csrf_middleware(req, next).await
}

fn csrf_app() -> Router {
    Router::new()
        .route(
            "/submit",
            // Echo the raw bytes back: proves the CSRF middleware rebuilt the
            // request body instead of consuming it.
            post(|body: axum::body::Bytes| async move {
                String::from_utf8_lossy(&body).into_owned()
            }),
        )
        .layer(middleware::from_fn(csrf_guard))
}

#[tokio::test]
async fn security_headers_and_csp_nonce_are_sent_over_real_http() {
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html("<html><body><h1>hello</h1></body></html>") }),
        )
        .layer(middleware::from_fn(security_headers_middleware));
    let base = serve(app).await;

    let resp = client()
        .get(format!("{base}/"))
        .send()
        .await
        .expect("GET / over TCP");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(resp.headers()["x-content-type-options"], "nosniff");
    assert_eq!(resp.headers()["x-frame-options"], "DENY");
    assert_eq!(
        resp.headers()["referrer-policy"],
        "strict-origin-when-cross-origin"
    );
    assert_eq!(resp.headers()["x-xss-protection"], "0");

    let csp = resp
        .headers()
        .get("content-security-policy")
        .expect("strict CSP must be emitted")
        .to_str()
        .expect("CSP must be valid ASCII");
    assert!(
        csp.contains("'nonce-") && !csp.contains("{nonce}"),
        "CSP must carry a resolved per-request nonce: {csp}"
    );

    let body = resp.text().await.expect("read HTML body");
    assert!(body.contains("hello"), "handler body must survive: {body}");
}

#[tokio::test]
async fn etag_supports_real_conditional_requests() {
    const BODY: &str = "{\"ok\":true}";
    let app = Router::new()
        .route(
            "/data",
            get(|| async { ([(header::CONTENT_TYPE, "application/json")], BODY) }),
        )
        .layer(middleware::from_fn(etag_middleware));
    let base = serve(app).await;
    let http = client();

    let first = http
        .get(format!("{base}/data"))
        .send()
        .await
        .expect("GET /data over TCP");
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let etag = first
        .headers()
        .get("etag")
        .expect("ETag header")
        .to_str()
        .expect("ETag must be ASCII")
        .to_string();
    assert_eq!(etag, compute_etag(BODY.as_bytes()));
    assert_eq!(first.text().await.expect("read body"), BODY);

    let second = http
        .get(format!("{base}/data"))
        .header("if-none-match", &etag)
        .send()
        .await
        .expect("conditional GET over TCP");
    assert_eq!(second.status(), reqwest::StatusCode::NOT_MODIFIED);
    assert_eq!(second.headers()["etag"], etag);
}

#[tokio::test]
async fn csrf_rejects_missing_and_invalid_tokens_over_real_http() {
    let base = serve(csrf_app()).await;
    let http = client();
    let url = format!("{base}/submit");

    let missing = http
        .post(&url)
        .body("message=hello")
        .send()
        .await
        .expect("POST without CSRF token");
    assert_eq!(missing.status(), reqwest::StatusCode::FORBIDDEN);

    let wrong = http
        .post(&url)
        .header("x-csrf-token", "attacker-guess")
        .body("message=hello")
        .send()
        .await
        .expect("POST with wrong CSRF token");
    assert_eq!(wrong.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn csrf_accepts_header_token_and_preserves_the_form_body() {
    let base = serve(csrf_app()).await;
    let form = format!("_csrf={CSRF_TOKEN}&message=hello");

    let resp = client()
        .post(format!("{base}/submit"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.clone())
        .send()
        .await
        .expect("POST with a valid form CSRF token");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(
        resp.text().await.expect("read echoed body"),
        form,
        "the handler must still see the original form body"
    );
}

/// Enabling `YavApp::otel(true)` installs automatic per-request HTTP spans.
/// Uses a unique path so the shared global tracer cannot make the assertion
/// flaky when other tests run in the same process.
#[tokio::test]
async fn otel_layer_records_a_span_and_echoes_traceparent() {
    const MARKER: &str = "/otel-instrumentation-probe";
    let tracer = global_tracer();
    tracer.clear();

    let app = YavApp::new()
        .session_secret(b"http_integration_otel_secret_32_by".to_vec())
        .otel(true)
        .get(MARKER, || async { Html("<p>probe</p>") })
        .into_router();
    let base = serve(app).await;

    let resp = client()
        .get(format!("{base}{MARKER}"))
        .send()
        .await
        .expect("GET probe over TCP");
    assert!(resp.status().is_success());

    let traceparent = resp
        .headers()
        .get("traceparent")
        .expect("traceparent must be echoed on the response")
        .to_str()
        .expect("traceparent must be ASCII");
    assert!(
        traceparent.starts_with("00-") && traceparent.len() == 55,
        "expected a W3C traceparent, got {traceparent:?}"
    );

    let matching: Vec<_> = tracer
        .recent_spans()
        .into_iter()
        .filter(|s| s.attributes.get("http.target").map(String::as_str) == Some(MARKER))
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "exactly one SERVER span should be recorded per request"
    );
    assert_eq!(matching[0].kind, "SERVER");
    assert_eq!(matching[0].status, "OK");
    assert_eq!(matching[0].attributes.get("http.method").unwrap(), "GET");
    assert_eq!(
        matching[0].attributes.get("http.status_code").unwrap(),
        "200"
    );
}

/// The layer is opt-in: without `otel(true)` no trace header is added.
#[tokio::test]
async fn otel_layer_is_off_by_default() {
    let app = YavApp::new()
        .session_secret(b"http_integration_otel_off_32_byte".to_vec())
        .into_router();
    let base = serve(app).await;

    let resp = client()
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("GET /health over TCP");
    assert!(resp.status().is_success());
    assert!(
        resp.headers().get("traceparent").is_none(),
        "no tracing header must be added unless explicitly enabled"
    );
}

#[tokio::test]
async fn yav_app_mounts_health_endpoints_on_a_live_server() {
    let app = YavApp::new()
        .session_secret(b"http_integration_test_secret_32_bytes".to_vec())
        .into_router();
    let base = serve(app).await;

    for path in ["/health", "/healthz", "/readyz", "/metrics"] {
        let resp = client()
            .get(format!("{base}{path}"))
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {path} over TCP: {e}"));
        assert!(
            resp.status().is_success(),
            "built-in {path} must be mounted, got {}",
            resp.status()
        );
    }
}
