//! # YAV Virtualized Infinite List (`<VirtualList />`)
//!
//! High-density virtualized viewport container capable of scrolling 100,000+
//! items at 60 FPS by only mounting visible DOM rows plus an overscan buffer
//! (rivaling `@tanstack/react-virtual` and `react-window`).

use yav_core::{Component, Element, Node};

use crate::utils::merge_classes;

/// Props for [`VirtualList`].
#[derive(Debug, Clone)]
pub struct VirtualListProps {
    pub id: String,
    pub total_count: usize,
    pub row_height_px: usize,
    pub viewport_height_px: usize,
    pub start_index: usize,
    pub overscan: usize,
    pub fetch_url: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for VirtualListProps {
    fn default() -> Self {
        Self {
            id: "yav-virtual-list".to_string(),
            total_count: 0,
            row_height_px: 40,
            viewport_height_px: 480,
            start_index: 0,
            overscan: 5,
            fetch_url: None,
            class: String::new(),
            children: Vec::new(),
        }
    }
}

/// Virtualized scroll container component.
#[derive(Debug, Clone)]
pub struct VirtualList {
    pub id: String,
    pub total_count: usize,
    pub row_height_px: usize,
    pub viewport_height_px: usize,
    pub start_index: usize,
    pub overscan: usize,
    pub fetch_url: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for VirtualList {
    fn default() -> Self {
        Self::from_props(VirtualListProps::default())
    }
}

impl VirtualList {
    pub fn from_props(props: VirtualListProps) -> Self {
        Self {
            id: props.id,
            total_count: props.total_count,
            row_height_px: props.row_height_px,
            viewport_height_px: props.viewport_height_px,
            start_index: props.start_index,
            overscan: props.overscan,
            fetch_url: props.fetch_url,
            class: props.class,
            children: props.children,
        }
    }

    pub fn new(total_count: usize, row_height_px: usize) -> Self {
        Self {
            total_count,
            row_height_px,
            ..Default::default()
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn viewport_height_px(mut self, px: usize) -> Self {
        self.viewport_height_px = px;
        self
    }

    pub fn start_index(mut self, idx: usize) -> Self {
        self.start_index = idx;
        self
    }

    pub fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    pub fn fetch_url(mut self, url: impl Into<String>) -> Self {
        self.fetch_url = Some(url.into());
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, node: impl Into<Node>) -> Self {
        self.children.push(node.into());
        self
    }

    pub fn items(mut self, nodes: Vec<Node>) -> Self {
        self.children = nodes;
        self
    }
}

impl Component for VirtualList {
    fn render(&self) -> Node {
        let total_height_px = self.total_count.saturating_mul(self.row_height_px);
        let offset_y_px = self.start_index.saturating_mul(self.row_height_px);
        let classes = merge_classes(&[
            "yav-virtual-list relative overflow-y-auto border border-gray-200 dark:border-gray-800 rounded-lg",
            &self.class,
        ]);

        let mut viewport = Element::new("div")
            .attr("id", &self.id)
            .attr("class", &classes)
            .attr("role", "list")
            .attr("style", format!("height:{}px;", self.viewport_height_px))
            .attr("data-total-count", self.total_count.to_string())
            .attr("data-row-height", self.row_height_px.to_string())
            .attr("data-overscan", self.overscan.to_string());

        if let Some(ref url) = self.fetch_url {
            viewport = viewport.attr("yav:virtual-fetch", url);
        }

        let mut translated = Element::new("div")
            .attr("class", "yav-virtual-window absolute inset-x-0 top-0")
            .attr("style", format!("transform:translateY({}px);", offset_y_px));

        for (i, item) in self.children.iter().enumerate() {
            let row_idx = self.start_index + i;
            let row_el = Element::new("div")
                .attr("class", "yav-virtual-row flex items-center px-3 border-b border-gray-100 dark:border-gray-800/60")
                .attr("role", "listitem")
                .attr("data-index", row_idx.to_string())
                .attr("style", format!("height:{}px;", self.row_height_px))
                .child(item.clone());
            translated = translated.child(Node::Element(row_el));
        }

        let runway = Element::new("div")
            .attr("class", "yav-virtual-runway relative w-full")
            .attr("style", format!("height:{}px;", total_height_px))
            .child(Node::Element(translated));

        viewport = viewport.child(Node::Element(runway));
        Node::Element(viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_virtual_list_computes_total_runway_height() {
        let vlist = VirtualList::new(100_000, 36)
            .viewport_height_px(500)
            .start_index(100)
            .fetch_url("/api/logs/slice")
            .child(Node::Text("Log #100".to_string()))
            .render();

        let html = render_to_string(&vlist);
        assert!(html.contains("height:3600000px;"));
        assert!(html.contains("transform:translateY(3600px);"));
        assert!(html.contains("data-index=\"100\""));
        assert!(html.contains("yav:virtual-fetch=\"/api/logs/slice\""));
    }
}
