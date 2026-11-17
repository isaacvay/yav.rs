//! Toast notification components and auto-dismiss runtime.
//!
//! Provides accessible, styled toast notifications for user feedback and flash messages.

use yav_core::{Element, Node};

/// Notification severity level for toasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    /// CSS classes for border, background, and icon color.
    pub fn class(&self) -> &'static str {
        match self {
            Self::Info => {
                "border-blue-500 bg-blue-50 text-blue-900 dark:bg-blue-950 dark:text-blue-100"
            }
            Self::Success => {
                "border-green-500 bg-green-50 text-green-900 dark:bg-green-950 dark:text-green-100"
            }
            Self::Warning => {
                "border-amber-500 bg-amber-50 text-amber-900 dark:bg-amber-950 dark:text-amber-100"
            }
            Self::Error => {
                "border-red-500 bg-red-50 text-red-900 dark:bg-red-950 dark:text-red-100"
            }
        }
    }

    /// Icon SVG for the severity level.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => {
                r#"<svg class="w-5 h-5 text-blue-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>"#
            }
            Self::Success => {
                r#"<svg class="w-5 h-5 text-green-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>"#
            }
            Self::Warning => {
                r#"<svg class="w-5 h-5 text-amber-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/></svg>"#
            }
            Self::Error => {
                r#"<svg class="w-5 h-5 text-red-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>"#
            }
        }
    }
}

/// Properties for the [`Toast`] component.
#[derive(Debug, Clone, Default)]
pub struct ToastProps {
    /// Notification message text.
    pub message: String,
    /// Optional title header.
    pub title: Option<String>,
    /// Severity level (defaults to Info).
    pub level: ToastLevel,
    /// Whether the user can manually close this toast (default true).
    pub dismissible: Option<bool>,
    /// Additional CSS classes.
    pub class: Option<String>,
}

/// Functional RSX component: `<Toast message="..." level=ToastLevel::Success />`.
#[allow(non_snake_case)]
pub fn Toast(props: ToastProps) -> Node {
    let dismissible = props.dismissible.unwrap_or(true);
    let level_class = props.level.class();
    let custom_class = props.class.as_deref().unwrap_or("");
    let full_class = format!(
        "yav-toast pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-lg border-l-4 p-4 shadow-lg transition-all duration-300 transform translate-y-0 opacity-100 {level_class} {custom_class}"
    );

    let mut el = Element::new("div")
        .attr("class", full_class)
        .attr("role", "alert")
        .attr("data-toast", "true");

    // Icon
    el = el.child(Node::raw(props.level.icon()));

    // Content container
    let mut content = Element::new("div").attr("class", "flex-1 min-w-0");
    if let Some(title) = props.title {
        content = content.child(Node::Element(
            Element::new("h4")
                .attr("class", "text-sm font-semibold mb-0.5")
                .child(Node::text(title)),
        ));
    }
    content = content.child(Node::Element(
        Element::new("p")
            .attr("class", "text-sm leading-tight")
            .child(Node::text(props.message)),
    ));
    el = el.child(Node::Element(content));

    // Close button
    if dismissible {
        let close_btn = Element::new("button")
            .attr("type", "button")
            .attr("class", "text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors shrink-0")
            .attr("aria-label", "Close")
            .attr("data-dismiss", "toast")
            .child(Node::raw(r#"<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>"#));
        el = el.child(Node::Element(close_btn));
    }

    Node::Element(el)
}

/// Properties for [`ToastContainer`].
#[derive(Debug, Clone, Default)]
pub struct ToastContainerProps {
    pub children: Vec<Node>,
    /// Placement on screen (e.g. "bottom-right", "top-right", "top-center"). Defaults to "bottom-right".
    pub position: Option<String>,
}

/// Functional RSX component: `<ToastContainer> <Toast ... /> </ToastContainer>`.
#[allow(non_snake_case)]
pub fn ToastContainer(props: ToastContainerProps) -> Node {
    let pos_class = match props.position.as_deref().unwrap_or("bottom-right") {
        "top-right" => "top-4 right-4",
        "top-left" => "top-4 left-4",
        "top-center" => "top-4 left-1/2 -translate-x-1/2",
        "bottom-left" => "bottom-4 left-4",
        "bottom-center" => "bottom-4 left-1/2 -translate-x-1/2",
        _ => "bottom-4 right-4",
    };

    let class =
        format!("fixed z-50 pointer-events-none flex flex-col gap-2 max-w-sm w-full {pos_class}");
    let mut el = Element::new("div")
        .attr("class", class)
        .attr("id", "yav-toast-container")
        .attr("aria-live", "polite");

    for child in props.children {
        el = el.child(child);
    }

    Node::Element(el)
}

/// Injects a lightweight script to automatically dismiss toasts after 5 seconds.
pub fn toast_script() -> Node {
    let script = r#"(function(){
function initToasts() {
  document.querySelectorAll('[data-toast="true"]:not([data-initialized])').forEach(toast => {
    toast.setAttribute('data-initialized', 'true');
    setTimeout(() => {
      toast.style.opacity = '0';
      toast.style.transform = 'translateY(8px)';
      setTimeout(() => toast.remove(), 300);
    }, 5000);
  });
}
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initToasts);
} else {
  initToasts();
}
window.addEventListener('yav:navigate', initToasts);
})();"#;

    let mut el = Element::new("script");
    el = el.child(Node::raw(script));
    Node::Element(el)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_rendering() {
        use yav_core::render_to_string;

        let toast = Toast(ToastProps {
            message: "Enregistré avec succès !".into(),
            title: Some("Succès".into()),
            level: ToastLevel::Success,
            dismissible: Some(true),
            class: None,
        });

        let html = render_to_string(&toast);
        assert!(html.contains("Enregistré avec succès !"));
        assert!(html.contains("Succès"));
        assert!(html.contains("border-green-500"));
        assert!(html.contains(r#"role="alert""#));
    }

    #[test]
    fn test_toast_container() {
        use yav_core::render_to_string;

        let container = ToastContainer(ToastContainerProps {
            children: vec![Toast(ToastProps {
                message: "Test".into(),
                ..Default::default()
            })],
            position: Some("top-right".into()),
        });

        let html = render_to_string(&container);
        assert!(html.contains("top-4 right-4"));
        assert!(html.contains("Test"));
    }
}
