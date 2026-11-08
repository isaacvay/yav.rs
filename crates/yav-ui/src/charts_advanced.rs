//! Advanced SVG Data Visualizations: Activity Heatmap, Sankey Flow & Radar Charts.
//!
//! Provides zero-JS, responsive vector charts rendered directly on the server
//! for engineering dashboards, financial flows, and analytics.

use yav_core::{Component, Element, Node};

/// Contribution or activity data point for the `<Heatmap />`.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapCell {
    pub date: String,
    pub count: u32,
}

/// GitHub-style Activity & Contribution Heatmap Matrix Component.
#[derive(Debug, Clone)]
pub struct Heatmap {
    pub title: String,
    pub cells: Vec<HeatmapCell>,
    pub weeks: usize,
}

impl Heatmap {
    /// Create a new Heatmap with a title (e.g. `"Historique des déploiements"`).
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            cells: Vec::new(),
            weeks: 24, // 24 weeks default view
        }
    }

    #[must_use]
    pub fn with_weeks(mut self, weeks: usize) -> Self {
        self.weeks = weeks;
        self
    }

    #[must_use]
    pub fn record(mut self, date: impl Into<String>, count: u32) -> Self {
        self.cells.push(HeatmapCell {
            date: date.into(),
            count,
        });
        self
    }
}

impl Component for Heatmap {
    fn render(&self) -> Node {
        let cell_size = 11.0;
        let gap = 3.0;
        let width = (self.weeks as f64) * (cell_size + gap) + 40.0;
        let height = 7.0 * (cell_size + gap) + 50.0;

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.1} {height:.1}" class="w-full h-auto text-slate-400">
  <g transform="translate(30, 20)">
"##
        );

        let colors = ["#1e293b", "#0e4429", "#006d32", "#26a641", "#39d353"];

        for w in 0..self.weeks {
            for d in 0..7 {
                let idx = w * 7 + d;
                let (date_label, count) = if let Some(cell) = self.cells.get(idx) {
                    (cell.date.as_str(), cell.count)
                } else {
                    ("", 0)
                };

                let color = match count {
                    0 => colors[0],
                    1..=2 => colors[1],
                    3..=5 => colors[2],
                    6..=9 => colors[3],
                    _ => colors[4],
                };

                let x = (w as f64) * (cell_size + gap);
                let y = (d as f64) * (cell_size + gap);

                svg.push_str(&format!(
                    r##"    <rect x="{x:.1}" y="{y:.1}" width="{cell_size:.1}" height="{cell_size:.1}" rx="2" fill="{color}">
      <title>{date_label}: {count} actions</title>
    </rect>
"##
                ));
            }
        }

        // Add legend at bottom
        let legend_y = 7.0 * (cell_size + gap) + 12.0;
        let legend_x = width - 130.0;
        svg.push_str(&format!(
            r##"    <g transform="translate({legend_x:.1}, {legend_y:.1})">
      <text x="-32" y="9" font-size="9" fill="#64748b" font-family="sans-serif">Moins</text>
      <rect x="0" y="0" width="10" height="10" rx="2" fill="{c0}" />
      <rect x="14" y="0" width="10" height="10" rx="2" fill="{c1}" />
      <rect x="28" y="0" width="10" height="10" rx="2" fill="{c2}" />
      <rect x="42" y="0" width="10" height="10" rx="2" fill="{c3}" />
      <rect x="56" y="0" width="10" height="10" rx="2" fill="{c4}" />
      <text x="72" y="9" font-size="9" fill="#64748b" font-family="sans-serif">Plus</text>
    </g>
  </g>
</svg>"##,
            c0 = colors[0],
            c1 = colors[1],
            c2 = colors[2],
            c3 = colors[3],
            c4 = colors[4]
        ));

        let header = Element::new("div")
            .attr("class", "flex items-center justify-between mb-3")
            .child(
                Element::new("h4")
                    .attr("class", "text-sm font-semibold text-white")
                    .child(Node::text(self.title.clone())),
            )
            .child(
                Element::new("span")
                    .attr("class", "text-xs font-mono text-slate-400")
                    .child(Node::text(format!("{} semaines", self.weeks))),
            );

        Node::Element(
            Element::new("div")
                .attr(
                    "class",
                    "yav-heatmap rounded-xl border border-slate-800 bg-slate-950 p-4 shadow-lg",
                )
                .child(header)
                .child(Node::raw(svg)),
        )
    }
}

