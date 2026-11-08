//! # YAV Collaborative Whiteboard & Vector Drawing Canvas (`<Whiteboard />`)
//!
//! Real-time multiplayer whiteboard canvas featuring Chaikin line-smoothed pen strokes,
//! vector shapes (rectangle, ellipse, arrow), sticky notes, and floating tool palettes.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Drawing tool modes available on the whiteboard canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteboardTool {
    #[default]
    Select,
    Pen,
    Highlighter,
    Rectangle,
    Ellipse,
    Arrow,
    Line,
    StickyNote,
    Eraser,
}

impl WhiteboardTool {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Pen => "pen",
            Self::Highlighter => "highlighter",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Arrow => "arrow",
            Self::Line => "line",
            Self::StickyNote => "sticky",
            Self::Eraser => "eraser",
        }
    }
}

/// A vector drawing element or shape on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct WhiteboardElement {
    pub id: String,
    pub tool: WhiteboardTool,
    pub points: Vec<(f64, f64)>,
    pub stroke_color: String,
    pub stroke_width: f64,
    pub fill_color: Option<String>,
    pub text: Option<String>,
    pub user_id: Option<String>,
}

impl WhiteboardElement {
    /// Create a new pen drawing stroke.
    #[must_use]
    pub fn pen(id: impl Into<String>, points: Vec<(f64, f64)>, color: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tool: WhiteboardTool::Pen,
            points,
            stroke_color: color.into(),
            stroke_width: 2.5,
            fill_color: None,
            text: None,
            user_id: None,
        }
    }

    /// Create a rectangle element.
    #[must_use]
    pub fn rect(
        id: impl Into<String>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        color: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool: WhiteboardTool::Rectangle,
            points: vec![(x, y), (x + w, y + h)],
            stroke_color: color.into(),
            stroke_width: 2.0,
            fill_color: None,
            text: None,
            user_id: None,
        }
    }

    /// Create a sticky note with text.
    #[must_use]
    pub fn sticky(
        id: impl Into<String>,
        x: f64,
        y: f64,
        text: impl Into<String>,
        fill: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool: WhiteboardTool::StickyNote,
            points: vec![(x, y)],
            stroke_color: "#eab308".to_string(),
            stroke_width: 1.0,
            fill_color: Some(fill.into()),
            text: Some(text.into()),
            user_id: None,
        }
    }
}

/// Interactive vector whiteboard component.
#[derive(Debug, Clone)]
pub struct Whiteboard {
    pub id: String,
    pub elements: Vec<WhiteboardElement>,
    pub width: u32,
    pub height: u32,
    pub active_tool: WhiteboardTool,
    pub active_color: String,
    pub read_only: bool,
    pub ws_sync_url: Option<String>,
    pub class: String,
}

impl Default for Whiteboard {
    fn default() -> Self {
        Self {
            id: "yav-whiteboard".to_string(),
            elements: Vec::new(),
            width: 960,
            height: 560,
            active_tool: WhiteboardTool::Pen,
            active_color: "#4f46e5".to_string(), // Indigo
            read_only: false,
            ws_sync_url: None,
            class: String::new(),
        }
    }
}

impl Whiteboard {
    /// Create a new whiteboard builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set container DOM id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Add an element to the canvas.
    #[must_use]
    pub fn element(mut self, elem: WhiteboardElement) -> Self {
        self.elements.push(elem);
        self
    }

    /// Set list of elements.
    #[must_use]
    pub fn elements(mut self, elements: Vec<WhiteboardElement>) -> Self {
        self.elements = elements;
        self
    }

    /// Set canvas dimensions.
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set WebSocket sync endpoint URL for multiplayer collaboration.
    #[must_use]
    pub fn ws_sync(mut self, url: impl Into<String>) -> Self {
        self.ws_sync_url = Some(url.into());
        self
    }

