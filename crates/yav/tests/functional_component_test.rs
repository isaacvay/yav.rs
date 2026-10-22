use yav::prelude::*;

// 1. Functional component with individual arguments
#[component]
pub fn UserBadge(name: String, is_admin: bool) -> Node {
    rsx! {
        <div class="user-badge flex items-center gap-2">
            <span class="font-bold">{name}</span>
            {if is_admin {
                rsx! { <span class="bg-red-500 text-white text-xs px-2 py-0.5 rounded">Admin</span> }
            } else {
                rsx! { <span class="text-xs text-gray-500">Membre</span> }
            }}
        </div>
    }
}

// 2. Functional component with manual props struct
#[derive(Clone, Default)]
pub struct StatCardProps {
    pub label: String,
    pub value: u32,
    pub is_positive: bool,
}

#[component]
pub fn StatCard(props: StatCardProps) -> Node {
    rsx! {
        <div class="stat-card p-4 rounded-xl border">
            <p class="text-sm text-gray-500">{props.label}</p>
            <h3 class="text-2xl font-bold">{props.value}</h3>
            {if props.is_positive {
                rsx! { <span class="text-green-600">+ En hausse</span> }
            } else {
                rsx! { <span class="text-red-600">- En baisse</span> }
            }}
        </div>
    }
}

// 3. Component that accepts children
#[component]
pub fn Panel(title: String, children: Vec<Node>) -> Node {
    rsx! {
        <div class="panel border rounded-lg p-4">
            <h2 class="font-bold border-b pb-2">{title}</h2>
            <div class="panel-body mt-2">
                {children}
            </div>
        </div>
    }
}

#[test]
fn test_raw_text_in_rsx() {
    let count = 42;
    let node = rsx! {
        <div class="container">
            <h1>Bienvenue sur Yav</h1>
            <p>Compteur : {count}</p>
            <button class="btn btn-primary">Voir détails</button>
        </div>
    };

    let html = render_to_string(&node);
    assert!(html.contains("<h1>Bienvenue sur Yav</h1>"));
    assert!(html.contains("<p>Compteur: 42</p>"));
    assert!(html.contains("<button class=\"btn btn-primary\">Voir détails</button>"));
}

#[test]
fn test_component_invocation_in_rsx() {
    let node = rsx! {
        <div class="team-list">
            <UserBadge name="Alice" is_admin=true />
            <UserBadge name="Bob" is_admin=false />
        </div>
    };

    let html = render_to_string(&node);
    assert!(html.contains("Alice"));
    assert!(html.contains("Admin"));
    assert!(html.contains("Bob"));
    assert!(html.contains("Membre"));
}

#[test]
fn test_manual_props_component_invocation() {
    let node = rsx! {
        <div class="grid grid-cols-2 gap-4">
            <StatCard label="Ventes" value=150 is_positive=true />
            <StatCard label="Retours" value=12 is_positive=false />
        </div>
    };

    let html = render_to_string(&node);
    assert!(html.contains("Ventes"));
    assert!(html.contains("150"));
    assert!(html.contains("+ En hausse"));
    assert!(html.contains("Retours"));
    assert!(html.contains("12"));
    assert!(html.contains("- En baisse"));
}

#[test]
fn test_component_with_children() {
    let node = rsx! {
        <Panel title="Mon Panneau">
            <p class="text-sm">Contenu textuel de test</p>
            <UserBadge name="Charlie" is_admin=true />
        </Panel>
    };

    let html = render_to_string(&node);
    assert!(html.contains("<h2 class=\"font-bold border-b pb-2\">Mon Panneau</h2>"));
    assert!(html.contains("Contenu textuel de test"));
    assert!(html.contains("Charlie"));
    assert!(html.contains("Admin"));
}

#[test]
fn test_dynamic_boolean_and_optional_attributes_in_rsx() {
    let is_disabled = false;
    let is_required = true;
    let maybe_id: Option<&str> = Some("email-field");
    let missing_title: Option<String> = None;

    let node = rsx! {
        <input
            r#type="email"
            id={maybe_id}
            title={missing_title}
            disabled={is_disabled}
            required={is_required}
        />
    };

    let html = render_to_string(&node);
    assert!(html.contains(r#"type="email""#));
    assert!(!html.contains("r#type"));
    assert!(html.contains(r#"id="email-field""#));
    assert!(!html.contains("title="));
    assert!(!html.contains("disabled"));
    assert!(html.contains("required"));
}

#[test]
fn test_yav_ui_and_module_qualified_components_in_rsx() {
    let node = rsx! {
        <Container size="lg">
            <yav::ui::Button variant="primary" label="Save" />
            <Input name="username" r#type="text" placeholder="Enter username" required=true />
            <Checkbox name="terms" label="Accept terms" checked=true />
            <Switch name="notifications" label="Notifications" checked=true />
            <Badge label="Active" variant="success" />
            <Icon name="heart" class="text-red-500" />
        </Container>
    };

    let html = render_to_string(&node);
    assert!(html.contains("max-w-screen-lg"));
    assert!(html.contains("Save"));
    assert!(html.contains(r#"name="username""#));
    assert!(html.contains(r#"placeholder="Enter username""#));
    assert!(html.contains("Accept terms"));
    assert!(html.contains("Notifications"));
    assert!(html.contains("Active"));
    assert!(html.contains("<svg"));
    assert!(html.contains("text-red-500"));
}
