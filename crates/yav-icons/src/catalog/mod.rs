//! Unified icon catalog supporting Classic (Lucide), Minimal (MynaUI), and System (Phosphor) families.
//!
//! # Attribution
//! - **Classic**: Based on [Lucide](https://lucide.dev), licensed under the ISC License. Portions derived from Feather (MIT).
//! - **Minimal**: Based on [MynaUI Icons](https://mynaui.com), licensed under the MIT License.
//! - **System**: Based on [Phosphor Icons](https://phosphoricons.com), licensed under the MIT License.

use std::fmt;

/// Design aesthetic / family of an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconFamily {
    /// Classic stroke style based on Lucide (24x24, 2px stroke).
    #[default]
    Classic,
    /// Minimalist modern style based on MynaUI (24x24, 1.5px stroke or solid).
    Minimal,
    /// System geometric style based on Phosphor (256x256, multiple weights).
    System,
}

impl IconFamily {
    /// Returns the canonical string identifier for this family.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Minimal => "minimal",
            Self::System => "system",
        }
    }

    /// Parse a family name or alias from string.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "classic" | "lucide" => Some(Self::Classic),
            "minimal" | "mynaui" | "myna" => Some(Self::Minimal),
            "system" | "phosphor" | "ph" => Some(Self::System),
            _ => None,
        }
    }
}

impl fmt::Display for IconFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Visual style or weight variant of an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IconStyle {
    /// Standard / regular outline style (default).
    #[default]
    Regular,
    /// Solid / filled style.
    Solid,
    /// Bold stroke weight.
    Bold,
    /// Light stroke weight.
    Light,
    /// Thin stroke weight.
    Thin,
    /// Duotone (two-tone with transparent secondary layer).
    Duotone,
}

impl IconStyle {
    /// Returns the canonical string identifier for this style.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Solid => "solid",
            Self::Bold => "bold",
            Self::Light => "light",
            Self::Thin => "thin",
            Self::Duotone => "duotone",
        }
    }

    /// Parse a style variant from string.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "regular" | "normal" | "outline" | "line" => Some(Self::Regular),
            "solid" | "fill" | "filled" => Some(Self::Solid),
            "bold" => Some(Self::Bold),
            "light" => Some(Self::Light),
            "thin" => Some(Self::Thin),
            "duotone" => Some(Self::Duotone),
            _ => None,
        }
    }
}

impl fmt::Display for IconStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata and SVG path data for a single icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconData {
    /// Canonical kebab-case name of the icon (e.g. `"home"`, `"arrow-left"`).
    pub name: &'static str,
    /// Design family (Classic, Minimal, System).
    pub family: IconFamily,
    /// Visual style/weight (Regular, Solid, Bold, Light, Thin, Duotone).
    pub style: IconStyle,
    /// Inner SVG elements without the wrapping `<svg>` tag.
    pub svg_content: &'static str,
    /// Logical category of the icon.
    pub category: &'static str,
    /// SVG viewBox (e.g. `"0 0 24 24"` or `"0 0 256 256"`).
    pub view_box: &'static str,
    /// Default stroke-width (e.g. `"2"`, `"1.5"`, `"16"`, or `""` for solid).
    pub stroke_width: &'static str,
    /// Whether this is a solid/fill icon.
    pub is_fill: bool,
}

#[cfg(feature = "classic")]
pub mod classic;
#[cfg(not(feature = "classic"))]
pub mod classic {
    pub static CLASSIC_REGULAR: &[super::IconData] = &[];
}

#[cfg(feature = "minimal")]
pub mod minimal;
#[cfg(not(feature = "minimal"))]
pub mod minimal {
    pub static MINIMAL_REGULAR: &[super::IconData] = &[];
    pub static MINIMAL_SOLID: &[super::IconData] = &[];
}

#[cfg(feature = "system")]
pub mod system;
#[cfg(not(feature = "system"))]
pub mod system {
    pub static SYSTEM_REGULAR: &[super::IconData] = &[];
    pub static SYSTEM_BOLD: &[super::IconData] = &[];
    pub static SYSTEM_SOLID: &[super::IconData] = &[];
    pub static SYSTEM_DUOTONE: &[super::IconData] = &[];
    pub static SYSTEM_LIGHT: &[super::IconData] = &[];
    pub static SYSTEM_THIN: &[super::IconData] = &[];
}

/// Attribution notice for icons in this catalog.
pub const LUCIDE_ATTRIBUTION: &str =
    "Icons based on Lucide (https://lucide.dev), licensed under the ISC License.";

