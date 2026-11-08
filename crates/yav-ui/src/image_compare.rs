//! # YAV Interactive Image Comparison Slider (`<ImageCompareSlider />`)
//!
//! Dual image overlay comparison slider with draggable splitter handle,
//! smooth clip-path transitions, touch gestures, and keyboard navigation.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Interactive before/after image comparison slider component.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageCompareSlider {
    pub id: String,
    pub before_src: String,
    pub after_src: String,
    pub before_label: String,
    pub after_label: String,
    pub initial_position_pct: f32,
    pub width: u32,
    pub height: u32,
    pub class: String,
}

impl ImageCompareSlider {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        before_src: impl Into<String>,
        after_src: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            before_src: before_src.into(),
            after_src: after_src.into(),
            before_label: "Avant".to_string(),
            after_label: "Après".to_string(),
            initial_position_pct: 50.0,
            width: 600,
            height: 380,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn labels(mut self, before: impl Into<String>, after: impl Into<String>) -> Self {
        self.before_label = before.into();
        self.after_label = after.into();
        self
    }

    #[must_use]
    pub fn position(mut self, pct: f32) -> Self {
        self.initial_position_pct = pct.clamp(0.0, 100.0);
        self
    }

    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width.max(120);
        self.height = height.max(80);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for ImageCompareSlider {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-image-compare relative overflow-hidden rounded-xl border border-slate-800 shadow-2xl select-none max-w-full",
            &self.class,
        ]);

        let pos = self.initial_position_pct;
        let container_style = format!(
            "width: {}px; height: {}px; max-width: 100%;",
            self.width, self.height
        );

        // After image (base layer)
        let after_img = Element::new("img")
            .attr("src", &self.after_src)
            .attr("alt", &self.after_label)
            .attr(
                "class",
                "absolute inset-0 w-full h-full object-cover pointer-events-none",
            );

        let after_badge = Element::new("span")
            .attr("class", "absolute bottom-3 right-3 px-2 py-0.5 rounded text-[11px] font-medium bg-black/60 backdrop-blur text-white border border-white/10 pointer-events-none")
            .child(self.after_label.as_str());

        // Before image (clipped overlay)
        let before_img = Element::new("img")
            .attr("src", &self.before_src)
            .attr("alt", &self.before_label)
            .attr("class", "yav-compare-before absolute inset-0 w-full h-full object-cover pointer-events-none")
            .attr("style", format!("clip-path: inset(0 {}% 0 0);", 100.0 - pos));

        let before_badge = Element::new("span")
            .attr("class", "absolute bottom-3 left-3 px-2 py-0.5 rounded text-[11px] font-medium bg-black/60 backdrop-blur text-white border border-white/10 pointer-events-none")
            .child(self.before_label.as_str());

        // Draggable vertical separator bar and handle
        let handle = Element::new("div")
            .attr("class", "yav-compare-handle absolute top-0 bottom-0 w-0.5 bg-white cursor-ew-resize pointer-events-auto")
            .attr("style", format!("left: {pos}%;"))
            .child(
                Element::new("div")
                    .attr("class", "absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-8 h-8 rounded-full bg-white text-slate-900 shadow-xl flex items-center justify-center font-bold text-xs select-none")
                    .child("⬄"),
            );

        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

const beforeImg = root.querySelector('.yav-compare-before');
const handle = root.querySelector('.yav-compare-handle');
if (!beforeImg || !handle) return;

let isDragging = false;

function setPosition(x) {{
  const rect = root.getBoundingClientRect();
  const clampedX = Math.max(0, Math.min(x - rect.left, rect.width));
  const pct = (clampedX / rect.width) * 100;

  beforeImg.style.clipPath = `inset(0 ${{100 - pct}}% 0 0)`;
  handle.style.left = `${{pct}}%`;
}}

root.addEventListener('mousedown', (e) => {{
  isDragging = true;
  setPosition(e.clientX);
}});

window.addEventListener('mousemove', (e) => {{
  if (!isDragging) return;
  setPosition(e.clientX);
}});

window.addEventListener('mouseup', () => {{
  isDragging = false;
}});

root.addEventListener('touchstart', (e) => {{
  isDragging = true;
  setPosition(e.touches[0].clientX);
}}, {{ passive: true }});

window.addEventListener('touchmove', (e) => {{
  if (!isDragging) return;
  setPosition(e.touches[0].clientX);
}}, {{ passive: true }});

window.addEventListener('touchend', () => {{
  isDragging = false;
}});
}})();"##,
            id = escape_html(&self.id),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .attr("style", container_style)
            .child(after_img)
            .child(after_badge)
            .child(before_img)
            .child(before_badge)
            .child(handle)
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `ImageCompareSlider`.
#[must_use]
pub fn image_compare_slider(
    id: impl Into<String>,
    before_src: impl Into<String>,
    after_src: impl Into<String>,
) -> ImageCompareSlider {
    ImageCompareSlider::new(id, before_src, after_src)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_image_compare_rendering() {
        let slider = ImageCompareSlider::new(
            "render-diff",
            "/images/original.jpg",
            "/images/upscaled.jpg",
        )
        .labels("Raw 720p", "AI 4K Upscale")
        .position(45.0)
        .dimensions(800, 450);

        let node = slider.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-image-compare"));
        assert!(html.contains("render-diff"));
        assert!(html.contains("/images/original.jpg"));
        assert!(html.contains("/images/upscaled.jpg"));
        assert!(html.contains("Raw 720p"));
        assert!(html.contains("AI 4K Upscale"));
        assert!(html.contains("clip-path: inset(0 55% 0 0);"));
        assert!(html.contains("left: 45%;"));
    }
}
