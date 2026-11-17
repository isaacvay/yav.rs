# Interactivity & HTMX Integration

Yav provides first-class support for **HTMX**, enabling rich client-side interactivity, live search, tabs, modal dialogs, and instant form submissions without complex client-side JavaScript frameworks.

---

## 1. Using HTMX Attributes in RSX

All standard HTMX attributes (`hx-get`, `hx-post`, `hx-target`, `hx-swap`, `hx-trigger`, `hx-indicator`, etc.) can be written directly on elements in `rsx!`:

```rust
use yav::prelude::*;

#[component]
pub fn CounterButton(count: u32) -> Node {
    rsx! {
        <div id="counter-widget" class="p-4 border rounded-xl flex items-center gap-4">
            <span class="text-2xl font-bold">Total: {count}</span>
            <button 
                hx-post="/api/counter/increment"
                hx-target="#counter-widget"
                hx-swap="outerHTML"
                class="px-4 py-2 bg-primary text-white rounded-lg hover:opacity-90 transition"
            >
                +1 Incrémenter
            </button>
        </div>
    }
}
```

---

## 2. Returning Partial HTML with `HtmlFragment`

When responding to an HTMX request, you typically only want to return the targeted component rather than the entire document shell (`<!DOCTYPE html>`, `<head>`, etc.).

Use `HtmlFragment` to return lightweight partials:

```rust
use yav::prelude::*;

pub async fn increment_counter() -> HtmlFragment {
    let new_count = 43; // fetched from state or DB

    HtmlFragment::new(rsx! {
        <CounterButton count={new_count} />
    })
    // Optional: trigger client-side events or push new URL
    .hx_trigger("counterUpdated")
    .hx_push_url("/dashboard?updated=true")
}
```

### HTMX Response Modifiers:
- `.hx_trigger("eventName")` — triggers custom DOM events on the client.
- `.hx_redirect("/path")` — forces a client-side redirect.
- `.hx_push_url("/new-url")` — pushes a new URL into the browser history.
- `.hx_reswap("outerHTML")` — overrides the swap strategy requested by the element.
- `.hx_retarget("#other-id")` — redirects the swap target to another element.

---

## 3. Detecting HTMX Requests with `HtmxRequest`

Use the `HtmxRequest` extractor in your handlers to return full pages for normal browser visits, and partials for HTMX interactions:

```rust
use yav::prelude::*;

pub async fn user_profile(htmx: HtmxRequest) -> impl IntoResponse {
    let user_card = rsx! {
        <div class="card p-6">
            <h2>Profil Utilisateur</h2>
            <p>Contenu dynamique du profil</p>
        </div>
    };

    if htmx.is_htmx {
        // Return only the fragment for HTMX modal or tab swap
        HtmlFragment::new(user_card).into_response()
    } else {
        // Return complete page with navigation and head for direct URL navigation
        HtmlPage::new(user_card).title("Profil").into_response()
    }
}
```

---

## 4. Native Zero-JS Micro-Actions

Yav v0.1.5 introduces a ultra-compact (<2KB), zero-dependency client micro-runtime embedded in `yav-core`. You write declarative `yav:*` attributes directly on HTML tags in `rsx!` without writing a single line of JavaScript.

### A. Popovers, Drawers, Modals & Menus
Toggle elements, close on outside clicks, and automatically dismiss on window resize:

```rust
rsx! {
    <div>
        // Trigger button
        <button 
            yav:toggle="mobile-drawer" 
            yav:group="nav-popovers"
            class="btn"
        >
            "Menu"
        </button>

        // Drawer dialog
        <dialog 
            id="mobile-drawer" 
            yav:click-outside="close" 
            yav:close-on-resize="768"
            class="drawer-content"
        >
            <nav>
                <a href="/dashboard">"Dashboard"</a>
                <a href="/settings">"Paramètres"</a>
            </nav>
        </dialog>
    </div>
}
```
* **Accessibility**: Automatically manages `aria-expanded` and keyboard navigation (`Escape` closes open popovers, `ArrowUp`/`ArrowDown` navigates menu items).
* **Exclusive groups**: Elements sharing the same `yav:group="name"` ensure only one popover in the group can be open at a time.

### B. Global Hotkeys (`yav:hotkey`)
Bind global keyboard shortcuts to open modals, trigger searches, or navigate:

