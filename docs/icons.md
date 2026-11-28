# Built-in Icon System

Yav includes a curated, high-performance icon system with roughly 13,600 compiled icon variants (`ICON_COUNT` with `full-icons`; ~1,900 with the default Classic-only build) organized into 3 aesthetic families. The upstream source tree contains more SVG files (~22,600) because it also holds styles that are not all compiled into the catalog:
- **Classic** (based on [Lucide Icons](https://lucide.dev/))
- **Minimal** (based on [MynaUI Icons](https://mynaui.com/))
- **System** (based on [Phosphor Icons](https://phosphoricons.com/))

Icons are embedded as pre-optimized SVG strings directly in the binary, requiring no external font downloads, SVG sprite sheets, or network requests.

## Feature gates (which icons are actually compiled)

The umbrella `yav` crate enables only the **Classic** family by default. This
keeps compile times and binary size low for applications that use the standard
Lucide-style set (~1,900 variants). Select what you need in `Cargo.toml`:

```toml
# Default: Classic only
yav = "0.1.8"

# All three families (~13,600 variants)
yav = { version = "0.1.7", features = ["full-icons"] }

# Or mix and match
yav = { version = "0.1.7", features = ["system-icons"] }
```

Available features: `classic-icons` (default), `minimal-icons`, `system-icons`
and `full-icons` (= minimal + system). Slices for disabled families are empty,
so `render_icon("minimal:bell", …)` returns the fallback SVG until the matching
feature is enabled. `ICON_COUNT` always reports the number of **compiled**
variants for the active feature set — it is not a constant 13,600.

`yav-icons` itself also defaults to `classic` only; enable `features = ["minimal", "system"]` (or `"full"` on `yav`) to include additional families, or enable `icon-minify` with `YAV_ICON_ALLOWLIST` for per-icon compile-time pruning.

## Basic Usage

Import the icon renderer and size enum:

```rust
use yav::prelude::*;
use yav::icons::{render_icon, IconSize};

// Rendering via helper function
let search_icon = render_icon("search", IconSize::Md, Some("text-zinc-500"), None);

// Rendering via RSX component
rsx! {
    <Icon name="settings" size="lg" class="text-indigo-600 hover:text-indigo-700" />
}
```

## Sizes

Yav icons support 5 standard sizes calibrated to Tailwind typography and spacing:

| Size | Enum | Dimensions | Standard Use |
|------|------|------------|--------------|
| `xs` | `IconSize::Xs` | 12x12px (`w-3 h-3`) | Badges, indicators, inline tags |
| `sm` | `IconSize::Sm` | 16x16px (`w-4 h-4`) | Buttons, table rows, breadcrumbs |
| `md` | `IconSize::Md` | 20x20px (`w-5 h-5`) | Navbars, cards, inputs (default) |
| `lg` | `IconSize::Lg` | 24x24px (`w-6 h-6`) | Section headers, feature cards |
| `xl` | `IconSize::Xl` | 32x32px (`w-8 h-8`) | Hero sections, stat cards, alerts |

## Accessibility & Semantics

When an icon conveys meaning (e.g. an icon-only button), always provide an `aria-label`:

```rust
// Accessible icon-only button
rsx! {
    <button class="p-2 rounded-lg hover:bg-zinc-100" aria-label="Close dialog">
        <Icon name="x" size="md" aria_label="Close" />
    </button>
}
```

If `aria_label` is omitted, the icon automatically receives `aria-hidden="true"` so screen readers safely ignore decorative icons.

## CLI Icon Explorer

You can search and explore all 13,000+ available icons across all three families (`Classic`, `Minimal`, `System`) directly from your terminal using `yav icon`:

```bash
# List all built-in icons
yav icon list

# Search for icons by keyword
yav icon search user
yav icon search arrow
yav icon search chevron
```

## Supported Icons Catalog

The catalog includes over 80 commonly needed application and navigation icons:

- **Navigation**: `arrow-left`, `arrow-right`, `arrow-up`, `arrow-down`, `chevron-left`, `chevron-right`, `chevron-up`, `chevron-down`, `menu`, `x`, `more-horizontal`, `more-vertical`, `external-link`
- **Actions**: `search`, `plus`, `minus`, `edit`, `trash`, `trash-2`, `copy`, `check`, `download`, `upload`, `refresh-cw`, `filter`, `share`, `share-2`, `save`, `sliders`
- **Objects & Layout**: `folder`, `file`, `file-text`, `database`, `server`, `terminal`, `layers`, `grid`, `layout`, `code`, `cpu`, `globe`, `zap`, `activity`, `chart-bar`, `pie-chart`, `trending-up`, `trending-down`
- **User & Security**: `user`, `users`, `user-check`, `user-plus`, `shield`, `shield-check`, `lock`, `unlock`, `key`, `eye`, `eye-off`
- **Feedback & Alerts**: `bell`, `bell-off`, `info`, `alert-circle`, `alert-triangle`, `check-circle`, `x-circle`, `help-circle`
- **Communication & Media**: `mail`, `message-square`, `phone`, `image`, `play`, `pause`, `volume-2`, `volume-x`
- **Theme & Misc**: `sun`, `moon`, `laptop`, `settings`, `calendar`, `clock`, `tag`, `hash`, `heart`, `star`, `sparkles`, `command`

## Attribution & Licenses

- **Classic** ([Lucide Icons](https://lucide.dev/)): Licensed under the [ISC License](https://github.com/lucide-icons/lucide/blob/main/LICENSE).
- **Minimal** ([MynaUI Icons](https://mynaui.com/)): Licensed under the [MIT License](https://github.com/mynaui/icons/blob/main/LICENSE).
- **System** ([Phosphor Icons](https://phosphoricons.com/)): Licensed under the [MIT License](https://github.com/phosphor-icons/core/blob/main/LICENSE).
