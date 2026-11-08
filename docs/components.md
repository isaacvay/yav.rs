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

### High-Performance Responsive Image (`<Image />`)

The `Image` component provides Next.js `next/image` parity in Rust: automatic width/height aspect-ratio reservation to eliminate Cumulative Layout Shift (CLS), responsive `srcset` generation, lazy/async loading, and seamless integration with the server-side image optimizer (`/_yav/image`):

- **Automatic Optimization (`.optimize(true)`)**: Automatically rewrites local asset paths to `/_yav/image?url=...&w=...&q=...` for on-the-fly server-side resizing, format negotiation, and caching.
- **Zero-CLS Blur Placeholder (`.blur_placeholder(true)`)**: Injects an instantaneous lightweight SVG Gaussian-blur LQIP (Low-Quality Image Placeholder) background data URI before the image finishes loading.
- **Custom Blur (`.blur_data_url("data:image/svg+xml;utf8,...")`)**: Allows specifying custom blurred image data.
- **Priority Loading (`.priority(true)`)**: Automatically sets `loading="eager"` and `fetchpriority="high"` for hero/above-the-fold banners.

#### RSX Declarative Syntax:
```rust
use yav::prelude::*;

rsx! {
    <div class="hero-media relative overflow-hidden rounded-2xl">
        <Image 
            src="/images/hero-banner.webp"
            alt="Plateforme Yav en production"
            width={1200}
            height={600}
            aspect_ratio="16/9"
            object_fit="cover"
            priority=true
            optimize=true
            quality={85}
            blur_placeholder=true
            class="shadow-2xl"
        />
    </div>
}
```

#### Builder Pattern Syntax:
```rust
use yav::ui::Image;

let hero_image = Image::new("/images/hero.jpg", "Hero banner")
    .dimensions(1200, 600)
    .aspect_ratio("16/9")
    .object_fit("cover")
    .priority(true)
    .optimize(true)
    .quality(90)
    .blur_placeholder(true)
    .render();
```

---

### Enterprise Charts, DataGrid & Complex Application Widgets

Yav includes server-rendered SVG charts, interactive SaaS components, and enterprise productivity widgets:

#### 1. Server-Rendered SVG Charts (`yav::ui::charts`)
Interactive SVG charts rendered 100% on the server with zero client JavaScript libraries:
- `LineChart`: Smooth cubic Bézier spline or linear line series with area gradient fills and interactive hover tooltips.
- `BarChart`: Grouped or standalone vertical categorical bar charts with auto-computed axis grids.
- `DonutChart`: Proportional pie/donut segments with central metric readout.
- `Sparkline`: Ultra-compact trendline for embedding inside KPI cards and tables.

```rust
use yav::ui::charts::{LineChart, BarChart, DonutChart, Sparkline, ChartSeries, DonutSegment};

// Line Chart
let line_chart = LineChart::new("monthly-revenue")
    .series(ChartSeries::new("MRR", vec![32000.0, 45000.0, 58000.0, 72000.0, 89000.0]).color("#6366f1"))
    .labels(vec!["Jan", "Fév", "Mar", "Avr", "Mai"])
    .height(280)
    .render();

// Donut Chart
let donut = DonutChart::new("traffic-sources")
    .segment(DonutSegment::new("Direct", 45.0, "#6366f1"))
    .segment(DonutSegment::new("Organic", 35.0, "#10b981"))
    .segment(DonutSegment::new("Referral", 20.0, "#f59e0b"))
    .render();
```

#### 2. Enterprise DataGrid (`yav::ui::datagrid`)
Feature-complete table capable of sorting, client/server search filtering, row selection checkboxes, and pagination:

```rust
use yav::ui::datagrid::{DataGrid, DataGridColumn, DataGridRow};

let grid = DataGrid::new("users-grid")
    .column(DataGridColumn::new("name", "Utilisateur").sortable(true))
    .column(DataGridColumn::new("role", "Rôle"))
    .column(DataGridColumn::new("status", "Statut").sortable(true))
    .row(DataGridRow::new("u1", vec!["Alice Martin".into(), "Admin".into(), "Actif".into()]))
    .row(DataGridRow::new("u2", vec!["Bob Dupont".into(), "Membre".into(), "En attente".into()]))
    .searchable(true)
    .paginate(10)
    .render();
```

#### 3. Kanban Board (`yav::ui::kanban`)
Multi-column drag-and-drop workflow board with automated `$O(1)$` fractional `LexoRank` card reordering and server mutation persistence (`yav:drop-url`):

