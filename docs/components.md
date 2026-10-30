# Components Guide

## Overview

Yav components are functions or structs that produce `Node` trees. There are two
ways to create components:

### 1. Functional Components (`#[component]` like React / Next.js)

The most ergonomic way to build components in Yav is with the `#[component]` attribute macro. Just like in React or Next.js, components are simply functions:

#### A. Multiple Arguments (Auto-Generated Props)
```rust
use yav::prelude::*;

#[component]
pub fn UserBadge(name: String, is_admin: bool) -> Node {
    rsx! {
        <div class="user-badge flex items-center gap-2">
            <span class="font-bold">{name}</span>
            {if is_admin {
                rsx! { <span class="badge bg-red-500 text-white">Admin</span> }
            } else {
                rsx! { <span class="badge text-gray-500">Membre</span> }
            }}
        </div>
    }
}
```

You can invoke it directly in other components using JSX-like capitalized tags:
```rust
rsx! {
    <div class="team">
        <UserBadge name="Alice" is_admin=true />
        <UserBadge name="Bob" is_admin=false />
    </div>
}
```

#### B. Manual Props Struct
```rust
use yav::prelude::*;

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
        </div>
    }
}
```

#### C. Components with Children
```rust
use yav::prelude::*;

#[component]
pub fn Panel(title: String, children: Vec<Node>) -> Node {
    rsx! {
        <div class="panel border rounded-lg p-4">
            <h2 class="font-bold">{title}</h2>
            <div class="panel-body mt-2">
                {children}
            </div>
        </div>
    }
}

// Usage:
rsx! {
    <Panel title="Mon Panneau">
        <p>Contenu enfant directement imbriqué</p>
    </Panel>
}
```

#### D. Raw HTML Text in RSX (No Quotes Needed!)
Just like in JSX/HTML, you don't need quotes around regular text:
```rust
rsx! {
    <div class="container">
        <h1>Bienvenue sur Yav</h1>
        <p>Compteur: {count}</p>
        <button class="btn btn-primary">Voir détails</button>
    </div>
}
```

#### E. Native `if` and `for` Control Flow (No Closure Hack Required)
In Yav v0.1.4, you write standard Rust control flow directly inside `rsx!` without needing curly braces or `.iter().map(...)` closures:

```rust
let is_logged_in = true;
let roles = ["Admin", "Maintainer", "Author"];

rsx! {
    <div class="user-panel">
        if is_logged_in {
            <span class="status online">"Connecté"</span>
        } else {
            <a href="/login" class="btn">"Se connecter"</a>
        }

        <ul class="role-list">
            for role in roles.iter() {
                <li class="badge">{*role}</li>
            }
        </ul>
    </div>
}
```

#### F. Short Boolean Attributes
HTML boolean attributes can be written without `={true}`:

```rust
rsx! {
    <input type="text" disabled required autofocus data-nav />
    <button disabled>Chargement...</button>
}
```

#### G. Class Directives (`class:<name>={bool}`)
Conditionally toggle CSS utility classes cleanly without string concatenation:

```rust
let is_active = true;
let is_disabled = false;

rsx! {
    <button
        class="btn py-2 px-4 rounded"
        class:active={is_active}
        class:opacity-50={is_disabled}
        class:border-oxide=true
    >
        "Action"
    </button>
}
```
If `is_active` is `true`, it automatically merges into `class="btn py-2 px-4 rounded active border-oxide"`.

#### H. Zero-Cost Compile-Time Static Subtree Folding
Subtrees in `rsx!` that contain only static tags and text are pre-rendered into `Node::Static` at compile time:

```rust
rsx! {
    <div class="app">
        <header class="navbar p-4 border-b">
            <h1>"Application Yav"</h1>
            <p>"Pre-folded at compile-time into static memory"</p>
        </header>
        <main>{dynamic_content}</main>
    </div>
}
```
The `<header>` block is evaluated at compile time, eliminating VDOM node construction and string allocations during SSR requests!

#### I. `<Head>` Hoisting
Declare page titles, canonical links, and Open Graph tags anywhere in your component tree:

