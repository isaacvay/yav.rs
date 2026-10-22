//! Floating UI primitives: Tooltip and Popover components.
//!
//! Provides accessible floating layers (`Tooltip`, `Popover`) with directional placement,
//! anchor arrows, smooth transitions, and keyboard/focus management.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// Directional placement for floating elements relative to their trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl Placement {
    /// Return the CSS positioning and transform classes for this placement.
    pub fn tooltip_classes(&self) -> (&'static str, &'static str) {
        match self {
            Placement::Top => (
                "bottom-full left-1/2 -translate-x-1/2 mb-2",
                "top-full left-1/2 -translate-x-1/2 border-t-gray-900 border-x-transparent border-b-transparent",
            ),
            Placement::Bottom => (
                "top-full left-1/2 -translate-x-1/2 mt-2",
                "bottom-full left-1/2 -translate-x-1/2 border-b-gray-900 border-x-transparent border-t-transparent",
            ),
            Placement::Left => (
                "right-full top-1/2 -translate-y-1/2 mr-2",
                "left-full top-1/2 -translate-y-1/2 border-l-gray-900 border-y-transparent border-r-transparent",
            ),
            Placement::Right => (
                "left-full top-1/2 -translate-y-1/2 ml-2",
                "right-full top-1/2 -translate-y-1/2 border-r-gray-900 border-y-transparent border-l-transparent",
            ),
        }
    }
}

/// A floating tooltip providing contextual information on hover or focus.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tooltip {
    text: String,
    placement: Placement,
    class: String,
    children: Vec<Node>,
}

impl Tooltip {
    /// Create a new tooltip with the given text message.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            placement: Placement::Top,
            class: String::new(),
            children: Vec::new(),
        }
    }

    /// Set the directional placement.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Add custom classes to the tooltip bubble.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set the trigger element.
    pub fn child(mut self, child: impl IntoNode) -> Self {
        self.children.push(child.into_node());
        self
    }

    /// Render the tooltip wrapper into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let (pos_class, arrow_class) = self.placement.tooltip_classes();

        let mut wrapper = Element::new("div")
            .attr("class", "relative inline-flex group")
            .attr("data-tooltip-trigger", "true");

        // Trigger children
        for child in self.children {
            wrapper = wrapper.child(child);
        }

        // Floating Bubble
        let bubble_base = "pointer-events-none absolute z-50 whitespace-nowrap rounded-lg bg-gray-900 px-2.5 py-1 text-xs font-medium text-white shadow-lg opacity-0 scale-95 transition-all duration-200 group-hover:opacity-100 group-hover:scale-100 group-focus-within:opacity-100 group-focus-within:scale-100 dark:bg-gray-800 dark:border dark:border-gray-700";
        let full_bubble = merge_classes(&[bubble_base, pos_class, &self.class]);

        let mut bubble = Element::new("div")
            .attr("class", full_bubble)
            .attr("role", "tooltip")
            .child(Node::text(self.text));

        // Arrow
        let arrow = Element::new("span")
            .attr("class", format!("absolute border-4 {arrow_class}"))
            .attr("aria-hidden", "true");
        bubble = bubble.child(Node::Element(arrow));

        wrapper = wrapper.child(Node::Element(bubble));
        Node::Element(wrapper)
    }
}

impl IntoNode for Tooltip {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Tooltip> for Node {
    fn from(t: Tooltip) -> Self {
        t.render()
    }
}

impl Component for Tooltip {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`Tooltip`].
pub fn tooltip(text: impl Into<String>) -> Tooltip {
    Tooltip::new(text)
}

/// A click-activated floating popover card for rich interactive content.
#[derive(Debug, Clone, PartialEq)]
pub struct Popover {
    id: String,
    placement: Placement,
    trigger: Option<Node>,
    content: Option<Node>,
    class: String,
}

impl Default for Popover {
    fn default() -> Self {
        Self {
            id: "popover".to_string(),
            placement: Placement::Bottom,
            trigger: None,
            content: None,
            class: String::new(),
        }
    }
}

impl Popover {
    /// Create a new popover.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set container ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set placement.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the trigger element.
    pub fn trigger(mut self, trigger: impl IntoNode) -> Self {
        self.trigger = Some(trigger.into_node());
        self
    }

    /// Set the popover card content.
    pub fn content(mut self, content: impl IntoNode) -> Self {
        self.content = Some(content.into_node());
        self
    }

