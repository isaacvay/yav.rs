//! # YAV 3D Model & Mesh Visualizer (`<ModelViewer3D />`)
//!
//! Pure-Rust 3D linear algebra and wireframe/mesh visualizer rendered into SVG.
//! Includes Euler rotation matrices, perspective/orthographic projections,
//! built-in geometric primitives (Cube, Pyramid, Octahedron, Cylinder),
//! and interactive client-side orbit mouse drag controls.

use std::f64::consts::PI;
use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// A 3D coordinate point in Cartesian space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Rotate around X axis (pitch) by angle in radians.
    #[must_use]
    pub fn rotate_x(&self, rad: f64) -> Self {
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    /// Rotate around Y axis (yaw) by angle in radians.
    #[must_use]
    pub fn rotate_y(&self, rad: f64) -> Self {
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    /// Rotate around Z axis (roll) by angle in radians.
    #[must_use]
    pub fn rotate_z(&self, rad: f64) -> Self {
        let cos = rad.cos();
        let sin = rad.sin();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }

    /// Rotate by Euler angles (yaw, pitch, roll) in radians.
    #[must_use]
    pub fn rotate_euler(&self, yaw: f64, pitch: f64, roll: f64) -> Self {
        self.rotate_y(yaw).rotate_x(pitch).rotate_z(roll)
    }

    /// Project 3D point to 2D canvas coordinates using perspective projection.
    #[must_use]
    pub fn project_perspective(
        &self,
        canvas_width: f64,
        canvas_height: f64,
        focal_length: f64,
        camera_distance: f64,
    ) -> (f64, f64, f64) {
        let z_eff = self.z + camera_distance;
        let scale = if z_eff <= 0.01 {
            focal_length / 0.01
        } else {
            focal_length / z_eff
        };
        let sx = canvas_width / 2.0 + self.x * scale;
        let sy = canvas_height / 2.0 - self.y * scale; // Inverted Y for screen space
        (sx, sy, z_eff)
    }
}

/// 3D Mesh containing vertices, edge connections, and polygonal faces.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelMesh3D {
    pub vertices: Vec<Point3D>,
    pub edges: Vec<(usize, usize)>,
    pub faces: Vec<Vec<usize>>,
}