```rust
rsx! {
    <main>
        <Head>
            <title>"Page de Paramètres | Yav"</title>
            <meta name="description" content="Gérez votre compte et préférences." />
            <link rel="canonical" href="https://yav.rs/settings" />
        </Head>
        <h1>"Paramètres"</h1>
    </main>
}
```
Yav hoists all `<Head>` tags into the `<head>` of the final document shell during SSR with zero allocations.

### 2. Struct Components (with Component trait)

```rust
use yav::prelude::*;

struct UserCard {
    name: String,
    email: String,
    avatar_url: Option<String>,
}

impl Component for UserCard {
    fn render(&self) -> Node {
        Node::Element(
            Element::new("div")
                .attr("class", "user-card rounded-lg border p-4")
                .child(Node::Element(
                    Element::new("h3")
                        .attr("class", "text-lg font-semibold")
                        .child(Node::text(&self.name))
                ))
                .child(Node::Element(
                    Element::new("p")
                        .attr("class", "text-gray-600")
                        .child(Node::text(&self.email))
                ))
        )
    }
}
```

### 3. File-Based Components (`.yavml` / `.yav` / `.rsx` like JSX/TSX)

For a clean separation of concerns and editor syntax highlighting, you can author components in dedicated `.yavml` (or `.yav` / `.rsx`) files using either modern ESM-style syntax (`import` + `export default function`) or frontmatter (`--- ... ---`).

#### Option A: Modern `.yavml` ESM Syntax (`src/components/ModernCard.yavml`)

```tsx
import { Badge, Button } from "yav/ui";

export default function ModernCard(
    title: String,
    subtitle: String = "Default subtitle".to_string(),
    active: bool = true,
) {
    <div class="p-6 rounded-xl border bg-slate-900 text-white">
        <div class="flex items-center justify-between">
            <h3 class="text-xl font-bold">{title}</h3>
            {if active {
                rsx! { <Badge variant="success" label="Active" pill=true /> }
            } else {
                rsx! { <Badge variant="secondary" label="Archived" pill=true /> }
            }}
        </div>
        <p class="mt-2 text-slate-400">{subtitle}</p>
    </div>
}
```

#### Option B: Frontmatter `.yavml` / `.yav` Syntax (`src/components/UserBadge.yavml`)

```html
---
use yav::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Props {
    pub username: String,
    pub is_admin: bool,
}
---

<div class="user-badge flex items-center gap-2 p-2 rounded-lg bg-gray-50 border">
    <span class="font-bold text-gray-900">{props.username.clone()}</span>
    <span class="px-2 py-0.5 text-xs rounded bg-blue-100 text-blue-800">
        {if props.is_admin { "Admin" } else { "Membre" }}
    </span>
</div>
```

#### Use in Rust with `yav_component!`

```rust
use yav::prelude::*;

// Generates the UserBadge struct, Props, and Component implementation:
yav_component!("src/components/UserBadge.yavml");

fn my_view() -> Node {
    let badge = UserBadge::new(Props {
        username: "Alexandre".into(),
        is_admin: true,
    });

    rsx! {
        <div class="p-6">
            <h1>"Profil Utilisateur"</h1>
            {badge}
        </div>
    }
}
```

#### Step 3: Pure Template Inclusion with `include_rsx!` / `include_yav!`

If you just want to externalize a template and inject local variables:

```rust
// In your route handler:
let title = "Tableau de bord";
let count = 42;

// Automatically hot-reloads when template.rsx is edited on disk:
let page: Node = include_rsx!("templates/dashboard.rsx");
```


## RSX Syntax

### Elements
```rust
rsx! { <div class="container">"Content"</div> }
```

### Self-Closing Elements
```rust
rsx! { <input type="text" name="query" /> }
```

### Nested Elements
```rust
rsx! {
    <ul>
        <li>"Item 1"</li>
        <li>"Item 2"</li>
    </ul>
}
```

### Expressions
```rust
let name = "World";
rsx! { <p>{format!("Hello, {}!", name)}</p> }
```

### Conditional Rendering
```rust
let show = true;
let node = if show {
    rsx! { <p>"Visible"</p> }
} else {
    Node::empty()
};
```

