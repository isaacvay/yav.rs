//! Design tokens and multi-theme configuration for `yav-ui`.
//!
//! Provides color palettes, typography scales, spacing scales, and border radii
//! across 8 built-in design themes:
//! - [`ThemePreset::Light`] (`"light"`)
//! - [`ThemePreset::Dark`] (`"dark"`)
//! - [`ThemePreset::EditorialBlueprint`] (`"editorial-blueprint"` — Yav.rs flagship design system)
//! - [`ThemePreset::ObsidianMinimal`] (`"obsidian-minimal"`)
//! - [`ThemePreset::NordicPaper`] (`"nordic-paper"`)
//! - [`ThemePreset::CyberTerminal`] (`"cyber-terminal"`)
//! - [`ThemePreset::SolarizedLab`] (`"solarized-lab"`)
//! - [`ThemePreset::GlassAurora`] (`"glass-aurora"`)

use yav_core::{Element, Node};

/// Built-in design theme presets available in `yav-ui`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemePreset {
    /// Clean classic light theme (`"light"`).
    #[default]
    Light,
    /// Slate dark theme (`"dark"`).
    Dark,
    /// Flagship Yav.rs Editorial Blueprint theme (`"editorial-blueprint"`):
    /// warm carbon `#0b0b0c`, warm paper `#ece9e3`, and Rust oxide `#e0592f`.
    EditorialBlueprint,
    /// Deep OLED obsidian minimal theme (`"obsidian-minimal"`):
    /// `#09090b` canvas, zinc hairlines, and electric indigo `#6366f1`.
    ObsidianMinimal,
    /// Warm Scandinavian editorial paper theme (`"nordic-paper"`):
    /// alabaster `#f7f5f0`, charcoal ink `#1c1b18`, and terracotta `#c85a32`.
    NordicPaper,
    /// High-contrast retro-futuristic terminal theme (`"cyber-terminal"`):
    /// phosphor black `#050805`, matrix green `#00ff66`, and amber `#ffb000`.
    CyberTerminal,
    /// Precision scientific notebook theme (`"solarized-lab"`):
    /// warm parchment `#fdf6e3`, deep teal `#002b36`, and cyan `#2aa198`.
    SolarizedLab,
    /// Deep space frosted aurora theme (`"glass-aurora"`):
    /// midnight `#070913`, elevated navy `#111629`, and violet `#8b5cf6`.
    GlassAurora,
}

impl ThemePreset {
    /// Returns all 8 built-in theme presets.
    pub const fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::Light,
            ThemePreset::Dark,
            ThemePreset::EditorialBlueprint,
            ThemePreset::ObsidianMinimal,
            ThemePreset::NordicPaper,
            ThemePreset::CyberTerminal,
            ThemePreset::SolarizedLab,
            ThemePreset::GlassAurora,
        ]
    }

    /// Canonical slug identifier used in `data-theme="<slug>"`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ThemePreset::Light => "light",
            ThemePreset::Dark => "dark",
            ThemePreset::EditorialBlueprint => "editorial-blueprint",
            ThemePreset::ObsidianMinimal => "obsidian-minimal",
            ThemePreset::NordicPaper => "nordic-paper",
            ThemePreset::CyberTerminal => "cyber-terminal",
            ThemePreset::SolarizedLab => "solarized-lab",
            ThemePreset::GlassAurora => "glass-aurora",
        }
    }

    /// Human-readable display label for UI selectors.
    pub const fn label(&self) -> &'static str {
        match self {
            ThemePreset::Light => "Light",
            ThemePreset::Dark => "Dark",
            ThemePreset::EditorialBlueprint => "Editorial Blueprint",
            ThemePreset::ObsidianMinimal => "Obsidian Minimal",
            ThemePreset::NordicPaper => "Nordic Paper",
            ThemePreset::CyberTerminal => "Cyber Terminal",
            ThemePreset::SolarizedLab => "Solarized Lab",
            ThemePreset::GlassAurora => "Glass Aurora",
        }
    }

    /// Returns `true` if this preset uses a dark background luminance.
    pub const fn is_dark(&self) -> bool {
        matches!(
            self,
            ThemePreset::Dark
                | ThemePreset::EditorialBlueprint
                | ThemePreset::ObsidianMinimal
                | ThemePreset::CyberTerminal
                | ThemePreset::GlassAurora
        )
    }

    /// Parse a theme preset from its slug or name (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "light" => Some(ThemePreset::Light),
            "dark" => Some(ThemePreset::Dark),
            "editorial-blueprint" | "editorial_blueprint" | "editorial" | "blueprint" => {
                Some(ThemePreset::EditorialBlueprint)
            }
            "obsidian-minimal" | "obsidian_minimal" | "obsidian" => {
                Some(ThemePreset::ObsidianMinimal)
            }
            "nordic-paper" | "nordic_paper" | "nordic" | "paper" => Some(ThemePreset::NordicPaper),
            "cyber-terminal" | "cyber_terminal" | "cyber" | "terminal" => {
                Some(ThemePreset::CyberTerminal)
            }
            "solarized-lab" | "solarized_lab" | "solarized" => Some(ThemePreset::SolarizedLab),
            "glass-aurora" | "glass_aurora" | "aurora" => Some(ThemePreset::GlassAurora),
            _ => None,
        }
    }
}