impl ModelMesh3D {
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
        }
    }

    /// Construct a regular 3D Cube.
    #[must_use]
    pub fn cube(size: f64) -> Self {
        let s = size / 2.0;
        let vertices = vec![
            Point3D::new(-s, -s, -s), // 0
            Point3D::new(s, -s, -s),  // 1
            Point3D::new(s, s, -s),   // 2
            Point3D::new(-s, s, -s),  // 3
            Point3D::new(-s, -s, s),  // 4
            Point3D::new(s, -s, s),   // 5
            Point3D::new(s, s, s),    // 6
            Point3D::new(-s, s, s),   // 7
        ];

        let edges = vec![
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // Back face
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4), // Front face
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7), // Connecting edges
        ];

        let faces = vec![
            vec![0, 1, 2, 3], // Back
            vec![4, 5, 6, 7], // Front
            vec![0, 4, 7, 3], // Left
            vec![1, 5, 6, 2], // Right
            vec![3, 2, 6, 7], // Top
            vec![0, 1, 5, 4], // Bottom
        ];

        Self {
            vertices,
            edges,
            faces,
        }
    }

    /// Construct a 3D Pyramid with square base.
    #[must_use]
    pub fn pyramid(base: f64, height: f64) -> Self {
        let b = base / 2.0;
        let h = height / 2.0;
        let vertices = vec![
            Point3D::new(-b, -h, -b),  // 0: base
            Point3D::new(b, -h, -b),   // 1: base
            Point3D::new(b, -h, b),    // 2: base
            Point3D::new(-b, -h, b),   // 3: base
            Point3D::new(0.0, h, 0.0), // 4: apex
        ];

        let edges = vec![
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // base
            (0, 4),
            (1, 4),
            (2, 4),
            (3, 4), // sides
        ];

        let faces = vec![
            vec![0, 1, 2, 3], // base
            vec![0, 1, 4],    // front
            vec![1, 2, 4],    // right
            vec![2, 3, 4],    // back
            vec![3, 0, 4],    // left
        ];

        Self {
            vertices,
            edges,
            faces,
        }
    }

    /// Construct a regular Octahedron.
    #[must_use]
    pub fn octahedron(size: f64) -> Self {
        let s = size / 2.0;
        let vertices = vec![
            Point3D::new(s, 0.0, 0.0),  // 0
            Point3D::new(-s, 0.0, 0.0), // 1
            Point3D::new(0.0, s, 0.0),  // 2
            Point3D::new(0.0, -s, 0.0), // 3
            Point3D::new(0.0, 0.0, s),  // 4
            Point3D::new(0.0, 0.0, -s), // 5
        ];

        let edges = vec![
            (0, 2),
            (2, 1),
            (1, 3),
            (3, 0),
            (0, 4),
            (1, 4),
            (2, 4),
            (3, 4),
            (0, 5),
            (1, 5),
            (2, 5),
            (3, 5),
        ];

        let faces = vec![
            vec![0, 2, 4],
            vec![2, 1, 4],
            vec![1, 3, 4],
            vec![3, 0, 4],
            vec![0, 2, 5],
            vec![2, 1, 5],
            vec![1, 3, 5],
            vec![3, 0, 5],
        ];

        Self {
            vertices,
            edges,
            faces,
        }
    }

    /// Construct a 3D Cylinder.
    #[must_use]
    pub fn cylinder(radius: f64, height: f64, segments: usize) -> Self {
        let n = segments.max(6);
        let h2 = height / 2.0;
        let mut vertices = Vec::with_capacity(n * 2);
        let mut edges = Vec::new();
        let mut faces = Vec::new();

        for i in 0..n {
            let theta = 2.0 * PI * (i as f64) / (n as f64);
            let x = radius * theta.cos();
            let z = radius * theta.sin();
            vertices.push(Point3D::new(x, -h2, z));
        }
        for i in 0..n {
            let theta = 2.0 * PI * (i as f64) / (n as f64);
            let x = radius * theta.cos();
            let z = radius * theta.sin();
            vertices.push(Point3D::new(x, h2, z));
        }

        for i in 0..n {
            let next = (i + 1) % n;
            edges.push((i, next));
            edges.push((n + i, n + next));
            edges.push((i, n + i));
            faces.push(vec![i, next, n + next, n + i]);
        }

        Self {
            vertices,
            edges,
            faces,
        }
    }
}

impl Default for ModelMesh3D {
    fn default() -> Self {
        Self::cube(100.0)
    }
}

/// 3D Wireframe & Mesh Visualizer Component.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelViewer3D {
    pub id: String,
    pub mesh: ModelMesh3D,
    pub width: u32,
    pub height: u32,
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub roll_deg: f64,
    pub focal_length: f64,
    pub camera_distance: f64,
    pub wireframe_color: String,
    pub face_fill_color: Option<String>,
    pub show_axes: bool,
    pub interactive_orbit: bool,
    pub class: String,
}

