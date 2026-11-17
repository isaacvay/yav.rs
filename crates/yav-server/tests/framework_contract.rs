//! HTTP framework contract tests.
//!
//! Boots a real [`YavApp`], reduces it to an Axum `Router` via the public
//! `into_router()` method, and drives it through `axum_test::TestServer` —
//! in-process, with no TCP listener and no network access. The observable
//! response contract (status, exact body, content-type) is pinned with inline
//! `insta` snapshots so no external `.snap` file is needed.
//!
//! The second half property-checks the XSS escaping invariant of
//! `yav_core::escape_html`: for any input, no raw `<` or `>` may survive.

use axum::http::StatusCode;
use axum_test::TestServer;
use proptest::prelude::*;
use yav_server::{html_response, YavApp};

/// Build a real application and wrap its router in an in-process test server.
fn contract_server() -> TestServer {
    let app = YavApp::new()
        // Keep the representation deterministic: no ETag/304 layer in play.
        .etag(false)
        .get("/contract", |_req| async {
            html_response(StatusCode::OK, "<h1>Contract OK</h1>")
        });

    TestServer::new(app.into_router())
}

#[tokio::test]
async fn contract_route_returns_documented_response() {
    let server = contract_server();

    let response = server.get("/contract").await;

    // 1. Status must be 200.
    response.assert_status_ok();
    assert_eq!(response.status_code(), StatusCode::OK);

    // 2. Content-Type must be the HTML type the framework promises.
    let content_type = response.header("content-type");
    let content_type: &str = content_type
        .to_str()
        .expect("content-type is an ASCII header value");
    insta::assert_snapshot!(content_type, @r###"text/html; charset=utf-8"###);

    // 3. Exact body. The default nonce-based CSP pass rewrites only `<script>`
    //    tags, so this script-free HTML is returned byte-for-byte.
    insta::assert_snapshot!(response.text(), @r###"<h1>Contract OK</h1>"###);
}

proptest! {
    /// No matter what the input contains, `escape_html` must replace every
    /// angle bracket with an entity — a raw `<` or `>` would reopen markup.
    #[test]
    fn escape_html_never_emits_raw_angle_brackets(
        input in proptest::collection::vec(
            prop_oneof![
                Just('<'),
                Just('>'),
                Just('&'),
                Just('"'),
                Just('\''),
                any::<char>(),
            ],
            0..64,
        )
        .prop_map(|chars| chars.into_iter().collect::<String>()),
    ) {
        let escaped = yav_core::escape_html(&input);
        let rendered: &str = escaped.as_ref();

        prop_assert!(
            !rendered.contains('<'),
            "unescaped '<' for input {input:?}: {rendered:?}"
        );
        prop_assert!(
            !rendered.contains('>'),
            "unescaped '>' for input {input:?}: {rendered:?}"
        );
    }
}
