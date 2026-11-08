//! # YAV Timeline & Audit Trail Visualizer (`<Timeline />`)
//!
//! Chronological audit trail and lifecycle visualizer with status badges,
//! connected vertical/horizontal track stems, and actor tags.

use yav_core::{Component, Element, Node};

use crate::utils::merge_classes;

/// Status indicators for timeline events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelineStatus {
    #[default]
    Info,
    Success,
    Warning,
    Danger,
}

impl TimelineStatus {
    #[must_use]
    pub fn dot_classes(&self) -> &'static str {
        match self {
            Self::Info => "bg-indigo-500 border-indigo-400/40 text-indigo-400",
            Self::Success => "bg-emerald-500 border-emerald-400/40 text-emerald-400",
            Self::Warning => "bg-amber-500 border-amber-400/40 text-amber-400",
            Self::Danger => "bg-rose-500 border-rose-400/40 text-rose-400",
        }
    }
}

/// An atomic chronological event in the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItem {
    pub id: String,
    pub title: String,
    pub timestamp: String,
    pub description: Option<String>,
    pub actor: Option<String>,
    pub status: TimelineStatus,
    pub badge: Option<String>,
}

impl TimelineItem {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            timestamp: timestamp.into(),
            description: None,
            actor: None,
            status: TimelineStatus::default(),
            badge: None,
        }
    }

    #[must_use]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    #[must_use]
    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    #[must_use]
    pub fn status(mut self, status: TimelineStatus) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

/// Interactive vertical or horizontal timeline component.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub id: String,
    pub items: Vec<TimelineItem>,
    pub class: String,
}

impl Timeline {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            class: String::new(),
        }
    }

    #[must_use]
    pub fn item(mut self, item: TimelineItem) -> Self {
        self.items.push(item);
        self
    }

    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = TimelineItem>) -> Self {
        self.items.extend(items);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for Timeline {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-timeline relative flex flex-col gap-6 pl-6 border-l-2 border-slate-800 my-2 select-text",
            &self.class,
        ]);

        let mut children = Vec::new();

        for item in &self.items {
            let status_dot_cls = item.status.dot_classes();

            let mut item_el = Element::new("div")
                .attr("class", "relative group flex flex-col gap-1")
                .attr("data-id", &item.id);

            // Left connected dot
            let dot = Element::new("div")
                .attr(
                    "class",
                    format!(
                        "absolute -left-[31px] top-1 w-3.5 h-3.5 rounded-full border-2 border-slate-950 ring-4 ring-slate-900 transition-transform group-hover:scale-125 {status_dot_cls}"
                    ),
                );
            item_el = item_el.child(dot);

            // Top Header: Title, Timestamp, Badge
            let mut top_line =
                Element::new("div").attr("class", "flex items-center gap-2 flex-wrap");
            top_line = top_line.child(
                Element::new("h4")
                    .attr("class", "text-sm font-semibold text-slate-100")
                    .child(item.title.as_str()),
            );

            top_line = top_line.child(
                Element::new("span")
                    .attr("class", "text-xs font-mono text-slate-400")
                    .child(item.timestamp.as_str()),
            );

            if let Some(badge) = &item.badge {
                top_line = top_line.child(
                    Element::new("span")
                        .attr(
                            "class",
                            "px-2 py-0.2 text-[10px] font-semibold uppercase rounded bg-slate-800 text-slate-300 border border-slate-700",
                        )
                        .child(badge.as_str()),
                );
            }

            if let Some(actor) = &item.actor {
                top_line = top_line.child(
                    Element::new("span")
                        .attr(
                            "class",
                            "px-1.5 py-0.2 text-[11px] rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 font-medium",
                        )
                        .child(actor.as_str()),
                );
            }

            item_el = item_el.child(top_line);

            if let Some(desc) = &item.description {
                item_el = item_el.child(
                    Element::new("p")
                        .attr("class", "text-xs text-slate-400 leading-relaxed max-w-xl")
                        .child(desc.as_str()),
                );
            }

            children.push(item_el);
        }

        let mut container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class);

        for child in children {
            container = container.child(child);
        }

        Node::Element(container)
    }
}

/// Helper function to create a new `Timeline`.
#[must_use]
pub fn timeline(id: impl Into<String>) -> Timeline {
    Timeline::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_timeline_rendering() {
        let tl = Timeline::new("order-lifecycle")
            .item(
                TimelineItem::new("t1", "Order Created", "10:00")
                    .description("Order #1042 initialized by customer.")
                    .actor("Alice")
                    .status(TimelineStatus::Info),
            )
            .item(
                TimelineItem::new("t2", "Payment Cleared", "10:05")
                    .badge("Stripe")
                    .status(TimelineStatus::Success),
            )
            .item(
                TimelineItem::new("t3", "Warehouse Dispatched", "11:30")
                    .actor("Carrier DHL")
                    .status(TimelineStatus::Warning),
            );

        let node = tl.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-timeline"));
        assert!(html.contains("order-lifecycle"));
        assert!(html.contains("Order Created"));
        assert!(html.contains("Payment Cleared"));
        assert!(html.contains("Warehouse Dispatched"));
        assert!(html.contains("Alice"));
        assert!(html.contains("Stripe"));
    }
}