/// A node in a Sankey flow diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    pub id: String,
    pub label: String,
    pub color: &'static str,
}

/// A directed flow link between two Sankey nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLink {
    pub source: String,
    pub target: String,
    pub value: f64,
}

/// Server-Rendered Vector Sankey Flow Diagram.
#[derive(Debug, Clone)]
pub struct SankeyChart {
    pub title: String,
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
}

impl SankeyChart {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }

    #[must_use]
    pub fn node(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        color: &'static str,
    ) -> Self {
        self.nodes.push(SankeyNode {
            id: id.into(),
            label: label.into(),
            color,
        });
        self
    }

    #[must_use]
    pub fn link(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        value: f64,
    ) -> Self {
        self.links.push(SankeyLink {
            source: source.into(),
            target: target.into(),
            value,
        });
        self
    }
}

impl Component for SankeyChart {
    fn render(&self) -> Node {
        let width = 640.0;
        let height = 300.0;

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" class="w-full h-auto">
  <defs>
    <linearGradient id="flow-grad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#6366f1" stop-opacity="0.4" />
      <stop offset="100%" stop-color="#06b6d4" stop-opacity="0.4" />
    </linearGradient>
  </defs>
"##
        );

        // Render dynamic nodes and connecting cubic Bezier paths
        let x_left = 60.0;
        let x_right = 520.0;
        let node_w = 18.0;