### Iteration
```rust
let items = vec!["A", "B", "C"];
let list_items: Vec<Node> = items.iter()
    .map(|item| rsx! { <li>{item.to_string()}</li> })
    .collect();

let list = Node::Element(
    Element::new("ul").children(list_items)
);
```

### Boolean Attributes
```rust
rsx! { <input type="checkbox" checked disabled /> }
```

### Dynamic Attributes
```rust
let id = "main-content";
rsx! { <div id={id}>"Content"</div> }
```

### RSX Fragments (`<> ... </>`)
Group multiple siblings without introducing an unwanted wrapper `<div>`:
```rust
rsx! {
    <>
        <header>"Navigation"</header>
        <main>"Contenu"</main>
        <footer>"Pied de page"</footer>
    </>
}
```

### Directive & Namespaced Attributes (Colon Syntax)
RSX seamlessly parses colon-separated attributes for client directives, XML namespaces, and island hydration:
```rust
rsx! {
    <div client:load client:visible="true" xml:lang="fr">
        "Contenu avec directives de réhydratation"
    </div>
}
```


## Built-in UI Components

Yav provides pre-styled, accessible UI components with full support for both JSX-like `rsx!` tags and Rust builder patterns:

### Using with `rsx!` (JSX Syntax)
```rust
use yav::prelude::*;
use yav::ui::{Button, Card, CardHeader, CardTitle, CardContent, Alert, Badge};

rsx! {
    <Card class="max-w-md shadow-md">
        <CardHeader>
            <CardTitle text="Notifications" level=3 />
        </CardHeader>
        <CardContent>
            <Alert variant="info" message="You have 3 unread messages." />
            <div class="mt-4 flex gap-2">
                <Button variant="primary">"Mark as Read"</Button>
                <Badge variant="success" label="Active" pill=true />
            </div>
        </CardContent>
    </Card>
}
```

### Using with Builder API
```rust
use yav::ui::{button, card, CardTitle, alert, Variant};

let my_card = card()
    .child(CardTitle::new("Notifications").level(3))
    .child(alert().message("You have 3 unread messages."))
    .child(button("Mark as Read").variant(Variant::Primary))
    .render();
```

### High-Performance `<Image />` Component
Optimized image element with automatic lazy loading, async decoding, aspect-ratio preservation to prevent Cumulative Layout Shift (CLS), and responsive sizes:

```rust
use yav::ui::Image;

rsx! {
    // Standard responsive image
    <Image 
        src="/public/images/dashboard.webp" 
        alt="Dashboard Analytics Preview" 
        width=1200 
        height=675 
        aspect_ratio="16/9"
        class="rounded-xl shadow-lg border"
    />

    // Above-the-fold hero image with high priority
    <Image 
        src="/public/images/hero.webp" 
        alt="Hero banner" 
        priority=true 
    />
}
```

### Dark Mode & `<ThemeToggle />`
Seamless dark mode switcher with local storage persistence and anti-FOUC (Flash of Unstyled Content) protection:

```rust
use yav::ui::{ThemeToggle, anti_fouc_script};

// 1. Inject anti-FOUC script inside your HtmlPage document head:
let page = HtmlPage::new(body).head_extra(anti_fouc_script());

// 2. Insert the toggle button anywhere in your navbar or header:
rsx! {
    <header class="flex items-center justify-between p-4">
        <h1>"Mon Application"</h1>
        <ThemeToggle class="p-2 rounded-full" />
    </header>
}
```

### Form Fields with Validation (`<FormField />`)
Ergonomic form field combining `<label>`, `<input>`, help text, and accessible error message:

```rust
use yav::ui::{Form, FormField, Button};

rsx! {
    <Form action="/api/register" method="post" class="space-y-4 max-w-md">
        <FormField 
            label="Adresse Email" 
            name="email" 
            input_type="email" 
            placeholder="vous@domaine.com" 
            required=true 
            error={errors.first("email").map(str::to_string)}
        />
        <FormField 
            label="Mot de passe" 
            name="password" 
            input_type="password" 
            help="8 caractères minimum." 
            required=true 
        />
        <Button btn_type="submit" variant="primary">"Créer mon compte"</Button>
    </Form>
}
```