/// Complete slice of the classic icon collection (Lucide regular) for backwards compatibility.
pub static ICONS: &[IconData] = classic::CLASSIC_REGULAR;

/// Total number of icons available across all families and styles.
pub const ICON_COUNT: usize = classic::CLASSIC_REGULAR.len()
    + minimal::MINIMAL_REGULAR.len()
    + minimal::MINIMAL_SOLID.len()
    + system::SYSTEM_REGULAR.len()
    + system::SYSTEM_BOLD.len()
    + system::SYSTEM_SOLID.len()
    + system::SYSTEM_DUOTONE.len()
    + system::SYSTEM_LIGHT.len()
    + system::SYSTEM_THIN.len();

/// Helper function to perform binary search on a pre-sorted icon slice.
#[inline]
fn find_in_slice(slice: &'static [IconData], name: &str) -> Option<&'static IconData> {
    slice
        .binary_search_by_key(&name, |icon| icon.name)
        .ok()
        .map(|idx| &slice[idx])
}

/// Look up an icon by name and optional family and style.
pub fn get_icon_by(
    name: &str,
    family: Option<IconFamily>,
    style: Option<IconStyle>,
) -> Option<&'static IconData> {
    let mut normalized = name.trim().to_ascii_lowercase();
    if normalized == "close" {
        normalized = "x".to_string();
    }

    let target_family = family.unwrap_or(IconFamily::Classic);
    let target_style = style.unwrap_or(IconStyle::Regular);

    // Exact family & style lookup
    if let Some(icon) = match (target_family, target_style) {
        (IconFamily::Classic, IconStyle::Regular) => {
            find_in_slice(classic::CLASSIC_REGULAR, &normalized)
        }
        (IconFamily::Classic, _) => None,

        (IconFamily::Minimal, IconStyle::Regular) => {
            find_in_slice(minimal::MINIMAL_REGULAR, &normalized)
        }
        (IconFamily::Minimal, IconStyle::Solid) => {
            find_in_slice(minimal::MINIMAL_SOLID, &normalized)
        }
        (IconFamily::Minimal, _) => None,

        (IconFamily::System, IconStyle::Regular) => {
            find_in_slice(system::SYSTEM_REGULAR, &normalized)
        }
        (IconFamily::System, IconStyle::Bold) => find_in_slice(system::SYSTEM_BOLD, &normalized),
        (IconFamily::System, IconStyle::Solid) => find_in_slice(system::SYSTEM_SOLID, &normalized),
        (IconFamily::System, IconStyle::Duotone) => {
            find_in_slice(system::SYSTEM_DUOTONE, &normalized)
        }
        (IconFamily::System, IconStyle::Light) => find_in_slice(system::SYSTEM_LIGHT, &normalized),
        (IconFamily::System, IconStyle::Thin) => find_in_slice(system::SYSTEM_THIN, &normalized),
    } {
        return Some(icon);
    }

    // Fallbacks if family was unspecified
    if family.is_none() {
        match target_style {
            IconStyle::Regular => {
                if let Some(i) = find_in_slice(minimal::MINIMAL_REGULAR, &normalized) {
                    return Some(i);
                }
                if let Some(i) = find_in_slice(system::SYSTEM_REGULAR, &normalized) {
                    return Some(i);
                }
            }
            IconStyle::Solid => {
                if let Some(i) = find_in_slice(minimal::MINIMAL_SOLID, &normalized) {
                    return Some(i);
                }
                if let Some(i) = find_in_slice(system::SYSTEM_SOLID, &normalized) {
                    return Some(i);
                }
            }
            IconStyle::Bold => {
                if let Some(i) = find_in_slice(system::SYSTEM_BOLD, &normalized) {
                    return Some(i);
                }
            }
            IconStyle::Duotone => {
                if let Some(i) = find_in_slice(system::SYSTEM_DUOTONE, &normalized) {
                    return Some(i);
                }
            }
            IconStyle::Light => {
                if let Some(i) = find_in_slice(system::SYSTEM_LIGHT, &normalized) {
                    return Some(i);
                }
            }
            IconStyle::Thin => {
                if let Some(i) = find_in_slice(system::SYSTEM_THIN, &normalized) {
                    return Some(i);
                }
            }
        }
    }

    None
}

