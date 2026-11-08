//! Server-Rendered SVG Data Visualization & Charting Suite (`yav::ui::charts` — Recharts / Tremor / Visx equivalent).
//!
//! Renders crisp, responsive, accessible SVG charts directly on the server with zero client-side
//! charting library payload:
//! - [`struct@LineChart`]: Multi-point SVG line & gradient area chart with grid lines and tooltips.
//! - [`struct@BarChart`]: Responsive vertical SVG bar chart with rounded columns and value labels.
//! - [`struct@DonutChart`]: Ring/donut proportional breakdown chart with center KPI and legend.
//! - [`struct@Sparkline`]: Compact inline trend line for KPI cards and data tables.

use crate::utils::merge_classes;
use yav_core::{escape_attribute, escape_html, Component, Element, IntoNode, Node};

/// A single data point in a chart series.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
}

impl ChartPoint {
    /// Create a new chart data point `(label, value)`.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    /// Assign a custom hex/CSS color to this data point (used in [`DonutChart`] and [`BarChart`]).
    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. LineChart & AreaChart
// ─────────────────────────────────────────────────────────────────────────────

/// Responsive SVG Line and Area Chart component.
#[derive(Debug, Clone, PartialEq)]
pub struct LineChart {
    pub title: Option<String>,
    pub data: Vec<ChartPoint>,
    pub stroke_color: String,
    pub area_fill: bool,
    pub height: u32,
    pub class: String,
}

impl Default for LineChart {
    fn default() -> Self {
        Self {
            title: None,
            data: Vec::new(),
            stroke_color: "#3b82f6".to_string(),
            area_fill: true,
            height: 240,
            class: String::new(),
        }
    }
}

impl LineChart {
    pub fn new(data: Vec<ChartPoint>) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn stroke_color(mut self, color: impl Into<String>) -> Self {
        self.stroke_color = color.into();
        self
    }

