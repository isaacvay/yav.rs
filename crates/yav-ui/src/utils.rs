//! Utility functions and helpers for yav-ui components.

/// Combine multiple CSS class slices into a single normalized class string.
///
/// Empty or whitespace-only elements are filtered out, and valid classes
/// are joined with a single space.
///
/// # Examples
/// ```
/// use yav_ui::utils::merge_classes;
///
/// let class = merge_classes(&["btn", "", "btn-primary", "   "]);
/// assert_eq!(class, "btn btn-primary");
/// ```
pub fn merge_classes(classes: &[&str]) -> String {
    classes
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_classes_empty() {
        assert_eq!(merge_classes(&[]), "");
        assert_eq!(merge_classes(&["", "   "]), "");
    }

    #[test]
    fn test_merge_classes_multiple() {
        assert_eq!(
            merge_classes(&["px-4", "py-2", "", "text-sm"]),
            "px-4 py-2 text-sm"
        );
    }
}
