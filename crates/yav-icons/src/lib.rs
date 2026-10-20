//! # YAV Icons
//!
//! Built-in SVG icon system for the Yav web framework, featuring an extensive catalog
//! of over 13,000 icons organized into 3 aesthetic families with multiple visual weights:
//!
//! - **Classic** (`classic:`, based on Lucide): Universal 24x24 stroke icons (`stroke-width="2"`).
//! - **Minimal** (`minimal:`, based on MynaUI): Contemporary 24x24 icons in **Regular** (1.5px) and **Solid**.
//! - **System** (`system:`, based on Phosphor): Precise 256x256 geometric icons in **Regular**, **Bold**,
//!   **Solid** (Fill), **Duotone**, **Light**, and **Thin**.
//!
//! ## Features
//! - **Unified Design Nomenclature**: Seamlessly switch between Classic, Minimal, and System aesthetics.
//! - **Multi-Weight & Style Support**: Normal/Regular, Solid/Fill, Bold, Duotone, Light, and Thin.
//! - **Automatic ViewBox & Stroke Adaptation**: Phosphor (256x256), Lucide & MynaUI (24x24).
//! - **Accessible**: Supports explicit `aria-label` or automatically defaults to `aria-hidden="true"`.
//! - **Instant Lookup**: Pre-sorted slices with binary search $O(\log N)$ lookup.
//! - **Zero External Runtime Dependencies**: Compiles cleanly with only `yav-core`.
//!
//! ## Attribution
//! - **Classic**: Based on [Lucide](https://lucide.dev), licensed under the ISC License.
//! - **Minimal**: Based on [MynaUI](https://mynaui.com), licensed under the MIT License.
//! - **System**: Based on [Phosphor Icons](https://phosphoricons.com), licensed under the MIT License.
//!
//! ## Quick Start
//!
//! ### Using `render_icon` with aesthetic prefixes
//! ```rust
//! use yav_icons::{render_icon, IconSize};
//!
//! // Classic regular icon (Lucide style)
//! let home = render_icon("home", IconSize::Md, Some("text-blue-500"), Some("Home page"));
//! let html = yav_core::render_to_string(&home);
//! assert!(html.contains(r#"viewBox="0 0 24 24""#));
//!
//! // Minimal solid icon (MynaUI style)
//! let act = render_icon("minimal:solid:activity", IconSize::Md, None, None);
//! let html_act = yav_core::render_to_string(&act);
//! assert!(html_act.contains(r#"fill="currentColor""#));
//!
//! // System bold icon (Phosphor style)
//! let acorn = render_icon("system:bold:acorn", IconSize::Lg, None, None);
//! let html_acorn = yav_core::render_to_string(&acorn);
//! assert!(html_acorn.contains(r#"viewBox="0 0 256 256""#));
//! ```
//!
//! ### Using the `Icon` Component
//! ```rust
//! use yav_icons::{Icon, IconFamily, IconStyle, IconSize};
//! use yav_core::Component;
//!
//! let icon = Icon::new("acorn")
//!     .family(IconFamily::System)
//!     .bold()
//!     .size(IconSize::Lg)
//!     .class("search-icon")
//!     .aria_label("Acorn symbol");
//!
//! let html = yav_core::render_to_string(&icon.render());
//! assert!(html.contains(r#"width="24""#));
//! assert!(html.contains(r#"viewBox="0 0 256 256""#));
//! ```
//!
//! ### Searching and Filtering Icons
//! ```rust
//! use yav_icons::{get_icon, search_icons, icons_in_category};
//!
//! // Look up a specific icon
//! let icon = get_icon("arrow-left").expect("arrow-left should exist");
//! assert_eq!(icon.name, "arrow-left");
//!
//! // Search icons by substring
//! let arrows = search_icons("arrow");
//! assert!(!arrows.is_empty());
//! ```

pub mod catalog;
pub mod render;

// Re-exports from catalog
pub use catalog::{
    all_categories, all_icons, all_icons_iter, get_icon, get_icon_by, icons_in_category,
    search_icons, IconData, IconFamily, IconStyle, ICON_COUNT, LUCIDE_ATTRIBUTION,
};

/// Complete slice of the classic icon collection (Lucide regular) for backwards compatibility.
pub static ICONS: &[IconData] = catalog::classic::CLASSIC_REGULAR;

// Re-exports from render
pub use render::{render_icon, Icon, IconSize, FALLBACK_SVG};

/// Convenience module for common imports.
pub mod prelude {
    pub use crate::catalog::{
        all_categories, all_icons, all_icons_iter, get_icon, get_icon_by, icons_in_category,
        search_icons, IconData, IconFamily, IconStyle,
    };
    pub use crate::render::{render_icon, Icon, IconSize};
    pub use crate::ICONS;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use yav_core::render_to_string;

    #[test]
    fn known_icon_lookup_succeeds() {
        assert!(get_icon("home").is_some());
        assert!(get_icon("search").is_some());
        assert!(get_icon("user").is_some());
        assert!(get_icon("mail").is_some());
        assert!(get_icon("settings").is_some());
        assert!(get_icon("alert-circle").is_some());

        // Alias for "x"
        assert_eq!(get_icon("close"), get_icon("x"));
    }

