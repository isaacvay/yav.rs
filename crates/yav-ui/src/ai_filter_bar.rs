//! # YAV Natural Language AI Filter Bar (`<AiFilterBar />`)
//!
//! Smart search & filter input converting conversational English or French prompts
//! into structured SQL/DataGrid filter tokens with interactive filter chips and quick suggestions.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// A parsed structured filter condition extracted from natural language.
#[derive(Debug, Clone, PartialEq)]
pub struct AiFilterToken {
    /// Target column or attribute (e.g. "status", "revenue", "country").
    pub field: String,
    /// Comparison operator (e.g. "=", ">=", "contains", "in").
    pub operator: String,
    /// Value or threshold (e.g. "active", "500", "France").
    pub value: String,
    /// Model extraction confidence score (`0.0..=1.0`).
    pub confidence: f32,
}

impl AiFilterToken {
    /// Create a new filter token.
    #[must_use]
    pub fn new(
        field: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            operator: operator.into(),
            value: value.into(),
            confidence: 1.0,
        }
    }

    /// Set confidence score.
    #[must_use]
    pub fn confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Natural Language AI Filter Bar component.
#[derive(Debug, Clone)]
pub struct AiFilterBar {
    pub id: String,
    pub placeholder: String,
    pub prompt_value: String,
    pub endpoint: String,
    pub target_grid_id: Option<String>,
    pub active_filters: Vec<AiFilterToken>,
    pub suggestions: Vec<String>,
    pub loading: bool,
    pub class: String,
}

impl Default for AiFilterBar {
    fn default() -> Self {
        Self {
            id: "yav-ai-filter-bar".to_string(),
            placeholder:
                "Filter with AI: 'Show active enterprise users from France created this month'..."
                    .to_string(),
            prompt_value: String::new(),
            endpoint: "/api/ai/filter-extract".to_string(),
            target_grid_id: None,
            active_filters: Vec::new(),
            suggestions: vec![
                "Status is active".to_string(),
                "Revenue > $1,000".to_string(),
                "Created this week".to_string(),
            ],
            loading: false,
            class: String::new(),
        }
    }
}

impl AiFilterBar {
    /// Create a new AI filter bar builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set container DOM id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set input placeholder text.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set current prompt text.
    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt_value = prompt.into();
        self
    }

    /// Set backend AI extraction endpoint URL.
    #[must_use]
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Bind this filter bar to a target `<DataGrid />` or `<Table />` DOM id.
    #[must_use]
    pub fn target_grid(mut self, target_id: impl Into<String>) -> Self {
        self.target_grid_id = Some(target_id.into());
        self
    }

    /// Add an active parsed filter token.
    #[must_use]
    pub fn filter(mut self, token: AiFilterToken) -> Self {
        self.active_filters.push(token);
        self
    }

    /// Add quick prompt suggestions.
    #[must_use]
    pub fn suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// Toggle loading state indicator.
    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set extra container classes.
    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for AiFilterBar {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-ai-filter-bar flex flex-col space-y-2.5 w-full rounded-2xl border border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 p-3 shadow-md transition-all duration-200",
            &self.class,
        ]);

        // Sparkle AI Icon
        let sparkles_svg = "<svg class=\"h-5 w-5 text-indigo-500 shrink-0 animate-pulse\" fill=\"none\" viewBox=\"0 0 24 24\" stroke=\"currentColor\" stroke-width=\"2\">\
            <path stroke-linecap=\"round\" stroke-linejoin=\"round\" d=\"M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z\"/>\
        </svg>";

        // Target grid attribute for htmx or client dispatch
        let hx_target = if let Some(ref gid) = self.target_grid_id {
            format!("hx-target=\"#{}\"", escape_html(gid))
        } else {
            String::new()
        };

        // Active filter pills
        let mut tokens_markup = String::new();
        if !self.active_filters.is_empty() {
            tokens_markup.push_str("<div class=\"flex flex-wrap items-center gap-1.5 pt-1\">");
            tokens_markup.push_str("<span class=\"text-[11px] font-semibold text-gray-400 uppercase tracking-wider mr-1\">Active filters:</span>");
            for token in &self.active_filters {
                tokens_markup.push_str(&format!(
                    "<span class=\"inline-flex items-center space-x-1 rounded-full bg-indigo-50 dark:bg-indigo-950/70 border border-indigo-200 dark:border-indigo-800/80 px-2.5 py-0.5 text-xs font-medium text-indigo-700 dark:text-indigo-300 shadow-sm\">\
                        <span class=\"font-bold text-indigo-900 dark:text-indigo-100\">{}</span>\
                        <span class=\"text-indigo-400\">{}</span>\
                        <span>\"{}\"</span>\
                        <button type=\"button\" class=\"ml-1 text-indigo-400 hover:text-indigo-600 dark:hover:text-indigo-200 focus:outline-none\" aria-label=\"Remove filter\">&times;</button>\
                    </span>",
                    escape_html(&token.field),
                    escape_html(&token.operator),
                    escape_html(&token.value)
                ));
            }
            tokens_markup.push_str("</div>");
        }

        // Suggestions pills
        let mut suggestions_markup = String::new();
        if !self.suggestions.is_empty() {
            suggestions_markup.push_str("<div class=\"flex flex-wrap items-center gap-1.5 text-xs text-gray-500 dark:text-gray-400 pt-0.5\">");
            suggestions_markup
                .push_str("<span class=\"text-[10px] text-gray-400 font-medium\">Try:</span>");
            for sugg in &self.suggestions {
                suggestions_markup.push_str(&format!(
                    "<button type=\"button\" class=\"rounded-md bg-gray-100 dark:bg-gray-800 hover:bg-gray-200 dark:hover:bg-gray-700 px-2 py-0.5 text-[11px] transition-colors focus:outline-none\">{}</button>",
                    escape_html(sugg)
                ));
            }
            suggestions_markup.push_str("</div>");
        }

        let input_form = format!(
            "<form action=\"{endpoint}\" method=\"POST\" hx-post=\"{endpoint}\" {hx_target} hx-swap=\"outerHTML\" class=\"flex items-center space-x-2.5 w-full\">\
                {sparkles}\
                <input type=\"text\" name=\"nl_query\" value=\"{val}\" placeholder=\"{placeholder}\" class=\"w-full bg-transparent text-sm text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none\" />\
                <button type=\"submit\" class=\"shrink-0 inline-flex items-center justify-center rounded-xl bg-indigo-600 px-3.5 py-1.5 text-xs font-semibold text-white shadow hover:bg-indigo-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500/50\">\
                    Extract & Filter\
                </button>\
            </form>",
            endpoint = yav_core::escape_url(&self.endpoint).unwrap_or_else(|| "/".to_string()),
            hx_target = hx_target,
            sparkles = sparkles_svg,
            val = escape_html(&self.prompt_value),
            placeholder = escape_html(&self.placeholder)
        );

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(Node::Raw(input_form))
                .child(Node::Raw(tokens_markup))
                .child(Node::Raw(suggestions_markup)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_ai_filter_bar_rendering() {
        let bar = AiFilterBar::new()
            .id("users-filter")
            .prompt("Show active users in France")
            .target_grid("users-grid")
            .filter(AiFilterToken::new("status", "=", "active"))
            .filter(AiFilterToken::new("country", "=", "France"));

        let node = bar.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"users-filter\""));
        assert!(html.contains("status"));
        assert!(html.contains("active"));
        assert!(html.contains("France"));
        assert!(html.contains("hx-target=\"#users-grid\""));
    }
}