/// Complete design token theme definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub colors: ColorPalette,
    pub spacing: SpacingScale,
    pub radii: BorderRadii,
    pub fonts: Typography,
}

/// Color tokens for UI elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPalette {
    pub primary: &'static str,
    pub primary_hover: &'static str,
    pub secondary: &'static str,
    pub secondary_hover: &'static str,
    pub background: &'static str,
    pub surface: &'static str,
    pub text: &'static str,
    pub text_muted: &'static str,
    pub border: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub danger: &'static str,
    pub info: &'static str,
}

/// Spacing scale tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpacingScale {
    pub xs: &'static str,
    pub sm: &'static str,
    pub md: &'static str,
    pub lg: &'static str,
    pub xl: &'static str,
    pub xxl: &'static str,
}

/// Border radius tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderRadii {
    pub none: &'static str,
    pub sm: &'static str,
    pub md: &'static str,
    pub lg: &'static str,
    pub full: &'static str,
}

/// Font family and sizing tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Typography {
    pub sans: &'static str,
    pub serif: &'static str,
    pub mono: &'static str,
    pub base_size: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// Default light theme specification.
    pub fn light() -> Self {
        Self {
            name: "light",
            colors: ColorPalette {
                primary: "#2563eb",
                primary_hover: "#1d4ed8",
                secondary: "#f3f4f6",
                secondary_hover: "#e5e7eb",
                background: "#ffffff",
                surface: "#f9fafb",
                text: "#111827",
                text_muted: "#6b7280",
                border: "#e5e7eb",
                success: "#16a34a",
                warning: "#eab308",
                danger: "#dc2626",
                info: "#0284c7",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.25rem",
                md: "0.375rem",
                lg: "0.5rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "ui-sans-serif, system-ui, sans-serif",
                serif: "ui-serif, Georgia, serif",
                mono: "ui-monospace, SFMono-Regular, monospace",
                base_size: "16px",
            },
        }
    }

    /// Default dark theme specification.
    pub fn dark() -> Self {
        Self {
            name: "dark",
            colors: ColorPalette {
                primary: "#3b82f6",
                primary_hover: "#60a5fa",
                secondary: "#1f2937",
                secondary_hover: "#374151",
                background: "#0f172a",
                surface: "#1e293b",
                text: "#f9fafb",
                text_muted: "#9ca3af",
                border: "#374151",
                success: "#22c55e",
                warning: "#facc15",
                danger: "#ef4444",
                info: "#38bdf8",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.25rem",
                md: "0.375rem",
                lg: "0.5rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "ui-sans-serif, system-ui, sans-serif",
                serif: "ui-serif, Georgia, serif",
                mono: "ui-monospace, SFMono-Regular, monospace",
                base_size: "16px",
            },
        }
    }

    /// Construct a [`Theme`] from a [`ThemePreset`].
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::Light => Self::light(),
            ThemePreset::Dark => Self::dark(),
            ThemePreset::EditorialBlueprint => Self::editorial_blueprint(),
            ThemePreset::ObsidianMinimal => Self::obsidian_minimal(),
            ThemePreset::NordicPaper => Self::nordic_paper(),
            ThemePreset::CyberTerminal => Self::cyber_terminal(),
            ThemePreset::SolarizedLab => Self::solarized_lab(),
            ThemePreset::GlassAurora => Self::glass_aurora(),
        }
    }

    /// Look up a built-in [`Theme`] by its slug/name.
    pub fn by_name(name: &str) -> Option<Self> {
        ThemePreset::from_name(name).map(Self::from_preset)
    }

    /// Returns all 8 built-in [`Theme`] presets.
    pub fn all_presets() -> Vec<Self> {
        ThemePreset::all()
            .iter()
            .copied()
            .map(Self::from_preset)
            .collect()
    }

    /// Flagship Yav.rs **Editorial Blueprint** theme (`"editorial-blueprint"`).
    ///
    /// Inspired by architectural blueprints and warm carbon paper:
    /// - Background (`--ink`): `#0b0b0c`
    /// - Surface (`--ink-2`): `#111113`
    /// - Text (`--paper`): `#ece9e3`
    /// - Muted (`--paper-2`): `#a39e93`
    /// - Primary (`--oxide`): `#e0592f`
    /// - Hairline border (`--rule`): `rgba(236, 233, 227, 0.11)`
    pub fn editorial_blueprint() -> Self {
        Self {
            name: "editorial-blueprint",
            colors: ColorPalette {
                primary: "#e0592f",
                primary_hover: "#ff7247",
                secondary: "#17171a",
                secondary_hover: "#222226",
                background: "#0b0b0c",
                surface: "#111113",
                text: "#ece9e3",
                text_muted: "#a39e93",
                border: "rgba(236, 233, 227, 0.11)",
                success: "#4ec994",
                warning: "#e5a93c",
                danger: "#ef4444",
                info: "#38bdf8",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "2px",
                md: "2px",
                lg: "4px",
                full: "9999px",
            },
            fonts: Typography {
                sans: "'Geist', -apple-system, BlinkMacSystemFont, system-ui, sans-serif",
                serif: "'Instrument Serif', Georgia, serif",
                mono: "'Geist Mono', ui-monospace, SFMono-Regular, monospace",
                base_size: "15px",
            },
        }
    }

    /// Deep OLED **Obsidian Minimal** theme (`"obsidian-minimal"`).
    pub fn obsidian_minimal() -> Self {
        Self {
            name: "obsidian-minimal",
            colors: ColorPalette {
                primary: "#6366f1",
                primary_hover: "#818cf8",
                secondary: "#18181b",
                secondary_hover: "#27272a",
                background: "#09090b",
                surface: "#121215",
                text: "#fafafa",
                text_muted: "#a1a1aa",
                border: "#27272a",
                success: "#10b981",
                warning: "#f59e0b",
                danger: "#f43f5e",
                info: "#06b6d4",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.25rem",
                md: "0.375rem",
                lg: "0.5rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "'Inter', -apple-system, BlinkMacSystemFont, system-ui, sans-serif",
                serif: "ui-serif, Georgia, serif",
                mono: "'JetBrains Mono', ui-monospace, monospace",
                base_size: "15px",
            },
        }
    }

    /// Warm Scandinavian **Nordic Paper** theme (`"nordic-paper"`).
    pub fn nordic_paper() -> Self {
        Self {
            name: "nordic-paper",
            colors: ColorPalette {
                primary: "#c85a32",
                primary_hover: "#b04b25",
                secondary: "#ebe7df",
                secondary_hover: "#dfd9ce",
                background: "#f7f5f0",
                surface: "#efeae1",
                text: "#1c1b18",
                text_muted: "#6e6a61",
                border: "#dcd6ca",
                success: "#4a7c59",
                warning: "#d97706",
                danger: "#b91c1c",
                info: "#2563eb",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.125rem",
                md: "0.25rem",
                lg: "0.375rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "'Geist', -apple-system, BlinkMacSystemFont, system-ui, sans-serif",
                serif: "'Instrument Serif', Georgia, serif",
                mono: "'Geist Mono', ui-monospace, monospace",
                base_size: "16px",
            },
        }
    }

    /// Retro-futuristic **Cyber Terminal** theme (`"cyber-terminal"`).
    pub fn cyber_terminal() -> Self {
        Self {
            name: "cyber-terminal",
            colors: ColorPalette {
                primary: "#00ff66",
                primary_hover: "#33ff85",
                secondary: "#0d160d",
                secondary_hover: "#142414",
                background: "#050805",
                surface: "#0a110a",
                text: "#d4ffd4",
                text_muted: "#5c995c",
                border: "rgba(0, 255, 102, 0.24)",
                success: "#00ff66",
                warning: "#ffb000",
                danger: "#ff3355",
                info: "#00e5ff",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0px",
                md: "0px",
                lg: "2px",
                full: "0px",
            },
            fonts: Typography {
                sans: "'Geist Mono', 'JetBrains Mono', ui-monospace, monospace",
                serif: "'Geist Mono', ui-monospace, monospace",
                mono: "'Geist Mono', 'JetBrains Mono', ui-monospace, monospace",
                base_size: "14px",
            },
        }
    }

    /// Scientific notebook **Solarized Lab** theme (`"solarized-lab"`).
    pub fn solarized_lab() -> Self {
        Self {
            name: "solarized-lab",
            colors: ColorPalette {
                primary: "#2aa198",
                primary_hover: "#268bd2",
                secondary: "#eee8d5",
                secondary_hover: "#e4ddc7",
                background: "#fdf6e3",
                surface: "#eee8d5",
                text: "#002b36",
                text_muted: "#586e75",
                border: "#d8d1be",
                success: "#859900",
                warning: "#b58900",
                danger: "#dc322f",
                info: "#268bd2",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.25rem",
                md: "0.375rem",
                lg: "0.5rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "ui-sans-serif, system-ui, sans-serif",
                serif: "ui-serif, Georgia, serif",
                mono: "ui-monospace, SFMono-Regular, monospace",
                base_size: "16px",
            },
        }
    }

    /// Frosted deep-space **Glass Aurora** theme (`"glass-aurora"`).
    pub fn glass_aurora() -> Self {
        Self {
            name: "glass-aurora",
            colors: ColorPalette {
                primary: "#8b5cf6",
                primary_hover: "#a78bfa",
                secondary: "#161d36",
                secondary_hover: "#1e294a",
                background: "#070913",
                surface: "#111629",
                text: "#f1f5f9",
                text_muted: "#94a3b8",
                border: "rgba(139, 92, 246, 0.22)",
                success: "#10b981",
                warning: "#f59e0b",
                danger: "#f43f5e",
                info: "#06b6d4",
            },
            spacing: SpacingScale {
                xs: "0.25rem",
                sm: "0.5rem",
                md: "1rem",
                lg: "1.5rem",
                xl: "2rem",
                xxl: "3rem",
            },
            radii: BorderRadii {
                none: "0px",
                sm: "0.375rem",
                md: "0.625rem",
                lg: "0.875rem",
                full: "9999px",
            },
            fonts: Typography {
                sans: "'Geist', -apple-system, BlinkMacSystemFont, system-ui, sans-serif",
                serif: "'Instrument Serif', Georgia, serif",
                mono: "'Geist Mono', ui-monospace, monospace",
                base_size: "16px",
            },
        }
    }

    /// Generate CSS custom properties (`:root`, `html.dark`, or `:root[data-theme="<name>"]`).
    pub fn to_css_variables(&self) -> String {
        let selector = match self.name {
            "light" => ":root, :root[data-theme=\"light\"]".to_string(),
            "dark" => "html.dark, :root[data-theme=\"dark\"]".to_string(),
            other => format!(":root[data-theme=\"{other}\"]"),
        };

        format!(
            "{selector} {{\n\
  --color-primary: {primary};\n\
  --color-primary-hover: {primary_hover};\n\
  --color-secondary: {secondary};\n\
  --color-secondary-hover: {secondary_hover};\n\
  --color-background: {background};\n\
  --color-surface: {surface};\n\
  --color-text: {text};\n\
  --color-text-muted: {text_muted};\n\
  --color-border: {border};\n\
  --color-success: {success};\n\
  --color-warning: {warning};\n\
  --color-danger: {danger};\n\
  --color-info: {info};\n\
  --ink: {background};\n\
  --ink-2: {surface};\n\
  --paper: {text};\n\
  --paper-2: {text_muted};\n\
  --oxide: {primary};\n\
  --oxide-hi: {primary_hover};\n\
  --rule: {border};\n\
  --signal: {success};\n\
  --amber: {warning};\n\
  --spacing-xs: {sp_xs};\n\
  --spacing-sm: {sp_sm};\n\
  --spacing-md: {sp_md};\n\
  --spacing-lg: {sp_lg};\n\
  --spacing-xl: {sp_xl};\n\
  --spacing-xxl: {sp_xxl};\n\
  --radius-sm: {r_sm};\n\
  --radius-md: {r_md};\n\
  --radius-lg: {r_lg};\n\
  --radius-full: {r_full};\n\
  --font-sans: {font_sans};\n\
  --font-serif: {font_serif};\n\
  --font-mono: {font_mono};\n\
  --font-base-size: {font_base};\n\
}}",
            primary = self.colors.primary,
            primary_hover = self.colors.primary_hover,
            secondary = self.colors.secondary,
            secondary_hover = self.colors.secondary_hover,
            background = self.colors.background,
            surface = self.colors.surface,
            text = self.colors.text,
            text_muted = self.colors.text_muted,
            border = self.colors.border,
            success = self.colors.success,
            warning = self.colors.warning,
            danger = self.colors.danger,
            info = self.colors.info,
            sp_xs = self.spacing.xs,
            sp_sm = self.spacing.sm,
            sp_md = self.spacing.md,
            sp_lg = self.spacing.lg,
            sp_xl = self.spacing.xl,
            sp_xxl = self.spacing.xxl,
            r_sm = self.radii.sm,
            r_md = self.radii.md,
            r_lg = self.radii.lg,
            r_full = self.radii.full,
            font_sans = self.fonts.sans,
            font_serif = self.fonts.serif,
            font_mono = self.fonts.mono,
            font_base = self.fonts.base_size,
        )
    }

    /// Generate a combined CSS stylesheet containing all 8 built-in themes.
    pub fn all_themes_css() -> String {
        let mut out = String::new();
        for theme in Self::all_presets() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&theme.to_css_variables());
        }
        out
    }

    /// Render a `<style>` tag containing this theme's CSS variables, with an optional CSP nonce.
    pub fn inject_css_with_nonce(&self, nonce: Option<&str>) -> Node {
        let css = self.to_css_variables();
        let mut style_el = Element::new("style")
            .attr("data-theme", self.name)
            .child(Node::raw(css));
        if let Some(n) = nonce {
            style_el = style_el.attr("nonce", n);
        }
        Node::Element(style_el)
    }

    /// Render a `<style>` tag containing this theme's CSS variables.
    pub fn inject_css(&self) -> Node {
        self.inject_css_with_nonce(None)
    }

    /// Render a `<style data-yav-themes="all">` tag containing all 8 built-in themes, with an optional CSP nonce.
    pub fn inject_all_themes_css_with_nonce(nonce: Option<&str>) -> Node {
        let css = Self::all_themes_css();
        let mut style_el = Element::new("style")
            .attr("data-yav-themes", "all")
            .child(Node::raw(css));
        if let Some(n) = nonce {
            style_el = style_el.attr("nonce", n);
        }
        Node::Element(style_el)
    }

    /// Render a `<style data-yav-themes="all">` tag containing all 8 built-in themes.
    pub fn inject_all_themes_css() -> Node {
        Self::inject_all_themes_css_with_nonce(None)
    }
}

