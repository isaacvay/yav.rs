//! Declarative motion and animation components inspired by Framer Motion.
//!
//! Provides declarative physics-based motion primitives (`Motion`, `MotionGroup`,
//! `ScrollProgressBar`, `MotionState`, `Transition`) with spring physics, scroll-triggered
//! reveals (`inView`), hover/tap gestures, and scroll progress tracking.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// The target visual state (transform, opacity, filter) of an animated element.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionState {
    pub opacity: Option<f64>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub scale: Option<f64>,
    pub rotate: Option<f64>,
    pub blur: Option<f64>,
}

impl MotionState {
    /// Create a new, empty motion state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set opacity (0.0 to 1.0).
    pub fn opacity(mut self, opacity: f64) -> Self {
        self.opacity = Some(opacity.clamp(0.0, 1.0));
        self
    }

    /// Set horizontal translation in pixels.
    pub fn x(mut self, x: impl Into<f64>) -> Self {
        self.x = Some(x.into());
        self
    }

    /// Set vertical translation in pixels.
    pub fn y(mut self, y: impl Into<f64>) -> Self {
        self.y = Some(y.into());
        self
    }

    /// Set scale multiplier (e.g. 1.0 for original size, 1.05 for 5% zoom).
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Set rotation in degrees.
    pub fn rotate(mut self, degrees: f64) -> Self {
        self.rotate = Some(degrees);
        self
    }

    /// Set blur filter in pixels.
    pub fn blur(mut self, px: f64) -> Self {
        self.blur = Some(px);
        self
    }

    /// Convert state to inline CSS style string for initial SSR render.
    pub fn to_style(&self) -> String {
        let mut parts = Vec::new();

        if let Some(op) = self.opacity {
            parts.push(format!("opacity: {op};"));
        }

        let mut transforms = Vec::new();
        if self.x.is_some() || self.y.is_some() {
            let x = self.x.unwrap_or(0.0);
            let y = self.y.unwrap_or(0.0);
            transforms.push(format!("translate3d({x}px, {y}px, 0)"));
        }
        if let Some(s) = self.scale {
            transforms.push(format!("scale({s})"));
        }
        if let Some(r) = self.rotate {
            transforms.push(format!("rotate({r}deg)"));
        }

        if !transforms.is_empty() {
            parts.push(format!("transform: {};", transforms.join(" ")));
        }

        if let Some(b) = self.blur {
            parts.push(format!("filter: blur({b}px);"));
        }

        parts.join(" ")
    }

    /// Convert state to JSON string for client-side runtime hydration.
    pub fn to_json(&self) -> String {
        let mut fields = Vec::new();
        if let Some(op) = self.opacity {
            fields.push(format!("\"opacity\":{op}"));
        }
        if let Some(x) = self.x {
            fields.push(format!("\"x\":{x}"));
        }
        if let Some(y) = self.y {
            fields.push(format!("\"y\":{y}"));
        }
        if let Some(s) = self.scale {
            fields.push(format!("\"scale\":{s}"));
        }
        if let Some(r) = self.rotate {
            fields.push(format!("\"rotate\":{r}"));
        }
        if let Some(b) = self.blur {
            fields.push(format!("\"blur\":{b}"));
        }
        format!("{{{}}}", fields.join(","))
    }
}

/// Animation timing, easing, and spring physics configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub is_spring: bool,
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
    pub duration_ms: u32,
    pub delay_ms: u32,
    pub ease: String,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            is_spring: true,
            stiffness: 260.0,
            damping: 20.0,
            mass: 1.0,
            duration_ms: 400,
            delay_ms: 0,
            ease: "cubic-bezier(0.16, 1, 0.3, 1)".to_string(),
        }
    }
}

impl Transition {
    /// Create a spring-based physics transition.
    pub fn spring() -> Self {
        Self {
            is_spring: true,
            ..Default::default()
        }
    }

    /// Create a tween-based duration transition.
    pub fn tween() -> Self {
        Self {
            is_spring: false,
            ..Default::default()
        }
    }

    /// Set spring stiffness (higher = faster, snappier snap; default 260.0).
    pub fn stiffness(mut self, stiffness: f64) -> Self {
        self.stiffness = stiffness;
        self
    }

    /// Set spring damping (higher = less bounce/oscillation; default 20.0).
    pub fn damping(mut self, damping: f64) -> Self {
        self.damping = damping;
        self
    }