/// Look up an icon by query string (case-insensitive).
///
/// Supports optional family and style prefixes in the form:
/// - `"classic:home"` or `"home"` (Classic regular)
/// - `"minimal:home"` (Minimal regular)
/// - `"minimal:solid:home"` or `"minimal:fill:home"` (Minimal solid)
/// - `"system:home"` (System regular)
/// - `"system:bold:home"` (System bold)
/// - `"system:solid:home"` (System solid)
/// - `"system:duotone:home"` (System duotone)
/// - `"system:light:home"` (System light)
/// - `"system:thin:home"` (System thin)
/// - `"bold:home"` (System bold)
/// - `"solid:home"` (Minimal solid or System solid)
///
/// Aliases such as `"close"` for `"x"` are automatically resolved.
pub fn get_icon(query: &str) -> Option<&'static IconData> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }

    let parts: Vec<&str> = q
        .split(':')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    match parts.len() {
        3 => {
            let fam = IconFamily::parse_str(parts[0]);
            let sty = IconStyle::parse_str(parts[1]);
            get_icon_by(parts[2], fam, sty)
        }
        2 => {
            if let Some(fam) = IconFamily::parse_str(parts[0]) {
                get_icon_by(parts[1], Some(fam), None)
            } else if let Some(sty) = IconStyle::parse_str(parts[0]) {
                get_icon_by(parts[1], None, Some(sty))
            } else {
                get_icon_by(parts[1], None, None)
            }
        }
        _ => get_icon_by(q, None, None),
    }
}

/// Return an iterator over all icons across all families and styles.
pub fn all_icons_iter() -> impl Iterator<Item = &'static IconData> {
    classic::CLASSIC_REGULAR
        .iter()
        .chain(minimal::MINIMAL_REGULAR.iter())
        .chain(minimal::MINIMAL_SOLID.iter())
        .chain(system::SYSTEM_REGULAR.iter())
        .chain(system::SYSTEM_BOLD.iter())
        .chain(system::SYSTEM_SOLID.iter())
        .chain(system::SYSTEM_DUOTONE.iter())
        .chain(system::SYSTEM_LIGHT.iter())
        .chain(system::SYSTEM_THIN.iter())
}

/// Returns a slice of the default curated icons (Classic regular collection).
#[inline]
pub fn all_icons() -> &'static [IconData] {
    classic::CLASSIC_REGULAR
}

/// Search for icons whose name contains the given substring (case-insensitive).
pub fn search_icons(query: &str) -> Vec<&'static IconData> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return all_icons().iter().collect();
    }
    all_icons_iter()
        .filter(|icon| icon.name.contains(&q) || icon.category.contains(&q))
        .collect()
}

/// Retrieve all icons belonging to a specific category (case-insensitive).
pub fn icons_in_category(category: &str) -> Vec<&'static IconData> {
    let cat = category.trim().to_ascii_lowercase();
    all_icons_iter()
        .filter(|icon| icon.category.eq_ignore_ascii_case(&cat))
        .collect()
}

/// Returns a slice of common category names.
pub fn all_categories() -> &'static [&'static str] {
    &[
        "navigation",
        "actions",
        "communication",
        "media",
        "content",
        "users",
        "ui",
        "general",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_lookup_succeeds() {
        assert!(get_icon("home").is_some());
        assert!(get_icon("classic:home").is_some());
        assert_eq!(get_icon("close"), get_icon("x"));
    }

    #[test]
    fn minimal_lookup_succeeds() {
        assert!(get_icon("minimal:activity").is_some());
        assert!(get_icon("minimal:solid:activity").is_some());
        let solid = get_icon("minimal:solid:activity").unwrap();
        assert!(solid.is_fill);
        assert_eq!(solid.family, IconFamily::Minimal);
        assert_eq!(solid.style, IconStyle::Solid);
    }

    #[test]
    fn system_weights_lookup_succeeds() {
        let reg = get_icon("system:acorn").unwrap();
        assert_eq!(reg.family, IconFamily::System);
        assert_eq!(reg.style, IconStyle::Regular);

        let bold = get_icon("system:bold:acorn").unwrap();
        assert_eq!(bold.style, IconStyle::Bold);

        let fill = get_icon("system:solid:acorn").unwrap();
        assert_eq!(fill.style, IconStyle::Solid);

        let duotone = get_icon("system:duotone:acorn").unwrap();
        assert_eq!(duotone.style, IconStyle::Duotone);
    }

    #[test]
    fn unknown_icon_returns_none() {
        assert!(get_icon("nonexistent-icon-xyz").is_none());
        assert!(get_icon("").is_none());
    }
}
