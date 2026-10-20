use yav::prelude::*;

mod user_badge {
    use yav::prelude::*;
    yav_component!("tests/fixtures/UserBadge.yav");
}

mod modern_card {
    use yav::prelude::*;
    yav_component!("tests/fixtures/ModernCard.yav");
}

#[test]
fn test_include_rsx() {
    let title = "Titre de test";
    let desc = "Description de test";

    let node: Node = include_rsx!("tests/fixtures/Card.rsx");
    let html = render_to_string(&node);

    assert!(html.contains(r#"<div class="card p-4 rounded border">"#));
    assert!(html.contains(r#"<h3 class="text-lg font-bold">Titre de test</h3>"#));
    assert!(html.contains(r#"<p class="text-gray-600">Description de test</p>"#));
}

#[test]
fn test_yav_component() {
    let badge = user_badge::UserBadge::new(user_badge::Props {
        username: "Alexandre".to_string(),
        is_admin: true,
    });

    let html = render_to_string(&badge.render());

    assert!(html.contains(r#"<div class="user-badge flex items-center gap-2">"#));
    assert!(html.contains(r#"<span class="font-bold">Alexandre</span>"#));
    assert!(html.contains(r#"<span class="badge">Admin</span>"#));
}

#[test]
fn test_yav_component_as_node() {
    let badge = user_badge::UserBadge::new(user_badge::Props {
        username: "Sophie".to_string(),
        is_admin: false,
    });

    // Test IntoNode implementation
    let node: Node = badge.into_node();
    let html = render_to_string(&node);

    assert!(html.contains("Sophie"));
    assert!(html.contains("User"));
}

#[test]
fn test_modern_card_esm_imports_and_props() {
    // Tests ESM import { Button } from "yav/ui" and props { ... } syntax
    let card = modern_card::ModernCard::new(modern_card::Props {
        title: "Projet Alpha".to_string(),
        count: 42,
        is_hot: true,
    });

    let html = render_to_string(&card.render());

    assert!(html.contains(r#"<div class="modern-card p-4 rounded-xl border">"#));
    assert!(html.contains(r#"<h3 class="text-xl font-bold">Projet Alpha</h3>"#));
    assert!(html.contains("Compteur: 42"));
    assert!(html.contains("Populaire"));
    assert!(html.contains("Voir détails"));
}

#[test]
fn test_modern_card_default_props() {
    // Tests Default implementation generated from props block defaults
    let card = modern_card::ModernCard::new(modern_card::Props {
        title: "Défaut".to_string(),
        ..Default::default()
    });

    assert_eq!(card.props.count, 10);
    assert!(card.props.is_hot);
}
