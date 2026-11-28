//! # YAV Spotlight Tour & Product Onboarding (`<SpotlightTour />`)
//!
//! Interactive step-by-step guided tour with element spotlighting,
//! dark cutout mask overlay, popover tooltips, and local storage state persistence.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Placement of the tour tooltip relative to the target DOM element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TourPlacement {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
    Center,
}

impl TourPlacement {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
        }
    }
}

/// A single step in an onboarding or product tour.
#[derive(Debug, Clone, PartialEq)]
pub struct TourStep {
    pub id: String,
    pub target_selector: String,
    pub title: String,
    pub content: String,
    pub placement: TourPlacement,
    pub badge_text: Option<String>,
}

impl TourStep {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        target_selector: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            target_selector: target_selector.into(),
            title: title.into(),
            content: content.into(),
            placement: TourPlacement::Bottom,
            badge_text: None,
        }
    }

    #[must_use]
    pub fn placement(mut self, placement: TourPlacement) -> Self {
        self.placement = placement;
        self
    }

    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge_text = Some(badge.into());
        self
    }
}

/// Guided onboarding tour component.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotlightTour {
    pub id: String,
    pub steps: Vec<TourStep>,
    pub storage_key: String,
    pub show_progress: bool,
    pub allow_skip: bool,
    pub mask_opacity: f32,
    pub class: String,
}

impl SpotlightTour {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into();
        let default_storage_key = format!("yav_tour_{id_str}_completed");
        Self {
            id: id_str,
            steps: Vec::new(),
            storage_key: default_storage_key,
            show_progress: true,
            allow_skip: true,
            mask_opacity: 0.75,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn step(mut self, step: TourStep) -> Self {
        self.steps.push(step);
        self
    }

    #[must_use]
    pub fn steps(mut self, steps: impl IntoIterator<Item = TourStep>) -> Self {
        self.steps.extend(steps);
        self
    }

    #[must_use]
    pub fn storage_key(mut self, key: impl Into<String>) -> Self {
        self.storage_key = key.into();
        self
    }

    #[must_use]
    pub fn show_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    #[must_use]
    pub fn allow_skip(mut self, allow: bool) -> Self {
        self.allow_skip = allow;
        self
    }

    #[must_use]
    pub fn mask_opacity(mut self, opacity: f32) -> Self {
        self.mask_opacity = opacity.clamp(0.1, 0.95);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for SpotlightTour {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-spotlight-tour fixed inset-0 pointer-events-none z-[9999] hidden",
            &self.class,
        ]);

        let total_steps = self.steps.len();

        let steps_payload: Vec<serde_json::Value> = self
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "target": s.target_selector,
                    "title": s.title,
                    "content": s.content,
                    "placement": s.placement.as_str(),
                    "badge": s.badge_text,
                })
            })
            .collect();
        let steps_json = yav_core::escape_script_json(
            &serde_json::to_string(&steps_payload).unwrap_or_else(|_| "[]".to_string()),
        );

        let mut steps_dom = Vec::new();
        for (idx, step) in self.steps.iter().enumerate() {
            let step_num = idx + 1;
            let hidden_class = if idx == 0 { "" } else { " hidden" };

            let mut step_card = Element::new("div")
                .attr(
                    "class",
                    format!(
                        "yav-tour-step-card absolute bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-5 w-80 text-white pointer-events-auto transition-all duration-300{hidden_class}"
                    ),
                )
                .attr("data-step-index", idx.to_string())
                .attr("data-target", &step.target_selector)
                .attr("data-placement", step.placement.as_str());

            // Header with badge and step counter
            let mut header = Element::new("div").attr(
                "class",
                "flex items-center justify-between gap-2 mb-2 pb-2 border-b border-slate-800",
            );

            let mut left_header = Element::new("div").attr("class", "flex items-center gap-2");
            if let Some(badge) = &step.badge_text {
                left_header = left_header.child(
                    Element::new("span")
                        .attr(
                            "class",
                            "px-2 py-0.5 text-xs font-semibold rounded bg-indigo-500/20 text-indigo-400 border border-indigo-500/30",
                        )
                        .child(badge.as_str()),
                );
            }

            if self.show_progress {
                left_header = left_header.child(
                    Element::new("span")
                        .attr("class", "text-xs font-mono text-slate-400")
                        .child(format!("{step_num} / {total_steps}")),
                );
            }
            header = header.child(left_header);

            if self.allow_skip {
                header = header.child(
                    Element::new("button")
                        .attr("type", "button")
                        .attr(
                            "class",
                            "yav-tour-skip text-xs text-slate-400 hover:text-slate-200 transition-colors",
                        )
                        .attr("title", "Passer la visite")
                        .child("✕"),
                );
            }
            step_card = step_card.child(header);

            // Title
            step_card = step_card.child(
                Element::new("h4")
                    .attr("class", "text-sm font-semibold text-slate-100 mb-1")
                    .child(step.title.as_str()),
            );

            // Content
            step_card = step_card.child(
                Element::new("p")
                    .attr("class", "text-xs text-slate-300 leading-relaxed mb-4")
                    .child(step.content.as_str()),
            );

            // Footer navigation buttons
            let mut footer =
                Element::new("div").attr("class", "flex items-center justify-between gap-2 pt-2");

            // Previous button
            let prev_disabled = if idx == 0 {
                " opacity-40 cursor-not-allowed pointer-events-none"
            } else {
                ""
            };
            let prev_btn = Element::new("button")
                .attr("type", "button")
                .attr(
                    "class",
                    format!(
                        "yav-tour-prev px-2.5 py-1 text-xs font-medium rounded bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors{prev_disabled}"
                    ),
                )
                .child("Précédent");
            footer = footer.child(prev_btn);

            // Next or Finish button
            let is_last = idx == total_steps - 1;
            let (next_label, next_class) = if is_last {
                ("Terminer", "bg-emerald-600 hover:bg-emerald-500 text-white")
            } else {
                ("Suivant", "bg-indigo-600 hover:bg-indigo-500 text-white")
            };

            let next_btn = Element::new("button")
                .attr("type", "button")
                .attr(
                    "class",
                    format!(
                        "yav-tour-next px-3 py-1 text-xs font-semibold rounded transition-colors shadow-sm {next_class}"
                    ),
                )
                .child(next_label);
            footer = footer.child(next_btn);

            step_card = step_card.child(footer);
            steps_dom.push(step_card);
        }

        // SVG Mask layer with dynamic cutout circle/rect
        let svg_mask = Element::new("svg")
            .attr(
                "class",
                "yav-tour-mask absolute inset-0 w-full h-full pointer-events-auto transition-all duration-300",
            )
            .attr(
                "style",
                format!("background-color: rgba(0, 0, 0, {});", self.mask_opacity),
            );

        // Interactive Tour Script
        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;