```rust
use yav::ui::kanban::{KanbanBoard, KanbanColumn, KanbanCard};

let board = KanbanBoard::new("sprint-board")
    .drop_url("/api/kanban/move")
    .column(
        KanbanColumn::new("todo", "À Faire")
            .card(KanbanCard::new("ISS-1", "Moteur CRDT", "U").tag("core").assignee("@alice"))
            .card(KanbanCard::new("ISS-2", "Optimiseur d'images", "V").tag("media").assignee("@bob")),
    )
    .column(
        KanbanColumn::new("in_progress", "En Cours")
            .card(KanbanCard::new("ISS-3", "Tests E2E", "W").tag("qa").assignee("@charlie")),
    )
    .column(KanbanColumn::new("done", "Terminé"))
    .render();
```

#### 4. Node-Graph Workflow Canvas (`yav::ui::flow_canvas`)
Interactive visual programming DAG builder with interactive pointer dragging and **real-time cubic Bézier edge rerouting**:

```rust
use yav::ui::flow_canvas::{FlowCanvas, FlowNode, FlowEdge};

let canvas = FlowCanvas::new("pipeline-editor")
    .dimensions(960, 540)
    .node(FlowNode::new("n1", "Webhook Ingest", 40.0, 120.0).kind("input"))
    .node(FlowNode::new("n2", "LLM Processing Agent", 300.0, 120.0).kind("processor"))
    .node(FlowNode::new("n3", "Database Sink", 580.0, 120.0).kind("output"))
    .edge(FlowEdge::new("n1", "n2").label("Payload JSON").animated(true))
    .edge(FlowEdge::new("n2", "n3").label("Vector Embedding"))
    .render();
```
*Lors du déplacement d'un nœud, le runtime recalcule automatiquement les coordonnées des courbes de Bézier SVG `M x1 y1 C cx1 y1, cx2 y2, x2 y2` et recentre les étiquettes sans aucun décalage.*

#### 5. Virtualized List (`yav::ui::virtual_list`)
Smooth 60 FPS scrolling over 100,000+ items with calculated runway offset and dynamic chunk windowing:

```rust
use yav::ui::virtual_list::VirtualList;

let virtual_list = VirtualList::new("audit-log-scroller")
    .total_items(100_000)
    .row_height(48)
    .viewport_height(600)
    .fetch_url("/api/audit/page")
    .render();
```

#### 6. Rich Text Editor & AI Streaming Chat (`yav::ui::rich_editor`)
- **`RichTextEditor`**: Accessible WYSIWYG editor surface with toolbar commands (Bold, Italic, Headings, Code block) synchronizing directly to a hidden form input.
- **`AiChat`**: Multi-turn chat UI with user/assistant bubbles, auto-scroll, and streaming SSE fragment integration (`yav:ai-stream`).

```rust
use yav::ui::rich_editor::{RichTextEditor, AiChat, AiChatMessage};

let editor = RichTextEditor::new("article-body")
    .placeholder("Rédigez votre article en markdown ou texte riche...")
    .initial_html("<p>Contenu initial pré-rendu en SSR.</p>")
    .render();

let chat = AiChat::new("agent-assistant")
    .stream_url("/api/ai/stream")
    .message(AiChatMessage::user("Comment configurer un cluster PostgreSQL ?"))
    .message(AiChatMessage::assistant("Voici les étapes pour initialiser vos répliques..."))
    .render();
```

#### 7. Modern Form & Overlay Primitives (`yav::ui::widgets`)
Enterprise input and layout primitives:
- `DatePicker`: Calendar picker with localized dates and accessibility.
- `Combobox`: Filterable autocomplete dropdown with keyboard navigation.
- `InputOtp`: Multi-segment code input for 2FA / TOTP authentication.
- `FileDropzone`: Drag-and-drop file upload zone with file type and size validation.
- `Sheet`: Sliding slide-over drawer modal.
- `ResizablePanels`: Multi-pane resizable layout splitters (horizontal / vertical).
- `Kbd`: Keyboard shortcut badge (e.g. `⌘K`, `Ctrl+P`).

```rust
use yav::ui::widgets::{DatePicker, Combobox, ComboboxOption, InputOtp, FileDropzone, Sheet, Kbd};

rsx! {
    <div class="space-y-4">
        <InputOtp name="otp_code" length={6} />
        <FileDropzone name="avatar" max_size_mb={5} accept="image/png, image/jpeg" />
        <div class="flex items-center gap-2">
            <span>"Ouvrir la palette"</span>
            <Kbd>"⌘K"</Kbd>
        </div>
    </div>
}
```

