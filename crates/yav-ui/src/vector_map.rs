//! # YAV Vector Map & Geospatial Data Visualizer (`<VectorMap />`)
//!
//! Zero-dependency SVG slippy vector map with Mercator projection math,
//! choropleth regions, GPS pin clusters, and interactive hover cards.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// A point of interest or GPS pin positioned on the vector map.
#[derive(Debug, Clone, PartialEq)]
pub struct MapPin {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub label: String,
    pub color: String,
    pub radius: f64,
    pub tooltip: Option<String>,
}

impl MapPin {
    #[must_use]
    pub fn new(id: impl Into<String>, lat: f64, lon: f64, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            lat,
            lon,
            label: label.into(),
            color: "#6366f1".to_string(), // Indigo 500
            radius: 6.0,
            tooltip: None,
        }
    }

    #[must_use]
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    #[must_use]
    pub fn radius(mut self, radius: f64) -> Self {
        self.radius = radius.max(2.0);
        self
    }

    #[must_use]
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
}

/// A closed geographic territory or choropleth polygon region on the map.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRegion {
    pub id: String,
    pub coordinates: Vec<(f64, f64)>, // (lat, lon) vertices
    pub fill_color: String,
    pub stroke_color: String,
    pub label: Option<String>,
}

impl MapRegion {
    #[must_use]
    pub fn new(id: impl Into<String>, coordinates: Vec<(f64, f64)>) -> Self {
        Self {
            id: id.into(),
            coordinates,
            fill_color: "rgba(99, 102, 241, 0.15)".to_string(),
            stroke_color: "#6366f1".to_string(),
            label: None,
        }
    }