const storageKey = '{storage_key}';
if (localStorage.getItem(storageKey) === 'true') return;

let currentStep = 0;
const cards = root.querySelectorAll('.yav-tour-step-card');
if (cards.length === 0) return;

function showTour() {{
  root.classList.remove('hidden');
  updateStep(0);
}}

function updateStep(idx) {{
  if (idx < 0 || idx >= cards.length) return;
  currentStep = idx;
  cards.forEach((c, i) => {{
    if (i === idx) {{
      c.classList.remove('hidden');
      positionCard(c);
    }} else {{
      c.classList.add('hidden');
    }}
  }});
}}

function positionCard(card) {{
  const targetSel = card.getAttribute('data-target');
  const target = document.querySelector(targetSel);
  if (!target) {{
    card.style.top = '50%';
    card.style.left = '50%';
    card.style.transform = 'translate(-50%, -50%)';
    return;
  }}
  const rect = target.getBoundingClientRect();
  const placement = card.getAttribute('data-placement') || 'bottom';
  const margin = 12;

  let top = rect.bottom + margin;
  let left = rect.left;

  if (placement === 'top') {{
    top = Math.max(margin, rect.top - card.offsetHeight - margin);
    left = rect.left;
  }} else if (placement === 'left') {{
    top = rect.top;
    left = Math.max(margin, rect.left - card.offsetWidth - margin);
  }} else if (placement === 'right') {{
    top = rect.top;
    left = rect.right + margin;
  }} else if (placement === 'center') {{
    top = rect.top + (rect.height / 2) - (card.offsetHeight / 2);
    left = rect.left + (rect.width / 2) - (card.offsetWidth / 2);
  }}

  card.style.transform = 'none';
  card.style.top = top + 'px';
  card.style.left = left + 'px';

  target.scrollIntoView({{ behavior: 'smooth', block: 'nearest' }});
}}

function finishTour() {{
  localStorage.setItem(storageKey, 'true');
  root.classList.add('hidden');
}}

root.addEventListener('click', (e) => {{
  if (e.target.closest('.yav-tour-next')) {{
    if (currentStep < cards.length - 1) {{
      updateStep(currentStep + 1);
    }} else {{
      finishTour();
    }}
  }} else if (e.target.closest('.yav-tour-prev')) {{
    if (currentStep > 0) {{
      updateStep(currentStep - 1);
    }}
  }} else if (e.target.closest('.yav-tour-skip')) {{
    finishTour();
  }}
}});

window.addEventListener('keydown', (e) => {{
  if (root.classList.contains('hidden')) return;
  if (e.key === 'Escape') finishTour();
  if (e.key === 'ArrowRight' && currentStep < cards.length - 1) updateStep(currentStep + 1);
  if (e.key === 'ArrowLeft' && currentStep > 0) updateStep(currentStep - 1);
}});

window.yavStartTour_{id} = showTour;
window.yavResetTour_{id} = function() {{ localStorage.removeItem(storageKey); showTour(); }};

setTimeout(showTour, 300);
}})();"##,
            id = escape_html(&self.id),
            storage_key = escape_html(&self.storage_key),
        );

        let mut container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .attr("data-steps", steps_json)
            .child(svg_mask);

        for card in steps_dom {
            container = container.child(card);
        }

        container = container.child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `SpotlightTour`.
#[must_use]
pub fn spotlight_tour(id: impl Into<String>) -> SpotlightTour {
    SpotlightTour::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_spotlight_tour_rendering() {
        let tour = SpotlightTour::new("onboarding")
            .storage_key("app_tour_done")
            .step(
                TourStep::new(
                    "welcome",
                    "#app-header",
                    "Bienvenue sur Yav !",
                    "Voici l'en-tête de navigation principale.",
                )
                .badge("Nouveau")
                .placement(TourPlacement::Bottom),
            )
            .step(
                TourStep::new(
                    "analytics",
                    "#metrics-panel",
                    "Tableau de Bord Métriques",
                    "Consultez ici vos indicateurs de latence et requêtes/sec.",
                )
                .placement(TourPlacement::Top),
            );

        let node = tour.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-spotlight-tour"));
        assert!(html.contains("onboarding"));
        assert!(html.contains("Bienvenue sur Yav !"));
        assert!(html.contains("Tableau de Bord Métriques"));
        assert!(html.contains("Nouveau"));
        assert!(html.contains("1 / 2"));
        assert!(html.contains("2 / 2"));
        assert!(html.contains("app_tour_done"));
        assert!(html.contains("yavStartTour_onboarding"));
    }
}