#### 8. In-App DevTools Overlay (`yav::ui::devtools`)
A zero-dependency floating HUD for development inspections, showing active routes, registered routes, SSR latency in microseconds, cache status, and live Core Web Vitals telemetry.

```rust
use yav::ui::devtools::{DevTools, DevToolsProps};

rsx! {
    <DevTools
        props={DevToolsProps {
            active_route: "/dashboard".to_string(),
            total_routes: 42,
            ssr_duration_us: 420,
            cache_status: "HIT".to_string(),
            position: "bottom-right".to_string(),
        }}
    />
}
```

#### 9. Calendar & Appointment Scheduler (`yav::ui::calendar`)
Server-rendered responsive monthly calendar grids and time-slot booking interfaces with timezone support.

```rust
use yav::ui::calendar::{Calendar, CalendarEvent, CalendarProps, Scheduler, TimeSlot};

// Monthly Grid:
rsx! {
    <Calendar
        props={CalendarProps {
            month_label: "Mars 2026".to_string(),
            days_in_month: 31,
            first_day_offset: 6, // Dimanche
            events: vec![
                CalendarEvent::new(15, "10:00", "Lancement Produit").color("indigo"),
                CalendarEvent::new(24, "14:30", "Revue Architecture").color("emerald"),
            ],
            timezone: "Europe/Paris".to_string(),
        }}
    />
}

// Appointment Slot Picker:
rsx! {
    <Scheduler
        title="Consultation Technique".to_string()
        date_label="Mardi 17 Mars 2026".to_string()
        duration_minutes={45}
        slots={vec![
            TimeSlot::available("09:00"),
            TimeSlot::booked("10:00"),
            TimeSlot::available("11:00"),
            TimeSlot::available("14:00"),
        ]}
        booking_endpoint="/api/appointments/book".to_string()
    />
}
```

#### 10. Reactive Spreadsheet & Formula Engine (`yav::ui::spreadsheet`)
An Excel-like spreadsheet with interactive editing, cell formatting, CSV export, and a built-in formula evaluator supporting `=SUM(A1:A5)`, `=AVG(...)`, `=MIN(...)`, `=MAX(...)`, and arithmetic expressions.

```rust
use yav::ui::spreadsheet::Spreadsheet;

let mut sheet = Spreadsheet::new(8, 5)
    .with_title("Budget Prévisionnel Q1")
    .with_cell(0, 0, "Dépenses Cloud")
    .with_cell(0, 1, "1200")
    .with_cell(1, 0, "Salaires")
    .with_cell(1, 1, "8500")
    .with_cell(2, 0, "Total")
    .with_cell(2, 1, "=SUM(B1:B2)");

// Renders interactive table with dynamic formulas
rsx! {
    <div>{sheet.render()}</div>
}
```

#### 11. Git Code Diff Viewer & Custom Media Player (`yav::ui::media_diff`)
- `DiffViewer`: Side-by-side or unified line-by-line diff calculated via Longest Common Subsequence (LCS).
- `MediaPlayer`: Custom accessible HTML5 audio/video player with chapter markers and subtitle tracks.

```rust
use yav::ui::media_diff::{DiffViewer, MediaPlayer, MediaChapter, SubtitleTrack};

// Code Diff:
let old_code = "fn main() {\n    println!(\"Hello\");\n}";
let new_code = "fn main() {\n    println!(\"Hello Yav.rs!\");\n}";

rsx! {
    <DiffViewer
        old_text={old_code.to_string()}
        new_text={new_code.to_string()}
        old_filename="src/main.rs (v1)".to_string()
        new_filename="src/main.rs (v2)".to_string()
        unified=false
    />
}

// Media Player:
rsx! {
    <MediaPlayer
        src="https://cdn.example.com/videos/keynote.mp4".to_string()
        is_video=true
        title="Yav.rs 2.0 Keynote".to_string()
        chapters={vec![
            MediaChapter::new(0.0, "Introduction"),
            MediaChapter::new(120.0, "Architecture SSR & Zero-JS"),
            MediaChapter::new(360.0, "Benchmarks"),
        ]}
        subtitles={vec![
            SubtitleTrack::new("Français", "fr", "/subtitles/fr.vtt").default_track(),
            SubtitleTrack::new("English", "en", "/subtitles/en.vtt"),
        ]}
        autoplay=false
    />
}
```

