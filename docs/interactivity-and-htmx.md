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
