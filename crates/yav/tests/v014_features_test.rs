use yav::prelude::*;

#[test]
fn test_raw_script_and_style_rsx() {
    let view = rsx! {
        <div>
            <style>
                ".btn { background-color: #0b0b0d; font-size: 14px; }"
            </style>
            <script>
                "function toggle() { const x = 10; if (x > 5) { console.log('ok'); } }"
            </script>
        </div>
    };
    let html = render_to_string(&view);
    assert!(html.contains(".btn { background-color: #0b0b0d; font-size: 14px; }"));
    assert!(html.contains("function toggle() { const x = 10; if (x > 5) { console.log('ok'); } }"));
}

#[test]
fn test_short_boolean_attributes() {
    let view = rsx! {
        <input type="text" disabled required data-nav />
    };
    let html = render_to_string(&view);
    assert!(html.contains("disabled"));
    assert!(html.contains("required"));
    assert!(html.contains("data-nav"));
}

#[test]
fn test_head_hoisting_and_document_shell() {
    let view = rsx! {
        <main>
            <Head>
                <title>"My Dogfooding Page"</title>
                <meta name="description" content="Yav v0.1.4 release" />
                <link rel="canonical" href="https://yav.rs" />
            </Head>
            <h1>"Content"</h1>
        </main>
    };
    let body = render_to_string(&view);
    let doc = document_shell_with_lang("Default Title", "", &body, "", "fr");
    assert!(doc.contains("<title>My Dogfooding Page</title>"));
    assert!(doc.contains(r#"<meta name="description" content="Yav v0.1.4 release" />"#));
    assert!(doc.contains(r#"<link rel="canonical" href="https://yav.rs" />"#));
    assert!(!doc.contains("data-yav-head"));
    assert!(doc.contains(r#"<html lang="fr">"#));
    assert!(doc.contains(r#"data-yav-toggle"#) || doc.contains(r#"yav:toggle"#));
}

#[test]
fn test_t_macro_fallback() {
    let s = t!("welcome.message");
    assert_eq!(s, "welcome.message");
}

#[test]
fn test_url_for_reverse_routing() {
    let u1 = yav::router::url_for("/users/:id", &[("id", "42")]);
    assert_eq!(u1, "/users/42");

    let u2 = yav::router::url_for("/docs/*slug", &[("slug", "getting-started/quickstart")]);
    assert_eq!(u2, "/docs/getting-started/quickstart");

    let u3 = yav::router::url_for("/posts/[year]/[slug]", &[("year", "2026"), ("slug", "announcement")]);
    assert_eq!(u3, "/posts/2026/announcement");
}

#[test]
fn test_micro_actions_features() {
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("dialog[open]"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("role=\"dialog\""));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("role=\"menu\""));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("ArrowDown"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("ArrowUp"));
}

#[derive(Debug)]
struct TestLogin {
    email: String,
}

impl FromFormData for TestLogin {
    fn from_form(form: &FormData) -> Result<Self, FormErrors> {
        let mut errors = FormErrors::new();
        let email = form.get_or_empty("email");
        errors.require("email", email, "Email is required");
        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(Self { email: email.to_string() })
        }
    }
}

#[test]
fn test_from_form_data() {
    let mut map = std::collections::HashMap::new();
    map.insert("email".to_string(), "admin@yav.rs".to_string());
    let form = FormData(map);

    let parsed = form.parse_validated::<TestLogin>();
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().email, "admin@yav.rs");

    let empty_form = FormData::new();
    let err = empty_form.parse_validated::<TestLogin>();
    assert!(err.is_err());
    assert_eq!(err.unwrap_err().first("email"), Some("Email is required"));
}

#[test]
fn test_style_pseudo_classes_and_script_dynamic_expr() {
    let dynamic_js = "window.APP_VERSION = '0.1.4';";
    let view = rsx! {
        <div>
            <style>
                .btn:hover { color: #ff6b00; }
            </style>
            <script>{dynamic_js}</script>
        </div>
    };
    let html = render_to_string(&view);
    assert!(
        html.contains(".btn:hover"),
        "Pseudo-class should not have space after colon: {html}"
    );
    assert!(html.contains("window.APP_VERSION = '0.1.4';"));
}

#[test]
fn test_code_block_lifetimes_inline_comments_and_line_num() {
    let snippet = "pub fn borrow<'a>(s: &'static str) -> &'a str { s } // lifetime test";
    let view = rsx! {
        <CodeBlock language="rust" line_numbers=true>
            {snippet}
        </CodeBlock>
    };
    let html = render_to_string(&view);
    assert!(html.contains("data-line-num"));
    assert!(html.contains("// lifetime test"));
    assert!(html.contains("&#x27;a"));
    assert!(html.contains("&#x27;static"));
    assert!(html.contains("text-amber-300 italic"));
}

#[test]
fn test_i18n_xss_safe_json_escaping() {
    let i18n = I18n::new("en").add_translations(
        "en",
        [("xss.test", "</script><script>alert(1)</script>")],
    );

    let json = i18n.to_json();
    assert!(
        !json.contains("</script>"),
        "I18n JSON must escape forward slashes to prevent </script> breakout: {json}"
    );
    assert!(json.contains(r#"<\/script>"#));
}

#[test]
fn test_csrf_token_entropy_and_verification() {
    let s1 = yav::server::Session::new();
    let s2 = yav::server::Session::new();
    let t1 = CsrfToken::from_session(&s1);
    let t2 = CsrfToken::from_session(&s2);
    // CSPRNG per-token salt ensures two tokens for distinct sessions are distinct
    assert_ne!(t1.as_str(), t2.as_str());
    assert!(t1.verify_token(t1.as_str()));
    assert!(!t1.verify_token("invalid-token"));
}

#[test]
fn test_html_page_auto_locale_and_zero_clone_head() {
    let mut ctx = yav_core::context::ContextMap::new();
    ctx.insert("fr".to_string());

    let html = yav_core::context::with_context_map(ctx, || {
        let view = rsx! {
            <div>
                <Head>
                    <title>"Titre Dynamique"</title>
                    <meta name="theme-color" content="#0b0b0d" />
                </Head>
                <p>"Bonjour"</p>
            </div>
        };
        HtmlPage::new(view).render()
    });

    assert!(html.contains(r#"<html lang="fr">"#));
    assert!(html.contains("<title>Titre Dynamique</title>"));
    assert!(html.contains(r##"<meta name="theme-color" content="#0b0b0d" />"##));
    assert!(!html.contains("data-yav-head"));
}

#[tokio::test]
async fn test_route_with_layout_and_static_export() {
    fn root_layout(child: Node) -> Node {
        rsx! {
            <div class="layout-shell">
                {child}
            </div>
        }
    }

    async fn index_page() -> HtmlPage {
        HtmlPage::new(rsx! { <h1>"Exported Home"</h1> }).title("Home")
    }

    let app = YavApp::new().route_with_layout(
        "/",
        Method::GET,
        index_page,
        root_layout,
    );
    let out_dir = std::env::temp_dir().join("yav_test_static_export_v014");
    let _ = std::fs::remove_dir_all(&out_dir);

    let exported = app
        .export_static(&out_dir, "https://yav.rs")
        .await
        .expect("Static export should succeed");
    assert_eq!(exported, 1);

    let index_html = std::fs::read_to_string(out_dir.join("index.html")).unwrap();
    assert!(index_html.contains("layout-shell"));
    assert!(index_html.contains("Exported Home"));

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_native_if_and_for_in_rsx() {
    let is_pro = true;
    let items = ["Axum", "Tokio", "Tailwind"];

    let view = rsx! {
        <section class="features">
            if is_pro {
                <span class="badge-pro">"Pro Plan"</span>
            } else {
                <span class="badge-free">"Free Plan"</span>
            }
            <ul>
                for item in items.iter() {
                    <li>{*item}</li>
                }
            </ul>
        </section>
    };

    let html = render_to_string(&view);
    assert!(html.contains(r#"<span class="badge-pro">Pro Plan</span>"#));
    assert!(!html.contains("Free Plan"));
    assert!(html.contains("<li>Axum</li><li>Tokio</li><li>Tailwind</li>"));
}

#[test]
fn test_class_directives_and_static_subtree_folding() {
    let is_active = true;
    let is_disabled = false;

    let view = rsx! {
        <div class="wrapper">
            <header class="static-header">
                <h1>"Static Blueprint"</h1>
            </header>
            <button
                class="btn"
                class:active={is_active}
                class:opacity-50={is_disabled}
                class:border-oxide=true
            >
                "Action"
            </button>
        </div>
    };

    // Verify that the static <header> child was constant-folded into Node::Static at compile time
    if let Node::Element(ref el) = view {
        assert!(matches!(el.children.first(), Some(Node::Static(_))));
    } else {
        panic!("Root should be Node::Element");
    }

    let html = render_to_string(&view);
    assert!(html.contains(r#"<header class="static-header"><h1>Static Blueprint</h1></header>"#));
    assert!(html.contains(r#"class="btn active border-oxide""#));
    assert!(!html.contains("opacity-50"));
}

#[test]
fn test_multi_theme_system_and_theme_selector_rsx() {
    let view = rsx! {
        <div>
            { Theme::inject_all_themes_css() }
            <ThemeSelector default_theme="editorial-blueprint" id="my-theme-selector" />
        </div>
    };

    let html = render_to_string(&view);
    assert!(html.contains("data-yav-themes=\"all\""));
    assert!(html.contains(":root[data-theme=\"editorial-blueprint\"]"));
    assert!(html.contains(":root[data-theme=\"obsidian-minimal\"]"));
    assert!(html.contains(":root[data-theme=\"nordic-paper\"]"));
    assert!(html.contains(":root[data-theme=\"cyber-terminal\"]"));
    assert!(html.contains(":root[data-theme=\"solarized-lab\"]"));
    assert!(html.contains(":root[data-theme=\"glass-aurora\"]"));
    assert!(html.contains("--oxide: #e0592f;"));
    assert!(html.contains("id=\"my-theme-selector\""));
}

#[test]
fn test_declarative_ajax_boost_and_prefetch_runtime() {
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:post"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:swap"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:boost"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("yav:prefetch"));
    assert!(YAV_MICRO_ACTIONS_SCRIPT.contains("startViewTransition"));
}

#[tokio::test]
async fn test_app_state_use_state_and_cached_routes() {
    #[derive(Clone)]
    struct AppConfig {
        site_name: String,
    }

    let app = YavApp::new()
        .state(AppConfig {
            site_name: "Yav Editorial".to_string(),
        })
        .get("/state", || async {
            let cfg = use_state::<AppConfig>().expect("AppConfig should be in SSR state");
            HtmlPage::new(rsx! { <h1>{cfg.site_name}</h1> })
        })
        .get_cached("/cached", std::time::Duration::from_secs(60), || async {
            HtmlPage::new(rsx! { <p>"Cached Page Content"</p> })
        });

    let out_dir = std::env::temp_dir().join("yav_test_state_and_cache_v014");
    let _ = std::fs::remove_dir_all(&out_dir);

    let exported = app
        .export_static(&out_dir, "https://yav.rs")
        .await
        .expect("Static export should succeed");
    assert_eq!(exported, 2);

    let state_html = std::fs::read_to_string(out_dir.join("state").join("index.html")).unwrap();
    assert!(state_html.contains("<h1>Yav Editorial</h1>"));

    let cached_html = std::fs::read_to_string(out_dir.join("cached").join("index.html")).unwrap();
    assert!(cached_html.contains("<p>Cached Page Content</p>"));
    assert_eq!(app.ssr_cache.len(), 1);

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_markdown_engine_and_ws_broadcast() {
    let md = parse_markdown("---\ntitle: Architecture\nslug: arch\n---\n# Architecture\nHello **Rust** `0.1.4`.");
    assert_eq!(md.title, "Architecture");
    assert_eq!(md.slug, "arch");
    assert_eq!(md.headings.len(), 1);
    assert!(md.html.contains("<strong>Rust</strong>"));

    let hub = WsBroadcast::new(8);
    let mut rx = hub.subscribe();
    let badge = rsx! { <span class="live">"Online"</span> };
    hub.send_swap("#status", "innerHTML", &badge);
    let msg = rx.try_recv().unwrap();
    assert!(msg.contains("#status"));
    assert!(msg.contains("Online"));
}

#[test]
fn test_t_macro_with_interpolation() {
    let i18n = I18n::new("fr").add_translation("fr", "greeting", "Bonjour {name} !");
    let rendered = provide_context(i18n, || {
        t!("greeting", "name" => "Alexandre")
    });
    assert_eq!(rendered, "Bonjour Alexandre !");
}

#[test]
fn test_ssr_tree_shaking_cursor_motion_and_signals() {
    // 1. Plain static page -> 0 optional runtimes injected
    let plain_doc = HtmlPage::new(rsx! { <div><h1>"Simple Page"</h1></div> }).render();
    assert!(plain_doc.contains("id=\"yav-micro-actions\""));
    assert!(!plain_doc.contains("id=\"yav-cursor-runtime\""));
    assert!(!plain_doc.contains("id=\"yav-motion-runtime\""));
    assert!(!plain_doc.contains("id=\"yav-signals-runtime\""));

    // 2. Page with CursorSpotlight / TiltPlate / Magnetic -> injects cursor runtime only
    let cursor_doc = HtmlPage::new(rsx! {
        <div>
            <CursorSpotlight radius_px={450} class="plate p-6">
                <TiltPlate max_deg={8.0}>
                    <Magnetic strength={0.35}>
                        <button>"Hover me"</button>
                    </Magnetic>
                </TiltPlate>
            </CursorSpotlight>
            <CursorFollower smoothing={0.2} />
        </div>
    })
    .render();
    assert!(cursor_doc.contains("id=\"yav-cursor-runtime\""));
    assert!(cursor_doc.contains("--yav-mouse-x"));
    assert!(!cursor_doc.contains("id=\"yav-signals-runtime\""));

    // 3. Page with Motion / CountUp / AnimatedBar / TypewriterCode -> injects motion runtime
    let motion_doc = HtmlPage::new(rsx! {
        <div>
            <div
                yav:initial="opacity:0, y:24"
                yav:animate="opacity:1, y:0"
                yav:hover="scale:1.02, y:-4"
                yav:tap="scale:0.97"
                yav:spring="320, 18, 1"
            >
                <CountUp to={124500.0} duration_ms={1400} />
                <AnimatedBar target_pct={97.3} class="hbar us" />
                <TypewriterCode speed_ms={14}>
                    <div><span class="ln">"1"</span>"fn main() {}"</div>
                </TypewriterCode>
            </div>
        </div>
    })
    .render();
    assert!(motion_doc.contains("id=\"yav-motion-runtime\""));
    assert!(motion_doc.contains("solveSpring"));
    // Anti-FOUC compile-time style from yav:initial
    assert!(motion_doc.contains("style=\"opacity: 0; transform: translate3d(0px, 24px, 0);\""));

    // 4. Page with declarative reactive signals -> injects signals runtime
    let signals_doc = HtmlPage::new(rsx! {
        <div yav:state="count: 42">
            <button yav:on-click="count += 1">"+"</button>
            <span yav:text="count">"42"</span>
        </div>
    })
    .render();
    assert!(signals_doc.contains("id=\"yav-signals-runtime\""));
}