/// Returns an inline JavaScript `<script>` node that prevents Flash of Unstyled Content (FOUC),
/// with an optional CSP nonce.
///
/// Restores both `localStorage.getItem('yav-theme')` (via `data-theme` on `<html>`) and
/// `localStorage.getItem('theme')` (`.dark` class on `<html>`) synchronously before body render.
pub fn anti_fouc_script_with_nonce(nonce: Option<&str>) -> Node {
    let script = r#"try{var yt=localStorage.getItem('yav-theme');if(yt){document.documentElement.setAttribute('data-theme',yt);}var s=localStorage.getItem('theme');if(s==='light'||yt==='light'||yt==='nordic-paper'||yt==='solarized-lab'){document.documentElement.classList.remove('dark')}else{document.documentElement.classList.add('dark')}}catch(e){}"#;
    let mut el = Element::new("script")
        .attr("data-yav-anti-fouc", "true")
        .child(Node::raw(script));
    if let Some(n) = nonce {
        el = el.attr("nonce", n);
    }
    Node::Element(el)
}

/// Returns an inline JavaScript `<script>` node that prevents Flash of Unstyled Content (FOUC).
pub fn anti_fouc_script() -> Node {
    anti_fouc_script_with_nonce(None)
}

/// Returns a CSS `<style>` node respecting the user's `prefers-reduced-motion` setting,
/// with an optional CSP nonce.
///
/// Disables gratuitous animations and instantizes transitions when reduced motion is preferred.
pub fn reduced_motion_style_with_nonce(nonce: Option<&str>) -> Node {
    let css = r#"@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}"#;
    let mut el = Element::new("style")
        .attr("data-yav-reduced-motion", "true")
        .child(Node::raw(css));
    if let Some(n) = nonce {
        el = el.attr("nonce", n);
    }
    Node::Element(el)
}