        if self.nodes.is_empty() {
            let node_h = 60.0;
            // Render left column node
            svg.push_str(&format!(
                r##"  <rect x="{x_left}" y="50" width="{node_w}" height="{node_h}" rx="3" fill="#6366f1" />
  <text x="{}" y="85" fill="#f8fafc" font-size="12" font-family="sans-serif" text-anchor="end">Source</text>
"##,
                x_left - 8.0
            ));

            // Render right column node
            svg.push_str(&format!(
                r##"  <rect x="{x_right}" y="70" width="{node_w}" height="{node_h}" rx="3" fill="#06b6d4" />
  <text x="{}" y="105" fill="#f8fafc" font-size="12" font-family="sans-serif">Target</text>
"##,
                x_right + node_w + 8.0
            ));

            // Connecting flow ribbon
            let y1 = 70.0;
            let y2 = 90.0;
            let ribbon_h = 30.0;
            let dx = (x_right - x_left) * 0.5;

            svg.push_str(&format!(
                r##"  <path d="M {x1} {y1} C {cx1} {y1}, {cx2} {y2}, {x2} {y2} L {x2} {y2_bot} C {cx2} {y2_bot}, {cx1} {y1_bot}, {x1} {y1_bot} Z" fill="url(#flow-grad)" />
</svg>"##,
                x1 = x_left + node_w,
                y1 = y1,
                cx1 = x_left + node_w + dx,
                cx2 = x_right - dx,
                x2 = x_right,
                y2 = y2,
                y2_bot = y2 + ribbon_h,
                y1_bot = y1 + ribbon_h
            ));
        } else {
            let target_ids: std::collections::HashSet<_> =
                self.links.iter().map(|l| l.target.as_str()).collect();
            let source_ids: std::collections::HashSet<_> =
                self.links.iter().map(|l| l.source.as_str()).collect();

            let left_nodes: Vec<_> = self
                .nodes
                .iter()
                .filter(|n| {
                    !target_ids.contains(n.id.as_str()) || source_ids.contains(n.id.as_str())
                })
                .collect();
            let right_nodes: Vec<_> = self
                .nodes
                .iter()
                .filter(|n| target_ids.contains(n.id.as_str()) && !left_nodes.contains(n))
                .collect();

            let left_count = left_nodes.len().max(1);
            let right_count = right_nodes.len().max(1);

            let mut node_coords = std::collections::HashMap::new();

            for (i, node) in left_nodes.iter().enumerate() {
                let y = 30.0 + (i as f64) * ((height - 60.0) / (left_count as f64));
                let node_h = ((height - 60.0) / (left_count as f64) * 0.7).clamp(20.0, 80.0);
                node_coords.insert(node.id.as_str(), (x_left, y, node_h, true));
                let escaped_lbl = yav_core::escape_html(&node.label);
                svg.push_str(&format!(
                    r##"  <rect x="{x_left}" y="{y:.1}" width="{node_w}" height="{node_h:.1}" rx="3" fill="{color}" />
  <text x="{tx:.1}" y="{ty:.1}" fill="#f8fafc" font-size="12" font-family="sans-serif" text-anchor="end">{escaped_lbl}</text>
"##,
                    color = node.color,
                    tx = x_left - 8.0,
                    ty = y + node_h / 2.0 + 4.0
                ));
            }

            for (i, node) in right_nodes.iter().enumerate() {
                let y = 30.0 + (i as f64) * ((height - 60.0) / (right_count as f64));
                let node_h = ((height - 60.0) / (right_count as f64) * 0.7).clamp(20.0, 80.0);
                node_coords.insert(node.id.as_str(), (x_right, y, node_h, false));
                let escaped_lbl = yav_core::escape_html(&node.label);
                svg.push_str(&format!(
                    r##"  <rect x="{x_right}" y="{y:.1}" width="{node_w}" height="{node_h:.1}" rx="3" fill="{color}" />
  <text x="{tx:.1}" y="{ty:.1}" fill="#f8fafc" font-size="12" font-family="sans-serif" text-anchor="start">{escaped_lbl}</text>
"##,
                    color = node.color,
                    tx = x_right + node_w + 8.0,
                    ty = y + node_h / 2.0 + 4.0
                ));
            }

            let dx = (x_right - x_left) * 0.5;
            for link in &self.links {
                if let (Some(&(x1, y1, h1, _)), Some(&(x2, y2, h2, _))) = (
                    node_coords.get(link.source.as_str()),
                    node_coords.get(link.target.as_str()),
                ) {
                    let ribbon_h = (link.value.clamp(5.0, 50.0)).min(h1).min(h2);
                    let sy = y1 + h1 * 0.2;
                    let ty = y2 + h2 * 0.2;
                    svg.push_str(&format!(
                        r##"  <path d="M {x1} {sy:.1} C {cx1:.1} {sy:.1}, {cx2:.1} {ty:.1}, {x2} {ty:.1} L {x2} {ty_bot:.1} C {cx2:.1} {ty_bot:.1}, {cx1:.1} {sy_bot:.1}, {x1} {sy_bot:.1} Z" fill="url(#flow-grad)" />
"##,
                        x1 = x1 + node_w,
                        sy = sy,
                        cx1 = x1 + node_w + dx,
                        cx2 = x2 - dx,
                        x2 = x2,
                        ty = ty,
                        ty_bot = ty + ribbon_h,
                        sy_bot = sy + ribbon_h
                    ));
                }
            }
            svg.push_str("</svg>");
        }

        Node::Element(
            Element::new("div")
                .attr(
                    "class",
                    "yav-sankey-chart rounded-xl border border-slate-800 bg-slate-950 p-4 shadow-lg",
                )
                .child(
                    Element::new("h4")
                        .attr("class", "text-sm font-semibold text-white mb-2")
                        .child(Node::text(self.title.clone())),
                )
                .child(Node::raw(svg)),
        )
    }
}

/// Multi-Axis Spider / Radar Chart Component.
#[derive(Debug, Clone)]
pub struct RadarChart {
    pub title: String,
    pub axes: Vec<String>,
    pub values: Vec<f64>, // Normalized values 0.0 to 1.0
}

impl RadarChart {
    pub fn new(title: impl Into<String>, axes: Vec<&str>, values: Vec<f64>) -> Self {
        Self {
            title: title.into(),
            axes: axes.into_iter().map(|s| s.to_string()).collect(),
            values,
        }
    }
}

impl Component for RadarChart {
    fn render(&self) -> Node {
        let size = 300.0;
        let center = size / 2.0;
        let radius = 100.0;
        let n = self.axes.len();

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" class="w-full h-auto">
  <g transform="translate({center}, {center})">
"##
        );

