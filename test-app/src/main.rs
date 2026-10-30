mod components;
mod layouts;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Read PORT from environment (e.g. Render, Railway, Fly.io, Heroku) or default to 3000
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    // YavApp automatically loads [server] configuration from package.yavml
    // and auto_routes! discovers and registers all routes under src/routes
    let app = yav::server::YavApp::new()
        .port(port)
        .static_dir("public");
    let app = yav::auto_routes!(app);

    println!("  ➜  Local:   http://{}:{}", app.host, app.port);
    app.serve().await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_default_health_route() {
        let app = yav::server::YavApp::new().static_dir("public");
        let app = yav::auto_routes!(app);
        let router = app.into_router();

        let req = Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "yav");
    }

    #[tokio::test]
    async fn test_custom_not_found_route() {
        let app = yav::server::YavApp::new().static_dir("public");
        let app = yav::auto_routes!(app);
        let router = app.into_router();

        let req = Request::builder()
            .uri("/unknown-test-path")
            .body(axum::body::Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(html.contains("404 Not Found"));
        assert!(html.contains("Page not found"));
    }

    #[test]
    fn test_port_env_reading() {
        std::env::set_var("PORT", "7890");
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);
        assert_eq!(port, 7890);

        let app = yav::server::YavApp::new().port(port);
        assert_eq!(app.port, 7890);
        std::env::remove_var("PORT");
    }

    yav::typed_routes!("src/routes");

    #[test]
    fn test_typed_routes_generation() {
        assert_eq!(Routes::ROOT, "/");
        assert_eq!(Routes::ABOUT, "/about");
        assert_eq!(Routes::HEALTH, "/health");
    }
}
