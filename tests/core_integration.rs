//! Integration tests for yav-core rendering

use yav_core::*;

#[test]
fn full_page_rendering() {
    let page = Node::Element(
        Element::new("main")
            .attr("class", "min-h-screen p-8")
            .child(Node::Element(
                Element::new("nav")
                    .attr("class", "flex items-center justify-between")
                    .child(Node::Element(
                        Element::new("a")
                            .attr("href", "/")
                            .attr("class", "font-bold")
                            .child(Node::text("yav")),
                    ))
                    .child(Node::Element(
                        Element::new("a")
                            .attr("href", "/docs")
                            .child(Node::text("Documentation")),
                    )),
            ))
            .child(Node::Element(
                Element::new("section")
                    .attr("class", "mx-auto mt-20 max-w-3xl")
                    .child(Node::Element(
                        Element::new("h1")
                            .attr("class", "text-5xl font-bold")
                            .child(Node::text("Build something great")),
                    ))
                    .child(Node::Element(
                        Element::new("p")
                            .attr("class", "mt-4 text-gray-600")
                            .child(Node::text("Build modern web applications with Rust.")),
                    )),
            )),
    );

    let html = render_to_string(&page);

    assert!(html.contains(r#"<main class="min-h-screen p-8">"#));
    assert!(html.contains(r#"<a href="/" class="font-bold">yav</a>"#));
    assert!(html.contains("<h1"));
    assert!(html.contains("Build something great"));
    assert!(html.contains("Build modern web applications with Rust."));
    assert!(html.ends_with("</main>"));
}

#[test]
fn document_shell_integration() {
    let body = render_to_string(&Node::Element(
        Element::new("div")
            .attr("id", "app")
            .child(Node::text("Hello")),
    ));

    let doc = document_shell(
        "My App",
        r#"<link rel="stylesheet" href="/styles/app.css" />"#,
        &body,
        "",
    );

    assert!(doc.starts_with("<!DOCTYPE html>"));
    assert!(doc.contains("<title>My App</title>"));
    assert!(doc.contains("app.css"));
    assert!(doc.contains(r#"<div id="app">Hello</div>"#));
    assert!(doc.contains("</html>"));
}

#[test]
fn xss_prevention_in_text() {
    let malicious = Node::Element(
        Element::new("div")
            .child(Node::text("<img src=x onerror=alert(1)>")),
    );
    let html = render_to_string(&malicious);
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
}

#[test]
fn xss_prevention_in_attributes() {
    let malicious = Node::Element(
        Element::new("div")
            .attr("title", r#"" onclick="alert(1)"#),
    );
    let html = render_to_string(&malicious);
    assert!(!html.contains(r#"onclick"#));
    assert!(html.contains("&quot;"));
}

#[test]
fn conditional_rendering() {
    let show_banner = true;
    let banner = if show_banner {
        Node::Element(
            Element::new("div")
                .attr("class", "banner")
                .child(Node::text("Welcome!")),
        )
    } else {
        Node::empty()
    };

    let html = render_to_string(&banner);
    assert!(html.contains("Welcome!"));

    let no_banner: Node = None::<String>.into_node();
    assert_eq!(render_to_string(&no_banner), "");
}

#[test]
fn iteration_rendering() {
    let items = vec!["Apple", "Banana", "Cherry"];
    let list = Node::Element(
        Element::new("ul").children(
            items
                .iter()
                .map(|item| {
                    Node::Element(
                        Element::new("li").child(Node::text(*item)),
                    )
                })
                .collect::<Vec<_>>(),
        ),
    );

    let html = render_to_string(&list);
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>Apple</li>"));
    assert!(html.contains("<li>Banana</li>"));
    assert!(html.contains("<li>Cherry</li>"));
    assert!(html.contains("</ul>"));
}

#[test]
fn component_trait_rendering() {
    struct Card {
        title: String,
        body: String,
    }

    impl Component for Card {
        fn render(&self) -> Node {
            Node::Element(
                Element::new("div")
                    .attr("class", "card")
                    .child(Node::Element(
                        Element::new("h2").child(Node::text(&self.title)),
                    ))
                    .child(Node::Element(
                        Element::new("p").child(Node::text(&self.body)),
                    )),
            )
        }
    }

    let card = Card {
        title: "Hello".to_string(),
        body: "World".to_string(),
    };
    let html = render_to_string(&card.render());
    assert!(html.contains(r#"<div class="card">"#));
    assert!(html.contains("<h2>Hello</h2>"));
    assert!(html.contains("<p>World</p>"));
}

#[test]
fn layout_composition() {
    struct MainLayout;

    impl Layout for MainLayout {
        fn render_with(&self, children: Node) -> Node {
            Node::Element(
                Element::new("div")
                    .attr("class", "layout")
                    .child(Node::Element(
                        Element::new("header")
                            .child(Node::text("Site Header")),
                    ))
                    .child(Node::Element(
                        Element::new("main").child(children),
                    ))
                    .child(Node::Element(
                        Element::new("footer")
                            .child(Node::text("Site Footer")),
                    )),
            )
        }
    }

    let layout = MainLayout;
    let page_content = Node::Element(
        Element::new("h1").child(Node::text("Home")),
    );
    let full_page = layout.render_with(page_content);
    let html = render_to_string(&full_page);

    assert!(html.contains("Site Header"));
    assert!(html.contains("<h1>Home</h1>"));
    assert!(html.contains("Site Footer"));
}

#[test]
fn form_rendering() {
    let form = Node::Element(
        Element::new("form")
            .attr("method", "post")
            .attr("action", "/login")
            .child(Node::Element(
                Element::new("label")
                    .attr("for", "email")
                    .child(Node::text("Email")),
            ))
            .child(Node::Element(
                Element::new("input")
                    .attr("type", "email")
                    .attr("id", "email")
                    .attr("name", "email")
                    .bool_attr("required", true),
            ))
            .child(Node::Element(
                Element::new("button")
                    .attr("type", "submit")
                    .child(Node::text("Sign In")),
            )),
    );

    let html = render_to_string(&form);
    assert!(html.contains(r#"<form method="post" action="/login">"#));
    assert!(html.contains(r#"<label for="email">Email</label>"#));
    assert!(html.contains(r#"<input type="email" id="email" name="email" required />"#));
    assert!(html.contains("<button type=\"submit\">Sign In</button>"));
}

#[test]
fn void_elements_no_children() {
    let nodes = Node::Fragment(vec![
        Node::Element(Element::new("br")),
        Node::Element(Element::new("hr")),
        Node::Element(
            Element::new("img")
                .attr("src", "/logo.png")
                .attr("alt", "Logo"),
        ),
        Node::Element(
            Element::new("meta")
                .attr("charset", "utf-8"),
        ),
    ]);

    let html = render_to_string(&nodes);
    assert!(html.contains("<br />"));
    assert!(html.contains("<hr />"));
    assert!(html.contains(r#"<img src="/logo.png" alt="Logo" />"#));
    assert!(html.contains(r#"<meta charset="utf-8" />"#));
    // Void elements should NOT have closing tags
    assert!(!html.contains("</br>"));
    assert!(!html.contains("</hr>"));
    assert!(!html.contains("</img>"));
}

#[test]
fn children_container() {
    let mut children = Children::new();
    children.push(Node::text("Hello "));
    children.push(Node::Element(
        Element::new("strong").child(Node::text("world")),
    ));

    let wrapper = Node::Element(
        Element::new("p").child(children.into_node()),
    );
    let html = render_to_string(&wrapper);
    assert_eq!(html, "<p>Hello <strong>world</strong></p>");
}

#[test]
fn deeply_nested_tree() {
    let depth = 20;
    let mut node = Node::text("leaf");
    for i in (0..depth).rev() {
        node = Node::Element(
            Element::new("div")
                .attr("data-level", &i.to_string())
                .child(node),
        );
    }
    let html = render_to_string(&node);
    assert!(html.starts_with(r#"<div data-level="0">"#));
    assert!(html.contains("leaf"));
    // Count closing tags
    let close_count = html.matches("</div>").count();
    assert_eq!(close_count, depth);
}
