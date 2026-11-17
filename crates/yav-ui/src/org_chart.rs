//! # YAV Interactive Org Chart & Family Tree SVG Graph (`<OrgChart />`)
//!
//! Hierarchical organization chart and family tree component with orthogonal
//! SVG connectors, avatar badges, collapsible branches, and responsive pan/zoom shell.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Orientation of the organizational tree hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrgOrientation {
    /// Top to bottom (traditional executive org chart).
    #[default]
    Vertical,
    /// Left to right (horizontal flowchart style).
    Horizontal,
}

/// A person, role, or team member node in the organizational chart.
#[derive(Debug, Clone, PartialEq)]
pub struct OrgNode {
    pub id: String,
    pub name: String,
    pub title: String,
    pub department: Option<String>,
    pub avatar_url: Option<String>,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub collapsed: bool,
    /// Computed 2D coordinates for SVG rendering.
    pub x: f64,
    pub y: f64,
}

impl OrgNode {
    /// Create a new org chart node with id, name, and job title.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            title: title.into(),
            department: None,
            avatar_url: None,
            parent_id: None,
            tags: Vec::new(),
            collapsed: false,
            x: 0.0,
            y: 0.0,
        }
    }

    /// Set parent manager / leader id.
    #[must_use]
    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set department or business unit.
    #[must_use]
    pub fn department(mut self, dept: impl Into<String>) -> Self {
        self.department = Some(dept.into());
        self
    }

    /// Set avatar image URL.
    #[must_use]
    pub fn avatar(mut self, url: impl Into<String>) -> Self {
        self.avatar_url = Some(url.into());
        self
    }

    /// Add a skill or role tag.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Mark this branch as initially collapsed.
    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

/// Props for constructing an [`OrgChart`].
#[derive(Debug, Clone, Default)]
pub struct OrgChartProps {
    pub id: String,
    pub nodes: Vec<OrgNode>,
    pub orientation: OrgOrientation,
    pub node_width: f64,
    pub node_height: f64,
    pub gap_x: f64,
    pub gap_y: f64,
    pub class: String,
}

/// Interactive Organizational Chart component with SVG connectors.
#[derive(Debug, Clone)]
pub struct OrgChart {
    pub id: String,
    pub nodes: Vec<OrgNode>,
    pub orientation: OrgOrientation,
    pub node_width: f64,
    pub node_height: f64,
    pub gap_x: f64,
    pub gap_y: f64,
    pub class: String,
}

impl Default for OrgChart {
    fn default() -> Self {
        Self {
            id: "yav-org-chart".to_string(),
            nodes: Vec::new(),
            orientation: OrgOrientation::Vertical,
            node_width: 220.0,
            node_height: 96.0,
            gap_x: 48.0,
            gap_y: 64.0,
            class: String::new(),
        }
    }
}