/// Returns a CSS `<style>` node respecting the user's `prefers-reduced-motion` setting.
pub fn reduced_motion_style() -> Node {
    reduced_motion_style_with_nonce(None)
}

/// A dark mode toggle button with Sun / Moon icons and local storage persistence.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThemeToggle {
    class: String,
    id: Option<String>,
}

impl ThemeToggle {
    /// Create a new ThemeToggle component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set element ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Render to virtual DOM node.
    pub fn render(&self) -> Node {
        let base_class = "inline-flex items-center justify-center p-2 rounded-lg text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-white dark:hover:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-gray-200 dark:focus:ring-gray-700";
        let full_class = crate::utils::merge_classes(&[base_class, &self.class]);

        let sun_icon = yav_icons::render_icon(
            "sun",
            yav_icons::IconSize::Md,
            Some("hidden dark:block"),
            None,
        );
        let moon_icon = yav_icons::render_icon(
            "moon",
            yav_icons::IconSize::Md,
            Some("block dark:hidden"),
            None,
        );

        let mut el = Element::new("button")
            .attr("type", "button")
            .attr("class", full_class)
            .attr("aria-label", "Toggle dark mode")
            .attr("data-theme-toggle", "")
            .child(sun_icon)
            .child(moon_icon);

        if let Some(ref id) = self.id {
            el = el.attr("id", id);
        }

        Node::Element(el)
    }
}