    #[test]
    fn family_and_style_resolutions() {
        let classic = get_icon("classic:home").expect("classic:home should exist");
        assert_eq!(classic.family, IconFamily::Classic);
        assert_eq!(classic.style, IconStyle::Regular);

        let minimal_reg = get_icon("minimal:activity").expect("minimal:activity should exist");
        assert_eq!(minimal_reg.family, IconFamily::Minimal);
        assert_eq!(minimal_reg.style, IconStyle::Regular);

        let minimal_solid =
            get_icon("minimal:solid:activity").expect("minimal:solid:activity should exist");
        assert_eq!(minimal_solid.family, IconFamily::Minimal);
        assert_eq!(minimal_solid.style, IconStyle::Solid);

        let system_reg = get_icon("system:acorn").expect("system:acorn should exist");
        assert_eq!(system_reg.family, IconFamily::System);
        assert_eq!(system_reg.style, IconStyle::Regular);

        let system_bold = get_icon("system:bold:acorn").expect("system:bold:acorn should exist");
        assert_eq!(system_bold.family, IconFamily::System);
        assert_eq!(system_bold.style, IconStyle::Bold);

        let system_solid = get_icon("system:solid:acorn").expect("system:solid:acorn should exist");
        assert_eq!(system_solid.family, IconFamily::System);
        assert_eq!(system_solid.style, IconStyle::Solid);

        let system_duotone =
            get_icon("system:duotone:acorn").expect("system:duotone:acorn should exist");
        assert_eq!(system_duotone.family, IconFamily::System);
        assert_eq!(system_duotone.style, IconStyle::Duotone);

        let system_thin = get_icon("system:thin:acorn").expect("system:thin:acorn should exist");
        assert_eq!(system_thin.family, IconFamily::System);
        assert_eq!(system_thin.style, IconStyle::Thin);

        let system_light = get_icon("system:light:acorn").expect("system:light:acorn should exist");
        assert_eq!(system_light.family, IconFamily::System);
        assert_eq!(system_light.style, IconStyle::Light);

        // Style-only prefixes (without family)
        let bold_acorn = get_icon("bold:acorn").expect("bold:acorn should exist");
        assert_eq!(bold_acorn.style, IconStyle::Bold);

        let duo_acorn = get_icon("duotone:acorn").expect("duotone:acorn should exist");
        assert_eq!(duo_acorn.style, IconStyle::Duotone);

        let solid_act = get_icon("solid:activity").expect("solid:activity should exist");
        assert_eq!(solid_act.style, IconStyle::Solid);
    }

    #[test]
    fn unknown_icon_returns_none() {
        assert!(get_icon("nonexistent-icon-name").is_none());
        assert!(get_icon("").is_none());
        assert!(get_icon("unknown-xyz-123").is_none());
    }

    #[test]
    fn unknown_icon_renders_fallback() {
        let fallback_node = render_icon("unknown_icon", IconSize::Md, None, None);
        let html = render_to_string(&fallback_node);

        assert!(html.starts_with("<svg"));
        assert!(html.contains(FALLBACK_SVG));
    }

    #[test]
    fn search_finds_icons_by_name_substring() {
        let nav_arrows = search_icons("arrow");
        assert!(nav_arrows.len() >= 4);

        let chv = search_icons("chevron");
        assert!(chv.len() >= 4);

        let files = search_icons("file");
        assert!(files.len() >= 2);
    }

    #[test]
    fn category_filtering_works() {
        let categories = all_categories();
        for cat in categories {
            let icons = icons_in_category(cat);
            for icon in &icons {
                assert!(icon.category.eq_ignore_ascii_case(cat));
            }
        }
    }

    #[test]
    fn rendered_icon_is_valid_svg_node() {
        let node = render_icon(
            "home",
            IconSize::Lg,
            Some("nav-home-icon"),
            Some("Home navigation"),
        );
        let html = render_to_string(&node);

        assert!(html.starts_with("<svg"));
        assert!(html.ends_with("</svg>"));
        assert!(html.contains(r#"xmlns="http://www.w3.org/2000/svg""#));
        assert!(html.contains(r#"viewBox="0 0 24 24""#));
        assert!(html.contains(r#"width="24""#));
        assert!(html.contains(r#"height="24""#));
        assert!(html.contains(r#"fill="none""#));
        assert!(html.contains(r#"stroke="currentColor""#));
        assert!(html.contains(r#"stroke-width="2""#));
        assert!(html.contains(r#"stroke-linecap="round""#));
        assert!(html.contains(r#"stroke-linejoin="round""#));
        assert!(html.contains(r#"class="nav-home-icon""#));
        assert!(html.contains(r#"aria-label="Home navigation""#));
        assert!(html.contains(r#"role="img""#));
    }

    #[test]
    fn all_icons_have_unique_names() {
        let mut seen = HashSet::new();
        for icon in all_icons() {
            assert!(
                seen.insert(icon.name),
                "Duplicate icon name found in default catalog: '{}'",
                icon.name
            );
        }
    }

    #[test]
    fn icon_count_matches_expected() {
        let total = all_icons_iter().count();
        assert_eq!(total, ICON_COUNT);
        assert!(total > 10000);
    }

    #[test]
    fn icons_slice_compatibility() {
        assert!(!ICONS.is_empty());
        assert_eq!(ICONS.len(), all_icons().len());
        assert_eq!(ICONS[0].family, IconFamily::Classic);
    }
}
