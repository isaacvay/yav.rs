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

Yav v0.1.4 introduces a ultra-compact (<2KB), zero-dependency client micro-runtime embedded in `yav-core`. You write declarative `yav:*` attributes directly on HTML tags in `rsx!` without writing a single line of JavaScript.

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