### Progressive Navigation `<Link />` & Prefetching
Turn navigation into fast, instant, client-side SPA-like page transitions without losing SSR accessibility:

```rust
use yav::ui::{Link, client_router_script};

// 1. In your document root or layout, include the client router:
rsx! {
    <body>
        <main>
            <Link href="/dashboard" prefetch=true class="text-blue-600 hover:underline">
                "Accéder au Dashboard"
            </Link>
        </main>
        {client_router_script()}
    </body>
}
```
* **Instant Hover Prefetching**: Automatically pre-loads target HTML in the background via `IntersectionObserver` or cursor hover.
* **Seamless DOM Swapping**: Replaces the active `<main>` element and updates browser history (`history.pushState`) with scroll restoration.
* **100% Graceful Fallback**: Functions as a standard HTML hyperlink when JavaScript is disabled.

### SEO & Social Metadata (`<MetaTags />` / `Metadata`)
Declaratively construct complete OpenGraph, Twitter Cards, canonical links, and Schema.org JSON-LD structured data:

```rust
use yav::ui::{Metadata, MetaTags, TwitterCard};

let meta = Metadata::new()
    .title("Yav - The High Performance Rust Web Framework")
    .description("Full-stack web framework combining Rust speed and modern ergonomics.")
    .canonical("https://yav.dev")
    .og_image("https://yav.dev/og-banner.png")
    .og_type("website")
    .twitter_card(TwitterCard::SummaryLargeImage)
    .twitter_site("@yavframework")
    .json_ld(r#"{"@context":"https://schema.org","@type":"WebSite","name":"Yav"}"#);

rsx! {
    <head>
        <MetaTags metadata={Some(meta)} />
    </head>
}
```

### Toast Notifications (`<Toast />` & `<ToastContainer />`)
Display accessible, animated feedback toasts for actions, alerts, and flash messages:

```rust
use yav::ui::{Toast, ToastContainer, ToastLevel, toast_script};

rsx! {
    <ToastContainer position="bottom-right">
        <Toast 
            level=ToastLevel::Success 
            title="Succès" 
            message="Vos modifications ont été enregistrées avec succès !" 
        />
    </ToastContainer>
    {toast_script()}
}
```

### Micro-Islands Web Components
Zero-overhead client-side interactivity powered by native browser Custom Elements (< 1.5 KB) without React or heavy JS runtimes:

```rust
use yav::ui::{Accordion, AccordionItem, Counter, web_components_script};

rsx! {
    <div class="space-y-6 max-w-lg">
        // Interactive counter
        <Counter initial=1 min=0 max=100 />

        // Accessible collapsible FAQ accordion
        <Accordion>
            <AccordionItem title="Pourquoi choisir Yav ?" open=true>
                "Yav combine la sécurité mémoire et la rapidité de Rust avec l'ergonomie de Next.js."
            </AccordionItem>
            <AccordionItem title="Comment fonctionne le streaming SSR ?">
                "Le premier chunk part en moins de 5ms, puis les blocs asynchrones remplacent les squelettes."
            </AccordionItem>
        </Accordion>
    </div>
    {web_components_script()}
}
```

### Motion & Animations (`yav-motion`)
Brings the ergonomics and fluid animations of **Framer Motion** directly to Rust and Server-Side Rendering.
Declaratively define entry animations, interactive gestures, viewport reveals, and spring physics with zero build-step overhead:

```rust
use yav::ui::{Motion, MotionGroup, MotionState, Transition, ScrollProgressBar, motion_script};

// 1. Single Element Motion
let hero_animation = Motion::new(rsx! {
    <h1 class="text-4xl font-extrabold text-white">"Bienvenue sur Yav"</h1>
})
.initial(MotionState::new().opacity(0.0).y(30.0).scale(0.95))
.animate(MotionState::new().opacity(1.0).y(0.0).scale(1.0))
.while_hover(MotionState::new().scale(1.03).y(-2.0))
.while_tap(MotionState::new().scale(0.97))
.transition(Transition::spring().stiffness(200.0).damping(18.0));

// 2. Staggered Group Animation (reveals as items scroll into view)
let mut cards = MotionGroup::new(vec![card1, card2, card3])
    .stagger(100)       // 100ms delay between each child
    .in_view(true);     // Trigger automatically via IntersectionObserver

rsx! {
    <div>
        // Reading scroll progress bar at the top of the viewport
        {ScrollProgressBar::new().color("#3b82f6").height(3)}

        <div class="hero">
            {hero_animation}
        </div>

        <div class="grid grid-cols-3 gap-6">
            {cards}
        </div>

        // Web Animations API client runtime (<2KB, zero npm dependencies)
        {motion_script()}
    </div>
}
```

#### Motion Properties & States:
* `MotionState`: `opacity`, `x`, `y`, `scale`, `rotate`, `blur`.
* `Transition`:
  * `Transition::spring()`: Realistic physics with `stiffness`, `damping`, and `mass`.
  * `Transition::tween()`: Smooth timeline transitions with `duration(ms)`, `delay(ms)`, and easing curve (`ease_in`, `ease_out`, `ease_in_out`).
* **Micro-Runtime**: `{motion_script()}` leverages the browser's native **Web Animations API** (WAAPI) for 60/120 FPS hardware-accelerated execution without external JavaScript libraries.

---

### Floating UI Primitives (`<Tooltip />` & `<Popover />`)
Accessible, collision-aware popups and tooltips with smart directional placement:

```rust
use yav::ui::{Tooltip, Popover, Placement, Button, floating_script};

rsx! {
    <div class="flex items-center gap-6 p-6">
        // Tooltip on hover/focus
        <Tooltip 
            placement=Placement::Top 
            delay_ms=150 
            trigger=Button::new("Survoler moi")
            content="Informations complémentaires utiles"
        />

        // Interactive popover modal on click
        <Popover 
            placement=Placement::Bottom 
            trigger=Button::new("Options")
            content=rsx! {
                <div class="p-4 space-y-2 w-64">
                    <h4 class="font-bold text-sm">Paramètres rapides</h4>
                    <p class="text-xs text-gray-500">Configurez vos préférences sans quitter la page.</p>
                </div>
            }
        />

        // Floating UI event listener (handles outside clicks and escape key)
        {floating_script()}
    </div>
}
```

---

### Sliding Drawer / Sheet (`<Drawer />`)
Accessible sliding panels anchored to the viewport edge (`Right`, `Left`, `Top`, `Bottom`) with backdrop blur and smooth transitions:

```rust
use yav::ui::{Drawer, DrawerSide, Button};

let drawer = Drawer::new("cart-drawer", "Panier d'achats", rsx! {
    <div class="space-y-4">
        <p class="text-sm text-gray-600">3 articles dans votre panier.</p>
        <Button variant="primary" class="w-full">"Passer la commande"</Button>
    </div>
})
.side(DrawerSide::Right)
.width("420px");

rsx! {
    <div>
        <button onclick="document.getElementById('cart-drawer').classList.remove('hidden')">
            "Voir le panier"
        </button>
        {drawer}
    </div>
}
```

---

### Command Palette (`<CommandMenu />` / `⌘K`)
Keyboard-first Spotlight / Command Menu with live fuzzy searching, group categories, keyboard navigation, and action callbacks:

```rust
use yav::ui::{CommandMenu, CommandItem, command_palette_script};

let menu = CommandMenu::new()
    .placeholder("Rechercher une action ou naviguer... (⌘K)")
    .item(
        CommandItem::new("home", "Tableau de bord")
            .icon("layout-dashboard")
            .shortcut("G D")
            .on_select("/dashboard")
    )
    .item(
        CommandItem::new("cars", "Consulter le catalogue")
            .icon("car")
            .shortcut("G C")
            .on_select("/catalogue")
    )
    .item(
        CommandItem::new("settings", "Paramètres du profil")
            .icon("settings")
            .shortcut("⌘ ,")
            .on_select("/settings")
    );

rsx! {
    <div>
        {menu}
        // Activates global ⌘K / Ctrl+K shortcut and navigation listeners
        {command_palette_script()}
    </div>
}
```

