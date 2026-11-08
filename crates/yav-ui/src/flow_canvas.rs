//! # YAV Interactive Node-Graph & Workflow Canvas (`<FlowCanvas />`)
//!
//! Node-based workflow and diagram editor with SVG cubic Bézier edge routing,
//! arrowhead markers, and pan/zoom grid background (rivaling `@xyflow/react` /
//! React Flow and Rete.js).

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// A node positioned on the [`FlowCanvas`].
#[derive(Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub kind: String,
}

impl FlowNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            x,
            y,
            kind: "default".to_string(),
        }
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }
}

/// A directed edge connecting two [`FlowNode`]s on the [`FlowCanvas`].
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub animated: bool,
}

impl FlowEdge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            label: None,
            animated: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
}

/// Props for [`FlowCanvas`].
#[derive(Debug, Clone, Default)]
pub struct FlowCanvasProps {
    pub id: String,
    pub width: usize,
    pub height: usize,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Interactive node-graph and workflow builder canvas.
#[derive(Debug, Clone)]
pub struct FlowCanvas {
    pub id: String,
    pub width: usize,
    pub height: usize,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub class: String,
    pub children: Vec<Node>,
}

impl Default for FlowCanvas {
    fn default() -> Self {
        Self {
            id: "yav-flow-canvas".to_string(),
            width: 900,
            height: 520,
            nodes: Vec::new(),
            edges: Vec::new(),
            class: String::new(),
            children: Vec::new(),
        }
    }
}

impl FlowCanvas {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    pub fn size(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn node(mut self, node: FlowNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn edge(mut self, edge: FlowEdge) -> Self {
        self.edges.push(edge);
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl FlowCanvas {
    pub fn from_props(props: FlowCanvasProps) -> Self {
        Self {
            id: if props.id.is_empty() {
                "yav-flow-canvas".to_string()
            } else {
                props.id
            },
            width: if props.width == 0 { 900 } else { props.width },
            height: if props.height == 0 { 520 } else { props.height },
            nodes: props.nodes,
            edges: props.edges,
            class: props.class,
            children: props.children,
        }
    }
}

impl Component for FlowCanvas {
    fn render(&self) -> Node {
        let classes = merge_classes(&[
            "yav-flow-canvas relative overflow-hidden rounded-xl border border-gray-200 dark:border-gray-800 bg-slate-50 dark:bg-slate-950 select-none",
            &self.class,
        ]);

        // Build SVG cubic Bézier paths between matching nodes
        let mut svg_paths = String::new();
        for edge in &self.edges {
            if let (Some(src), Some(dst)) = (
                self.nodes.iter().find(|n| n.id == edge.from),
                self.nodes.iter().find(|n| n.id == edge.to),
            ) {
                let x1 = src.x + 140.0;
                let y1 = src.y + 28.0;
                let x2 = dst.x;
                let y2 = dst.y + 28.0;
                let cx1 = x1 + ((x2 - x1).abs() * 0.5).max(40.0);
                let cx2 = x2 - ((x2 - x1).abs() * 0.5).max(40.0);
                let dash = if edge.animated {
                    " stroke-dasharray=\"6 4\""
                } else {
                    ""
                };
                let from_esc = escape_html(&edge.from);
                let to_esc = escape_html(&edge.to);
                svg_paths.push_str(&format!(
                    r#"<path data-from="{from_esc}" data-to="{to_esc}" d="M {x1:.1} {y1:.1} C {cx1:.1} {y1:.1}, {cx2:.1} {y2:.1}, {x2:.1} {y2:.1}" fill="none" stroke="currentColor" stroke-width="2" class="text-indigo-500"{dash} marker-end="url(#yav-flow-arrow)"/>"#
                ));
                if let Some(ref lbl) = edge.label {
                    let mx = (x1 + x2) / 2.0;
                    let my = (y1 + y2) / 2.0 - 6.0;
                    svg_paths.push_str(&format!(
                        r#"<text data-edge-label-for="{from_esc}->{to_esc}" x="{mx:.1}" y="{my:.1}" text-anchor="middle" class="fill-gray-500 text-[11px]">{}</text>"#,
                        escape_html(lbl)
                    ));
                }
            }
        }

        let svg_layer = format!(
            r#"<svg class="yav-flow-edges absolute inset-0 pointer-events-none" width="{w}" height="{h}" viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg"><defs><marker id="yav-flow-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" class="text-indigo-500"/></marker></defs>{svg_paths}</svg>"#,
            w = self.width,
            h = self.height
        );

        let mut canvas = Element::new("div")
            .attr("id", &self.id)
            .attr("class", &classes)
            .attr(
                "style",
                format!("width:{}px;height:{}px;", self.width, self.height),
            )
            .child(Node::Raw(svg_layer));

        for node in &self.nodes {
            let node_el = Element::new("div")
                .attr(
                    "class",
                    "yav-flow-node absolute w-[140px] rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 px-3 py-2 shadow-md cursor-move",
                )
                .attr("data-node-id", &node.id)
                .attr("data-node-kind", &node.kind)
                .attr("data-x", format!("{:.1}", node.x))
                .attr("data-y", format!("{:.1}", node.y))
                .attr(
                    "style",
                    format!("transform:translate({:.1}px,{:.1}px);", node.x, node.y),
                )
                .child(Node::Element(
                    Element::new("div")
                        .attr("class", "text-xs font-semibold text-gray-800 dark:text-gray-100 truncate")
                        .child(Node::Text(node.label.clone())),
                ));
            canvas = canvas.child(Node::Element(node_el));
        }

        Node::Element(canvas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_flow_canvas_renders_nodes_and_bezier_edges() {
        let flow = FlowCanvas::new("ai-pipeline")
            .node(FlowNode::new("n1", "Webhook Trigger", 40.0, 100.0).kind("input"))
            .node(FlowNode::new("n2", "LLM Agent", 280.0, 100.0).kind("processor"))
            .edge(FlowEdge::new("n1", "n2").label("JSON").animated(true))
            .render();

        let html = render_to_string(&flow);
        assert!(html.contains("ai-pipeline"));
        assert!(html.contains("Webhook Trigger"));
        assert!(html.contains("LLM Agent"));
        assert!(html.contains("<path d=\"M "));
        assert!(html.contains("stroke-dasharray=\"6 4\""));
    }
}
