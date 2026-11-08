//! Interactive carousel and media gallery component.
//!
//! Provides a responsive carousel slider (`Carousel`, `CarouselSlide`) with dot indicators,
//! navigation arrows, optional autoplay, and full keyboard/touch accessibility.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// A single slide within a [`Carousel`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CarouselSlide {
    pub image_url: String,
    pub alt: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub link: Option<String>,
    pub content: Option<Node>,
}

impl CarouselSlide {
    /// Create a new slide with an image URL.
    pub fn new(image_url: impl Into<String>) -> Self {
        Self {
            image_url: image_url.into(),
            ..Default::default()
        }
    }

    /// Set the alternative text for the image.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = alt.into();
        self
    }

    /// Set an optional slide title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set an optional slide subtitle / description.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set a destination link when clicking the slide.
    pub fn link(mut self, link: impl Into<String>) -> Self {
        self.link = Some(link.into());
        self
    }

    /// Set custom overlay content.
    pub fn content(mut self, content: impl IntoNode) -> Self {
        self.content = Some(content.into_node());
        self
    }
}

/// An interactive image and content carousel.
#[derive(Debug, Clone, PartialEq)]
pub struct Carousel {
    pub id: String,
    pub slides: Vec<CarouselSlide>,
    pub autoplay: bool,
    pub interval_ms: u32,
    pub show_arrows: bool,
    pub show_dots: bool,
    pub aspect_ratio: String,
    pub class: String,
}

impl Default for Carousel {
    fn default() -> Self {
        Self {
            id: "carousel".to_string(),
            slides: Vec::new(),
            autoplay: false,
            interval_ms: 5000,
            show_arrows: true,
            show_dots: true,
            aspect_ratio: "aspect-[16/9]".to_string(),
            class: String::new(),
        }
    }
}

impl Carousel {
    /// Create a new carousel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the carousel element ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Add a slide to the carousel.
    pub fn slide(mut self, slide: CarouselSlide) -> Self {
        self.slides.push(slide);
        self
    }

    /// Add multiple slides.
    pub fn slides(mut self, slides: impl IntoIterator<Item = CarouselSlide>) -> Self {
        self.slides.extend(slides);
        self
    }

    /// Enable or disable automatic slide cycling.
    pub fn autoplay(mut self, autoplay: bool) -> Self {
        self.autoplay = autoplay;
        self
    }

    /// Set the autoplay cycle interval in milliseconds.
    pub fn interval_ms(mut self, interval_ms: u32) -> Self {
        self.interval_ms = interval_ms;
        self
    }