```rust
rsx! {
    // Pressing Cmd+K (Mac) or Ctrl+K (Windows/Linux) opens the command palette
    <button yav:toggle="search-modal" yav:hotkey="cmd+k" class="search-trigger">
        <span>"Rechercher..."</span>
        <kbd>"⌘K"</kbd>
    </button>
}
```

### C. Copy to Clipboard with Feedback (`yav:copy`)
Native clipboard copying with instantaneous button feedback and toast notifications:

```rust
rsx! {
    <button
        yav:copy="cargo install yav-cli"
        yav:copy-label="Copié !"
        yav:toast="Commande copiée dans le presse-papiers"
        class="copy-btn"
    >
        "cargo install yav-cli"
    </button>
}
```

### D. Multi-Pane Synchronized Tabs (`yav:tab-group`)
Tabs with active states and synchronized panel visibility:

```rust
rsx! {
    <div yav:tab-group="code-tabs">
        <div class="tabs-header">
            <button yav:tab="rust" class="active">"Rust"</button>
            <button yav:tab="html">"HTML"</button>
        </div>
        <div class="tabs-body">
            <div yav:tab-panel="rust">
                <pre>"fn main() {}"</pre>
            </div>
            <div yav:tab-panel="html" hidden>
                <pre>"<h1>Hello</h1>"</pre>
            </div>
        </div>
    </div>
}
```

### E. Declarative Client-Side Search & Filter
Instant text search and category filtering with zero server requests:

```rust
rsx! {
    <div>
        // Search input targeting cards
        <input 
            type="search" 
            yav:filter="input" 
            yav:filter-target=".project-card" 
            yav:filter-empty="#no-results"
            placeholder="Filtrer les projets..." 
        />

        <div class="projects">
            <div class="project-card" data-filter="Rust Web Framework">"Yav"</div>
            <div class="project-card" data-filter="Tailwind CSS">"Styles"</div>
        </div>

        <p id="no-results" hidden class="text-gray-500">
            "Aucun projet trouvé."
        </p>
    </div>
}
```

---

## 5. Declarative Micro-Signals (`yav:state`)

Build lightweight reactive calculators, dynamic gauges, or live text counters declaratively:

```rust
rsx! {
    <div yav:state="count: 0, max: 100" class="calculator p-6 border rounded-xl">
        <div class="flex items-center gap-4">
            <button yav:on-click="count = Math.max(0, count - 1)">"-"</button>
            <span yav:text="count" class="font-bold text-xl">"0"</span>
            <button yav:on-click="count = Math.min(max, count + 1)">"+"</button>
        </div>

        // Dynamic progress bar width
        <div class="track h-2 bg-gray-800 rounded mt-4">
            <div yav:style-width="(count / max * 100) + '%'" class="h-full bg-orange-500 rounded" />
        </div>
    </div>
}
```

---

## 6. Real-Time WebSockets Live Swap (`WsBroadcast`)

Push server-rendered HTML fragments to connected clients over WebSockets in real time:

```rust
use yav::server::ws::WsBroadcast;
use yav::prelude::*;

// 1. Create a broadcast hub (e.g. stored in AppState)
let hub = WsBroadcast::new(64);

// 2. Client subscribes via WebSocket endpoint
// When an event occurs on the server, push a targeted HTML swap:
let alert_badge = rsx! {
    <span class="badge bg-green-500 text-white animate-pulse">
        "Nouveau message reçu"
    </span>
};

hub.send_swap("#notification-area", "innerHTML", &alert_badge);
```

On the client, the micro-runtime receives the payload and replaces `#notification-area` smoothly with zero custom WebSocket handler code!

---

## 7. SSR Tree-Shaking Runtimes

Yav analyzes your rendered HTML page during SSR and **only injects the runtimes actually used by the components on that page**:

| Runtime | Size | Triggers |
|---------|------|----------|
| `yav-micro-actions` | ~2.1 KB | `yav:toggle`, `yav:copy`, `yav:filter`, `yav:hotkey`, `yav:tab`, etc. |
| `yav-cursor-runtime` | ~1.3 KB | `yav:cursor`, `CursorSpotlight`, `TiltPlate`, `Magnetic` |
| `yav-motion-runtime` | ~1.8 KB | `yav:animate`, `yav:reveal`, `CountUp`, `AnimatedBar`, `TypewriterCode` |
| `yav-signals-runtime` | ~0.9 KB | `yav:state`, `yav:bind`, `yav:text`, `yav:on-click` |

