//! Stepper and progress timeline component.
//!
//! Provides a step-by-step progress indicator (`Stepper`, `StepItem`, `StepStatus`)
//! commonly used in multi-step checkouts, onboarding workflows, or order tracking.

use yav_core::{Component, Element, IntoNode, Node};

use crate::layout::Orientation;
use crate::utils::merge_classes;

/// The completion status of a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepStatus {
    /// Step has been completed successfully.
    Complete,
    /// Step is currently active.
    Current,
    /// Step has not yet been started.
    #[default]
    Upcoming,
}

/// An individual step within a [`Stepper`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StepItem {
    pub title: String,
    pub description: Option<String>,
    pub status: StepStatus,
    pub icon: Option<Node>,
    pub class: String,
}

impl StepItem {
    /// Create a new step with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set an optional step description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the step completion status.
    pub fn status(mut self, status: StepStatus) -> Self {
        self.status = status;
        self
    }

    /// Set an optional custom icon string, node, or SVG.
    pub fn icon(mut self, icon: impl IntoNode) -> Self {
        self.icon = Some(icon.into_node());
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

/// A multi-step progress indicator.
#[derive(Debug, Clone, PartialEq)]
pub struct Stepper {
    pub steps: Vec<StepItem>,
    pub orientation: Orientation,
    pub current_step: Option<usize>,
    pub class: String,
}

impl Default for Stepper {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            orientation: Orientation::Horizontal,
            current_step: None,
            class: String::new(),
        }
    }
}

impl Stepper {
    /// Create a new stepper.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a step.
    pub fn step(mut self, step: StepItem) -> Self {
        self.steps.push(step);
        self
    }

    /// Add multiple steps.
    pub fn steps(mut self, steps: impl IntoIterator<Item = StepItem>) -> Self {
        self.steps.extend(steps);
        self
    }

    /// Set the active step index (0-based). Steps before this are marked Complete,
    /// this step is marked Current, and subsequent steps are marked Upcoming.
    pub fn current_step(mut self, step_idx: usize) -> Self {
        self.current_step = Some(step_idx);
        self
    }

    /// Set orientation to horizontal.
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    /// Set orientation to vertical.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the stepper into a virtual DOM [`Node`].
    pub fn render(mut self) -> Node {
        // If current_step was specified, update statuses of steps
        if let Some(curr) = self.current_step {
            for (i, step) in self.steps.iter_mut().enumerate() {
                if i < curr {
                    step.status = StepStatus::Complete;
                } else if i == curr {
                    step.status = StepStatus::Current;
                } else {
                    step.status = StepStatus::Upcoming;
                }
            }
        }

        let is_vertical = self.orientation == Orientation::Vertical;
        let base_class = if is_vertical {
            "flex flex-col space-y-4"
        } else {
            "flex items-center w-full"
        };
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut root = Element::new("ol")
            .attr("class", full_class)
            .attr("role", "list");

        let total = self.steps.len();
        for (i, step) in self.steps.into_iter().enumerate() {
            let is_last = i == total - 1;

            let (badge_bg, badge_text, badge_border, title_color) = match step.status {
                StepStatus::Complete => (
                    "bg-primary-600 dark:bg-primary-500",
                    "text-white",
                    "border-primary-600 dark:border-primary-500",
                    "text-gray-900 dark:text-white font-medium",
                ),
                StepStatus::Current => (
                    "bg-white dark:bg-gray-900",
                    "text-primary-600 dark:text-primary-400",
                    "border-2 border-primary-600 dark:border-primary-400",
                    "text-primary-600 dark:text-primary-400 font-semibold",
                ),
                StepStatus::Upcoming => (
                    "bg-gray-100 dark:bg-gray-800",
                    "text-gray-500 dark:text-gray-400",
                    "border border-gray-300 dark:border-gray-700",
                    "text-gray-500 dark:text-gray-400",
                ),
            };

            let item_class = if is_vertical {
                "relative flex items-start space-x-3.5"
            } else if is_last {
                "flex items-center"
            } else {
                "flex items-center flex-1"
            };

            let mut li = Element::new("li").attr("class", item_class).attr(
                "aria-current",
                if step.status == StepStatus::Current {
                    "step"
                } else {
                    "false"
                },
            );

            // Step circle indicator
            let indicator_class = format!("flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-semibold {badge_bg} {badge_text} {badge_border}");
            let mut indicator = Element::new("span").attr("class", indicator_class);

            if let Some(icon) = step.icon {
                indicator = indicator.child(icon);
            } else if step.status == StepStatus::Complete {
                // Checkmark SVG
                indicator = indicator.child(Node::raw(
                    r#"<svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/></svg>"#,
                ));
            } else {
                indicator = indicator.child(Node::text((i + 1).to_string()));
            }

            li = li.child(Node::Element(indicator));

            // Title & Description
            let mut label_wrap = Element::new("div").attr("class", "ml-2.5");
            let title_el = Element::new("p")
                .attr("class", format!("text-sm {title_color}"))
                .child(Node::text(step.title));
            label_wrap = label_wrap.child(Node::Element(title_el));

            if let Some(desc) = step.description {
                let desc_el = Element::new("p")
                    .attr("class", "text-xs text-gray-400 dark:text-gray-500 mt-0.5")
                    .child(Node::text(desc));
                label_wrap = label_wrap.child(Node::Element(desc_el));
            }

            li = li.child(Node::Element(label_wrap));

            // Connector line for horizontal layout
            if !is_vertical && !is_last {
                let line_color = if step.status == StepStatus::Complete {
                    "bg-primary-600 dark:bg-primary-500"
                } else {
                    "bg-gray-200 dark:bg-gray-700"
                };
                let line = Element::new("div")
                    .attr("class", format!("mx-4 h-0.5 flex-1 {line_color}"))
                    .attr("aria-hidden", "true");
                li = li.child(Node::Element(line));
            }

            root = root.child(Node::Element(li));
        }

        Node::Element(root)
    }
}

impl IntoNode for Stepper {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Stepper> for Node {
    fn from(s: Stepper) -> Self {
        s.render()
    }
}

impl Component for Stepper {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to create a new [`Stepper`].
pub fn stepper() -> Stepper {
    Stepper::new()
}

/// Properties for constructing a [`Stepper`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StepperProps {
    pub steps: Vec<StepItem>,
    pub orientation: Option<Orientation>,
    pub current_step: Option<usize>,
    pub class: String,
}

/// Functional component adapter for [`Stepper`], enabling `<Stepper steps={...} current_step={1} />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Stepper(props: StepperProps) -> Node {
    let mut s = Stepper::new().steps(props.steps);
    if let Some(Orientation::Vertical) = props.orientation {
        s = s.vertical();
    }
    if let Some(curr) = props.current_step {
        s = s.current_step(curr);
    }
    if !props.class.is_empty() {
        s = s.class(props.class);
    }
    s.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn stepper_renders_steps_with_status() {
        let s = stepper()
            .step(StepItem::new("Configure").description("Select engine and trim"))
            .step(StepItem::new("Financing").description("Calculate monthly rate"))
            .step(StepItem::new("Delivery").description("Choose pickup center"))
            .current_step(1)
            .render();

        let html = render_to_string(&s);
        assert!(html.contains("Configure"));
        assert!(html.contains("Financing"));
        assert!(html.contains("Delivery"));
        assert!(html.contains("Select engine and trim"));
        assert!(html.contains("aria-current=\"step\""));
        assert!(html.contains("aria-current=\"false\""));
    }
}
