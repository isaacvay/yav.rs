//! Yav SaaS Dashboard demonstration application entrypoint.

mod layouts;
mod routes;

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    // Configure and start YavApp with auto_routes!
    let app = yav::server::YavApp::new().static_dir("public");
    let app = yav::auto_routes!(app);

    println!(
        "🚀 Yav SaaS Dashboard running at http://{}:{}",
        app.host, app.port
    );
    app.serve().await.unwrap();
}
