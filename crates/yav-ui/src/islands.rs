//! Native Web Component micro-islands for zero-overhead client-side interactivity.
//!
//! Provides progressive Custom Elements (< 1.5 KB) that run client interactivity
//! locally within the element without requiring React, Vue, or any heavy JS bundle.

use yav_core::{Element, Node};

/// Properties for `<AccordionItem />`.
#[derive(Debug, Clone, Default)]
pub struct AccordionItemProps {
    pub title: String,
    pub children: Vec<Node>,
    pub open: bool,
}

/// An accordion section component.
#[allow(non_snake_case)]
pub fn AccordionItem(props: AccordionItemProps) -> Node {
    let mut details = Element::new("details").attr(
        "class",
        "group border-b border-gray-200 dark:border-gray-800 py-3",
    );

    if props.open {
        details = details.attr("open", "true");
    }

    let summary = Element::new("summary")
        .attr("class", "flex cursor-pointer items-center justify-between font-medium text-gray-900 dark:text-gray-100 list-none")
        .child(Node::text(props.title))
        .child(Node::raw(r#"<svg class="h-5 w-5 transition-transform duration-200 group-open:rotate-180 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>"#));

    details = details.child(Node::Element(summary));

    let mut content = Element::new("div").attr(
        "class",
        "mt-2 text-sm text-gray-600 dark:text-gray-400 leading-relaxed",
    );

    for child in props.children {
        content = content.child(child);
    }

    details = details.child(Node::Element(content));
    Node::Element(details)
}

/// Properties for `<Accordion />`.
#[derive(Debug, Clone, Default)]
pub struct AccordionProps {
    pub children: Vec<Node>,
    pub class: Option<String>,
}

/// Functional RSX component: `<Accordion> <AccordionItem ... /> </Accordion>`.
#[allow(non_snake_case)]
pub fn Accordion(props: AccordionProps) -> Node {
    let base_class = "w-full divide-y divide-gray-200 dark:divide-gray-800 rounded-lg border border-gray-200 dark:border-gray-800 px-4";
    let custom = props.class.as_deref().unwrap_or("");
    let full_class = format!("{base_class} {custom}").trim().to_string();

    let mut el = Element::new("yav-accordion").attr("class", full_class);
    for child in props.children {
        el = el.child(child);
    }
    Node::Element(el)
}

/// Client hydration directives defining when island JavaScript executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientDirective {
    /// Hydrate immediately when the HTML page is loaded.
    #[default]
    Load,
    /// Hydrate when the island enters the viewport via IntersectionObserver.
    Visible,
    /// Hydrate when the browser main thread is idle via requestIdleCallback.
    Idle,
}

impl ClientDirective {
    /// Directive attribute name (e.g. `"client:load"`, `"client:visible"`, `"client:idle"`).
    pub fn attribute_name(&self) -> &'static str {
        match self {
            Self::Load => "client:load",
            Self::Visible => "client:visible",
            Self::Idle => "client:idle",
        }
    }

    /// Directive keyword value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Visible => "visible",
            Self::Idle => "idle",
        }
    }
}

/// Properties for an `<Island />` wrapper component.
#[derive(Debug, Clone, Default)]
pub struct IslandProps {
    /// HTML tag name for the container (defaults to `"div"`).
    pub tag: Option<String>,
    /// Hydration directive determining activation timing.
    pub directive: ClientDirective,
    /// Child nodes rendered inside the island.
    pub children: Vec<Node>,
    /// CSS class string applied to the island container.
    pub class: Option<String>,
}

/// A client-side micro-island boundary with explicit hydration scheduling.
///
/// # Examples
/// ```
/// use yav_ui::islands::{Island, IslandProps, ClientDirective};
/// use yav_core::Node;
///
/// let island = Island(IslandProps {
///     directive: ClientDirective::Visible,
///     children: vec![Node::text("Hydrated on scroll")],
///     ..Default::default()
/// });
/// ```
#[allow(non_snake_case)]
pub fn Island(props: IslandProps) -> Node {
    let tag = props.tag.unwrap_or_else(|| "div".to_string());
    let mut el = Element::new(tag)
        .attr(props.directive.attribute_name(), "true")
        .attr("data-yav-island", props.directive.as_str());

    if let Some(ref class) = props.class {
        el = el.attr("class", class);
    }

    for child in props.children {
        el = el.child(child);
    }

    Node::Element(el)
}