    /// Set spring mass (default 1.0).
    pub fn mass(mut self, mass: f64) -> Self {
        self.mass = mass;
        self
    }

    /// Set animation duration in milliseconds (for tween transitions).
    pub fn duration_ms(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set animation start delay in milliseconds.
    pub fn delay_ms(mut self, ms: u32) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Set cubic-bezier easing string.
    pub fn ease(mut self, ease: impl Into<String>) -> Self {
        self.ease = ease.into();
        self
    }

    /// Convert transition to JSON string.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"is_spring\":{},\"stiffness\":{},\"damping\":{},\"mass\":{},\"duration_ms\":{},\"delay_ms\":{},\"ease\":\"{}\"}}",
            self.is_spring, self.stiffness, self.damping, self.mass, self.duration_ms, self.delay_ms, self.ease
        )
    }
}

/// A declarative animated motion wrapper component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Motion {
    tag: String,
    initial: Option<MotionState>,
    animate: Option<MotionState>,
    while_hover: Option<MotionState>,
    while_tap: Option<MotionState>,
    transition: Option<Transition>,
    in_view: bool,
    once: bool,
    class: String,
    children: Vec<Node>,
}

impl Motion {
    /// Create a new motion container wrapping a `"div"` by default.
    pub fn new() -> Self {
        Self {
            tag: "div".to_string(),
            in_view: false,
            once: true,
            ..Default::default()
        }
    }

    /// Set the HTML tag name (e.g. `"div"`, `"button"`, `"section"`, `"span"`).
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        let t = tag.into();
        if !t.is_empty()
            && t.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            self.tag = t;
        }
        self
    }

    /// Set the initial visual state before animation begins.
    pub fn initial(mut self, state: MotionState) -> Self {
        self.initial = Some(state);
        self
    }

    /// Set the target animated visual state.
    pub fn animate(mut self, state: MotionState) -> Self {
        self.animate = Some(state);
        self
    }

    /// Set interactive hover state.
    pub fn while_hover(mut self, state: MotionState) -> Self {
        self.while_hover = Some(state);
        self
    }

    /// Set interactive active / tap state.
    pub fn while_tap(mut self, state: MotionState) -> Self {
        self.while_tap = Some(state);
        self
    }

    /// Set animation transition settings.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Trigger the animation only when the element enters the viewport (via IntersectionObserver).
    pub fn in_view(mut self, in_view: bool) -> Self {
        self.in_view = in_view;
        self
    }

    /// If `in_view` is true, whether to run the animation only once.
    pub fn once(mut self, once: bool) -> Self {
        self.once = once;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Add a child node.
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple child nodes.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "will-change-transform";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut element = Element::new(self.tag)
            .attr("class", full_class)
            .attr("data-motion", "true");

        if let Some(ref init) = self.initial {
            let style = init.to_style();
            if !style.is_empty() {
                element = element.attr("style", style);
            }
            element = element.attr("data-motion-initial", init.to_json());
        }

        if let Some(ref anim) = self.animate {
            element = element.attr("data-motion-animate", anim.to_json());
        }

        if let Some(ref hover) = self.while_hover {
            element = element.attr("data-motion-hover", hover.to_json());
        }

        if let Some(ref tap) = self.while_tap {
            element = element.attr("data-motion-tap", tap.to_json());
        }

        if let Some(ref trans) = self.transition {
            element = element.attr("data-motion-transition", trans.to_json());
        }

        if self.in_view {
            element = element.attr("data-motion-inview", "true");
            element = element.attr("data-motion-once", if self.once { "true" } else { "false" });
        }

        for child in self.children {
            element = element.child(child);
        }

        Node::Element(element)
    }
}

impl IntoNode for Motion {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Motion> for Node {
    fn from(m: Motion) -> Self {
        m.render()
    }
}

impl Component for Motion {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`Motion`].
pub fn motion() -> Motion {
    Motion::new()
}

/// A container that orchestrates animations of children with staggered delays.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionGroup {
    stagger_ms: u32,
    in_view: bool,
    class: String,
    children: Vec<Node>,
}

impl MotionGroup {
    /// Create a new motion group.
    pub fn new() -> Self {
        Self {
            stagger_ms: 60,
            in_view: true,
            class: String::new(),
            children: Vec::new(),
        }
    }

