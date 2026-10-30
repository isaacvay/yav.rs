# Multi-Theme Design System (`yav-ui::theme`)

Yav v0.1.4 features a complete, zero-dependency multi-theme design system equipped with **8 built-in themes**, seamless client-side switching, cookie & `localStorage` persistence, and guaranteed **zero Flash of Unstyled Content (FOUC)**.

---

## 1. The 8 Built-in Themes

| Theme Slug | Name | Background (`--bg`) | Text (`--fg`) | Accent / Oxide | Personality & Use Case |
|------------|------|---------------------|---------------|----------------|------------------------|
| `editorial-blueprint` | **Editorial Blueprint** | `#0b0b0c` (Carbon) | `#ece9e3` (Paper) | `#e0592f` (Rust Oxide) | Flagship technical editorial publication aesthetic |
| `obsidian-minimal` | **Obsidian Minimal** | `#09090b` (OLED) | `#f4f4f5` (Zinc) | `#6366f1` (Electric Indigo) | Sleek developer tools and high-contrast dashboards |
| `nordic-paper` | **Nordic Paper** | `#f7f5f0` (Alabaster) | `#1c1b18` (Charcoal) | `#c85a32` (Terracotta) | Warm Scandinavian publication, essays & documentation |
| `cyber-terminal` | **Cyber Terminal** | `#050805` (Phosphor) | `#00ff66` (Matrix Green) | `#ffb000` (Amber CRT) | Retro-futuristic terminal, CLI, and hacker dashboards |
| `solarized-lab` | **Solarized Lab** | `#fdf6e3` (Parchment) | `#002b36` (Deep Teal) | `#2aa198` (Cyan) | Scientific papers, notebooks, and research tools |
| `glass-aurora` | **Glass Aurora** | `#070913` (Midnight) | `#f1f5f9` (Starlight) | `#8b5cf6` (Frosted Violet) | Translucent blurred glassmorphism & SaaS landing pages |
| `light` | **Classic Light** | `#ffffff` (Pure White) | `#0f172a` (Slate 900) | `#2563eb` (Royal Blue) | Clean, corporate, daytime administrative portals |
| `dark` | **Slate Dark** | `#0f172a` (Slate 900) | `#f8fafc` (Slate 50) | `#38bdf8` (Sky 400) | Standard comfortable dark interface |

---

## 2. Quick Setup in Your Application

### Step 1: Inject Theme CSS in Document `<head>`
In your root layout or document template, inject the global theme CSS variables:

```rust
use yav::ui::Theme;
use yav::prelude::*;

#[component]
pub fn RootLayout(children: Node) -> Node {
    rsx! {
        <html lang="fr" data-theme="editorial-blueprint">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                {Theme::inject_all_themes_css()}
                <link rel="stylesheet" href="/public/styles.css" />
            </head>
            <body class="bg-theme-bg text-theme-fg min-h-screen">
                {children}
            </body>
        </html>
    }
}
```

### Step 2: Add the `<ThemeSelector />` Component
Place the `<ThemeSelector />` in your navigation bar or header:

```rust
use yav::ui::ThemeSelector;
use yav::prelude::*;

rsx! {
    <header class="navbar flex items-center justify-between p-4 border-b">
        <div class="logo font-black">"Mon Application"</div>
        <ThemeSelector default_theme="editorial-blueprint" />
    </header>
}
```

---

## 3. How Theme Switching Works

- **Persistence**: When the user picks a theme, the selection is immediately saved to:
  1. `localStorage.setItem('yav_theme', theme)`
  2. `document.cookie = 'yav_theme=' + theme + '; path=/; max-age=31536000'`
- **Anti-FOUC**: `Theme::inject_all_themes_css()` includes an ultra-fast inline script that executes before HTML body parsing:
  ```js
  (function(){
    var t = localStorage.getItem('yav_theme');
    if (!t) {
      var m = document.cookie.match(/(^| )yav_theme=([^;]+)/);
      if (m) t = decodeURIComponent(m[2]);
    }
    if (t) document.documentElement.setAttribute('data-theme', t);
  })();
  ```
  This guarantees **zero flash of unstyled theme** on page reload!

---

## 4. Semantic CSS Variables Reference

Every theme defines standard CSS custom properties on `:root[data-theme="<slug>"]`:

```css
:root[data-theme="editorial-blueprint"] {
  --bg: #0b0b0c;
  --fg: #ece9e3;
  --muted: #8e8b82;
  --card: #141416;
  --card-fg: #ece9e3;
  --border: #262523;
  --oxide: #e0592f;
  --accent: #e0592f;
  --font-sans: "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", monospace;
}
```

You can use these variables in custom CSS or map them in Tailwind:

```css
.card {
  background-color: var(--card);
  color: var(--card-fg);
  border: 1px solid var(--border);
}

.brand-button {
  background-color: var(--oxide);
  color: #ffffff;
}
```

---

## 5. Adding the Component via CLI

You can also scaffold the theme switcher component directly into your project:

```bash
yav add theme-selector
```
This generates `src/components/theme_selector.rs` for complete customization!