/// Properties for interactive `<Counter />` micro-island.
#[derive(Debug, Clone, Default)]
pub struct CounterProps {
    pub initial: Option<i32>,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub step: Option<i32>,
    pub class: Option<String>,
    /// Optional client hydration directive (e.g. `ClientDirective::Visible`).
    pub directive: Option<ClientDirective>,
}

/// Functional RSX component: `<Counter initial=0 min=0 max=100 />`.
#[allow(non_snake_case)]
pub fn Counter(props: CounterProps) -> Node {
    let initial = props.initial.unwrap_or(0);
    let min = props.min.unwrap_or(i32::MIN);
    let max = props.max.unwrap_or(i32::MAX);
    let step = props.step.unwrap_or(1);

    let mut el = Element::new("yav-counter")
        .attr("data-initial", initial.to_string())
        .attr("data-min", min.to_string())
        .attr("data-max", max.to_string())
        .attr("data-step", step.to_string())
        .attr(
            "class",
            format!(
                "inline-flex items-center gap-3 rounded-lg border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 p-1.5 shadow-sm {}",
                props.class.as_deref().unwrap_or("")
            )
            .trim(),
        );

    if let Some(directive) = props.directive {
        el = el
            .attr(directive.attribute_name(), "true")
            .attr("data-yav-island", directive.as_str());
    }

    // Decrement button
    el = el.child(Node::Element(
        Element::new("button")
            .attr("type", "button")
            .attr("data-action", "dec")
            .attr("class", "flex h-8 w-8 items-center justify-center rounded-md bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 font-bold transition-colors")
            .attr("aria-label", "Diminuer")
            .child(Node::text("-")),
    ));

    // Value display
    el = el.child(Node::Element(
        Element::new("span")
            .attr("data-value", "true")
            .attr(
                "class",
                "min-w-[2rem] text-center font-mono font-semibold text-gray-900 dark:text-gray-100",
            )
            .child(Node::text(initial.to_string())),
    ));

    // Increment button
    el = el.child(Node::Element(
        Element::new("button")
            .attr("type", "button")
            .attr("data-action", "inc")
            .attr("class", "flex h-8 w-8 items-center justify-center rounded-md bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 font-bold transition-colors")
            .attr("aria-label", "Augmenter")
            .child(Node::text("+")),
    ));

    Node::Element(el)
}

