//! Icon allowlist recorded at build time by the `icon-minify` feature.
//!
//! Set `YAV_ICON_ALLOWLIST=home,user,settings` before building to record a
//! sorted list of icon names. This module is a deterministic audit/generator
//! hook: it does not remove icons from the compiled catalog, because silently
//! changing the public API from an environment variable would be unsafe for a
//! library crate.

/// Names supplied through `YAV_ICON_ALLOWLIST`, sorted and deduplicated.
pub fn allowed_icon_names() -> impl Iterator<Item = &'static str> {
    const ALLOWLIST: &str = include_str!(concat!(env!("OUT_DIR"), "/allowed_icons.txt"));
    ALLOWLIST.lines().filter(|line| !line.is_empty())
}

/// Whether `name` was supplied through `YAV_ICON_ALLOWLIST`.
pub fn is_allowed(name: &str) -> bool {
    allowed_icon_names().any(|allowed| allowed == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_is_readable() {
        let names: Vec<_> = allowed_icon_names().collect();
        assert!(names.windows(2).all(|w| w[0] <= w[1]));
    }
}