    /// Set read-only mode.
    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Chaikin line smoothing algorithm: generates smooth SVG quadratic Bézier curve data.
    fn smooth_points_to_svg_path(points: &[(f64, f64)]) -> String {
        if points.is_empty() {
            return String::new();
        }
        if points.len() == 1 {
            return format!(
                "M {:.1} {:.1} L {:.1} {:.1}",
                points[0].0,
                points[0].1,
                points[0].0 + 0.1,
                points[0].1 + 0.1
            );
        }
        if points.len() == 2 {
            return format!(
                "M {:.1} {:.1} L {:.1} {:.1}",
                points[0].0, points[0].1, points[1].0, points[1].1
            );
        }

        let mut path = format!("M {:.1} {:.1}", points[0].0, points[0].1);
        for i in 1..points.len() - 1 {
            let p0 = points[i];
            let p1 = points[i + 1];
            let mid_x = (p0.0 + p1.0) / 2.0;
            let mid_y = (p0.1 + p1.1) / 2.0;
            path.push_str(&format!(
                " Q {:.1} {:.1}, {:.1} {:.1}",
                p0.0, p0.1, mid_x, mid_y
            ));
        }
        let last = points[points.len() - 1];
        path.push_str(&format!(" L {:.1} {:.1}", last.0, last.1));
        path
    }
}