    /// Set the delay increment between each child animation in milliseconds (default 60ms).
    pub fn stagger_ms(mut self, ms: u32) -> Self {
        self.stagger_ms = ms;
        self
    }

    /// Trigger animations when group enters viewport.
    pub fn in_view(mut self, in_view: bool) -> Self {
        self.in_view = in_view;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Add a child.
    pub fn child(mut self, child: impl IntoNode) -> Self {
        self.children.push(child.into_node());
        self
    }

    /// Add multiple children.
    pub fn children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }

    /// Render the motion group into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let mut el = Element::new("div")
            .attr("class", &self.class)
            .attr("data-motion-group", "true")
            .attr("data-stagger", self.stagger_ms.to_string());

        if self.in_view {
            el = el.attr("data-motion-inview", "true");
        }

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

impl IntoNode for MotionGroup {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<MotionGroup> for Node {
    fn from(g: MotionGroup) -> Self {
        g.render()
    }
}

impl Component for MotionGroup {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`MotionGroup`].
pub fn motion_group() -> MotionGroup {
    MotionGroup::new()
}

/// A scroll progress bar that tracks page or container scroll percentage from 0% to 100%.
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollProgressBar {
    class: String,
}

impl Default for ScrollProgressBar {
    fn default() -> Self {
        Self {
            class: "fixed top-0 left-0 right-0 h-1 bg-primary-600 z-50 origin-left".to_string(),
        }
    }
}

impl ScrollProgressBar {
    /// Create a new scroll progress bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Custom CSS classes for position and gradient color.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the scroll progress bar.
    pub fn render(self) -> Node {
        let el = Element::new("div")
            .attr("class", &self.class)
            .attr("style", "transform: scaleX(0); will-change: transform;")
            .attr("data-scroll-progress", "true")
            .attr("aria-hidden", "true");
        Node::Element(el)
    }
}

impl IntoNode for ScrollProgressBar {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<ScrollProgressBar> for Node {
    fn from(s: ScrollProgressBar) -> Self {
        s.render()
    }
}

impl Component for ScrollProgressBar {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`ScrollProgressBar`].
pub fn scroll_progress_bar() -> ScrollProgressBar {
    ScrollProgressBar::new()
}

/// Returns the zero-dependency client script powering Yav Motion's spring physics,
/// scroll reveal, gesture animations, and scroll progress tracking.
pub fn motion_script() -> Node {
    let script = r#"<script>
(function() {
  function initYavMotion() {
    // 1. Scroll Progress Bars
    const progressBars = document.querySelectorAll('[data-scroll-progress]');
    if (progressBars.length > 0) {
      function updateProgress() {
        const scrollTop = window.scrollY || document.documentElement.scrollTop;
        const docHeight = document.documentElement.scrollHeight - window.innerHeight;
        const progress = docHeight > 0 ? Math.min(Math.max(scrollTop / docHeight, 0), 1) : 0;
        progressBars.forEach(bar => {
          bar.style.transform = `scaleX(${progress})`;
        });
      }
      window.addEventListener('scroll', updateProgress, { passive: true });
      updateProgress();
    }

    // 2. Motion Elements Animation
    function animateTo(el, target, transition) {
      if (!target) return;
      const trans = transition || {};
      const duration = trans.duration_ms || 400;
      const ease = trans.is_spring
        ? 'cubic-bezier(0.175, 0.885, 0.32, 1.275)'
        : (trans.ease || 'cubic-bezier(0.16, 1, 0.3, 1)');

      const transforms = [];
      if (target.x !== undefined || target.y !== undefined) {
        transforms.push(`translate3d(${target.x || 0}px, ${target.y || 0}px, 0)`);
      }
      if (target.scale !== undefined) {
        transforms.push(`scale(${target.scale})`);
      }
      if (target.rotate !== undefined) {
        transforms.push(`rotate(${target.rotate}deg)`);
      }

      el.style.transition = `all ${duration}ms ${ease}`;
      if (target.opacity !== undefined) {
        el.style.opacity = target.opacity;
      }
      if (transforms.length > 0) {
        el.style.transform = transforms.join(' ');
      }
      if (target.blur !== undefined) {
        el.style.filter = `blur(${target.blur}px)`;
      }
    }

    // 3. Staggered Groups & InView Observers
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (!entry.isIntersecting) return;
        const target = entry.target;

        if (target.hasAttribute('data-motion-group')) {
          const stagger = parseInt(target.getAttribute('data-stagger') || '60', 10);
          const items = target.querySelectorAll('[data-motion]');
          items.forEach((item, idx) => {
            const anim = JSON.parse(item.getAttribute('data-motion-animate') || '{}');
            const trans = JSON.parse(item.getAttribute('data-motion-transition') || '{}');
            setTimeout(() => {
              animateTo(item, anim, trans);
            }, idx * stagger);
          });
          observer.unobserve(target);
        } else if (target.hasAttribute('data-motion')) {
          const anim = JSON.parse(target.getAttribute('data-motion-animate') || '{}');
          const trans = JSON.parse(target.getAttribute('data-motion-transition') || '{}');
          const delay = trans.delay_ms || 0;
          setTimeout(() => {
            animateTo(target, anim, trans);
          }, delay);

          const once = target.getAttribute('data-motion-once') !== 'false';
          if (once) observer.unobserve(target);
        }
      });
    }, { threshold: 0.15 });

