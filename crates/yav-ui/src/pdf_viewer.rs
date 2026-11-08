//! # YAV Vector PDF Document Reader & Annotator (`<PdfViewer />`)
//!
//! Embedded PDF document reader with page navigation controls, zoom toolbar,
//! vector annotation highlights, and download/print actions.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// An annotation box or highlight placed on a PDF page.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfAnnotation {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
    pub color: String,
}

impl PdfAnnotation {
    #[must_use]
    pub fn highlight(page: u32, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            page,
            x,
            y,
            width,
            height,
            text: String::new(),
            color: "rgba(254, 240, 138, 0.4)".to_string(), // Yellow highlight
        }
    }

    #[must_use]
    pub fn note(page: u32, x: f64, y: f64, note_text: impl Into<String>) -> Self {
        Self {
            page,
            x,
            y,
            width: 140.0,
            height: 80.0,
            text: note_text.into(),
            color: "#fef08a".to_string(),
        }
    }
}

/// Interactive PDF document viewer component.
#[derive(Debug, Clone)]
pub struct PdfViewer {
    pub id: String,
    pub src_url: String,
    pub title: String,
    pub total_pages: u32,
    pub current_page: u32,
    pub zoom: f64,
    pub annotations: Vec<PdfAnnotation>,
    pub class: String,
}

impl Default for PdfViewer {
    fn default() -> Self {
        Self {
            id: "yav-pdf-viewer".to_string(),
            src_url: String::new(),
            title: "Document.pdf".to_string(),
            total_pages: 1,
            current_page: 1,
            zoom: 1.0,
            annotations: Vec::new(),
            class: String::new(),
        }
    }
}

