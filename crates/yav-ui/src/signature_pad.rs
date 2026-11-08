//! # YAV Touch & Mouse Signature Pad (`<SignaturePad />`)
//!
//! Captures handwritten signatures via HTML5 canvas with smooth stroke rendering,
//! Clear / Undo controls, and hidden form field base64 serialization.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Interactive handwritten signature capture pad component.
#[derive(Debug, Clone, PartialEq)]
pub struct SignaturePad {
    pub id: String,
    pub name: String,
    pub label: Option<String>,
    pub width: u32,
    pub height: u32,
    pub pen_color: String,
    pub background_color: String,
    pub pen_width: f32,
    pub required: bool,
    pub class: String,
}

impl SignaturePad {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            name: id_str.clone(),
            id: id_str,
            label: Some("Signature".to_string()),
            width: 420,
            height: 160,
            pen_color: "#6366f1".to_string(),        // Indigo
            background_color: "#020617".to_string(), // Slate 950
            pen_width: 2.5,
            required: false,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width.max(150);
        self.height = height.max(80);
        self
    }

    #[must_use]
    pub fn pen_color(mut self, color: impl Into<String>) -> Self {
        self.pen_color = color.into();
        self
    }

    #[must_use]
    pub fn pen_width(mut self, width: f32) -> Self {
        self.pen_width = width.max(1.0);
        self
    }

    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for SignaturePad {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-signature-pad flex flex-col gap-2 p-3 bg-slate-900 border border-slate-800 rounded-xl shadow-lg max-w-lg w-full",
            &self.class,
        ]);

        let mut header = Element::new("div").attr(
            "class",
            "flex items-center justify-between text-xs text-slate-400",
        );
        if let Some(lbl) = &self.label {
            let req_star = if self.required {
                r#" <span class="text-rose-500">*</span>"#
            } else {
                ""
            };
            header = header.child(Node::Raw(format!(
                r#"<label for="{}_canvas" class="font-medium text-slate-200">{}{}</label>"#,
                escape_html(&self.id),
                escape_html(lbl),
                req_star
            )));
        }

        let clear_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-sig-clear px-2 py-0.5 text-xs text-slate-400 hover:text-slate-200 transition-colors",
            )
            .child("Effacer");
        header = header.child(clear_btn);

        let canvas = Element::new("canvas")
            .attr("id", format!("{}_canvas", self.id))
            .attr(
                "class",
                "yav-sig-canvas rounded-lg border border-slate-700/80 cursor-crosshair touch-none",
            )
            .attr("width", self.width.to_string())
            .attr("height", self.height.to_string())
            .attr(
                "style",
                format!("background-color: {};", self.background_color),
            );

        let hidden_input = Element::new("input")
            .attr("type", "hidden")
            .attr("name", &self.name)
            .attr("id", format!("{}_data", self.id))
            .attr("class", "yav-sig-data");

        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

const canvas = root.querySelector('.yav-sig-canvas');
const clearBtn = root.querySelector('.yav-sig-clear');
const dataInput = root.querySelector('.yav-sig-data');
if (!canvas) return;

const ctx = canvas.getContext('2d');
let isDrawing = false;
let hasSigned = false;

ctx.strokeStyle = '{pen_color}';
ctx.lineWidth = {pen_width};
ctx.lineCap = 'round';
ctx.lineJoin = 'round';

function getPos(e) {{
  const rect = canvas.getBoundingClientRect();
  const clientX = e.touches ? e.touches[0].clientX : e.clientX;
  const clientY = e.touches ? e.touches[0].clientY : e.clientY;
  return [clientX - rect.left, clientY - rect.top];
}}

function start(e) {{
  e.preventDefault();
  isDrawing = true;
  hasSigned = true;
  const [x, y] = getPos(e);
  ctx.beginPath();
  ctx.moveTo(x, y);
}}

function move(e) {{
  if (!isDrawing) return;
  e.preventDefault();
  const [x, y] = getPos(e);
  ctx.lineTo(x, y);
  ctx.stroke();
}}

function stop() {{
  if (!isDrawing) return;
  isDrawing = false;
  if (hasSigned && dataInput) {{
    dataInput.value = canvas.toDataURL('image/png');
  }}
}}

canvas.addEventListener('mousedown', start);
canvas.addEventListener('mousemove', move);
window.addEventListener('mouseup', stop);

canvas.addEventListener('touchstart', start, {{ passive: false }});
canvas.addEventListener('touchmove', move, {{ passive: false }});
window.addEventListener('touchend', stop);

clearBtn.addEventListener('click', () => {{
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  hasSigned = false;
  if (dataInput) dataInput.value = '';
}});
}})();"##,
            id = escape_html(&self.id),
            pen_color = escape_html(&self.pen_color),
            pen_width = self.pen_width,
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(header)
            .child(canvas)
            .child(hidden_input)
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `SignaturePad`.
#[must_use]
pub fn signature_pad(id: impl Into<String>) -> SignaturePad {
    SignaturePad::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_signature_pad_rendering() {
        let pad = SignaturePad::new("customer_sig")
            .label("Émargement Client")
            .name("client_signature")
            .dimensions(450, 180)
            .pen_color("#22c55e")
            .required(true);

        let node = pad.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-signature-pad"));
        assert!(html.contains("customer_sig"));
        assert!(html.contains("Émargement Client"));
        assert!(html.contains("client_signature"));
        assert!(html.contains("width=\"450\""));
        assert!(html.contains("height=\"180\""));
        assert!(html.contains("yav-sig-clear"));
        assert!(html.contains("yav-sig-data"));
    }
}
