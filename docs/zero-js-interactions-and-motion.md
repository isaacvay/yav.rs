# Zero-JS Declarative Interactivity & Motion Engine

Yav v0.1.4 introduces a groundbreaking architecture for rich modern web interfaces: **Zero-JS client interactivity powered by native Rust components and declarative HTML5 attributes**.

You can build drawers, modals, command palettes, 3D tilt cards, cursor-following spotlights, magnetic buttons, physical spring gestures, scroll-spy table-of-contents, animated count-ups, and reactive calculators **without writing a single line of JavaScript**.

---

## Architecture: How It Works

1. **Declarative Attributes**: You declare desired interactions directly in `rsx!` using `yav:*` attributes or type-safe Rust components from `yav::ui`.
2. **Compile-Time Anti-FOUC**: Initial styles (e.g. `yav:initial="opacity:0, y:20"`) are transformed at compile-time by `yav-macros` into inline CSS `style="opacity: 0; transform: translate3d(0px, 20px, 0);"`, guaranteeing **zero flash of unstyled content (FOUC)** before scripts load.
3. **Hardware-Accelerated Physics**: Motion is calculated with an exact harmonic oscillator solver (`solveSpring`) and executed directly on the browser's GPU compositor via the native **Web Animations API** (`Element.animate`).
4. **SSR Tree-Shaking**: The server inspects the rendered HTML page and **only injects the micro-runtimes actually required** by that page. A static page receives **0 bytes** of JavaScript.

---

## 1. Micro-Actions Reference

The micro-actions runtime (~2.1 KB) handles dialogs, menus, clipboard, tabs, filters, and keyboard shortcuts.

### Popovers, Menus & Drawers
```rust
rsx! {
    <div>
        // Toggle button with keyboard access
        <button 
            yav:toggle="mobile-menu" 
            yav:group="nav-menus"
            class="btn"
        >
            "Menu"
        </button>

        // Modal or drawer element
        <dialog 
            id="mobile-menu" 
            yav:click-outside="close" 
            yav:close-on-resize="980"
            class="modal-backdrop"
        >
            <nav>
                <a href="/docs">"Documentation"</a>
                <a href="/pricing">"Tarifs"</a>
            </nav>
        </dialog>
    </div>
}
```
- `yav:toggle="<id>"`: Toggles `[open]`, `[hidden]`, or standard dialog open state on click.
- `yav:click-outside="close"`: Automatically dismisses when clicking outside the container.
- `yav:group="<name>"`: Ensures only one popover within this named group is active at a time.
- `yav:close-on-resize="<px>"`: Automatically closes if the window viewport expands beyond the given width in pixels (ideal for mobile drawers when rotating to desktop).
- `Escape`: Automatically closes open dialogs and returns focus to the trigger.
- `ArrowUp` / `ArrowDown`: Navigates keyboard focus within menus and dialogs.

### Global Hotkeys (`yav:hotkey`)
```rust
rsx! {
    <button yav:toggle="search-palette" yav:hotkey="cmd+k" class="hotkey-btn">
        <span>"Rechercher"</span>
        <kbd>"⌘K"</kbd>
    </button>
}
```
- Supports `"cmd+k"` / `"ctrl+k"`, `"/"`, `"esc"`, etc.

### Clipboard Copy with Feedback (`yav:copy`)
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
- `yav:copy="<text>"`: Copies specified string (or element inner text if empty).
- `yav:copy-label="<text>"`: Temporarily swaps button text for 2 seconds.
- `yav:toast="<msg>"`: Dispatches a custom toast event notification.

### Multi-Pane Synchronized Tabs (`yav:tab-group`)
```rust
rsx! {
    <div yav:tab-group="cli-install">
        <div class="tabs-header">
            <button yav:tab="cargo" class="tab-btn active">"Cargo"</button>
            <button yav:tab="curl" class="tab-btn">"cURL"</button>
        </div>
        <div class="tabs-panels">
            <div yav:tab-panel="cargo">
                <pre>"cargo install yav-cli"</pre>
            </div>
            <div yav:tab-panel="curl" hidden>
                <pre>"curl -fsSL https://yav.rs/install.sh | sh"</pre>
            </div>
        </div>
    </div>
}
```

