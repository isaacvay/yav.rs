use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[path = "../src/layouts/mod.rs"]
mod layouts;

#[path = "../src/routes/mod.rs"]
mod routes;

use routes::components::components as components_gallery;
use routes::icons::icons as icons_catalog;
use routes::index::index;
use routes::projects::_id_::_id_ as project_detail;
use routes::projects::index::index as projects_list;
use routes::settings::{settings_get, settings_post};

fn test_app() -> axum::Router {
    yav::server::YavApp::new()
        .get("/", index)
        .get("/components", components_gallery)
        .get("/icons", icons_catalog)
        .get("/projects", projects_list)
        .get("/projects/:id", project_detail)
        .get("/settings", settings_get)
        .post("/settings", settings_post)
        .into_router()
}

/// Explicit opt-out used to exercise the form handler itself. CSRF is enabled
/// by default since v0.1.5, so a real application would send `_csrf` or the
/// `x-csrf-token` header instead.
fn app_without_csrf() -> axum::Router {
    yav::server::YavApp::new()
        .csrf_protection(yav::server::CsrfConfig::disabled())
        .post("/settings", settings_post)
        .into_router()
}

#[tokio::test]
async fn test_dashboard_routes_render_html() {
    let app = test_app();

    // 1. Overview Page
    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(
        html.contains("Enterprise Metrics &amp; Activity")
            || html.contains("Enterprise Metrics & Activity")
    );
    assert!(html.contains("Monthly Recurring Revenue"));

    // 2. Component Gallery
    let req = Request::builder()
        .uri("/components")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("Component Showroom"));
    assert!(html.contains("Buttons &amp; Actions") || html.contains("Buttons & Actions"));

    // 3. Icons Catalog
    let req = Request::builder()
        .uri("/icons")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("Built-in Lucide Icons Catalog"));

    // 4. Projects List
    let req = Request::builder()
        .uri("/projects")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("Microservice Projects"));

    // 5. Dynamic Route Project Detail
    let req = Request::builder()
        .uri("/projects/yav-auth-service")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("Yav Auth Service"));
    assert!(html.contains("/projects/yav-auth-service"));

    // 6. Settings Form GET & POST
    let req = Request::builder()
        .uri("/settings")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let set_cookie = resp
        .headers()
        .get(axum::http::header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').next())
        .unwrap_or("")
        .to_string();
    let get_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let get_html = String::from_utf8_lossy(&get_bytes);
    assert!(get_html.contains("name=\"_csrf\""));
    let csrf_val = get_html
        .split("name=\"_csrf\"")
        .nth(1)
        .and_then(|s| s.split("value=\"").nth(1))
        .and_then(|s| s.split('"').next())
        .unwrap_or("")
        .to_string();
    assert!(!csrf_val.is_empty());
    assert!(!csrf_val.contains("demo-yav-csrf-token"));

    let req_post = Request::builder()
        .method("POST")
        .uri("/settings")
        .body(axum::body::Body::empty())
        .unwrap();
    // CSRF is enabled by default: an unprotected POST must be rejected.
    let resp_post = app.clone().oneshot(req_post).await.unwrap();
    assert_eq!(resp_post.status(), StatusCode::FORBIDDEN);

    // Valid CSRF token + session cookie passes through the real middleware.
    let req_valid = Request::builder()
        .method("POST")
        .uri("/settings")
        .header(axum::http::header::COOKIE, &set_cookie)
        .header(
            axum::http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(axum::body::Body::from(format!(
            "_csrf={csrf_val}&org_name=Acme"
        )))
        .unwrap();
    let resp_valid = app.oneshot(req_valid).await.unwrap();
    assert_eq!(resp_valid.status(), StatusCode::OK);

    // With CSRF explicitly disabled, the same handler renders its success page.
    let req_post = Request::builder()
        .method("POST")
        .uri("/settings")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp_post = app_without_csrf().oneshot(req_post).await.unwrap();
    assert_eq!(resp_post.status(), StatusCode::OK);
    let bytes_post = axum::body::to_bytes(resp_post.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let html_post = String::from_utf8_lossy(&bytes_post);
    assert!(html_post.contains("Settings Saved Successfully"));
}