impl ModelViewer3D {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            mesh: ModelMesh3D::cube(120.0),
            width: 480,
            height: 360,
            yaw_deg: 35.0,
            pitch_deg: -25.0,
            roll_deg: 0.0,
            focal_length: 350.0,
            camera_distance: 280.0,
            wireframe_color: "#6366f1".to_string(), // Indigo
            face_fill_color: Some("rgba(99, 102, 241, 0.12)".to_string()),
            show_axes: true,
            interactive_orbit: true,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn mesh(mut self, mesh: ModelMesh3D) -> Self {
        self.mesh = mesh;
        self
    }

    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width.max(100);
        self.height = height.max(100);
        self
    }

    #[must_use]
    pub fn rotation(mut self, yaw_deg: f64, pitch_deg: f64, roll_deg: f64) -> Self {
        self.yaw_deg = yaw_deg;
        self.pitch_deg = pitch_deg;
        self.roll_deg = roll_deg;
        self
    }

    #[must_use]
    pub fn wireframe_color(mut self, color: impl Into<String>) -> Self {
        self.wireframe_color = color.into();
        self
    }

    #[must_use]
    pub fn face_fill_color(mut self, color: Option<String>) -> Self {
        self.face_fill_color = color;
        self
    }

    #[must_use]
    pub fn show_axes(mut self, show: bool) -> Self {
        self.show_axes = show;
        self
    }

    #[must_use]
    pub fn interactive_orbit(mut self, interactive: bool) -> Self {
        self.interactive_orbit = interactive;
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for ModelViewer3D {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-model-viewer-3d bg-slate-950 border border-slate-800 rounded-xl overflow-hidden shadow-xl flex flex-col relative select-none",
            &self.class,
        ]);

        let w = self.width as f64;
        let h = self.height as f64;

        let yaw_rad = self.yaw_deg.to_radians();
        let pitch_rad = self.pitch_deg.to_radians();
        let roll_rad = self.roll_deg.to_radians();

        let mut projected: Vec<(f64, f64, f64)> = Vec::with_capacity(self.mesh.vertices.len());
        for v in &self.mesh.vertices {
            let rotated = v.rotate_euler(yaw_rad, pitch_rad, roll_rad);
            let p = rotated.project_perspective(w, h, self.focal_length, self.camera_distance);
            projected.push(p);
        }

        let mut svg_content = String::new();

        if self.show_axes {
            let origin = Point3D::new(0.0, 0.0, 0.0)
                .rotate_euler(yaw_rad, pitch_rad, roll_rad)
                .project_perspective(w, h, self.focal_length, self.camera_distance);
            let ax_x = Point3D::new(60.0, 0.0, 0.0)
                .rotate_euler(yaw_rad, pitch_rad, roll_rad)
                .project_perspective(w, h, self.focal_length, self.camera_distance);
            let ax_y = Point3D::new(0.0, 60.0, 0.0)
                .rotate_euler(yaw_rad, pitch_rad, roll_rad)
                .project_perspective(w, h, self.focal_length, self.camera_distance);
            let ax_z = Point3D::new(0.0, 0.0, 60.0)
                .rotate_euler(yaw_rad, pitch_rad, roll_rad)
                .project_perspective(w, h, self.focal_length, self.camera_distance);

            svg_content.push_str(&format!(
                r##"<g opacity="0.4" stroke-width="1.5">
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#ef4444" stroke-dasharray="3,2"/>
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#22c55e" stroke-dasharray="3,2"/>
<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#3b82f6" stroke-dasharray="3,2"/>
</g>"##,
                origin.0,
                origin.1,
                ax_x.0,
                ax_x.1,
                origin.0,
                origin.1,
                ax_y.0,
                ax_y.1,
                origin.0,
                origin.1,
                ax_z.0,
                ax_z.1,
            ));
        }

        if let Some(fill) = &self.face_fill_color {
            let mut face_depths: Vec<(usize, f64)> = self
                .mesh
                .faces
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let avg_z: f64 =
                        f.iter().map(|&idx| projected[idx].2).sum::<f64>() / (f.len() as f64);
                    (i, avg_z)
                })
                .collect();

            face_depths.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (face_idx, _) in face_depths {
                let face = &self.mesh.faces[face_idx];
                let points_str = face
                    .iter()
                    .map(|&idx| format!("{:.1},{:.1}", projected[idx].0, projected[idx].1))
                    .collect::<Vec<_>>()
                    .join(" ");

                svg_content.push_str(&format!(
                    r##"<polygon points="{}" fill="{}" stroke="{}" stroke-width="1" stroke-linejoin="round"/>"##,
                    points_str, fill, self.wireframe_color
                ));
            }
        }

        for &(a, b) in &self.mesh.edges {
            if a < projected.len() && b < projected.len() {
                svg_content.push_str(&format!(
                    r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="1.8" stroke-linecap="round"/>"##,
                    projected[a].0,
                    projected[a].1,
                    projected[b].0,
                    projected[b].1,
                    self.wireframe_color
                ));
            }
        }

        for p in &projected {
            svg_content.push_str(&format!(
                r##"<circle cx="{:.1}" cy="{:.1}" r="3" fill="#a5b4fc"/>"##,
                p.0, p.1
            ));
        }

        let header = Element::new("div")
            .attr("class", "flex items-center justify-between px-3 py-2 border-b border-slate-800/80 bg-slate-900/50 text-xs text-slate-400")
            .child(
                Element::new("div")
                    .attr("class", "flex items-center gap-2")
                    .child(Element::new("span").attr("class", "font-mono font-semibold text-indigo-400").child("3D MESH"))
                    .child(
                        Element::new("span")
                            .attr("class", "text-slate-500")
                            .child(format!("{} sommets, {} arêtes", self.mesh.vertices.len(), self.mesh.edges.len()))
                    )
            )
            .child(
                Element::new("div")
                    .attr("class", "flex items-center gap-2 font-mono text-[11px] text-slate-400")
                    .child(Element::new("span").child(format!("Yaw: {:.0}°", self.yaw_deg)))
                    .child(Element::new("span").child(format!("Pitch: {:.0}°", self.pitch_deg)))
            );

        let svg_markup = format!(
            r##"<svg id="{id}_svg" class="w-full h-auto block" viewBox="0 0 {w} {h}">{svg_content}</svg>"##,
            id = escape_html(&self.id),
        );

        let svg_wrapper = Element::new("div")
            .attr(
                "class",
                "relative w-full overflow-hidden cursor-grab active:cursor-grabbing",
            )
            .child(Node::Raw(svg_markup));

        let script = if self.interactive_orbit {
            let vertices_json: String = serde_json_like_vertices(&self.mesh.vertices);
            let edges_json: String = serde_json_like_edges(&self.mesh.edges);
            let safe_id = self
                .id
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace(['\n', '\r'], " ");
            let safe_color = self
                .wireframe_color
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace(['\n', '\r'], " ");

            format!(
                r##"(function(){{
const root = document.getElementById('{safe_id}');
if (!root) return;
const svg = root.querySelector('svg');
if (!svg) return;

const verts = {vertices_json};
const edges = {edges_json};
let yaw = {yaw_rad};
let pitch = {pitch_rad};
const focal = {focal_length};
const camDist = {camera_distance};
const w = {w};
const h = {h};
const color = '{safe_color}';

let isDragging = false;
let startX = 0;
let startY = 0;

function project(vx, vy, vz) {{
  const cosY = Math.cos(yaw), sinY = Math.sin(yaw);
  const x1 = vx * cosY + vz * sinY;
  const z1 = -vx * sinY + vz * cosY;

  const cosP = Math.cos(pitch), sinP = Math.sin(pitch);
  const y2 = vy * cosP - z1 * sinP;
  const z2 = vy * sinP + z1 * cosP;

  const zEff = z2 + camDist;
  const scale = zEff <= 0.01 ? focal / 0.01 : focal / zEff;
  return [w / 2 + x1 * scale, h / 2 - y2 * scale];
}}

function render3D() {{
  const proj = verts.map(v => project(v[0], v[1], v[2]));
  let html = '';
  edges.forEach(e => {{
    const p1 = proj[e[0]], p2 = proj[e[1]];
    if (p1 && p2) {{
      html += `<line x1="${{p1[0].toFixed(1)}}" y1="${{p1[1].toFixed(1)}}" x2="${{p2[0].toFixed(1)}}" y2="${{p2[1].toFixed(1)}}" stroke="${{color}}" stroke-width="1.8" stroke-linecap="round"/>`;
    }}
  }});
  proj.forEach(p => {{
    html += `<circle cx="${{p[0].toFixed(1)}}" cy="${{p[1].toFixed(1)}}" r="3" fill="#a5b4fc"/>`;
  }});
  svg.innerHTML = html;
}}

root.addEventListener('mousedown', (e) => {{
  isDragging = true;
  startX = e.clientX;
  startY = e.clientY;
}});

window.addEventListener('mousemove', (e) => {{
  if (!isDragging) return;
  const dx = e.clientX - startX;
  const dy = e.clientY - startY;
  startX = e.clientX;
  startY = e.clientY;
  yaw += dx * 0.01;
  pitch -= dy * 0.01;
  render3D();
}});

window.addEventListener('mouseup', () => {{
  isDragging = false;
}});
}})();"##,
                safe_id = safe_id,
                vertices_json = vertices_json,
                edges_json = edges_json,
                yaw_rad = yaw_rad,
                pitch_rad = pitch_rad,
                focal_length = self.focal_length,
                camera_distance = self.camera_distance,
                w = w,
                h = h,
                safe_color = safe_color,
            )
        } else {
            String::new()
        };

        let mut container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(header)
            .child(svg_wrapper);

        if !script.is_empty() {
            container = container.child(Element::new("script").child(script));
        }

        Node::Element(container)
    }
}