If a static page uses no interactive attributes, **none** of these runtimes are loaded, achieving absolute zero JavaScript overhead.

---

## 8. Server Actions (`#[server]`), `#[derive(FromFormData)]` & `#[derive(ApiSchema)]`

Inspired by Next.js `"use server"` and Leptos `#[server]`, Yav provides compile-time procedural macros that eliminate manual boilerplate for forms, RPC endpoints, and schema definitions.

### A. Procedural Server Action Macro (`#[server]`)
Transforms an asynchronous Rust function into both a directly callable server function and a mounted HTTP `POST` endpoint:

```rust
use yav::prelude::*;

#[derive(FromFormData, ApiSchema)]
pub struct CreateArticleForm {
    #[form(required = "Le titre est requis", min = 3)]
    pub title: String,
    #[form(required = "Le contenu est requis")]
    pub content: String,
    pub published: bool,
}

// Generates:
// 1. pub async fn create_article(input: CreateArticleForm) -> HtmlFragment
// 2. pub const fn create_article_url() -> &'static str ("/_actions/create-article")
// 3. pub fn create_article_handler() -> BoxedHandler
// 4. pub fn register_create_article(app: YavApp) -> YavApp
#[server("/_actions/create-article")]
pub async fn create_article(input: CreateArticleForm) -> HtmlFragment {
    // Business logic executed safely on the server
    HtmlFragment::new(rsx! {
        <div class="alert alert-success">
            "Article créé : " {input.title}
        </div>
    })
}
```

*When used with `auto_routes!(app)`, any function decorated with `#[server]` is **automatically registered** on the HTTP server with zero manual configuration.*

### B. Declarative Validation (`#[derive(FromFormData)]`)
Eliminates manual validation code. Supported field attributes:
- `#[form(required = "Message d'erreur")]`
- `#[form(email = "Format email invalide")]`
- `#[form(min = 6, message = "Au moins 6 caractères requis")]`
- `#[form(max = 120, message = "Au maximum 120 caractères")]`
- Checkboxes / Booleans automatically parsed from `"true"`, `"1"`, `"on"`, `"yes"`.

### C. OpenAPI & TypeScript Schema Generation (`#[derive(ApiSchema)]`)
Enables automatic discovery and documentation of your API payload types for OpenAPI 3.1 specifications (`yav codegen --openapi`) and TypeScript client types (`yav codegen --ts`).

---

## 9. Multi-Format Body Extraction in `FormData`

`FormData` in Yav automatically inspects the incoming request's `Content-Type` and decodes the payload transparently:

1. **`application/json`**: Directly parses JSON objects into field mappings (`FormData::from_json_bytes`). Numbers, booleans, and nested values are preserved as strings for seamless struct validation.
2. **`multipart/form-data; boundary=...`**: Parses multipart boundary parts (`FormData::from_multipart_bytes`), extracting form fields and text parts without external dependencies.
3. **`application/x-www-form-urlencoded`**: Standard form submission decoding.

Any endpoint extracting `form: FormData` can therefore accept JSON API calls, standard HTML forms, and multipart uploads simultaneously without code changes.

---

## 10. Fine-Grained Reactive Signals (`Signal<T>`, `Computed<T>`, `SignalScope`)

For fine-grained reactive state on the server and SSR rendering, `yav-core` includes a signals system inspired by Solid.js:

```rust
use yav_core::signals::{create_signal, create_memo, SignalScope};

// 1. Create a reactive signal
let (count, set_count) = create_signal(10);

// 2. Create a memoized derived computation
let double_count = create_memo(move || count.get() * 2);

assert_eq!(count.get(), 10);
assert_eq!(double_count.get(), 20);

// 3. Mutate the signal
set_count.set(25);
assert_eq!(count.get(), 25);
assert_eq!(double_count.get(), 50);

// 4. Render a reactive signal scope into the VDOM
let scope = SignalScope::new(move || {
    rsx! {
        <div class="counter-display">
            <span>"Valeur actuelle : " {count.get().to_string()}</span>
            <span>"Double : " {double_count.get().to_string()}</span>
        </div>
    }
});
```

---

## 11. Non-Destructive DOM Morphing (`window.__yavMorph`)