impl OrgChart {
    /// Create a new org chart builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set unique container DOM id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Add a node to the chart.
    #[must_use]
    pub fn node(mut self, node: OrgNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Set list of nodes.
    #[must_use]
    pub fn nodes(mut self, nodes: Vec<OrgNode>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Set tree orientation (Vertical or Horizontal).
    #[must_use]
    pub fn orientation(mut self, orientation: OrgOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set CSS classes on the outer container.
    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Compute hierarchical layered coordinates for all nodes in the tree.
    fn compute_layout(&self) -> (Vec<OrgNode>, f64, f64) {
        if self.nodes.is_empty() {
            return (Vec::new(), 400.0, 300.0);
        }

        let mut positioned = self.nodes.clone();
        // Group nodes by depth level from root
        let mut depths: Vec<usize> = vec![0; positioned.len()];

        for i in 0..positioned.len() {
            let mut curr = &positioned[i];
            let mut depth = 0;
            while let Some(ref p_id) = curr.parent_id {
                if let Some(parent_idx) = positioned.iter().position(|n| &n.id == p_id) {
                    depth += 1;
                    curr = &positioned[parent_idx];
                    if depth > 50 {
                        break; // Loop cycle protection
                    }
                } else {
                    break;
                }
            }
            depths[i] = depth;
        }

        let max_depth = *depths.iter().max().unwrap_or(&0);
        let mut max_width_coord = 0.0f64;
        let mut max_height_coord = 0.0f64;

        // Position nodes level by level
        for d in 0..=max_depth {
            let level_indices: Vec<usize> = depths
                .iter()
                .enumerate()
                .filter(|(_, &depth)| depth == d)
                .map(|(i, _)| i)
                .collect();

            for (col, &idx) in level_indices.iter().enumerate() {
                match self.orientation {
                    OrgOrientation::Vertical => {
                        let start_x = 40.0;
                        let x = start_x + (col as f64) * (self.node_width + self.gap_x);
                        let y = 40.0 + (d as f64) * (self.node_height + self.gap_y);
                        positioned[idx].x = x;
                        positioned[idx].y = y;
                        if x + self.node_width + 40.0 > max_width_coord {
                            max_width_coord = x + self.node_width + 40.0;
                        }
                        if y + self.node_height + 40.0 > max_height_coord {
                            max_height_coord = y + self.node_height + 40.0;
                        }
                    }
                    OrgOrientation::Horizontal => {
                        let x = 40.0 + (d as f64) * (self.node_width + self.gap_x);
                        let y = 40.0 + (col as f64) * (self.node_height + self.gap_y);
                        positioned[idx].x = x;
                        positioned[idx].y = y;
                        if x + self.node_width + 40.0 > max_width_coord {
                            max_width_coord = x + self.node_width + 40.0;
                        }
                        if y + self.node_height + 40.0 > max_height_coord {
                            max_height_coord = y + self.node_height + 40.0;
                        }
                    }
                }
            }
        }

        (
            positioned,
            max_width_coord.max(600.0),
            max_height_coord.max(400.0),
        )
    }
}

impl Component for OrgChart {
    fn render(&self) -> Node {
        let (positioned_nodes, canvas_width, canvas_height) = self.compute_layout();

        let container_class = merge_classes(&[
            "yav-org-chart relative overflow-auto rounded-2xl border border-gray-200 dark:border-gray-800 bg-slate-50/50 dark:bg-slate-950/50 p-6 select-none shadow-inner",
            &self.class,
        ]);

        // Build orthogonal SVG connecting lines between parents and children
        let mut connectors = String::new();
        for node in &positioned_nodes {
            if let Some(ref p_id) = node.parent_id {
                if let Some(parent) = positioned_nodes.iter().find(|n| &n.id == p_id) {
                    match self.orientation {
                        OrgOrientation::Vertical => {
                            let p_x = parent.x + self.node_width / 2.0;
                            let p_y = parent.y + self.node_height;
                            let c_x = node.x + self.node_width / 2.0;
                            let c_y = node.y;
                            let mid_y = p_y + (c_y - p_y) / 2.0;

                            connectors.push_str(&format!(
                                "<path d=\"M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"text-gray-300 dark:text-gray-700 transition-colors duration-200\" />\n",
                                p_x, p_y, p_x, mid_y, c_x, mid_y, c_x, c_y
                            ));
                        }
                        OrgOrientation::Horizontal => {
                            let p_x = parent.x + self.node_width;
                            let p_y = parent.y + self.node_height / 2.0;
                            let c_x = node.x;
                            let c_y = node.y + self.node_height / 2.0;
                            let mid_x = p_x + (c_x - p_x) / 2.0;

                            connectors.push_str(&format!(
                                "<path d=\"M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1}\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"text-gray-300 dark:text-gray-700 transition-colors duration-200\" />\n",
                                p_x, p_y, mid_x, p_y, mid_x, c_y, c_x, c_y
                            ));
                        }
                    }
                }
            }
        }

        // Build HTML cards for each node positioned absolutely
        let mut node_elements = Vec::new();
        for node in &positioned_nodes {
            let avatar_markup = if let Some(ref url) = node.avatar_url {
                // Validate the URL scheme (blocking javascript:/vbscript:/...) and
                // attribute-escape it. A refused URL falls back to an empty src.
                let safe_url = yav_core::escape_url(url).unwrap_or_default();
                format!(
                    "<img src=\"{}\" alt=\"{}\" class=\"h-10 w-10 rounded-full object-cover ring-2 ring-indigo-500/30\" />",
                    safe_url,
                    escape_html(&node.name)
                )
            } else {
                let initials = node
                    .name
                    .split_whitespace()
                    .filter_map(|w| w.chars().next())
                    .take(2)
                    .collect::<String>()
                    .to_uppercase();
                format!(
                    "<div class=\"flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-gradient-to-tr from-indigo-500 to-purple-600 font-bold text-white text-xs shadow\">{}</div>",
                    escape_html(&initials)
                )
            };

            let dept_badge = if let Some(ref dept) = node.department {
                format!(
                    "<span class=\"inline-flex items-center rounded-md bg-indigo-50 dark:bg-indigo-950/60 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800/50\">{}</span>",
                    escape_html(dept)
                )
            } else {
                String::new()
            };

            let card_html = format!(
                "<div id=\"org-node-{id}\" class=\"yav-org-node absolute rounded-xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-3 shadow-md hover:shadow-lg transition-all duration-200 hover:-translate-y-0.5 group cursor-pointer flex items-center space-x-3\" style=\"left: {x}px; top: {y}px; width: {w}px; height: {h}px;\">\
                    {avatar}\
                    <div class=\"min-w-0 flex-1 overflow-hidden\">\
                        <div class=\"flex items-center space-x-1.5\">\
                            <h4 class=\"truncate font-bold text-sm text-gray-900 dark:text-gray-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400\">{name}</h4>\
                            {dept}\
                        </div>\
                        <p class=\"truncate text-xs font-medium text-gray-500 dark:text-gray-400\">{title}</p>\
                    </div>\
                </div>",
                id = escape_html(&node.id),
                x = node.x,
                y = node.y,
                w = self.node_width,
                h = self.node_height,
                avatar = avatar_markup,
                name = escape_html(&node.name),
                dept = dept_badge,
                title = escape_html(&node.title)
            );
            node_elements.push(Node::Raw(card_html));
        }

        // Outer wrapper enclosing SVG connector layer and positioned HTML nodes
        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(
                    Element::new("div")
                        .attr("class", "relative")
                        .attr(
                            "style",
                            format!("width: {:.0}px; height: {:.0}px; min-width: 100%;", canvas_width, canvas_height),
                        )
                        .child(Node::Raw(format!(
                            "<svg class=\"pointer-events-none absolute inset-0 h-full w-full\" width=\"{:.0}\" height=\"{:.0}\">\n{}</svg>",
                            canvas_width, canvas_height, connectors
                        )))
                        .children(node_elements),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_org_chart_rendering_and_layout() {
        let chart = OrgChart::new()
            .id("engineering-team")
            .node(OrgNode::new("ceo", "Sarah Connor", "Chief Executive Officer").department("Exec"))
            .node(
                OrgNode::new("cto", "John Connor", "Chief Technology Officer")
                    .parent("ceo")
                    .department("Tech"),
            )
            .node(
                OrgNode::new("lead", "Kyle Reese", "Staff Engineer")
                    .parent("cto")
                    .department("Tech"),
            );

        let node = chart.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"engineering-team\""));
        assert!(html.contains("Sarah Connor"));
        assert!(html.contains("Chief Technology Officer"));
        assert!(html.contains("<svg"));
        assert!(html.contains("<path d="));
    }

    #[test]
    fn test_horizontal_orientation() {
        let chart = OrgChart::new()
            .orientation(OrgOrientation::Horizontal)
            .node(OrgNode::new("a", "Alpha", "Leader"))
            .node(OrgNode::new("b", "Beta", "Member").parent("a"));

        let node = chart.render();
        let html = render_to_string(&node);
        assert!(html.contains("Alpha"));
        assert!(html.contains("Beta"));
    }

    #[test]
    fn test_avatar_url_xss_protection() {
        let chart = OrgChart::new()
            .node(OrgNode::new("x", "Mallory", "Attacker").avatar("javascript:alert(1)"));
        let html = render_to_string(&chart.render());
        // The dangerous scheme must not survive into the img src attribute.
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("src=\"javascript"));
        assert!(html.contains("src=\"\""));

        let safe = OrgChart::new()
            .node(OrgNode::new("y", "Alice", "Engineer").avatar("https://example.com/a.png"));
        let safe_html = render_to_string(&safe.render());
        assert!(safe_html.contains("src=\"https://example.com/a.png\""));
    }
}