impl PdfViewer {
    #[must_use]
    pub fn new(src_url: impl Into<String>) -> Self {
        Self {
            src_url: src_url.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn pages(mut self, total: u32, current: u32) -> Self {
        self.total_pages = total.max(1);
        self.current_page = current.clamp(1, self.total_pages);
        self
    }

    #[must_use]
    pub fn annotation(mut self, ann: PdfAnnotation) -> Self {
        self.annotations.push(ann);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for PdfViewer {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-pdf-viewer flex flex-col rounded-2xl border border-gray-200 dark:border-gray-800 bg-gray-100 dark:bg-gray-950 shadow-xl overflow-hidden select-none",
            &self.class,
        ]);

        let toolbar = format!(
            "<div class=\"flex flex-wrap items-center justify-between gap-3 px-4 py-2.5 bg-white dark:bg-gray-900 border-b border-gray-200 dark:border-gray-800 text-xs text-gray-700 dark:text-gray-300 font-medium\">\
                <div class=\"flex items-center space-x-3\">\
                    <span class=\"font-bold truncate max-w-[200px] text-gray-900 dark:text-gray-100\">{title}</span>\
                    <span class=\"text-gray-400\">|</span>\
                    <div class=\"flex items-center space-x-1\">\
                        <button type=\"button\" class=\"p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-30\" {prev_dis}>&larr;</button>\
                        <span>Page {cur} / {total}</span>\
                        <button type=\"button\" class=\"p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 disabled:opacity-30\" {next_dis}>&rarr;</button>\
                    </div>\
                </div>\
                <div class=\"flex items-center space-x-2\">\
                    <button type=\"button\" class=\"px-2 py-1 rounded border border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800 text-xs\">&minus; Zoom</button>\
                    <span class=\"font-mono text-xs\">{zoom:.0}%</span>\
                    <button type=\"button\" class=\"px-2 py-1 rounded border border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800 text-xs\">+ Zoom</button>\
                    <a href=\"{url}\" download class=\"ml-2 px-3 py-1 rounded-lg bg-indigo-600 text-white font-semibold hover:bg-indigo-700 transition-colors shadow-sm\">Download</a>\
                </div>\
            </div>",
            title = escape_html(&self.title),
            cur = self.current_page,
            total = self.total_pages,
            prev_dis = if self.current_page <= 1 { "disabled" } else { "" },
            next_dis = if self.current_page >= self.total_pages { "disabled" } else { "" },
            zoom = self.zoom * 100.0,
            url = escape_html(&self.src_url)
        );

        // Page vector representation with A4 aspect ratio (595 x 842 pt)
        let page_w = 595.0 * self.zoom;
        let page_h = 842.0 * self.zoom;

        let mut annotations_svg = String::new();
        for ann in &self.annotations {
            if ann.page == self.current_page {
                if ann.text.is_empty() {
                    // Transparent highlight rectangle
                    annotations_svg.push_str(&format!(
                        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"{color}\" rx=\"2\" />\n",
                        x = ann.x * self.zoom,
                        y = ann.y * self.zoom,
                        w = ann.width * self.zoom,
                        h = ann.height * self.zoom,
                        color = escape_html(&ann.color)
                    ));
                } else {
                    // Sticky note callout
                    annotations_svg.push_str(&format!(
                        "<g transform=\"translate({x:.1}, {y:.1})\">\
                            <rect width=\"{w:.1}\" height=\"{h:.1}\" rx=\"6\" fill=\"{color}\" filter=\"drop-shadow(0 2px 4px rgba(0,0,0,0.15))\" />\
                            <text x=\"8\" y=\"18\" font-size=\"10\" font-weight=\"bold\" fill=\"#713f12\">Note</text>\
                            <text x=\"8\" y=\"34\" font-size=\"9\" fill=\"#854d0e\">{text}</text>\
                        </g>\n",
                        x = ann.x * self.zoom,
                        y = ann.y * self.zoom,
                        w = ann.width * self.zoom,
                        h = ann.height * self.zoom,
                        color = escape_html(&ann.color),
                        text = escape_html(&ann.text)
                    ));
                }
            }
        }

        let page_content = format!(
            "<div class=\"flex justify-center p-6 overflow-auto max-h-[680px]\">\
                <div class=\"relative bg-white shadow-2xl border border-gray-300 dark:border-gray-700 rounded-sm\" style=\"width: {pw:.0}px; height: {ph:.0}px;\">\
                    <!-- Embedded Vector Page Shell / Annotations Layer -->\
                    <svg viewBox=\"0 0 {pw:.0} {ph:.0}\" class=\"absolute inset-0 w-full h-full pointer-events-none\">\
                        {annotations}\
                    </svg>\
                    <!-- Fallback / PDF iframe render layer -->\
                    <iframe src=\"{url}#page={page}\" class=\"w-full h-full border-none pointer-events-auto\" title=\"{title}\"></iframe>\
                </div>\
            </div>",
            pw = page_w,
            ph = page_h,
            annotations = annotations_svg,
            url = escape_html(&self.src_url),
            page = self.current_page,
            title = escape_html(&self.title)
        );

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(Node::Raw(toolbar))
                .child(Node::Raw(page_content)),
        )
    }
}

/// Helper function to create a new `PdfViewer`.
#[must_use]
pub fn pdf_viewer(src_url: impl Into<String>) -> PdfViewer {
    PdfViewer::new(src_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_pdf_viewer_rendering() {
        let viewer = PdfViewer::new("/docs/invoice.pdf")
            .id("inv-view")
            .title("Invoice #2026-0042")
            .pages(5, 2)
            .annotation(PdfAnnotation::highlight(2, 40.0, 100.0, 200.0, 20.0))
            .annotation(PdfAnnotation::note(2, 40.0, 140.0, "Approved by Audit"));

        let node = viewer.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"inv-view\""));
        assert!(html.contains("Invoice #2026-0042"));
        assert!(html.contains("Page 2 / 5"));
        assert!(html.contains("Approved by Audit"));
        assert!(html.contains("src=\"/docs/invoice.pdf#page=2\""));
    }
}