    /// Add custom classes to the floating card.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the popover.
    pub fn render(self) -> Node {
        let mut root = Element::new("div")
            .attr("id", &self.id)
            .attr("class", "relative inline-block")
            .attr("data-popover-root", "true");

        // Trigger
        if let Some(trig) = self.trigger {
            let trig_wrap = Element::new("div")
                .attr("class", "cursor-pointer")
                .attr("data-popover-toggle", "true")
                .child(trig);
            root = root.child(Node::Element(trig_wrap));
        }

        // Panel Position
        let pos_class = match self.placement {
            Placement::Top => "bottom-full left-1/2 -translate-x-1/2 mb-2",
            Placement::Bottom => "top-full left-1/2 -translate-x-1/2 mt-2",
            Placement::Left => "right-full top-1/2 -translate-y-1/2 mr-2",
            Placement::Right => "left-full top-1/2 -translate-y-1/2 ml-2",
        };

        let card_base = "hidden absolute z-50 w-72 rounded-xl bg-white p-4 shadow-xl ring-1 ring-black/5 dark:bg-gray-800 dark:ring-white/10";
        let card_class = merge_classes(&[card_base, pos_class, &self.class]);

        let mut panel = Element::new("div")
            .attr("class", card_class)
            .attr("role", "dialog")
            .attr("data-popover-panel", "true");

        if let Some(cnt) = self.content {
            panel = panel.child(cnt);
        }

        root = root.child(Node::Element(panel));
        Node::Element(root)
    }
}

impl IntoNode for Popover {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Popover> for Node {
    fn from(p: Popover) -> Self {
        p.render()
    }
}

impl Component for Popover {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`Popover`].
pub fn popover() -> Popover {
    Popover::new()
}

/// Properties for constructing a [`Tooltip`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TooltipProps {
    pub text: String,
    pub placement: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Tooltip`], enabling `<Tooltip text="...">...</Tooltip>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Tooltip(props: TooltipProps) -> Node {
    let mut t = Tooltip::new(props.text);
    if let Some(p) = props.placement {
        match p.to_lowercase().as_str() {
            "top" => t = t.placement(Placement::Top),
            "bottom" => t = t.placement(Placement::Bottom),
            "left" => t = t.placement(Placement::Left),
            "right" => t = t.placement(Placement::Right),
            _ => {}
        }
    }
    if !props.class.is_empty() {
        t = t.class(props.class);
    }
    if !props.children.is_empty() {
        t = t.child(Node::Fragment(props.children));
    }
    t.render()
}

/// Properties for constructing a [`Popover`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopoverProps {
    pub id: Option<String>,
    pub placement: Option<String>,
    pub trigger: Option<Node>,
    pub content: Option<Node>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`Popover`], enabling `<Popover ...>...</Popover>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Popover(props: PopoverProps) -> Node {
    let mut p = Popover::new();
    if let Some(id) = props.id {
        p = p.id(id);
    }
    if let Some(pl) = props.placement {
        match pl.to_lowercase().as_str() {
            "top" => p = p.placement(Placement::Top),
            "bottom" => p = p.placement(Placement::Bottom),
            "left" => p = p.placement(Placement::Left),
            "right" => p = p.placement(Placement::Right),
            _ => {}
        }
    }
    if let Some(trig) = props.trigger {
        p = p.trigger(trig);
    }
    if let Some(cnt) = props.content {
        p = p.content(cnt);
    } else if !props.children.is_empty() {
        p = p.content(Node::Fragment(props.children));
    }
    if !props.class.is_empty() {
        p = p.class(props.class);
    }
    p.render()
}

/// Client-side script handling Popover outside-click dismiss and Escape key.
pub fn floating_script() -> Node {
    let script = r#"<script>
(function() {
  function initPopovers() {
    document.querySelectorAll('[data-popover-root]').forEach(root => {
      if (root.__popover_init) return;
      root.__popover_init = true;

      const toggle = root.querySelector('[data-popover-toggle]');
      const panel = root.querySelector('[data-popover-panel]');
      if (!toggle || !panel) return;

      toggle.addEventListener('click', (e) => {
        e.stopPropagation();
        const isOpen = !panel.classList.contains('hidden');
        document.querySelectorAll('[data-popover-panel]').forEach(p => p.classList.add('hidden'));
        if (!isOpen) {
          panel.classList.remove('hidden');
        }
      });
    });

    document.addEventListener('click', (e) => {
      document.querySelectorAll('[data-popover-panel]').forEach(panel => {
        if (!panel.contains(e.target)) {
          panel.classList.add('hidden');
        }
      });
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        document.querySelectorAll('[data-popover-panel]').forEach(panel => {
          panel.classList.add('hidden');
        });
      }
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initPopovers);
  } else {
    initPopovers();
  }
})();
</script>"#;
    Node::raw(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn tooltip_renders_bubble_and_role() {
        let t = tooltip("Carbon ceramic brakes")
            .placement(Placement::Bottom)
            .child(Node::text("Hover me"))
            .render();

        let html = render_to_string(&t);
        assert!(html.contains("role=\"tooltip\""));
        assert!(html.contains("Carbon ceramic brakes"));
        assert!(html.contains("top-full"));
        assert!(html.contains("Hover me"));
    }

    #[test]
    fn popover_renders_toggle_and_panel() {
        let p = popover()
            .id("specs-popover")
            .trigger(Node::text("Specs"))
            .content(Node::text("V8 Twin-Turbo 620 hp"))
            .render();

        let html = render_to_string(&p);
        assert!(html.contains("id=\"specs-popover\""));
        assert!(html.contains("data-popover-toggle=\"true\""));
        assert!(html.contains("data-popover-panel=\"true\""));
        assert!(html.contains("V8 Twin-Turbo 620 hp"));
    }
}