    #[must_use]
    pub fn style(mut self, fill: impl Into<String>, stroke: impl Into<String>) -> Self {
        self.fill_color = fill.into();
        self.stroke_color = stroke.into();
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Interactive SVG Vector Map component.
#[derive(Debug, Clone)]
pub struct VectorMap {
    pub id: String,
    pub pins: Vec<MapPin>,
    pub regions: Vec<MapRegion>,
    pub width: u32,
    pub height: u32,
    pub center_lat: f64,
    pub center_lon: f64,
    pub zoom: f64,
    pub interactive: bool,
    pub class: String,
}

impl Default for VectorMap {
    fn default() -> Self {
        Self {
            id: "yav-vector-map".to_string(),
            pins: Vec::new(),
            regions: Vec::new(),
            width: 800,
            height: 500,
            center_lat: 48.8566, // Paris by default
            center_lon: 2.3522,
            zoom: 1.0,
            interactive: true,
            class: String::new(),
        }
    }
}

impl VectorMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn pin(mut self, pin: MapPin) -> Self {
        self.pins.push(pin);
        self
    }

    #[must_use]
    pub fn region(mut self, region: MapRegion) -> Self {
        self.regions.push(region);
        self
    }

    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    #[must_use]
    pub fn center(mut self, lat: f64, lon: f64, zoom: f64) -> Self {
        self.center_lat = lat;
        self.center_lon = lon;
        self.zoom = zoom.clamp(0.5, 10.0);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Convert spherical GPS coordinates (lat, lon) to Web Mercator pixel coordinates.
    pub fn lat_lon_to_xy(&self, lat: f64, lon: f64) -> (f64, f64) {
        let lat_rad = lat.clamp(-85.0, 85.0).to_radians();
        let w = self.width as f64;
        let h = self.height as f64;

        // Standard Mercator projection
        let x0 = (lon + 180.0) / 360.0 * w;
        let y0 =
            (1.0 - ((lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f64::consts::PI)) / 2.0 * h;

        // Apply zoom and center offset
        let center_lat_rad = self.center_lat.clamp(-85.0, 85.0).to_radians();
        let cx0 = (self.center_lon + 180.0) / 360.0 * w;
        let cy0 = (1.0
            - ((center_lat_rad.tan() + (1.0 / center_lat_rad.cos())).ln() / std::f64::consts::PI))
            / 2.0
            * h;

        let screen_x = (w / 2.0) + (x0 - cx0) * self.zoom;
        let screen_y = (h / 2.0) + (y0 - cy0) * self.zoom;

        (screen_x, screen_y)
    }
}

impl Component for VectorMap {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-vector-map relative overflow-hidden rounded-2xl border border-gray-200 dark:border-gray-800 bg-[#0b1329] shadow-xl select-none",
            &self.class,
        ]);

        let mut svg_content = String::new();

        // 1. Grid lines (parallels & meridians)
        svg_content.push_str("<g stroke=\"rgba(255,255,255,0.06)\" stroke-width=\"1\">\n");
        for lon in (-180..=180).step_by(30) {
            let (x1, y1) = self.lat_lon_to_xy(-75.0, lon as f64);
            let (x2, y2) = self.lat_lon_to_xy(75.0, lon as f64);
            svg_content.push_str(&format!(
                "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" />\n",
                x1, y1, x2, y2
            ));
        }
        svg_content.push_str("</g>\n");

        // 2. Render Choropleth Regions
        for r in &self.regions {
            if r.coordinates.is_empty() {
                continue;
            }
            let mut path_d = String::new();
            for (i, &(lat, lon)) in r.coordinates.iter().enumerate() {
                let (px, py) = self.lat_lon_to_xy(lat, lon);
                if i == 0 {
                    path_d.push_str(&format!("M {:.1} {:.1}", px, py));
                } else {
                    path_d.push_str(&format!(" L {:.1} {:.1}", px, py));
                }
            }
            path_d.push_str(" Z");

            svg_content.push_str(&format!(
                "<path id=\"{}\" d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\" class=\"transition-colors duration-200 hover:opacity-80\" />\n",
                escape_html(&r.id),
                path_d,
                escape_html(&r.fill_color),
                escape_html(&r.stroke_color)
            ));
        }

        // 3. Render GPS Pins with glowing pulse
        for pin in &self.pins {
            let (px, py) = self.lat_lon_to_xy(pin.lat, pin.lon);
            let tooltip_attr = if let Some(ref tip) = pin.tooltip {
                format!("data-tooltip=\"{}\"", escape_html(tip))
            } else {
                String::new()
            };

            svg_content.push_str(&format!(
                "<g id=\"pin-{id}\" transform=\"translate({x:.1}, {y:.1})\" class=\"cursor-pointer group\" {tip}>\
                    <circle r=\"{r_pulse:.1}\" fill=\"{color}\" opacity=\"0.2\" class=\"animate-ping\" />\
                    <circle r=\"{r:.1}\" fill=\"{color}\" stroke=\"#ffffff\" stroke-width=\"2\" />\
                    <text y=\"-10\" text-anchor=\"middle\" fill=\"#ffffff\" font-size=\"11\" font-weight=\"bold\" class=\"opacity-0 group-hover:opacity-100 transition-opacity drop-shadow\">{label}</text>\
                </g>\n",
                id = escape_html(&pin.id),
                x = px,
                y = py,
                r = pin.radius,
                r_pulse = pin.radius * 2.2,
                color = escape_html(&pin.color),
                tip = tooltip_attr,
                label = escape_html(&pin.label)
            ));
        }

        let svg_markup = format!(
            "<svg viewBox=\"0 0 {w} {h}\" class=\"w-full h-full\">\
                {content}\
            </svg>",
            w = self.width,
            h = self.height,
            content = svg_content
        );

        let controls = if self.interactive {
            "<div class=\"absolute bottom-4 right-4 z-10 flex flex-col space-y-1.5\">\
                <button type=\"button\" class=\"h-8 w-8 rounded-lg bg-gray-900/80 backdrop-blur border border-gray-700 text-white font-bold hover:bg-gray-800 flex items-center justify-center shadow-lg\">+</button>\
                <button type=\"button\" class=\"h-8 w-8 rounded-lg bg-gray-900/80 backdrop-blur border border-gray-700 text-white font-bold hover:bg-gray-800 flex items-center justify-center shadow-lg\">&minus;</button>\
            </div>".to_string()
        } else {
            String::new()
        };

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .attr(
                    "style",
                    format!("height: {}px; max-width: {}px;", self.height, self.width),
                )
                .child(Node::Raw(svg_markup))
                .child(Node::Raw(controls)),
        )
    }
}

/// Helper function to create a new `VectorMap`.
#[must_use]
pub fn vector_map() -> VectorMap {
    VectorMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_vector_map_rendering() {
        let map = VectorMap::new()
            .id("europe-map")
            .center(48.8566, 2.3522, 1.5)
            .pin(MapPin::new("paris", 48.8566, 2.3522, "Paris HQ").color("#ec4899"))
            .pin(MapPin::new("london", 51.5074, -0.1278, "London Office").color("#6366f1"))
            .region(MapRegion::new(
                "france-zone",
                vec![(49.0, 1.0), (49.0, 3.0), (47.0, 3.0), (47.0, 1.0)],
            ));

        let node = map.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"europe-map\""));
        assert!(html.contains("pin-paris"));
        assert!(html.contains("Paris HQ"));
        assert!(html.contains("france-zone"));
    }
}