/// Injects the micro-island Custom Elements registration and hydration directives script (<1.8 KB).
pub fn web_components_script() -> Node {
    let script = r#"(function(){
if (customElements.get('yav-counter')) return;

class YavCounter extends HTMLElement {
  connectedCallback() {
    const min = parseInt(this.getAttribute('data-min') || '-999999', 10);
    const max = parseInt(this.getAttribute('data-max') || '999999', 10);
    const step = parseInt(this.getAttribute('data-step') || '1', 10);
    let val = parseInt(this.getAttribute('data-initial') || '0', 10);

    const valEl = this.querySelector('[data-value]');
    const decBtn = this.querySelector('[data-action="dec"]');
    const incBtn = this.querySelector('[data-action="inc"]');

    const update = () => {
      if (valEl) valEl.textContent = val;
      if (decBtn) decBtn.disabled = val <= min;
      if (incBtn) incBtn.disabled = val >= max;
      this.dispatchEvent(new CustomEvent('change', { detail: { value: val } }));
    };

    if (decBtn) decBtn.addEventListener('click', () => { if (val - step >= min) { val -= step; update(); } });
    if (incBtn) incBtn.addEventListener('click', () => { if (val + step <= max) { val += step; update(); } });
    update();
  }
}
customElements.define('yav-counter', YavCounter);

// Client hydration scheduler for islands: client:load, client:visible, client:idle
function initHydrationDirectives() {
  const hydrate = (el) => {
    if (el.__yav_hydrated) return;
    el.__yav_hydrated = true;
    el.setAttribute('data-hydrated', 'true');
    el.dispatchEvent(new CustomEvent('yav:island-hydrated', { bubbles: true, detail: { element: el } }));
  };

  // client:visible with IntersectionObserver
  if ('IntersectionObserver' in window) {
    const observer = new IntersectionObserver((entries, obs) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          obs.unobserve(entry.target);
          hydrate(entry.target);
        }
      });
    }, { rootMargin: '50px' });

    document.querySelectorAll('[client\\:visible], [data-yav-island="visible"]').forEach(el => observer.observe(el));
  } else {
    document.querySelectorAll('[client\\:visible], [data-yav-island="visible"]').forEach(hydrate);
  }

  // client:idle with requestIdleCallback
  const scheduleIdle = window.requestIdleCallback || ((cb) => setTimeout(cb, 100));
  document.querySelectorAll('[client\\:idle], [data-yav-island="idle"]').forEach(el => {
    scheduleIdle(() => hydrate(el));
  });

  // client:load immediate
  document.querySelectorAll('[client\\:load], [data-yav-island="load"]').forEach(hydrate);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initHydrationDirectives);
} else {
  initHydrationDirectives();
}
})();"#;

    let mut el = Element::new("script");
    el = el.child(Node::raw(script));
    Node::Element(el)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_counter_rendering() {
        let counter = Counter(CounterProps {
            initial: Some(5),
            min: Some(0),
            max: Some(10),
            ..Default::default()
        });

        let html = render_to_string(&counter);
        assert!(html.contains("yav-counter"));
        assert!(html.contains(r#"data-initial="5""#));
        assert!(html.contains("5"));
    }

    #[test]
    fn test_accordion_rendering() {
        let acc = Accordion(AccordionProps {
            children: vec![AccordionItem(AccordionItemProps {
                title: "Question 1".into(),
                children: vec![Node::text("Réponse 1")],
                open: true,
            })],
            class: None,
        });

        let html = render_to_string(&acc);
        assert!(html.contains("yav-accordion"));
        assert!(html.contains("Question 1"));
        assert!(html.contains("Réponse 1"));
        assert!(html.contains(r#"open="true""#));
    }

    #[test]
    fn test_island_rendering_with_directives() {
        let island_load = Island(IslandProps {
            directive: ClientDirective::Load,
            children: vec![Node::text("Load Island")],
            class: Some("island-load".into()),
            ..Default::default()
        });
        let html_load = render_to_string(&island_load);
        assert!(html_load.contains(r#"client:load="true""#));
        assert!(html_load.contains(r#"data-yav-island="load""#));
        assert!(html_load.contains("Load Island"));

        let island_visible = Island(IslandProps {
            directive: ClientDirective::Visible,
            children: vec![Node::text("Visible Island")],
            ..Default::default()
        });
        let html_visible = render_to_string(&island_visible);
        assert!(html_visible.contains(r#"client:visible="true""#));
        assert!(html_visible.contains(r#"data-yav-island="visible""#));

        let island_idle = Island(IslandProps {
            directive: ClientDirective::Idle,
            children: vec![Node::text("Idle Island")],
            ..Default::default()
        });
        let html_idle = render_to_string(&island_idle);
        assert!(html_idle.contains(r#"client:idle="true""#));
        assert!(html_idle.contains(r#"data-yav-island="idle""#));
    }

    #[test]
    fn test_counter_with_directive() {
        let counter = Counter(CounterProps {
            initial: Some(3),
            directive: Some(ClientDirective::Visible),
            ..Default::default()
        });

        let html = render_to_string(&counter);
        assert!(html.contains("yav-counter"));
        assert!(html.contains(r#"client:visible="true""#));
        assert!(html.contains(r#"data-yav-island="visible""#));
    }

    #[test]
    fn test_web_components_script() {
        let script = web_components_script();
        let html = render_to_string(&script);
        assert!(html.contains("YavCounter"));
        assert!(html.contains("IntersectionObserver"));
        assert!(html.contains("requestIdleCallback"));
    }
}