---

### Media Carousel (`<Carousel />`)
Touch-enabled, responsive content slider with auto-play, navigation arrows, and dot pagination:

```rust
use yav::ui::{Carousel, CarouselSlide, carousel_script};

let slides = vec![
    CarouselSlide::new(rsx! { <img src="/slide1.webp" alt="Slide 1" class="w-full h-80 object-cover rounded-xl" /> }),
    CarouselSlide::new(rsx! { <img src="/slide2.webp" alt="Slide 2" class="w-full h-80 object-cover rounded-xl" /> }),
    CarouselSlide::new(rsx! { <img src="/slide3.webp" alt="Slide 3" class="w-full h-80 object-cover rounded-xl" /> }),
];

let slider = Carousel::new(slides)
    .autoplay(6000)   // Transitions every 6s (pauses on hover)
    .arrows(true)
    .dots(true);

rsx! {
    <div>
        {slider}
        {carousel_script()}
    </div>
}
```

---

### Step-by-Step Flows (`<Stepper />`)
Display multi-step checkout, onboarding, or onboarding timelines with stateful steps:

```rust
use yav::ui::{Stepper, StepItem, StepStatus};

let steps = vec![
    StepItem::new("Coordonnées", "Informations personnelles").status(StepStatus::Completed),
    StepItem::new("Paiement", "Choix du mode de règlement").status(StepStatus::Active),
    StepItem::new("Confirmation", "Récapitulatif final").status(StepStatus::Upcoming),
];

rsx! {
    <div class="max-w-2xl mx-auto py-8">
        {Stepper::new(steps).vertical(false)}
    </div>
}
```

---

### Modern Form Controls (`<Switch />`, `<Slider />`, `<RangeSlider />`)
Accessible, styled input controls for rich forms and filter panels:

```rust
use yav::ui::{Switch, Slider, RangeSlider};

rsx! {
    <div class="space-y-6 max-w-md p-6 bg-white rounded-xl border">
        // Toggle Switch
        <Switch 
            name="newsletter" 
            checked=true 
            label="Recevoir nos newsletters promotionnelles" 
        />

        // Single Value Slider
        <Slider 
            name="radius" 
            label="Rayon de recherche (km)" 
            min=5.0 
            max=100.0 
            value=25.0 
            step=5.0 
        />

        // Dual-Thumb Range Slider
        <RangeSlider 
            name="price_range" 
            label="Fourchette de prix (€)" 
            min=0.0 
            max=1000.0 
            low=150.0 
            high=650.0 
        />
    </div>
}
```

---

### Feedback & Metrics (`<ProgressBar />`, `<EmptyState />`, `<StatCard />`)
Polished KPI analytics cards, data empty states, and dynamic progress bars:

```rust
use yav::ui::{ProgressBar, ProgressVariant, EmptyState, StatCard, StatTrend, Button};

rsx! {
    <div class="space-y-8">
        // KPI Stat Card with trend
        <div class="grid grid-cols-3 gap-4">
            <StatCard 
                title="Ventes Totales" 
                value="€48,250" 
                trend=StatTrend::up("+18.4%") 
                icon="trending-up" 
                subtext="vs mois précédent" 
            />
            <StatCard 
                title="Taux de Rebond" 
                value="24.1%" 
                trend=StatTrend::down("-3.2%") 
                icon="arrow-down-right" 
                subtext="amélioration" 
            />
        </div>

        // Animated Progress Bar
        <ProgressBar 
            value=72.0 
            variant=ProgressVariant::Primary 
            striped=true 
            animated=true 
            label=true 
        />

        // Zero-Data Empty State
        <EmptyState 
            title="Aucun véhicule disponible" 
            description="Modifiez vos filtres de recherche ou réinitialisez la sélection pour afficher des résultats." 
            icon="car" 
            action=Button::new("Réinitialiser les filtres").variant(yav::ui::Variant::Secondary) 
        />
    </div>
}
```

---

### Syntax Highlighting & Code (`<CodeBlock />`)
High-performance code container with line numbers, language tag, copy button, and syntax styling:

```rust
use yav::ui::CodeBlock;

let rust_code = r#"use yav::prelude::*;

#[component]
pub fn App() -> Node {
    rsx! { <h1 class="font-bold">"Bonjour Yav!"</h1> }
}"#;

rsx! {
    <CodeBlock language="rust" line_numbers=true class="my-6">
        {rust_code}
    </CodeBlock>
}
```

---

### Zero-JS Cursor & 3D Interactive Components

Interactive mouse-aware components driven entirely by native passive pointer tracking:

```rust
use yav::ui::{CursorSpotlight, TiltPlate, Magnetic, CursorFollower};

rsx! {
    <div class="space-y-6">
        // Radial spotlight glow following the cursor
        <CursorSpotlight radius_px={450} class="p-8 rounded-2xl border bg-zinc-950">
            // 3D Tilt perspective card with glare highlight
            <TiltPlate max_deg={10.0} glare=true class="p-6 rounded-xl border bg-zinc-900">
                <h3>"Carte 3D Interactive"</h3>
                <p>"S'incline dynamiquement sous le curseur."</p>

                // Magnetic button that snaps toward the cursor on hover
                <Magnetic strength={0.4}>
                    <button class="btn btn-primary mt-4">
                        "Bouton Magnétique"
                    </button>
                </Magnetic>
            </TiltPlate>
        </CursorSpotlight>

        // Smooth cursor follower orb
        <CursorFollower size_px={24} smoothing={0.18} />
    </div>
}
```

---

### Zero-JS Motion & Spring Physics (`<Motion />`, `<CountUp />`, `<AnimatedBar />`, `<TypewriterCode />`)

High-performance physical spring motion, gesture states, viewport scroll reveals, and count-ups without hand-written JavaScript:

```rust
use yav::ui::{Motion, MotionState, Transition, CountUp, AnimatedBar, TypewriterCode};

rsx! {
    <div class="space-y-8">
        // Physical Spring Motion with Gestures
        <Motion
            initial=MotionState::new().opacity(0.0).y(24.0)
            animate=MotionState::new().opacity(1.0).y(0.0)
            hover=MotionState::new().scale(1.03).y(-4.0)
            tap=MotionState::new().scale(0.97)
            transition=Transition::spring().stiffness(320.0).damping(18.0)
            class="p-6 bg-zinc-900 rounded-xl"
        >
            <h3>"Motion Card"</h3>
        </Motion>

        // Viewport-triggered animated numeric counter
        <div class="stat">
            <span class="text-4xl font-black">
                <CountUp to={124500.0} prefix="+" suffix=" req/s" duration_ms={1400} />
            </span>
        </div>

        // Viewport-triggered expanding progress gauge
        <AnimatedBar target_pct={98.4} duration_ms={1200} class="h-2 bg-zinc-800 rounded-full" />

        // Syntax-preserving typewriter code animation
        <TypewriterCode speed_ms={16} skip_on_click=true class="code-preview font-mono">
            <div><span class="text-purple-400">"pub fn "</span><span class="text-blue-400">"run_server"</span>"() {"</div>
            <div>"    println!(\"Serving with Yav\");"</div>
            <div>"}"</div>
        </TypewriterCode>
    </div>
}
```

---

### Theme Switcher (`<ThemeSelector />`)
Drop-in dropdown and switcher for the 8 built-in Yav design themes:

```rust
use yav::ui::{Theme, ThemeSelector};

rsx! {
    <header class="flex justify-between items-center p-4">
        <h1>"Mon Projet"</h1>
        <ThemeSelector default_theme="editorial-blueprint" />
    </header>
}
```

---

## Layouts

Use the `Layout` trait to create reusable page layouts:

```rust
struct AppLayout;

impl Layout for AppLayout {
    fn render_with(&self, children: Node) -> Node {
        Node::Element(
            Element::new("div")
                .attr("class", "min-h-screen")
                .child(/* navbar */)
                .child(Node::Element(
                    Element::new("main")
                        .attr("class", "container mx-auto p-4")
                        .child(children)
                ))
                .child(/* footer */)
        )
    }
}
```