    /// Show or hide next/previous navigation arrow buttons.
    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.show_arrows = show_arrows;
        self
    }

    /// Show or hide slide indicator dots.
    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.show_dots = show_dots;
        self
    }

    /// Set container aspect ratio or height class (e.g. `"aspect-[16/9]"` or `"h-96"`).
    pub fn aspect_ratio(mut self, aspect_ratio: impl Into<String>) -> Self {
        self.aspect_ratio = aspect_ratio.into();
        self
    }

    /// Add custom classes to the carousel wrapper.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the carousel into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "relative w-full overflow-hidden rounded-xl bg-gray-900 shadow-lg group";
        let full_class = merge_classes(&[base_class, &self.aspect_ratio, &self.class]);

        let mut root = Element::new("div")
            .attr("id", &self.id)
            .attr("class", full_class)
            .attr("role", "region")
            .attr("aria-roledescription", "carousel")
            .attr("data-carousel", "true")
            .attr(
                "data-autoplay",
                if self.autoplay { "true" } else { "false" },
            )
            .attr("data-interval", self.interval_ms.to_string());

        // Track container
        let mut track = Element::new("div")
            .attr("class", "relative h-full w-full overflow-hidden")
            .attr("data-carousel-track", "true");

        let total = self.slides.len();
        for (i, slide) in self.slides.into_iter().enumerate() {
            let active = i == 0;
            let slide_visibility = if active {
                "opacity-100 z-10"
            } else {
                "opacity-0 z-0 pointer-events-none"
            };
            let slide_class = format!("absolute inset-0 h-full w-full transition-opacity duration-700 ease-in-out {slide_visibility}");

            let mut slide_el = Element::new("div")
                .attr("class", slide_class)
                .attr("role", "group")
                .attr("aria-roledescription", "slide")
                .attr("aria-label", format!("{} of {}", i + 1, total))
                .attr("data-carousel-slide", i.to_string());

            // Image
            let img = Element::new("img")
                .attr("src", slide.image_url)
                .attr("alt", slide.alt)
                .attr("class", "h-full w-full object-cover object-center");
            slide_el = slide_el.child(Node::Element(img));

            // Overlay gradient & text content if title / subtitle exists
            if slide.title.is_some() || slide.subtitle.is_some() || slide.content.is_some() {
                let mut overlay = Element::new("div")
                    .attr("class", "absolute inset-0 bg-gradient-to-t from-black/80 via-black/30 to-transparent flex flex-col justify-end p-6 md:p-8 text-white");

                if let Some(title) = slide.title {
                    let t_el = Element::new("h3")
                        .attr("class", "text-xl md:text-3xl font-bold tracking-tight mb-2")
                        .child(Node::text(title));
                    overlay = overlay.child(Node::Element(t_el));
                }

                if let Some(subtitle) = slide.subtitle {
                    let s_el = Element::new("p")
                        .attr(
                            "class",
                            "text-sm md:text-base text-gray-200 line-clamp-2 mb-2",
                        )
                        .child(Node::text(subtitle));
                    overlay = overlay.child(Node::Element(s_el));
                }

                if let Some(content) = slide.content {
                    overlay = overlay.child(content);
                }

                if let Some(link) = slide.link {
                    let a_wrapper = Element::new("a")
                        .attr("href", link)
                        .attr("class", "absolute inset-0 z-20")
                        .attr("aria-label", "View slide link");
                    slide_el = slide_el.child(Node::Element(a_wrapper));
                }

                slide_el = slide_el.child(Node::Element(overlay));
            }

            track = track.child(Node::Element(slide_el));
        }

        root = root.child(Node::Element(track));

        // Navigation arrows
        if self.show_arrows && total > 1 {
            let prev_btn = Element::new("button")
                .attr("type", "button")
                .attr("class", "absolute left-4 top-1/2 -translate-y-1/2 z-20 flex h-10 w-10 items-center justify-center rounded-full bg-black/40 text-white backdrop-blur-sm transition-all hover:bg-black/70 focus:outline-none focus:ring-2 focus:ring-white")
                .attr("aria-label", "Previous slide")
                .attr("data-carousel-prev", "true")
                .child(Node::raw(r#"<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" /></svg>"#));

            let next_btn = Element::new("button")
                .attr("type", "button")
                .attr("class", "absolute right-4 top-1/2 -translate-y-1/2 z-20 flex h-10 w-10 items-center justify-center rounded-full bg-black/40 text-white backdrop-blur-sm transition-all hover:bg-black/70 focus:outline-none focus:ring-2 focus:ring-white")
                .attr("aria-label", "Next slide")
                .attr("data-carousel-next", "true")
                .child(Node::raw(r#"<svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" /></svg>"#));

            root = root.child(Node::Element(prev_btn));
            root = root.child(Node::Element(next_btn));
        }

        // Dot indicators
        if self.show_dots && total > 1 {
            let mut dots_container = Element::new("div")
                .attr(
                    "class",
                    "absolute bottom-4 left-1/2 -translate-x-1/2 z-20 flex space-x-2",
                )
                .attr("role", "tablist");

            for i in 0..total {
                let dot_active = if i == 0 {
                    "bg-white w-6"
                } else {
                    "bg-white/50 w-2.5 hover:bg-white/80"
                };
                let dot_class =
                    format!("h-2.5 rounded-full transition-all duration-300 {dot_active}");

                let dot = Element::new("button")
                    .attr("type", "button")
                    .attr("class", dot_class)
                    .attr("aria-label", format!("Go to slide {}", i + 1))
                    .attr("data-carousel-dot", i.to_string());

                dots_container = dots_container.child(Node::Element(dot));
            }

            root = root.child(Node::Element(dots_container));
        }

        Node::Element(root)
    }
}

impl IntoNode for Carousel {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Carousel> for Node {
    fn from(c: Carousel) -> Self {
        c.render()
    }
}

impl Component for Carousel {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to create a new [`Carousel`].
pub fn carousel() -> Carousel {
    Carousel::new()
}

/// Properties for constructing a [`Carousel`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarouselProps {
    pub id: Option<String>,
    pub slides: Vec<CarouselSlide>,
    pub autoplay: bool,
    pub interval_ms: Option<u32>,
    pub show_arrows: Option<bool>,
    pub show_dots: Option<bool>,
    pub aspect_ratio: Option<String>,
    pub class: String,
}

/// Functional component adapter for [`Carousel`], enabling `<Carousel slides={...} autoplay=true />` in `rsx!`.
#[allow(non_snake_case)]
pub fn Carousel(props: CarouselProps) -> Node {
    let mut c = Carousel::new()
        .slides(props.slides)
        .autoplay(props.autoplay);
    if let Some(id) = props.id {
        c = c.id(id);
    }
    if let Some(ms) = props.interval_ms {
        c = c.interval_ms(ms);
    }
    if let Some(arrows) = props.show_arrows {
        c = c.show_arrows(arrows);
    }
    if let Some(dots) = props.show_dots {
        c = c.show_dots(dots);
    }
    if let Some(ar) = props.aspect_ratio {
        c = c.aspect_ratio(ar);
    }
    if !props.class.is_empty() {
        c = c.class(props.class);
    }
    c.render()
}

/// Helper script that drives carousel navigation, indicators, and autoplay.
pub fn carousel_script() -> Node {
    carousel_script_with_nonce(None)
}

/// Helper script that drives carousel navigation, indicators, and autoplay, with an optional CSP nonce.
pub fn carousel_script_with_nonce(nonce: Option<&str>) -> Node {
    let nonce_attr = match nonce {
        Some(n) => format!(r#" nonce="{}""#, yav_core::escape_html(n)),
        None => String::new(),
    };
    let script = format!(
        r#"<script{nonce_attr}>
(function() {{
  function initCarousels() {{
    document.querySelectorAll('[data-carousel]').forEach(carousel => {{
      if (carousel.__carousel_init) return;
      carousel.__carousel_init = true;

      const slides = Array.from(carousel.querySelectorAll('[data-carousel-slide]'));
      const dots = Array.from(carousel.querySelectorAll('[data-carousel-dot]'));
      const prevBtn = carousel.querySelector('[data-carousel-prev]');
      const nextBtn = carousel.querySelector('[data-carousel-next]');
      const autoplay = carousel.dataset.autoplay === 'true';
      const interval = parseInt(carousel.dataset.interval || '5000', 10);

      let current = 0;
      let timer = null;

      function goTo(idx) {{
        if (slides.length <= 1) return;
        current = (idx + slides.length) % slides.length;

        slides.forEach((slide, i) => {{
          if (i === current) {{
            slide.classList.remove('opacity-0', 'pointer-events-none', 'z-0');
            slide.classList.add('opacity-100', 'z-10');
          }} else {{
            slide.classList.remove('opacity-100', 'z-10');
            slide.classList.add('opacity-0', 'pointer-events-none', 'z-0');
          }}
        }});

        dots.forEach((dot, i) => {{
          if (i === current) {{
            dot.classList.remove('bg-white/50', 'w-2.5');
            dot.classList.add('bg-white', 'w-6');
          }} else {{
            dot.classList.remove('bg-white', 'w-6');
            dot.classList.add('bg-white/50', 'w-2.5');
          }}
        }});
      }}

      function next() {{ goTo(current + 1); }}
      function prev() {{ goTo(current - 1); }}

      if (prevBtn) prevBtn.addEventListener('click', () => {{ prev(); restartAutoplay(); }});
      if (nextBtn) nextBtn.addEventListener('click', () => {{ next(); restartAutoplay(); }});

      dots.forEach((dot, i) => {{
        dot.addEventListener('click', () => {{ goTo(i); restartAutoplay(); }});
      }});

      function startAutoplay() {{
        if (autoplay && slides.length > 1) {{
          timer = setInterval(next, interval);
        }}
      }}

      function stopAutoplay() {{
        if (timer) clearInterval(timer);
      }}

      function restartAutoplay() {{
        stopAutoplay();
        startAutoplay();
      }}

      carousel.addEventListener('mouseenter', stopAutoplay);
      carousel.addEventListener('mouseleave', startAutoplay);
      startAutoplay();
    }});
  }}

  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', initCarousels);
  }} else {{
    initCarousels();
  }}
}})();
</script>"#,
        nonce_attr = nonce_attr
    );
    Node::raw(script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn carousel_renders_slides_arrows_and_dots() {
        let c = carousel()
            .id("hero-gallery")
            .autoplay(true)
            .interval_ms(4000)
            .slide(
                CarouselSlide::new("/images/car1.jpg")
                    .alt("Car 1")
                    .title("Ferrari Roma")
                    .subtitle("Unleash the elegance"),
            )
            .slide(
                CarouselSlide::new("/images/car2.jpg")
                    .alt("Car 2")
                    .title("Porsche 911 GT3"),
            )
            .render();

        let html = render_to_string(&c);
        assert!(html.contains("id=\"hero-gallery\""));
        assert!(html.contains("data-carousel=\"true\""));
        assert!(html.contains("data-autoplay=\"true\""));
        assert!(html.contains("data-interval=\"4000\""));
        assert!(html.contains("Ferrari Roma"));
        assert!(html.contains("Porsche 911 GT3"));
        assert!(html.contains("aria-label=\"Previous slide\""));
        assert!(html.contains("aria-label=\"Next slide\""));
        assert!(html.contains("data-carousel-dot=\"0\""));
        assert!(html.contains("data-carousel-dot=\"1\""));
    }

    #[test]
    fn test_carousel_script_nonce() {
        let script = carousel_script_with_nonce(Some("caro_nonce_123"));
        let html = render_to_string(&script);
        assert!(html.contains("nonce=\"caro_nonce_123\""));
    }
}
