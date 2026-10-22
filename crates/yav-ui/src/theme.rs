//! Design tokens and theme configuration.
//!
//! Provides color palettes, typography scales, spacing scales, and border radii
//! with default `light` and `dark` themes, plus CSS variable generation.

use yav_core::{Element, Node};

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

    /// Generate CSS custom properties (`:root { ... }` or `html.dark { ... }`).
    pub fn to_css_variables(&self) -> String {
        let selector = if self.name == "dark" {
            "html.dark, :root[data-theme=\"dark\"]"
        } else {
            ":root"
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

    /// Render a `<style>` tag containing this theme's CSS variables.
    pub fn inject_css(&self) -> Node {
        let css = self.to_css_variables();
        let style_el = Element::new("style")
            .attr("data-theme", self.name)
            .child(Node::raw(css));
        Node::Element(style_el)
    }
}

/// Returns an inline JavaScript `<script>` node that prevents Flash of Unstyled Content (FOUC).
///
/// Defaults to dark mode unless `localStorage.getItem('theme') === 'light'`.
/// Synchronously applies or removes the `.dark` class on `<html>` before body render.
pub fn anti_fouc_script() -> Node {
    let script = r#"try{var s=localStorage.getItem('theme');if(s==='light'){document.documentElement.classList.remove('dark')}else{document.documentElement.classList.add('dark')}}catch(e){}"#;
    Node::Element(
        Element::new("script")
            .attr("data-yav-anti-fouc", "true")
            .child(Node::raw(script)),
    )
}

/// Returns a CSS `<style>` node respecting the user's `prefers-reduced-motion` setting.
///
/// Disables gratuitous animations and instantizes transitions when reduced motion is preferred.
pub fn reduced_motion_style() -> Node {
    let css = r#"@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}"#;
    Node::Element(
        Element::new("style")
            .attr("data-yav-reduced-motion", "true")
            .child(Node::raw(css)),
    )
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

        let onclick_js = "var d=document.documentElement.classList.toggle('dark');localStorage.setItem('theme',d?'dark':'light');";

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
            .attr("onclick", onclick_js)
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
    fn theme_css_variables_generation() {
        let light = Theme::light();
        let css = light.to_css_variables();
        assert!(css.contains(":root {"));
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
}