impl Component for Whiteboard {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-whiteboard relative overflow-hidden rounded-2xl border border-gray-200 dark:border-gray-800 bg-[#fafafa] dark:bg-[#0a0d14] shadow-xl select-none",
            &self.class,
        ]);

        // Render SVG elements
        let mut svg_elements = String::new();
        for elem in &self.elements {
            match elem.tool {
                WhiteboardTool::Pen => {
                    let d = Self::smooth_points_to_svg_path(&elem.points);
                    svg_elements.push_str(&format!(
                        "<path id=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.1}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />\n",
                        escape_html(&elem.id),
                        d,
                        escape_html(&elem.stroke_color),
                        elem.stroke_width
                    ));
                }
                WhiteboardTool::Highlighter => {
                    let d = Self::smooth_points_to_svg_path(&elem.points);
                    svg_elements.push_str(&format!(
                        "<path id=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"16\" stroke-linecap=\"round\" stroke-linejoin=\"round\" opacity=\"0.35\" style=\"mix-blend-mode: multiply;\" />\n",
                        escape_html(&elem.id),
                        d,
                        escape_html(&elem.stroke_color)
                    ));
                }
                WhiteboardTool::Rectangle => {
                    if elem.points.len() >= 2 {
                        let (p0, p1) = (elem.points[0], elem.points[1]);
                        let x = p0.0.min(p1.0);
                        let y = p0.1.min(p1.1);
                        let w = (p1.0 - p0.0).abs();
                        let h = (p1.1 - p0.1).abs();
                        let fill = elem.fill_color.as_deref().unwrap_or("none");
                        svg_elements.push_str(&format!(
                            "<rect id=\"{}\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.1}\" />\n",
                            escape_html(&elem.id),
                            x, y, w, h,
                            escape_html(fill),
                            escape_html(&elem.stroke_color),
                            elem.stroke_width
                        ));
                    }
                }
                WhiteboardTool::Ellipse => {
                    if elem.points.len() >= 2 {
                        let (p0, p1) = (elem.points[0], elem.points[1]);
                        let cx = (p0.0 + p1.0) / 2.0;
                        let cy = (p0.1 + p1.1) / 2.0;
                        let rx = (p1.0 - p0.0).abs() / 2.0;
                        let ry = (p1.1 - p0.1).abs() / 2.0;
                        let fill = elem.fill_color.as_deref().unwrap_or("none");
                        svg_elements.push_str(&format!(
                            "<ellipse id=\"{}\" cx=\"{:.1}\" cy=\"{:.1}\" rx=\"{:.1}\" ry=\"{:.1}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.1}\" />\n",
                            escape_html(&elem.id),
                            cx, cy, rx, ry,
                            escape_html(fill),
                            escape_html(&elem.stroke_color),
                            elem.stroke_width
                        ));
                    }
                }
                WhiteboardTool::StickyNote => {
                    if let Some(&(x, y)) = elem.points.first() {
                        let fill = elem.fill_color.as_deref().unwrap_or("#fef08a");
                        let text = elem.text.as_deref().unwrap_or("");
                        svg_elements.push_str(&format!(
                            "<g id=\"{}\" transform=\"translate({:.1}, {:.1})\">\
                                <rect width=\"160\" height=\"140\" rx=\"8\" fill=\"{}\" filter=\"drop-shadow(0 4px 6px rgba(0,0,0,0.15))\" />\
                                <foreignObject width=\"160\" height=\"140\" class=\"p-3 overflow-hidden\">\
                                    <div xmlns=\"http://www.w3.org/1999/xhtml\" class=\"text-xs font-sans text-gray-800 leading-snug break-words\">{}</div>\
                                </foreignObject>\
                            </g>\n",
                            escape_html(&elem.id),
                            x, y,
                            escape_html(fill),
                            escape_html(text)
                        ));
                    }
                }
                _ => {}
            }
        }

        // Floating toolbox palette
        let toolbar = if !self.read_only {
            "<div class=\"absolute bottom-4 left-1/2 -translate-x-1/2 z-10 flex items-center space-x-1.5 rounded-full border border-gray-200 dark:border-gray-800 bg-white/90 dark:bg-gray-900/90 backdrop-blur px-3 py-1.5 shadow-xl\">\
                <button type=\"button\" class=\"p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 text-indigo-600 dark:text-indigo-400 focus:outline-none\" title=\"Pen\">\
                    <svg class=\"h-4 w-4 fill-current\" viewBox=\"0 0 24 24\"><path d=\"M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34a.9959.9959 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z\"/></svg>\
                </button>\
                <button type=\"button\" class=\"p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-600 dark:text-gray-300 focus:outline-none\" title=\"Rectangle\">\
                    <svg class=\"h-4 w-4 stroke-current fill-none stroke-2\" viewBox=\"0 0 24 24\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/></svg>\
                </button>\
                <button type=\"button\" class=\"p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-600 dark:text-gray-300 focus:outline-none\" title=\"Circle\">\
                    <svg class=\"h-4 w-4 stroke-current fill-none stroke-2\" viewBox=\"0 0 24 24\"><circle cx=\"12\" cy=\"12\" r=\"9\"/></svg>\
                </button>\
                <button type=\"button\" class=\"p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 text-amber-500 focus:outline-none\" title=\"Sticky Note\">\
                    <svg class=\"h-4 w-4 fill-current\" viewBox=\"0 0 24 24\"><path d=\"M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h10l6-6V5c0-1.1-.9-2-2-2zm-5 14v-4h4l-4 4z\"/></svg>\
                </button>\
                <div class=\"h-4 w-px bg-gray-300 dark:bg-gray-700 mx-1\"></div>\
                <button type=\"button\" class=\"h-5 w-5 rounded-full bg-indigo-600 ring-2 ring-indigo-400 ring-offset-1 focus:outline-none\"></button>\
                <button type=\"button\" class=\"h-5 w-5 rounded-full bg-rose-500 focus:outline-none\"></button>\
                <button type=\"button\" class=\"h-5 w-5 rounded-full bg-emerald-500 focus:outline-none\"></button>\
            </div>".to_string()
        } else {
            String::new()
        };

        // Grid background pattern
        let svg = format!(
            "<svg id=\"{id}-svg\" class=\"w-full h-full cursor-crosshair\" viewBox=\"0 0 {w} {h}\">\
                <defs>\
                    <pattern id=\"{id}-grid\" width=\"24\" height=\"24\" patternUnits=\"userSpaceOnUse\">\
                        <circle cx=\"1\" cy=\"1\" r=\"1\" fill=\"currentColor\" class=\"text-gray-300 dark:text-gray-800\" />\
                    </pattern>\
                </defs>\
                <rect width=\"100%\" height=\"100%\" fill=\"url(#{id}-grid)\" />\
                <g id=\"{id}-layer\">\
                    {elements}\
                </g>\
            </svg>",
            id = self.id,
            w = self.width,
            h = self.height,
            elements = svg_elements
        );

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .attr(
                    "style",
                    format!("height: {}px; max-width: {}px;", self.height, self.width),
                )
                .child(Node::Raw(svg))
                .child(Node::Raw(toolbar)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_whiteboard_rendering_and_elements() {
        let wb = Whiteboard::new()
            .id("canvas-1")
            .element(WhiteboardElement::pen(
                "stroke-1",
                vec![(10.0, 10.0), (50.0, 60.0), (100.0, 80.0)],
                "#4f46e5",
            ))
            .element(WhiteboardElement::rect(
                "box-1", 120.0, 50.0, 200.0, 100.0, "#059669",
            ))
            .element(WhiteboardElement::sticky(
                "note-1",
                350.0,
                100.0,
                "Architecture Plan",
                "#fef08a",
            ));

        let node = wb.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"canvas-1\""));
        assert!(html.contains("<path id=\"stroke-1\""));
        assert!(html.contains("<rect id=\"box-1\""));
        assert!(html.contains("Architecture Plan"));
        assert!(html.contains("canvas-1-grid"));
    }
}