### Client-Side Search & Filtering
```rust
rsx! {
    <div>
        <input 
            type="search" 
            yav:filter="input" 
            yav:filter-target=".component-item" 
            yav:filter-empty="#empty-msg"
            placeholder="Filtrer les composants..." 
        />

        <div class="list">
            <div class="component-item" data-filter="Button Action Form">"Button"</div>
            <div class="component-item" data-filter="Card Surface Layout">"Card"</div>
        </div>

        <p id="empty-msg" hidden>"Aucun composant correspondant."</p>
    </div>
}
```

---

## 2. Native Cursor Tracking & 3D Tilt Engine

The cursor runtime (~1.3 KB) tracks pointer position using passive `requestAnimationFrame` and publishes high-precision CSS variables onto the target element or `:root`.

### CSS Variables Emitted
- `--yav-mouse-x`: X pixel coordinate relative to element top-left
- `--yav-mouse-y`: Y pixel coordinate relative to element top-left
- `--yav-mouse-rx`: Normalized horizontal position (`-1.0` left, `0.0` center, `+1.0` right)
- `--yav-mouse-ry`: Normalized vertical position (`-1.0` top, `0.0` center, `+1.0` bottom)
- `--yav-mouse-cx`: Client X coordinate in viewport
- `--yav-mouse-cy`: Client Y coordinate in viewport
- `--yav-spotlight-opacity`: `1.0` on pointer enter, smoothly decaying to `0.0` on leave

### Declarative Cursor Attributes
```rust
rsx! {
    // Basic CSS variables injection
    <div yav:cursor="vars" class="interactive-card">
        <h3>"Coordonnées CSS Réactives"</h3>
    </div>

    // 3D Tilt with elastic physics
    <div yav:cursor="tilt" yav:tilt-max="12" class="tilt-card">
        <h3>"Perspective 3D"</h3>
    </div>

    // Magnetic attraction to cursor
    <button yav:cursor="magnetic" yav:magnetic-strength="0.4" class="magnetic-btn">
        "Aimanté"
    </button>
}
```

### Type-Safe Rust UI Components
Instead of raw attributes, you can use pre-built components from `yav::ui::cursor`:

```rust
use yav::ui::{CursorSpotlight, TiltPlate, Magnetic, CursorFollower};

rsx! {
    <CursorSpotlight radius_px={450} color="rgba(224, 89, 47, 0.15)">
        <TiltPlate max_deg={10.0} glare=true class="p-8 rounded-2xl border">
            <h2>"Carte Interactive à Effet Spotlight"</h2>
            <p>"Le halo suit le pointeur et la carte s'incline en 3D."</p>

            <Magnetic strength={0.35}>
                <button class="btn btn-primary mt-4">
                    "Bouton Magnétique"
                </button>
            </Magnetic>
        </TiltPlate>
    </CursorSpotlight>

    // Global cursor follower orb
    <CursorFollower size_px={28} smoothing={0.18} />
}
```

---

## 3. Harmonic Oscillator Spring Motion & Animations

The motion runtime (~1.8 KB) executes natural physical springs without large external libraries.

### Spring Physics Model
Animations are parameterized by:
- **Stiffness** ($k$): Tension of the spring (default `320.0`)
- **Damping** ($c$): Resistance to oscillation (default `18.0`)
- **Mass** ($m$): Inertia of the element (default `1.0`)

The differential equation $m \cdot x'' + c \cdot x' + k \cdot x = 0$ is solved into an analytical damped sine/cosine curve and fed to `Element.animate()`.