/// Convenience builder function for [`ThemeToggle`].
pub fn theme_toggle() -> ThemeToggle {
    ThemeToggle::new()
}

impl yav_core::IntoNode for ThemeToggle {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<ThemeToggle> for Node {
    fn from(t: ThemeToggle) -> Self {
        t.render()
    }
}

impl yav_core::Component for ThemeToggle {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Props for constructing a `ThemeToggle` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeToggleProps {
    pub class: String,
    pub id: Option<String>,
}

/// Functional component adapter for `ThemeToggle`, enabling `<ThemeToggle />` in `rsx!`.
#[allow(non_snake_case)]
pub fn ThemeToggle(props: ThemeToggleProps) -> Node {
    let mut t = ThemeToggle::new();
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    if let Some(id) = props.id {
        t = t.id(id);
    }
    t.render()
}

/// Interactive multi-theme selector widget (`<ThemeSelector />`) supporting all 8 built-in `yav-ui` themes.
///
/// Updates `document.documentElement.setAttribute('data-theme', ...)` and persists the selection
/// in `localStorage.setItem('yav-theme', ...)`.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSelector {
    default_preset: ThemePreset,
    class: String,
    id: Option<String>,
}

impl Default for ThemeSelector {
    fn default() -> Self {
        Self {
            default_preset: ThemePreset::EditorialBlueprint,
            class: String::new(),
            id: None,
        }
    }
}

impl ThemeSelector {
    /// Create a new [`ThemeSelector`] defaulting to [`ThemePreset::EditorialBlueprint`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initially selected theme preset.
    pub fn preset(mut self, preset: ThemePreset) -> Self {
        self.default_preset = preset;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set the `<select>` element ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Render the interactive `<select>` theme switcher node.
    pub fn render(&self) -> Node {
        let base_class = "inline-flex items-center px-3 py-1.5 text-xs font-mono border rounded-sm bg-transparent transition-colors focus:outline-none";
        let full_class = crate::utils::merge_classes(&[base_class, &self.class]);

        let onchange_js = "var v=this.value;document.documentElement.setAttribute('data-theme',v);localStorage.setItem('yav-theme',v);var light=(v==='light'||v==='nordic-paper'||v==='solarized-lab');document.documentElement.classList.toggle('dark',!light);localStorage.setItem('theme',light?'light':'dark');";

        let mut select_el = Element::new("select")
            .attr("class", full_class)
            .attr("aria-label", "Select design theme")
            .attr("data-yav-theme-selector", "true")
            .attr("onchange", onchange_js);

        if let Some(ref id) = self.id {
            select_el = select_el.attr("id", id);
        }

        for preset in ThemePreset::all() {
            let mut opt = Element::new("option")
                .attr("value", preset.as_str())
                .child(Node::text(preset.label()));
            if *preset == self.default_preset {
                opt = opt.bool_attr("selected", true);
            }
            select_el = select_el.child(Node::Element(opt));
        }

        Node::Element(select_el)
    }
}

/// Convenience builder for [`ThemeSelector`].
pub fn theme_selector() -> ThemeSelector {
    ThemeSelector::new()
}

impl yav_core::IntoNode for ThemeSelector {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<ThemeSelector> for Node {
    fn from(s: ThemeSelector) -> Self {
        s.render()
    }
}

impl yav_core::Component for ThemeSelector {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Props for `<ThemeSelector />` in `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeSelectorProps {
    pub default_theme: String,
    pub class: String,
    pub id: Option<String>,
}

/// Functional component adapter for `<ThemeSelector />` in `rsx!`.
#[allow(non_snake_case)]
pub fn ThemeSelector(props: ThemeSelectorProps) -> Node {
    let mut selector = ThemeSelector::new();
    if let Some(preset) = ThemePreset::from_name(&props.default_theme) {
        selector = selector.preset(preset);
    }
    if !props.class.is_empty() {
        selector = selector.class(props.class);
    }
    if let Some(id) = props.id {
        selector = selector.id(id);
    }
    selector.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn theme_defaults_are_valid() {
        let light = Theme::light();
        assert_eq!(light.name, "light");
        assert_eq!(light.colors.primary, "#2563eb");
        assert_eq!(light.colors.background, "#ffffff");

        let dark = Theme::dark();
        assert_eq!(dark.name, "dark");
        assert_eq!(dark.colors.primary, "#3b82f6");
        assert_eq!(dark.colors.background, "#0f172a");
    }

    #[test]
    fn theme_presets_and_multi_theme_catalog() {
        let all = Theme::all_presets();
        assert_eq!(all.len(), 8);

        let eb = Theme::editorial_blueprint();
        assert_eq!(eb.name, "editorial-blueprint");
        assert_eq!(eb.colors.primary, "#e0592f");
        assert_eq!(eb.colors.background, "#0b0b0c");
        assert_eq!(eb.colors.text, "#ece9e3");

        let cyber = Theme::by_name("cyber-terminal").unwrap();
        assert_eq!(cyber.colors.primary, "#00ff66");

        let nordic = Theme::from_preset(ThemePreset::NordicPaper);
        assert_eq!(nordic.name, "nordic-paper");
        assert!(!ThemePreset::NordicPaper.is_dark());
        assert!(ThemePreset::EditorialBlueprint.is_dark());

        let all_css = Theme::all_themes_css();
        assert!(all_css.contains(":root[data-theme=\"editorial-blueprint\"]"));
        assert!(all_css.contains(":root[data-theme=\"obsidian-minimal\"]"));
        assert!(all_css.contains(":root[data-theme=\"nordic-paper\"]"));
        assert!(all_css.contains(":root[data-theme=\"cyber-terminal\"]"));
        assert!(all_css.contains(":root[data-theme=\"solarized-lab\"]"));
        assert!(all_css.contains(":root[data-theme=\"glass-aurora\"]"));
        assert!(all_css.contains("--oxide: #e0592f;"));
    }

    #[test]
    fn theme_selector_renders_all_options() {
        let sel = theme_selector()
            .preset(ThemePreset::EditorialBlueprint)
            .id("theme-picker")
            .render();
        let html = render_to_string(&sel);
        assert!(html.contains("data-yav-theme-selector=\"true\""));
        assert!(html.contains("Editorial Blueprint"));
        assert!(html.contains("Obsidian Minimal"));
        assert!(html.contains("Nordic Paper"));
        assert!(html.contains("Cyber Terminal"));
        assert!(html.contains("Solarized Lab"));
        assert!(html.contains("Glass Aurora"));
    }

    #[test]
    fn theme_css_variables_generation() {
        let light = Theme::light();
        let css = light.to_css_variables();
        assert!(css.contains(":root"));
        assert!(css.contains("--color-primary: #2563eb;"));
        assert!(css.contains("--font-sans:"));

        let dark = Theme::dark();
        let dark_css = dark.to_css_variables();
        assert!(dark_css.contains("html.dark"));
        assert!(dark_css.contains("--color-background: #0f172a;"));
    }

    #[test]
    fn theme_inject_css_renders_style_tag() {
        let light = Theme::light();
        let node = light.inject_css();
        let html = render_to_string(&node);
        assert!(html.starts_with("<style"));
        assert!(html.contains("data-theme=\"light\""));
        assert!(html.contains("--color-primary: #2563eb;"));
        assert!(html.ends_with("</style>"));
    }

    #[test]
    fn test_theme_csp_nonce() {
        let light = Theme::light();
        let node = light.inject_css_with_nonce(Some("rAnd0m123"));
        let html = render_to_string(&node);
        assert!(html.contains("nonce=\"rAnd0m123\""));

        let fouc = anti_fouc_script_with_nonce(Some("nonce_abc"));
        let fouc_html = render_to_string(&fouc);
        assert!(fouc_html.contains("nonce=\"nonce_abc\""));

        let reduced = reduced_motion_style_with_nonce(Some("nonce_xyz"));
        let reduced_html = render_to_string(&reduced);
        assert!(reduced_html.contains("nonce=\"nonce_xyz\""));
    }
}