#### 12. Web ANSI Terminal & Live Shell (`yav::ui::terminal`)
Emulate a high-performance terminal emulator directly in the browser with full ANSI 16-color SGR formatting support, live command prompt input, and real-time streaming capability.

```rust
use yav::ui::terminal::{Terminal, AnsiSpan};

// Simple embedded terminal with custom prompt and title
let terminal = Terminal::new("main-shell")
    .title("zsh — production-cluster-01")
    .prompt("root@yav-node-01:~$ ")
    .line("Initializing Yav kernel v0.1.4...")
    .line("\x1b[32m[OK]\x1b[0m Axum HTTP runtime bound to 0.0.0.0:3000")
    .line("\x1b[33m[WARN]\x1b[0m Storage cache sync pending")
    .height("450px");

rsx! {
    <div>{terminal.render()}</div>
}
```

#### 13. Dynamic Form Builder from JSON Schema (`yav::ui::form_builder`)
Generate completely typed, fully styled, and accessible HTML5 forms dynamically at runtime directly from JSON Schema or OpenAPI specifications.

```rust
use yav::ui::form_builder::DynamicForm;
use serde_json::json;

let user_schema = json!({
    "type": "object",
    "required": ["email", "plan"],
    "properties": {
        "email": {
            "type": "string",
            "format": "email",
            "title": "Email Address",
            "description": "We will never share your email."
        },
        "age": {
            "type": "integer",
            "minimum": 18,
            "maximum": 120,
            "title": "Age"
        },
        "plan": {
            "type": "string",
            "title": "Subscription Tier",
            "enum": ["hobby", "pro", "enterprise"]
        },
        "subscribe_newsletter": {
            "type": "boolean",
            "title": "Subscribe to developer updates"
        }
    }
});

let form = DynamicForm::from_json_schema("signup-form", "/api/v1/subscribe", &user_schema)
    .expect("Valid JSON Schema")
    .submit_label("Create Account");

rsx! {
    <div class="max-w-xl mx-auto p-6 bg-slate-900 rounded-2xl">
        {form.render()}
    </div>
}
```

#### 14. Dual-Pane Live Markdown Editor (`yav::ui::markdown_editor`)
Side-by-side or stacked split-view Markdown editor with live preview, word and line counts, and a complete formatting toolbar (Bold, Italic, Strikethrough, Headings, Code Blocks, Quotes, Bulleted & Numbered Lists, Task Lists, Links, Images, Tables).

```rust
use yav::ui::markdown_editor::MarkdownEditor;

let initial_doc = r#"# Product Roadmap 2026

- [x] High-performance Rust SSR
- [x] Zero external JS dependencies
- [ ] Multi-region live synchronization
"#;

let editor = MarkdownEditor::new("doc-editor", initial_doc)
    .placeholder("Write your technical specifications here...")
    .min_height("500px");

rsx! {
    <div class="w-full">
        {editor.render()}
    </div>
}
```

#### 15. Advanced SVG Visualizations: Heatmap, Sankey & Radar (`yav::ui::charts_advanced`)
Native vector SVG data visualizers rendered completely on the server with zero client-side charting libraries.

```rust
use yav::ui::charts_advanced::{
    Heatmap, HeatmapCell, HeatmapMonthLabel,
    SankeyChart, SankeyNode, SankeyLink,
    RadarChart, RadarAxis, RadarSeries
};

// 1. GitHub-style Activity Calendar Heatmap:
let heatmap = Heatmap::new("commit-heatmap")
    .data(vec![
        HeatmapCell::new("2026-01-01", 3),
        HeatmapCell::new("2026-01-02", 7),
        HeatmapCell::new("2026-01-03", 0),
        HeatmapCell::new("2026-01-04", 12),
    ])
    .color_palette(&["#0e4429", "#006d32", "#26a641", "#39d353"]);

// 2. Sankey Flow Ribbon Diagram:
let sankey = SankeyChart::new("traffic-funnel")
    .node(SankeyNode::new("ad", "Paid Ads"))
    .node(SankeyNode::new("org", "Organic Search"))
    .node(SankeyNode::new("land", "Landing Page"))
    .node(SankeyNode::new("conv", "Conversions"))
    .link(SankeyLink::new("ad", "land", 400.0).color("#6366f1"))
    .link(SankeyLink::new("org", "land", 600.0).color("#10b981"))
    .link(SankeyLink::new("land", "conv", 250.0).color("#f59e0b"));

// 3. Multi-Metric Spider / Radar Chart:
let radar = RadarChart::new("framework-comparison")
    .axis(RadarAxis::new("perf", "Throughput (req/s)", 100.0))
    .axis(RadarAxis::new("memory", "Memory Efficiency", 100.0))
    .axis(RadarAxis::new("type_safety", "Type Safety", 100.0))
    .axis(RadarAxis::new("security", "OWASP Security", 100.0))
    .axis(RadarAxis::new("bundle", "Zero Bundle Size", 100.0))
    .series(RadarSeries::new("Yav.rs", vec![99.0, 95.0, 100.0, 98.0, 100.0]).color("#6366f1"))
    .series(RadarSeries::new("Next.js", vec![45.0, 40.0, 75.0, 70.0, 30.0]).color("#94a3b8"));

rsx! {
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div>{heatmap.render()}</div>
        <div>{sankey.render()}</div>
        <div>{radar.render()}</div>
    </div>
}
```