### Declarative Motion Attributes
```rust
rsx! {
    <div
        yav:initial="opacity:0, y:30, scale:0.95"
        yav:animate="opacity:1, y:0, scale:1"
        yav:hover="scale:1.03, y:-4"
        yav:tap="scale:0.97"
        yav:spring="320, 18, 1"
        yav:replay="hover"
        class="motion-box"
    >
        <h3>"Animation Physique"</h3>
    </div>
}
```
- `yav:initial`: Starting state (pre-rendered into CSS style during SSR).
- `yav:animate`: Target animation state reached after mount.
- `yav:reveal`: Triggered when element enters the viewport via `IntersectionObserver`.
- `yav:hover`: Transform applied on pointer hover.
- `yav:tap`: Transform applied on mouse down / touch tap.
- `yav:spring="<k>, <c>, <m>"`: Custom spring constants.
- `yav:stagger="<ms>"`: Delays children sequentially.

### Scroll Dynamics
- `yav:scroll-class="is-scrolled@12"`: Adds CSS class `is-scrolled` once the page is scrolled past 12px (perfect for sticky navbars with glassmorphism).
- `yav:scroll-progress`: Injects CSS variable `--yav-scroll-progress` from `0.0` to `1.0` representing total page scroll.
- `yav:scroll-spy`: Automatically toggles `.active` classes on navigation links matching the section currently in view.

### Numeric Counters & Typewriter
```rust
use yav::ui::{CountUp, AnimatedBar, TypewriterCode};

rsx! {
    // Counts from 0 to 124,500 on viewport entry
    <CountUp to={124500.0} prefix="+" suffix=" req/s" duration_ms={1400} />

    // Expanding horizontal gauge from 0% to 97.4%
    <AnimatedBar target_pct={97.4} duration_ms={1200} class="gauge-track" />

    // Syntax-preserving typewriter code block
    <TypewriterCode speed_ms={14} skip_on_click=true class="code-editor">
        <div><span class="text-orange-400">"pub fn "</span>"main() {"</div>
        <div>"    println!(\"Fast and Elegant\");"</div>
        <div>"}"</div>
    </TypewriterCode>
}
```

---

## 4. Declarative Reactive Signals

Build instant reactive controls and dynamic gauges directly in markup:

```rust
rsx! {
    <div yav:state="count: 1, unit_price: 49" class="p-6 border rounded-xl">
        <div class="flex items-center gap-4">
            <button yav:on-click="count = Math.max(1, count - 1)">"-"</button>
            <span yav:text="count" class="font-bold">"1"</span>
            <button yav:on-click="count = count + 1">"+"</button>
        </div>

        <p class="mt-4 text-lg">
            "Total : " 
            <span yav:text="'€' + (count * unit_price)">"€49"</span>
        </p>

        <div class="track h-2 bg-zinc-800 rounded mt-4">
            <div yav:style-width="Math.min(100, count * 10) + '%'" class="h-full bg-orange-500 rounded" />
        </div>
    </div>
}
```
- `yav:state="key: val, ..."`: Declares a local reactive state object.
- `yav:bind="key"`: Two-way binding for form inputs.
- `yav:text="expression"`: Dynamically updates text content.
- `yav:on-click="expression"`: Evaluates expression on click.
- `yav:style-<prop>="expression"`: Computes inline style property.

---

## 5. SSR Tree-Shaking Guarantee

Every script is tree-shaken on the server before response transmission:

| Feature Present in Rendered HTML | Injected Runtime | Overhead |
|----------------------------------|------------------|----------|
| `yav:toggle`, `yav:copy`, `yav:hotkey`, `yav:tab`, `yav:filter` | `yav-micro-actions` | ~2.1 KB |
| `yav:cursor`, `CursorSpotlight`, `TiltPlate`, `Magnetic` | `yav-cursor-runtime` | ~1.3 KB |
| `yav:animate`, `yav:reveal`, `CountUp`, `AnimatedBar`, `TypewriterCode` | `yav-motion-runtime` | ~1.8 KB |
| `yav:state`, `yav:bind`, `yav:text`, `yav:on-click` | `yav-signals-runtime` | ~0.9 KB |
| *None of the above* | **None** | **0 KB** |