        if n >= 3 {
            // Concentric web levels (0.25, 0.5, 0.75, 1.0)
            for level in [0.25, 0.5, 0.75, 1.0] {
                let r = radius * level;
                let mut points = Vec::new();
                for i in 0..n {
                    let angle = (i as f64) * 2.0 * std::f64::consts::PI / (n as f64)
                        - std::f64::consts::FRAC_PI_2;
                    let x = r * angle.cos();
                    let y = r * angle.sin();
                    points.push(format!("{x:.1},{y:.1}"));
                }
                svg.push_str(&format!(
                    r##"    <polygon points="{}" fill="none" stroke="#334155" stroke-dasharray="2,2" stroke-width="1" />
"##,
                    points.join(" ")
                ));
            }

            // Data Polygon
            let mut data_points = Vec::new();
            for (i, &val) in self.values.iter().enumerate() {
                let v = val.clamp(0.0, 1.0);
                let r = radius * v;
                let angle = (i as f64) * 2.0 * std::f64::consts::PI / (n as f64)
                    - std::f64::consts::FRAC_PI_2;
                let x = r * angle.cos();
                let y = r * angle.sin();
                data_points.push(format!("{x:.1},{y:.1}"));
            }

            svg.push_str(&format!(
                r##"    <polygon points="{}" fill="#6366f1" fill-opacity="0.35" stroke="#818cf8" stroke-width="2" />
"##,
                data_points.join(" ")
            ));

            // Axis spokes and labels
            for (i, axis_label) in self.axes.iter().enumerate() {
                let angle = (i as f64) * 2.0 * std::f64::consts::PI / (n as f64)
                    - std::f64::consts::FRAC_PI_2;
                let x_outer = radius * angle.cos();
                let y_outer = radius * angle.sin();
                let x_text = (radius + 16.0) * angle.cos();
                let y_text = (radius + 16.0) * angle.sin() + 4.0;
                let anchor = if x_text.abs() < 5.0 {
                    "middle"
                } else if x_text > 0.0 {
                    "start"
                } else {
                    "end"
                };

                let escaped_label = yav_core::escape_html(axis_label);
                svg.push_str(&format!(
                    r##"    <line x1="0" y1="0" x2="{x_outer:.1}" y2="{y_outer:.1}" stroke="#334155" stroke-width="1" />
    <text x="{x_text:.1}" y="{y_text:.1}" fill="#94a3b8" font-size="10" font-family="sans-serif" text-anchor="{anchor}">{escaped_label}</text>
"##
                ));
            }
        }

        svg.push_str("  </g>\n</svg>");

        Node::Element(
            Element::new("div")
                .attr(
                    "class",
                    "yav-radar-chart rounded-xl border border-slate-800 bg-slate-950 p-4 shadow-lg text-center",
                )
                .child(
                    Element::new("h4")
                        .attr("class", "text-sm font-semibold text-white mb-2")
                        .child(Node::text(self.title.clone())),
                )
                .child(Node::raw(svg)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_heatmap_rendering() {
        let hm = Heatmap::new("Activité")
            .record("2026-03-01", 4)
            .record("2026-03-02", 12);

        let html = render_to_string(&hm.render());
        assert!(html.contains("yav-heatmap"));
        assert!(html.contains("Activité"));
        assert!(html.contains("fill=\"#006d32\"")); // 4 actions
    }

    #[test]
    fn test_radar_chart_rendering() {
        let radar = RadarChart::new(
            "Performances",
            vec!["Speed", "Reliability", "DX", "Security"],
            vec![0.95, 0.90, 0.85, 0.99],
        );

        let html = render_to_string(&radar.render());
        assert!(html.contains("yav-radar-chart"));
        assert!(html.contains("<polygon"));
        assert!(html.contains("Speed"));
        assert!(html.contains("Security"));
    }

    #[test]
    fn test_sankey_chart_rendering() {
        let sankey = SankeyChart::new("User Flow")
            .node("home", "Homepage", "#6366f1")
            .node("signup", "Sign Up", "#10b981")
            .link("home", "signup", 120.0);

        let html = render_to_string(&sankey.render());
        assert!(html.contains("yav-sankey-chart"));
        assert!(html.contains("Homepage"));
        assert!(html.contains("Sign Up"));
        assert!(html.contains("fill=\"url(#flow-grad)\""));
    }
}