---

## 40. Interactive Org Chart & Family Tree (`<OrgChart />`)

The [`OrgChart`](file:///d:/deep/yav.rs/crates/yav-ui/src/org_chart.rs) component renders interactive organization charts and family trees with orthogonal SVG connectors, department pills, and avatars:

```rust
use yav::prelude::*;

let chart = OrgChart::new()
    .id("team-tree")
    .orientation(OrgOrientation::Vertical)
    .node(OrgNode::new("ceo", "Sarah Connor", "Chief Executive Officer").department("Exec"))
    .node(OrgNode::new("cto", "John Connor", "Chief Technology Officer").parent("ceo").department("Tech"))
    .node(OrgNode::new("lead", "Kyle Reese", "Staff Engineer").parent("cto").department("Tech"));

rsx! {
    <div class="p-6">
        {chart.render()}
    </div>
}
```

---

## 41. Interactive Audio Waveform Visualizer (`<AudioWaveform />`)

The [`AudioWaveform`](file:///d:/deep/yav.rs/crates/yav-ui/src/audio_waveform.rs) component displays audio amplitude peaks with dual-tone progress clipping, playhead scrubbing, and timecode stamps:

```rust
use yav::prelude::*;

let waveform = AudioWaveform::new()
    .id("audio-sample")
    .audio_url("/audio/podcast_ep42.mp3")
    .duration(185.0)
    .current_time(42.5)
    .peaks(vec![0.2, 0.4, 0.8, 0.6, 0.9, 0.3, 0.7, 0.5])
    .controls(true);

rsx! {
    <div class="max-w-xl mx-auto p-4">
        {waveform.render()}
    </div>
}
```

---

## 42. Native OpenTelemetry Logtail Inspector (`<LogViewer />`)

The [`LogViewer`](file:///d:/deep/yav.rs/crates/yav-ui/src/log_viewer.rs) component provides a terminal-grade real-time log inspector with level filtering (`INFO`, `WARN`, `ERROR`), search queries, and distributed trace ID deep-links:

```rust
use yav::prelude::*;

let viewer = LogViewer::new()
    .id("live-logs")
    .log(LogEntry::new("2026-11-05 12:00:00", LogLevel::Info, "server::http", "Listening on :3000"))
    .log(LogEntry::new("2026-11-05 12:00:01", LogLevel::Warn, "db::pool", "High latency (45ms)"))
    .log(LogEntry::new("2026-11-05 12:00:02", LogLevel::Error, "auth::jwt", "Expired session token")
        .trace_id("4bf92f3577b34da6a3ce929d0e0e4736"))
    .height(400);

rsx! {
    <div class="p-4">
        {viewer.render()}
    </div>
}
```

---

## 43. Collaborative Whiteboard & Drawing Canvas (`<Whiteboard />`)

The [`Whiteboard`](file:///d:/deep/yav.rs/crates/yav-ui/src/whiteboard.rs) component provides a vector drawing canvas with Chaikin line smoothing, shapes, sticky notes, and multiplayer presence support:

```rust
use yav::prelude::*;

let whiteboard = Whiteboard::new()
    .id("design-canvas")
    .dimensions(960, 560)
    .element(WhiteboardElement::pen("stroke-1", vec![(10.0, 10.0), (50.0, 60.0), (120.0, 80.0)], "#4f46e5"))
    .element(WhiteboardElement::rect("box-1", 140.0, 60.0, 200.0, 100.0, "#059669"))
    .element(WhiteboardElement::sticky("note-1", 380.0, 80.0, "Brainstorm architecture here", "#fef08a"));

rsx! {
    <div class="flex justify-center p-6">
        {whiteboard.render()}
    </div>
}
```

---

## 44. Natural Language AI Filter Bar (`<AiFilterBar />`)

The [`AiFilterBar`](file:///d:/deep/yav.rs/crates/yav-ui/src/ai_filter_bar.rs) component translates natural language text into structured criteria chips and dispatches filters to a target `<DataGrid />` or `<Table />`:

```rust
use yav::prelude::*;

let filter_bar = AiFilterBar::new()
    .id("customer-filter")
    .placeholder("Filter: 'Active users with revenue > $500 from France'...")
    .target_grid("customers-grid")
    .filter(AiFilterToken::new("status", "=", "active"))
    .filter(AiFilterToken::new("country", "=", "France"));

rsx! {
    <div class="max-w-2xl mx-auto p-4">
        {filter_bar.render()}
    </div>
}
```

---

## 45. Zero-Overhead WebAssembly Client Islands (`<WasmIsland />`)

The [`WasmIsland`](file:///d:/deep/yav.rs/crates/yav-core/src/wasm.rs) component loads and hydrates compiled WebAssembly bytecode modules directly into DOM containers with zero JS bundler dependencies:

```rust
use yav::prelude::*;

let island = WasmIsland::new("3d-viewport", "/wasm/physics_sim.wasm", "start_simulation")
    .memory_pages(16)
    .prop("gravity", "9.81");

rsx! {
    <div class="w-full h-96">
        {island.render()}
    </div>
}
```

---

## 46. Slippy Tile Vector Map & Geospatial Data Visualizer (`<VectorMap />`)

The [`VectorMap`](file:///d:/deep/yav.rs/crates/yav-ui/src/vector_map.rs) component renders interactive Web Mercator projection maps with GPS pin clusters, choropleth country/state polygon boundaries, and hover popups without external JS tile libraries:

```rust
use yav::prelude::*;

let map = vector_map()
    .id("offices-map")
    .center(48.8566, 2.3522, 1.5)
    .pin(MapPin::new("paris", 48.8566, 2.3522, "Paris Global HQ").color("#ec4899"))
    .pin(MapPin::new("london", 51.5074, -0.1278, "London Hub").color("#6366f1"))
    .region(MapRegion::new(
        "france-zone",
        vec![(49.0, 1.0), (49.0, 3.0), (47.0, 3.0), (47.0, 1.0)],
    ).fill("rgba(99, 102, 241, 0.2)"));

rsx! {
    <div class="max-w-4xl mx-auto p-4">
        {map.render()}
    </div>
}
```

---

## 47. Interactive Gantt Chart & Project Roadmap (`<GanttChart />`)

The [`GanttChart`](file:///d:/deep/yav.rs/crates/yav-ui/src/gantt_chart.rs) component visualizes schedules, dependencies, milestone diamonds, and completion percentages in pure SVG and Tailwind CSS:

```rust
use yav::prelude::*;

let chart = gantt_chart()
    .id("q4-roadmap")
    .days(30)
    .task(GanttTask::new("t1", "Frontend UI Architecture", 0, 8).progress(0.85))
    .task(GanttTask::new("t2", "Backend Distributed Cache", 6, 10).progress(0.60).depends_on("t1"))
    .task(GanttTask::milestone("m1", "Production Release v0.2.0", 24));

rsx! {
    <div class="overflow-x-auto p-4">
        {chart.render()}
    </div>
}
```

---

## 48. Vector PDF Document Reader & Annotator (`<PdfViewer />`)

The [`PdfViewer`](file:///d:/deep/yav.rs/crates/yav-ui/src/pdf_viewer.rs) component embeds PDF documents with zero external JavaScript dependencies, offering page navigation, zoom toolbars, and SVG annotation highlights/sticky notes:

```rust
use yav::prelude::*;

let viewer = pdf_viewer("/documents/q4-financial-report.pdf")
    .id("report-viewer")
    .title("Q4 Financial Report")
    .pages(12, 1)
    .zoom(1.25)
    .annotation(PdfAnnotation::highlight(1, 40.0, 120.0, 350.0, 24.0))
    .annotation(PdfAnnotation::note(1, 40.0, 160.0, "Approved by Audit Committee"));

rsx! {
    <div class="max-w-5xl mx-auto p-6">
        {viewer.render()}
    </div>
}
```

---

## 49. Spotlight Guided Tour & Onboarding Walkthrough (`<SpotlightTour />`)

The [`SpotlightTour`](file:///d:/deep/yav.rs/crates/yav-ui/src/spotlight_tour.rs) component provides step-by-step product walkthroughs with dynamic dark cutout overlays, element spotlighting, keyboard shortcuts (`Esc`, arrows), and local storage persistence:

```rust
use yav::prelude::*;

let tour = spotlight_tour("onboarding-tour")
    .storage_key("app_tour_completed")
    .step(
        TourStep::new(
            "welcome",
            "#nav-brand",
            "Welcome to Yav!",
            "Explore your application with zero JS runtime overhead.",
        )
        .badge("New")
        .placement(TourPlacement::Bottom),
    )
    .step(
        TourStep::new(
            "metrics",
            "#metrics-button",
            "Real-Time Metrics",
            "Monitor live latency percentiles and cache throughput.",
        )
        .placement(TourPlacement::Top),
    );

rsx! {
    <div>
        {tour.render()}
    </div>
}
```

---

## 50. Voice Memo & Live Audio Recorder (`<MediaRecorder />`)

The [`MediaRecorder`](file:///d:/deep/yav.rs/crates/yav-ui/src/media_recorder.rs) component provides modern browser audio recording with animated frequency waveforms, timer indicators, playback preview, and direct upload API bindings:

```rust
use yav::prelude::*;

let recorder = media_recorder("customer-voice-memo")
    .label("Voice Feedback")
    .upload_url("/api/upload-feedback-audio")
    .max_duration(120)
    .format(AudioFormat::Mp4);

rsx! {
    <div class="max-w-md mx-auto p-4">
        {recorder.render()}
    </div>
}
```

---

## 51. Pure-Rust 3D Wireframe & Mesh Visualizer (`<ModelViewer3D />`)

The [`ModelViewer3D`](file:///d:/deep/yav.rs/crates/yav-ui/src/model_viewer_3d.rs) component provides pure-Rust 3D linear algebra (Euler rotations, perspective projections) rendered directly into interactive SVG wireframes and shaded polygon meshes with mouse-drag orbit:

```rust
use yav::prelude::*;

let viewer = model_viewer_3d("cube-3d")
    .mesh(ModelMesh3D::cube(100.0))
    .dimensions(480, 360)
    .rotation(35.0, -25.0, 0.0)
    .wireframe_color("#6366f1")
    .face_fill_color(Some("rgba(99, 102, 241, 0.15)".to_string()))
    .show_axes(true)
    .interactive_orbit(true);

rsx! {
    <div class="flex justify-center p-4">
        {viewer.render()}
    </div>
}
```

---

## 52. Interactive Code Sandbox & REPL (`<CodePlayground />`)

The [`CodePlayground`](file:///d:/deep/yav.rs/crates/yav-ui/src/code_playground.rs) component offers a split-pane live code editor with synchronized line numbers, tab indentation, copy button, Ctrl+Enter hotkey runner, and console output/remote API sandbox integration:

```rust
use yav::prelude::*;

let repl = code_playground("rust-playground")
    .title("Yav Live REPL")
    .language("rust")
    .code("fn main() {\n    println!(\"Hello from Yav!\");\n}")
    .run_endpoint("/api/run-code")
    .height(380);

rsx! {
    <div class="max-w-4xl mx-auto p-4">
        {repl.render()}
    </div>
}
```

---

## 53. JSON / YAML Tree Explorer & Collapsible Viewer (`<JsonTreeViewer />`)

The [`JsonTreeViewer`](file:///d:/deep/yav.rs/crates/yav-ui/src/json_tree.rs) component displays hierarchical, syntax-highlighted JSON data with collapsible nodes, copy-to-clipboard for values and JSONPaths, and item count badges:

```rust
use yav::prelude::*;

let json_data = serde_json::json!({
    "user": {
        "id": "usr_99",
        "name": "Alice Dupont",
        "roles": ["admin", "developer"],
        "preferences": {
            "theme": "dark",
            "notifications": true
        }
    },
    "active": true
});

let viewer = json_tree_viewer("app-state-inspector", &json_data)
    .max_depth(3)
    .show_copy_buttons(true)
    .show_line_numbers(true);

rsx! {
    <div class="max-w-2xl mx-auto p-4">
        {viewer.render()}
    </div>
}
```

---

## 54. Hexagonal Grid & Isometric Spatial Mapping (`<HexGrid />`)

The [`HexGrid`](file:///d:/deep/yav.rs/crates/yav-ui/src/hex_grid.rs) component provides pure-SVG pointy-topped hexagonal grids with axial coordinates, Manhattan distance calculation, custom terrain fill colors, and interactive cell click handlers:

```rust
use yav::prelude::*;

let tiles = vec![
    HexTile::new(HexCoord::new(0, 0))
        .fill_color("#22c55e")
        .label("Base")
        .tooltip("Main headquarters"),
    HexTile::new(HexCoord::new(1, 0))
        .fill_color("#3b82f6")
        .label("Water"),
    HexTile::new(HexCoord::new(0, 1))
        .fill_color("#eab308")
        .label("Desert"),
];

let grid = hex_grid("strategy-map", tiles)
    .hex_radius(45.0)
    .gap(3.0)
    .stroke_color("#334155")
    .interactive(true);

rsx! {
    <div class="overflow-auto p-4 flex justify-center">
        {grid.render()}
    </div>
}
```

---

## 55. Vector Touch & Mouse Handwritten Signature Pad (`<SignaturePad />`)

The [`SignaturePad`](file:///d:/deep/yav.rs/crates/yav-ui/src/signature_pad.rs) component renders a zero-JS canvas touch/mouse signature pad with configurable stroke color/width, interactive clear button, and base64 PNG data-url synchronization for seamless form integration:

```rust
use yav::prelude::*;

let pad = signature_pad("contract-signature", "tenant_signature")
    .stroke_color("#1e293b")
    .stroke_width(2.5)
    .dimensions(500, 200)
    .clear_label("Effacer la signature")
    .placeholder_text("Signez ici avec votre doigt ou la souris");

rsx! {
    <form method="POST" action="/api/contracts/sign" class="space-y-4">
        {pad.render()}
        <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded">
            "Valider le contrat"
        </button>
    </form>
}
```

---

## 56. Interactive Timeline & Audit Trail (`<Timeline />`)

The [`Timeline`](file:///d:/deep/yav.rs/crates/yav-ui/src/timeline.rs) component renders vertical chronological event histories with status icons (`Success`, `Warning`, `Error`, `Info`, `Pending`), connecting vertical lines, tags, and actor badges:

```rust
use yav::prelude::*;

let items = vec![
    TimelineItem::new("item-1", "Commande validée", "2026-11-05 09:30")
        .description("Paiement par carte bancaire vérifié.")
        .status(TimelineStatus::Success)
        .actor("Alice"),
    TimelineItem::new("item-2", "Préparation en entrepôt", "2026-11-05 11:15")
        .description("Colis en cours d'emballage.")
        .status(TimelineStatus::Info)
        .tag("Logistique"),
    TimelineItem::new("item-3", "Expédition transporteur", "2026-11-05 14:00")
        .status(TimelineStatus::Pending),
];

let history = timeline("order-history", items)
    .dense(false)
    .show_line(true);

rsx! {
    <div class="max-w-xl mx-auto p-4">
        {history.render()}
    </div>
}
```

---

## 57. Before / After Image Comparison Slider (`<ImageCompareSlider />`)

The [`ImageCompareSlider`](file:///d:/deep/yav.rs/crates/yav-ui/src/image_compare.rs) component provides interactive before-and-after image overlays with horizontal split, draggable divider handle, and custom labels:

```rust
use yav::prelude::*;

let slider = image_compare_slider(
    "model-comparison",
    "/assets/before-optimization.jpg",
    "/assets/after-optimization.jpg",
)
.initial_position(50.0)
.before_label("Avant (Original)")
.after_label("Après (WebP 80%)")
.aspect_ratio("16/9");

rsx! {
    <div class="max-w-3xl mx-auto p-4">
        {slider.render()}
    </div>
}
```

---

## 58. GDPR & ePrivacy Cookie Consent Preference Center (`<CookieConsent />`)

The [`CookieConsent`](file:///d:/deep/yav.rs/crates/yav-ui/src/cookie_consent.rs) component renders an accessible GDPR-compliant banner with granular category preferences (`necessary`, `analytics`, `marketing`), `localStorage` persistence, and instant DOM dispatch events:

```rust
use yav::prelude::*;

let consent = cookie_consent("gdpr-banner")
    .title("Respect de votre vie privée")
    .description("Nous utilisons des cookies pour optimiser votre expérience et mesurer l'audience.")
    .privacy_policy_url("/privacy-policy")
    .accept_all_label("Tout accepter")
    .reject_optional_label("Refuser les cookies optionnels")
    .save_preferences_label("Enregistrer mes choix")
    .storage_key("app_cookie_consent_v1");

rsx! {
    <div>
        {consent.render()}
    </div>
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