Yav includes a fast, standalone DOM morphing algorithm (`window.__yavMorph(from, to)`) exposed on the client:
- **Preserves Focus & Selection**: Active `<input>`, `<textarea>`, and `<select>` elements retain their focus, cursor position, and text selection during swaps.
- **Preserves Unsaved User Input**: If an input is currently being edited, its active value is preserved instead of being overwritten by server updates.
- **Element Preservation Directives (`yav:preserve` / `data-yav-preserve`)**: Any DOM subtree marked with `yav:preserve` (e.g. video players, canvas elements, rich text editors) is skipped during morphing, preventing reset or flicker.

---

## 12. Real-Time Collaboration, CRDTs & Multiplayer Presence (`yav-core::collab`)

Build multiplayer, real-time collaborative applications with zero external dependencies:

### A. Conflict-Free Replicated Data Types (CRDTs)
- **`LwwRegister<T>`**: Last-Write-Wins scalar register with monotonic hybrid logical clocks (`CrdtClock`).
- **`LwwMap<K, V>`**: Commutative, associative, and idempotent key-value map with distributed conflict resolution.

```rust
use yav_core::collab::{LwwMap, CrdtClock};

let mut map_a = LwwMap::new();
let mut map_b = LwwMap::new();

let c1 = CrdtClock::new("node-1", 100);
let c2 = CrdtClock::new("node-2", 200);

map_a.set("title", "Document Draft A", c1);
map_b.set("title", "Document Draft B", c2);

// Merge maps deterministically regardless of arrival order:
map_a.merge(&map_b);
assert_eq!(map_a.get("title"), Some(&"Document Draft B"));
```

### B. Multiplayer Presence Tracking (`PresenceRoom`)
Track active users, live cursor coordinates `(x, y)`, and current selections across distributed collaboration rooms:

```rust
use yav_core::collab::{PresenceRoom, CollaboratorPresence};

let mut room = PresenceRoom::new("doc-42");
room.update_collaborator(
    CollaboratorPresence::new("user-1", "Alice")
        .cursor(240.0, 150.0)
        .selection(10, 45)
        .color("#6366f1"),
);

assert_eq!(room.active_count(), 1);
```

### C. Fractional Indexing (`LexoRank`)
Enables `$O(1)$` card and row reordering without recalculating indices or touching existing items:

```rust
use yav_core::collab::LexoRank;

let rank_a = LexoRank::new("0|i00000:");
let rank_b = LexoRank::new("0|i00008:");
let rank_between = rank_a.between(&rank_b);

assert!(rank_a < rank_between);
assert!(rank_between < rank_b);
```

---

## 13. Deterministic Statecharts & Saga Workflows (`yav-core::state_machine`)

Model complex business workflows, multi-step checkouts, and distributed transactions with mathematical guarantees:

### A. Finite State Machine (`StateMachine<S, E, C>`)
Enforces valid state transitions with conditional guards, action hooks, and transition history logging:

```rust
use yav_core::state_machine::{StateMachine, TransitionOutcome};

let mut sm = StateMachine::new("CartEmpty")
    .transition("CartEmpty", "ADD_ITEM", "CartNotEmpty")
    .guard("CartNotEmpty", "CHECKOUT", "CheckingOut", |ctx: &OrderContext| ctx.total > 0.0)
    .transition("CheckingOut", "PAYMENT_SUCCESS", "Paid");

let outcome = sm.send("ADD_ITEM");
assert_eq!(sm.current_state(), "CartNotEmpty");
```

### B. Saga Workflows with Compensating Rollbacks (`SagaWorkflow<C>`)
Executes sequential multi-step distributed operations (e.g. Reserve Inventory -> Charge Card -> Provision Account) with **automatic reverse-order rollback** if any intermediate step fails:

```rust
use yav_core::state_machine::{SagaWorkflow, SagaOutcome};

let mut saga = SagaWorkflow::new("order-fulfillment")
    .step(
        "reserve_inventory",
        |ctx| { ctx.reserved = true; Ok(()) },
        |ctx| { ctx.reserved = false; } // Compensating rollback
    )
    .step(
        "process_payment",
        |ctx| { Err("Insufficient funds".into()) }, // Fails here
        |ctx| { ctx.refunded = true; }
    );

let outcome = saga.execute(&mut context);
assert!(matches!(outcome, SagaOutcome::RolledBack { .. }));
assert_eq!(context.reserved, false); // Automatically rolled back!
```
