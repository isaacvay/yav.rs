//! # YAV Hexagonal Grid & Isometric Spatial Map (`<HexGrid />`)
//!
//! Pure SVG hexagonal coordinate grid with axial/cubic coordinates,
//! tile styling, Manhattan hexagonal distance math, and interactive selection.

use std::f64::consts::PI;
use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Axial and Cubic hexagonal coordinate representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexCoord {
    pub q: i32, // Column
    pub r: i32, // Row
}

impl HexCoord {
    #[must_use]
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Cubic S coordinate (where q + r + s = 0).
    #[must_use]
    pub const fn s(&self) -> i32 {
        -self.q - self.r
    }

    /// Manhattan distance between two hexagonal tiles.
    #[must_use]
    pub fn distance_to(&self, other: &HexCoord) -> u32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = (self.s() - other.s()).abs();
        ((dq + dr + ds) / 2) as u32
    }

    /// Convert axial coordinates to Cartesian 2D center coordinates (pointy-topped).
    #[must_use]
    pub fn to_pixel(&self, radius: f64) -> (f64, f64) {
        let x =
            radius * (3.0_f64.sqrt() * (self.q as f64) + 3.0_f64.sqrt() / 2.0 * (self.r as f64));
        let y = radius * (3.0 / 2.0 * (self.r as f64));
        (x, y)
    }
}

/// A single hexagonal tile placed on the grid.
#[derive(Debug, Clone, PartialEq)]
pub struct HexTile {
    pub coord: HexCoord,
    pub label: Option<String>,
    pub fill_color: String,
    pub stroke_color: String,
    pub tooltip: Option<String>,
}

impl HexTile {
    #[must_use]
    pub fn new(q: i32, r: i32) -> Self {
        Self {
            coord: HexCoord::new(q, r),
            label: None,
            fill_color: "#1e293b".to_string(),   // Slate 800
            stroke_color: "#334155".to_string(), // Slate 700
            tooltip: None,
        }
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn fill(mut self, fill: impl Into<String>) -> Self {
        self.fill_color = fill.into();
        self
    }

    #[must_use]
    pub fn stroke(mut self, stroke: impl Into<String>) -> Self {
        self.stroke_color = stroke.into();
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Hexagonal Grid component.
#[derive(Debug, Clone, PartialEq)]
pub struct HexGrid {
    pub id: String,
    pub tiles: Vec<HexTile>,
    pub tile_radius: f64,
    pub width: u32,
    pub height: u32,
    pub class: String,
}

impl HexGrid {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tiles: Vec::new(),
            tile_radius: 28.0,
            width: 600,
            height: 400,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn tile(mut self, tile: HexTile) -> Self {
        self.tiles.push(tile);
        self
    }

    #[must_use]
    pub fn tiles(mut self, tiles: impl IntoIterator<Item = HexTile>) -> Self {
        self.tiles.extend(tiles);
        self
    }

    #[must_use]
    pub fn tile_radius(mut self, radius: f64) -> Self {
        self.tile_radius = radius.max(8.0);
        self
    }

    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width.max(100);
        self.height = height.max(100);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Calculate the 6 polygon vertex coordinates for a pointy-topped hexagon.
    fn calculate_hex_corners(cx: f64, cy: f64, radius: f64) -> String {
        let mut points = Vec::with_capacity(6);
        for i in 0..6 {
            let angle_deg = 60.0 * (i as f64) - 30.0;
            let angle_rad = PI / 180.0 * angle_deg;
            let x = cx + radius * angle_rad.cos();
            let y = cy + radius * angle_rad.sin();
            points.push(format!("{x:.1},{y:.1}"));
        }
        points.join(" ")
    }
}

impl Component for HexGrid {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-hex-grid bg-slate-950 border border-slate-800 rounded-xl overflow-hidden shadow-2xl relative select-none",
            &self.class,
        ]);

        let center_x = self.width as f64 / 2.0;
        let center_y = self.height as f64 / 2.0;

        let mut svg_tiles = String::new();

        for tile in &self.tiles {
            let (rel_x, rel_y) = tile.coord.to_pixel(self.tile_radius);
            let cx = center_x + rel_x;
            let cy = center_y + rel_y;

            let points = Self::calculate_hex_corners(cx, cy, self.tile_radius - 1.5);
            let q = tile.coord.q;
            let r = tile.coord.r;

            let title_tag = if let Some(tip) = &tile.tooltip {
                format!("<title>{}</title>", escape_html(tip))
            } else {
                String::new()
            };

            let label_tag = if let Some(lbl) = &tile.label {
                format!(
                    r##"<text x="{cx:.1}" y="{cy:.1}" text-anchor="middle" dominant-baseline="central" fill="#f8fafc" font-size="10" font-family="monospace" pointer-events="none">{}</text>"##,
                    escape_html(lbl)
                )
            } else {
                String::new()
            };

            svg_tiles.push_str(&format!(
                r##"<g class="yav-hex-tile group cursor-pointer transition-transform duration-100" data-q="{q}" data-r="{r}">
                    <polygon points="{points}" fill="{}" stroke="{}" stroke-width="1.5" class="hover:stroke-indigo-400 hover:brightness-125 transition-all">
                        {title_tag}
                    </polygon>
                    {label_tag}
                </g>"##,
                escape_html(&tile.fill_color),
                escape_html(&tile.stroke_color),
            ));
        }

        let svg_markup = format!(
            r##"<svg id="{id}_svg" class="w-full h-auto block" viewBox="0 0 {w} {h}">{svg_tiles}</svg>"##,
            id = escape_html(&self.id),
            w = self.width,
            h = self.height,
        );

        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

root.addEventListener('click', (e) => {{
  const tile = e.target.closest('.yav-hex-tile');
  if (tile) {{
    const q = tile.getAttribute('data-q');
    const r = tile.getAttribute('data-r');
    root.dispatchEvent(new CustomEvent('hex-select', {{ detail: {{ q: Number(q), r: Number(r) }}, bubbles: true }}));
  }}
}});
}})();"##,
            id = escape_html(&self.id),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(Node::Raw(svg_markup))
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `HexGrid`.
#[must_use]
pub fn hex_grid(id: impl Into<String>) -> HexGrid {
    HexGrid::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_hex_coordinates_and_distance() {
        let a = HexCoord::new(0, 0);
        let b = HexCoord::new(2, -1);
        assert_eq!(a.distance_to(&b), 2);

        let c = HexCoord::new(1, 2);
        assert_eq!(a.distance_to(&c), 3);
    }

    #[test]
    fn test_hex_grid_rendering() {
        let grid = HexGrid::new("strategy-map")
            .tile_radius(30.0)
            .dimensions(500, 350)
            .tile(HexTile::new(0, 0).label("HQ").fill("#6366f1"))
            .tile(HexTile::new(1, 0).label("Depot").fill("#10b981"))
            .tile(HexTile::new(0, 1).label("Radar").fill("#f59e0b"));

        let node = grid.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-hex-grid"));
        assert!(html.contains("strategy-map"));
        assert!(html.contains("HQ"));
        assert!(html.contains("Depot"));
        assert!(html.contains("Radar"));
        assert!(html.contains("polygon"));
        assert!(html.contains("hex-select"));
    }
}