    document.querySelectorAll('[data-motion-inview="true"]').forEach(el => {
      observer.observe(el);
    });

    // Immediate animations (not requiring inView)
    document.querySelectorAll('[data-motion]:not([data-motion-inview="true"])').forEach(el => {
      const anim = JSON.parse(el.getAttribute('data-motion-animate') || '{}');
      const trans = JSON.parse(el.getAttribute('data-motion-transition') || '{}');
      const delay = trans.delay_ms || 0;
      setTimeout(() => {
        animateTo(el, anim, trans);
      }, delay);
    });

    // 4. Gestures: whileHover & whileTap
    document.querySelectorAll('[data-motion-hover], [data-motion-tap]').forEach(el => {
      const hoverState = el.getAttribute('data-motion-hover') ? JSON.parse(el.getAttribute('data-motion-hover')) : null;
      const tapState = el.getAttribute('data-motion-tap') ? JSON.parse(el.getAttribute('data-motion-tap')) : null;
      const animState = el.getAttribute('data-motion-animate') ? JSON.parse(el.getAttribute('data-motion-animate')) : null;
      const trans = JSON.parse(el.getAttribute('data-motion-transition') || '{}');

      if (hoverState) {
        el.addEventListener('mouseenter', () => animateTo(el, hoverState, trans));
        el.addEventListener('mouseleave', () => animateTo(el, animState || { x: 0, y: 0, scale: 1, opacity: 1 }, trans));
      }

      if (tapState) {
        el.addEventListener('mousedown', () => animateTo(el, tapState, { duration_ms: 100 }));
        el.addEventListener('mouseup', () => animateTo(el, hoverState || animState || { scale: 1 }, trans));
      }
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initYavMotion);
  } else {
    initYavMotion();
  }
})();
</script>"#;
    Node::raw(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn motion_state_style_and_json() {
        let state = MotionState::new().opacity(0.0).y(20.0).scale(0.95);
        let style = state.to_style();
        assert!(style.contains("opacity: 0"));
        assert!(style.contains("translate3d(0px, 20px, 0)"));
        assert!(style.contains("scale(0.95)"));

        let json = state.to_json();
        assert!(json.contains("\"opacity\":0"));
        assert!(json.contains("\"y\":20"));
        assert!(json.contains("\"scale\":0.95"));
    }

    #[test]
    fn motion_renders_attributes_and_children() {
        let node = motion()
            .initial(MotionState::new().opacity(0.0).y(30.0))
            .animate(MotionState::new().opacity(1.0).y(0.0))
            .while_hover(MotionState::new().scale(1.05))
            .while_tap(MotionState::new().scale(0.98))
            .transition(Transition::spring().stiffness(300.0).damping(18.0))
            .in_view(true)
            .child(Node::text("Animated Content"))
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("data-motion=\"true\""));
        assert!(html.contains("data-motion-initial"));
        assert!(html.contains("data-motion-animate"));
        assert!(html.contains("data-motion-hover"));
        assert!(html.contains("data-motion-tap"));
        assert!(html.contains("data-motion-transition"));
        assert!(html.contains("data-motion-inview=\"true\""));
        assert!(html.contains("Animated Content"));
    }

    #[test]
    fn scroll_progress_bar_renders() {
        let bar = scroll_progress_bar().render();
        let html = render_to_string(&bar);
        assert!(html.contains("data-scroll-progress=\"true\""));
        assert!(html.contains("scaleX(0)"));
    }
}