fn serde_json_like_vertices(vertices: &[Point3D]) -> String {
    let mut s = String::from("[");
    for (i, v) in vertices.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("[{:.2},{:.2},{:.2}]", v.x, v.y, v.z));
    }
    s.push(']');
    s
}

fn serde_json_like_edges(edges: &[(usize, usize)]) -> String {
    let mut s = String::from("[");
    for (i, &(a, b)) in edges.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("[{a},{b}]"));
    }
    s.push(']');
    s
}

/// Helper function to create a new `ModelViewer3D`.
#[must_use]
pub fn model_viewer_3d(id: impl Into<String>) -> ModelViewer3D {
    ModelViewer3D::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_point3d_rotations_and_projections() {
        let p = Point3D::new(10.0, 0.0, 0.0);
        let rotated = p.rotate_y(PI / 2.0); // 90 degrees around Y -> (0, 0, -10)
        assert!((rotated.x).abs() < 1e-6);
        assert!((rotated.z - (-10.0)).abs() < 1e-6);

        let (sx, sy, z_eff) = p.project_perspective(400.0, 300.0, 200.0, 100.0);
        assert!(sx > 200.0);
        assert_eq!(sy, 150.0);
        assert_eq!(z_eff, 100.0);
    }

    #[test]
    fn test_model_mesh_presets() {
        let cube = ModelMesh3D::cube(80.0);
        assert_eq!(cube.vertices.len(), 8);
        assert_eq!(cube.edges.len(), 12);
        assert_eq!(cube.faces.len(), 6);

        let pyramid = ModelMesh3D::pyramid(60.0, 80.0);
        assert_eq!(pyramid.vertices.len(), 5);
        assert_eq!(pyramid.edges.len(), 8);
        assert_eq!(pyramid.faces.len(), 5);

        let octa = ModelMesh3D::octahedron(50.0);
        assert_eq!(octa.vertices.len(), 6);
        assert_eq!(octa.edges.len(), 12);

        let cyl = ModelMesh3D::cylinder(40.0, 60.0, 8);
        assert_eq!(cyl.vertices.len(), 16);
    }

    #[test]
    fn test_model_viewer_3d_rendering() {
        let viewer = ModelViewer3D::new("cube_mesh")
            .mesh(ModelMesh3D::cube(100.0))
            .dimensions(500, 400)
            .rotation(45.0, -30.0, 0.0)
            .wireframe_color("#6366f1")
            .face_fill_color(Some("rgba(99, 102, 241, 0.2)".to_string()))
            .show_axes(true)
            .interactive_orbit(true);

        let node = viewer.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-model-viewer-3d"));
        assert!(html.contains("cube_mesh"));
        assert!(html.contains("3D MESH"));
        assert!(html.contains("8 sommets, 12 arêtes"));
        assert!(html.contains("polygon"));
        assert!(html.contains("line"));
        assert!(html.contains("circle"));
        assert!(html.contains("render3D"));
    }
}