    #[must_use]
    pub fn area_fill(mut self, enabled: bool) -> Self {
        self.area_fill = enabled;
        self
    }

    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.height = height.max(80);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn render(self) -> Node {
        let w = 640.0_f64;
        let h = self.height as f64;
        let pad_x = 36.0;
        let pad_y = 24.0;
        let plot_w = w - pad_x * 2.0;
        let plot_h = (h - pad_y * 2.0).max(40.0);

        let max_v = self
            .data
            .iter()
            .map(|p| p.value)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let min_v = self
            .data
            .iter()
            .map(|p| p.value)
            .fold(0.0_f64, f64::min)
            .min(0.0);
        let range = (max_v - min_v).max(1.0);

        let n = self.data.len();
        let mut coords: Vec<(f64, f64)> = Vec::with_capacity(n);
        for (idx, pt) in self.data.iter().enumerate() {
            let x = if n <= 1 {
                pad_x + plot_w / 2.0
            } else {
                pad_x + (idx as f64 / (n - 1) as f64) * plot_w
            };
            let norm = (pt.value - min_v) / range;
            let y = pad_y + plot_h - norm * plot_h;
            coords.push((x, y));
        }

        let mut svg = format!(
            "<svg viewBox=\"0 0 {w:.0} {h:.0}\" class=\"w-full h-auto overflow-visible\" role=\"img\" aria-label=\"{}\">",
            escape_attribute(self.title.as_deref().unwrap_or("Line chart"))
        );

        let stroke = escape_attribute(&self.stroke_color);
        svg.push_str(&format!(
            "<defs><linearGradient id=\"yav-area-grad\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">\
            <stop offset=\"0%\" stop-color=\"{stroke}\" stop-opacity=\"0.32\"/>\
            <stop offset=\"100%\" stop-color=\"{stroke}\" stop-opacity=\"0.0\"/>\
            </linearGradient></defs>"
        ));

        // Horizontal grid lines
        for step in 0..=4 {
            let gy = pad_y + (step as f64 / 4.0) * plot_h;
            svg.push_str(&format!(
                "<line x1=\"{pad_x:.1}\" y1=\"{gy:.1}\" x2=\"{:.1}\" y2=\"{gy:.1}\" stroke=\"currentColor\" stroke-opacity=\"0.08\" stroke-dasharray=\"4 4\" />",
                w - pad_x
            ));
        }

        if !coords.is_empty() {
            let path_d = coords
                .iter()
                .enumerate()
                .map(|(i, (x, y))| {
                    if i == 0 {
                        format!("M {x:.1} {y:.1}")
                    } else {
                        format!("L {x:.1} {y:.1}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            if self.area_fill && coords.len() >= 2 {
                let first_x = coords.first().unwrap().0;
                let last_x = coords.last().unwrap().0;
                let baseline_y = pad_y + plot_h;
                let area_d = format!(
                    "{path_d} L {last_x:.1} {baseline_y:.1} L {first_x:.1} {baseline_y:.1} Z"
                );
                svg.push_str(&format!(
                    "<path d=\"{area_d}\" fill=\"url(#yav-area-grad)\" />"
                ));
            }

            svg.push_str(&format!(
                "<path d=\"{path_d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />"
            ));

            for ((x, y), pt) in coords.iter().zip(self.data.iter()) {
                let lbl = escape_html(&pt.label);
                svg.push_str(&format!(
                    "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"4\" fill=\"{stroke}\" class=\"transition-transform hover:scale-125\"><title>{lbl}: {:.2}</title></circle>",
                    pt.value
                ));
                svg.push_str(&format!(
                    "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"currentColor\" opacity=\"0.65\">{lbl}</text>",
                    h - 4.0
                ));
            }
        }

        svg.push_str("</svg>");

        let wrapper_cls = merge_classes(&[
            "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4",
            &self.class,
        ]);
        let mut el = Element::new("div").attr("class", wrapper_cls);
        if let Some(t) = self.title {
            el = el.child(Node::Element(
                Element::new("h4")
                    .attr(
                        "class",
                        "text-sm font-semibold text-slate-900 dark:text-slate-100 mb-3",
                    )
                    .child(Node::Text(t)),
            ));
        }
        Node::Element(el.child(Node::Raw(svg)))
    }
}

impl IntoNode for LineChart {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl Component for LineChart {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LineChartProps {
    pub title: Option<String>,
    pub data: Vec<ChartPoint>,
    pub stroke_color: Option<String>,
    pub area_fill: Option<bool>,
    pub height: Option<u32>,
    pub class: String,
}

#[allow(non_snake_case)]
pub fn LineChart(props: LineChartProps) -> Node {
    let mut c = LineChart::new(props.data);
    if let Some(t) = props.title {
        c = c.title(t);
    }
    if let Some(sc) = props.stroke_color {
        c = c.stroke_color(sc);
    }
    if let Some(af) = props.area_fill {
        c = c.area_fill(af);
    }
    if let Some(h) = props.height {
        c = c.height(h);
    }
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    c.render()
}

pub fn line_chart(data: Vec<ChartPoint>) -> LineChart {
    LineChart::new(data)
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. BarChart
// ─────────────────────────────────────────────────────────────────────────────

/// Responsive SVG Vertical Bar Chart component.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChart {
    pub title: Option<String>,
    pub data: Vec<ChartPoint>,
    pub default_color: String,
    pub height: u32,
    pub class: String,
}

impl Default for BarChart {
    fn default() -> Self {
        Self {
            title: None,
            data: Vec::new(),
            default_color: "#6366f1".to_string(),
            height: 240,
            class: String::new(),
        }
    }
}

impl BarChart {
    pub fn new(data: Vec<ChartPoint>) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.default_color = color.into();
        self
    }

    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.height = height.max(80);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn render(self) -> Node {
        let w = 640.0_f64;
        let h = self.height as f64;
        let pad_x = 32.0;
        let pad_y = 28.0;
        let plot_w = w - pad_x * 2.0;
        let plot_h = (h - pad_y * 2.0).max(40.0);

        let max_v = self
            .data
            .iter()
            .map(|p| p.value)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let n = self.data.len().max(1);
        let slot_w = plot_w / (n as f64);
        let bar_w = (slot_w * 0.62).clamp(12.0, 64.0);

        let mut svg = format!(
            "<svg viewBox=\"0 0 {w:.0} {h:.0}\" class=\"w-full h-auto\" role=\"img\" aria-label=\"{}\">",
            escape_attribute(self.title.as_deref().unwrap_or("Bar chart"))
        );

        for (idx, pt) in self.data.iter().enumerate() {
            let ratio = (pt.value / max_v).clamp(0.0, 1.0);
            let bh = (ratio * plot_h).max(2.0);
            let x = pad_x + (idx as f64) * slot_w + (slot_w - bar_w) / 2.0;
            let y = pad_y + plot_h - bh;
            let fill = escape_attribute(pt.color.as_deref().unwrap_or(&self.default_color));
            let lbl = escape_html(&pt.label);

            svg.push_str(&format!(
                "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_w:.1}\" height=\"{bh:.1}\" rx=\"6\" fill=\"{fill}\"><title>{lbl}: {:.2}</title></rect>",
                pt.value
            ));
            svg.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"11\" fill=\"currentColor\" opacity=\"0.7\">{lbl}</text>",
                x + bar_w / 2.0,
                h - 6.0
            ));
        }

        svg.push_str("</svg>");

        let wrapper_cls = merge_classes(&[
            "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4",
            &self.class,
        ]);
        let mut el = Element::new("div").attr("class", wrapper_cls);
        if let Some(t) = self.title {
            el = el.child(Node::Element(
                Element::new("h4")
                    .attr(
                        "class",
                        "text-sm font-semibold text-slate-900 dark:text-slate-100 mb-3",
                    )
                    .child(Node::Text(t)),
            ));
        }
        Node::Element(el.child(Node::Raw(svg)))
    }
}

impl IntoNode for BarChart {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl Component for BarChart {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BarChartProps {
    pub title: Option<String>,
    pub data: Vec<ChartPoint>,
    pub color: Option<String>,
    pub height: Option<u32>,
    pub class: String,
}

#[allow(non_snake_case)]
pub fn BarChart(props: BarChartProps) -> Node {
    let mut c = BarChart::new(props.data);
    if let Some(t) = props.title {
        c = c.title(t);
    }
    if let Some(col) = props.color {
        c = c.color(col);
    }
    if let Some(h) = props.height {
        c = c.height(h);
    }
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    c.render()
}

pub fn bar_chart(data: Vec<ChartPoint>) -> BarChart {
    BarChart::new(data)
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. DonutChart
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_PALETTE: &[&str] = &[
    "#3b82f6", "#10b981", "#8b5cf6", "#f59e0b", "#ef4444", "#06b6d4",
];

/// Responsive SVG Donut / Ring Chart with center KPI and legend.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DonutChart {
    pub title: Option<String>,
    pub center_label: Option<String>,
    pub data: Vec<ChartPoint>,
    pub class: String,
}

impl DonutChart {
    pub fn new(data: Vec<ChartPoint>) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    #[must_use]
    pub fn center_label(mut self, label: impl Into<String>) -> Self {
        self.center_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn render(self) -> Node {
        let total: f64 = self
            .data
            .iter()
            .map(|p| p.value.max(0.0))
            .sum::<f64>()
            .max(1.0);
        let radius = 60.0_f64;
        let circumference = 2.0 * std::f64::consts::PI * radius;
        let mut cumulative = 0.0_f64;

        let mut svg = String::from(
            "<svg viewBox=\"0 0 160 160\" class=\"w-36 h-36 shrink-0\" role=\"img\" aria-label=\"Donut chart\">",
        );

        for (idx, pt) in self.data.iter().enumerate() {
            let frac = pt.value.max(0.0) / total;
            let dash = frac * circumference;
            let gap = circumference - dash;
            let offset = -(cumulative * circumference);
            cumulative += frac;

            let color = escape_attribute(
                pt.color
                    .as_deref()
                    .unwrap_or(DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()]),
            );
            let lbl = escape_html(&pt.label);
            svg.push_str(&format!(
                "<circle cx=\"80\" cy=\"80\" r=\"{radius:.0}\" fill=\"transparent\" stroke=\"{color}\" stroke-width=\"18\" stroke-dasharray=\"{dash:.2} {gap:.2}\" stroke-dashoffset=\"{offset:.2}\" transform=\"rotate(-90 80 80)\"><title>{lbl}: {:.1}%</title></circle>",
                frac * 100.0
            ));
        }

        if let Some(ref cl) = self.center_label {
            svg.push_str(&format!(
                "<text x=\"80\" y=\"85\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"700\" fill=\"currentColor\">{}</text>",
                escape_html(cl)
            ));
        }
        svg.push_str("</svg>");

        let mut legend = Element::new("div").attr("class", "flex flex-col gap-2 text-sm");
        for (idx, pt) in self.data.iter().enumerate() {
            let color = pt
                .color
                .as_deref()
                .unwrap_or(DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()]);
            legend = legend.child(Node::Element(
                Element::new("div")
                    .attr("class", "flex items-center justify-between gap-4")
                    .child(Node::Element(
                        Element::new("div")
                            .attr("class", "flex items-center gap-2")
                            .child(Node::Element(
                                Element::new("span")
                                    .attr("class", "inline-block w-3 h-3 rounded-full")
                                    .attr("style", format!("background-color:{color}")),
                            ))
                            .child(Node::Element(
                                Element::new("span")
                                    .attr("class", "text-slate-600 dark:text-slate-400")
                                    .child(Node::Text(pt.label.clone())),
                            )),
                    ))
                    .child(Node::Element(
                        Element::new("span")
                            .attr("class", "font-medium text-slate-900 dark:text-slate-100")
                            .child(Node::Text(format!("{:.0}", pt.value))),
                    )),
            ));
        }

        let wrapper_cls = merge_classes(&[
            "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4",
            &self.class,
        ]);
        let mut root = Element::new("div").attr("class", wrapper_cls);
        if let Some(t) = self.title {
            root = root.child(Node::Element(
                Element::new("h4")
                    .attr(
                        "class",
                        "text-sm font-semibold text-slate-900 dark:text-slate-100 mb-3",
                    )
                    .child(Node::Text(t)),
            ));
        }
        Node::Element(
            root.child(Node::Element(
                Element::new("div")
                    .attr("class", "flex items-center gap-6")
                    .child(Node::Raw(svg))
                    .child(Node::Element(legend)),
            )),
        )
    }
}

impl IntoNode for DonutChart {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl Component for DonutChart {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DonutChartProps {
    pub title: Option<String>,
    pub center_label: Option<String>,
    pub data: Vec<ChartPoint>,
    pub class: String,
}

#[allow(non_snake_case)]
pub fn DonutChart(props: DonutChartProps) -> Node {
    let mut c = DonutChart::new(props.data);
    if let Some(t) = props.title {
        c = c.title(t);
    }
    if let Some(cl) = props.center_label {
        c = c.center_label(cl);
    }
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    c.render()
}

pub fn donut_chart(data: Vec<ChartPoint>) -> DonutChart {
    DonutChart::new(data)
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Sparkline
// ─────────────────────────────────────────────────────────────────────────────

/// Compact inline SVG Sparkline trend indicator for KPI cards and data tables.
#[derive(Debug, Clone, PartialEq)]
pub struct Sparkline {
    pub values: Vec<f64>,
    pub color: String,
    pub width: u32,
    pub height: u32,
}

impl Sparkline {
    pub fn new(values: Vec<f64>) -> Self {
        Self {
            values,
            color: "#10b981".to_string(),
            width: 96,
            height: 28,
        }
    }

    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    pub fn render(self) -> Node {
        let w = self.width.max(24) as f64;
        let h = self.height.max(12) as f64;
        let min_v = self.values.iter().copied().fold(f64::INFINITY, f64::min);
        let max_v = self
            .values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let range = (max_v - min_v).max(1.0);
        let n = self.values.len();

        let points = self
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let x = if n <= 1 {
                    w / 2.0
                } else {
                    (i as f64 / (n - 1) as f64) * (w - 4.0) + 2.0
                };
                let y = h - 2.0 - ((v - min_v) / range) * (h - 4.0);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");

        let stroke = escape_attribute(&self.color);
        Node::Raw(format!(
            "<svg viewBox=\"0 0 {w:.0} {h:.0}\" width=\"{w:.0}\" height=\"{h:.0}\" class=\"inline-block overflow-visible\" aria-hidden=\"true\">\
            <polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{points}\" />\
            </svg>"
        ))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SparklineProps {
    pub values: Vec<f64>,
    pub color: Option<String>,
}

#[allow(non_snake_case)]
pub fn Sparkline(props: SparklineProps) -> Node {
    let mut s = Sparkline::new(props.values);
    if let Some(c) = props.color {
        s = s.color(c);
    }
    s.render()
}

pub fn sparkline(values: Vec<f64>) -> Sparkline {
    Sparkline::new(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_charts_render_valid_svg() {
        let pts = vec![
            ChartPoint::new("Jan", 120.0),
            ChartPoint::new("Feb", 210.0),
            ChartPoint::new("Mar", 180.0),
        ];

        let line_html = render_to_string(&line_chart(pts.clone()).title("Revenue").render());
        assert!(line_html.contains("<svg viewBox=\"0 0 640 240\""));
        assert!(line_html.contains("Revenue"));
        assert!(line_html.contains("<path d=\"M "));

        let bar_html = render_to_string(&bar_chart(pts.clone()).title("Signups").render());
        assert!(bar_html.contains("<rect "));
        assert!(bar_html.contains("Signups"));

        let donut_html = render_to_string(&donut_chart(pts).center_label("510").render());
        assert!(donut_html.contains("stroke-dasharray="));
        assert!(donut_html.contains("510"));

        let spark_html = render_to_string(&sparkline(vec![10.0, 25.0, 18.0, 32.0]).render());
        assert!(spark_html.contains("<polyline "));
    }
}
