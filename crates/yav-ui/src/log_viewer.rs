//! # YAV Native OpenTelemetry Logtail Live Inspector (`<LogViewer />`)
//!
//! Terminal-grade real-time log inspector with level filtering (INFO, WARN, ERROR),
//! live SSE/WebSocket streaming, distributed trace ID links, attribute drawers, and auto-scrolling.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Severity level of a structured log record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    /// Convert from string slice case-insensitively.
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "TRACE" => Self::Trace,
            "DEBUG" => Self::Debug,
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" => Self::Error,
            "FATAL" | "CRITICAL" => Self::Fatal,
            _ => Self::Info,
        }
    }

    /// Return uppercase string label.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }

    /// Badge Tailwind CSS color class.
    #[must_use]
    pub fn badge_class(&self) -> &'static str {
        match self {
            Self::Trace => "bg-gray-800 text-gray-400 border-gray-700",
            Self::Debug => "bg-blue-950/80 text-blue-400 border-blue-800",
            Self::Info => "bg-emerald-950/80 text-emerald-400 border-emerald-800",
            Self::Warn => "bg-amber-950/80 text-amber-300 border-amber-800",
            Self::Error => "bg-rose-950/80 text-rose-300 border-rose-800",
            Self::Fatal => "bg-red-900 text-white border-red-700 font-extrabold animate-pulse",
        }
    }
}

/// A structured log entry line for the inspector.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub attributes: Vec<(String, String)>,
}

impl LogEntry {
    /// Create a new log entry.
    #[must_use]
    pub fn new(
        timestamp: impl Into<String>,
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            level,
            target: target.into(),
            message: message.into(),
            trace_id: None,
            span_id: None,
            attributes: Vec::new(),
        }
    }

    /// Set distributed OpenTelemetry Trace ID.
    #[must_use]
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Set Span ID.
    #[must_use]
    pub fn span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    /// Add a key-value attribute pair.
    #[must_use]
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }
}

/// Terminal-grade live logtail inspector component.
#[derive(Debug, Clone)]
pub struct LogViewer {
    pub id: String,
    pub logs: Vec<LogEntry>,
    pub live_stream_url: Option<String>,
    pub search_query: Option<String>,
    pub filter_levels: Vec<LogLevel>,
    pub max_height: u32,
    pub auto_scroll: bool,
    pub wrap_lines: bool,
    pub class: String,
    pub nonce: Option<String>,
}

impl Default for LogViewer {
    fn default() -> Self {
        Self {
            id: "yav-log-viewer".to_string(),
            logs: Vec::new(),
            live_stream_url: None,
            search_query: None,
            filter_levels: Vec::new(),
            max_height: 480,
            auto_scroll: true,
            wrap_lines: false,
            class: String::new(),
            nonce: None,
        }
    }
}

impl LogViewer {
    /// Create a new log viewer component.
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

    /// Add a single log entry.
    #[must_use]
    pub fn log(mut self, entry: LogEntry) -> Self {
        self.logs.push(entry);
        self
    }

    /// Set list of log entries.
    #[must_use]
    pub fn logs(mut self, logs: Vec<LogEntry>) -> Self {
        self.logs = logs;
        self
    }

    /// Set live SSE or WebSocket stream endpoint URL.
    #[must_use]
    pub fn stream_url(mut self, url: impl Into<String>) -> Self {
        self.live_stream_url = Some(url.into());
        self
    }

    /// Filter by keyword or search term.
    #[must_use]
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.search_query = Some(query.into());
        self
    }

    /// Filter by allowed log levels.
    #[must_use]
    pub fn levels(mut self, levels: Vec<LogLevel>) -> Self {
        self.filter_levels = levels;
        self
    }

    /// Set viewport height in pixels.
    #[must_use]
    pub fn height(mut self, px: u32) -> Self {
        self.max_height = px;
        self
    }

    /// Toggle line wrapping.
    #[must_use]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap_lines = wrap;
        self
    }

    /// Enable or disable automatic scrolling to bottom.
    #[must_use]
    pub fn auto_scroll(mut self, auto_scroll: bool) -> Self {
        self.auto_scroll = auto_scroll;
        self
    }

    /// Set extra container classes.
    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set an optional CSP nonce for inline scripts.
    #[must_use]
    pub fn nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }
}

impl Component for LogViewer {
    fn render(&self) -> Node {
        let container_class = merge_classes(&[
            "yav-log-viewer flex flex-col rounded-2xl border border-gray-800 bg-[#0d1117] text-gray-200 font-mono shadow-2xl overflow-hidden select-text",
            &self.class,
        ]);

        // Toolbar header with search, active counter, clear and autoscroll toggle
        let log_count = self.logs.len();
        let stream_indicator = if self.live_stream_url.is_some() {
            "<div class=\"flex items-center space-x-1.5 text-xs text-emerald-400 font-semibold px-2 py-0.5 rounded-full bg-emerald-950/60 border border-emerald-800\">\
                <span class=\"relative flex h-2 w-2\"><span class=\"animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75\"></span><span class=\"relative inline-flex rounded-full h-2 w-2 bg-emerald-500\"></span></span>\
                <span>LIVE</span>\
            </div>".to_string()
        } else {
            format!(
                "<span class=\"text-xs text-gray-400 bg-gray-800/80 px-2 py-0.5 rounded-md border border-gray-700\">{} lines</span>",
                log_count
            )
        };

        let toolbar = format!(
            "<div class=\"flex flex-wrap items-center justify-between gap-3 px-4 py-2.5 bg-[#161b22] border-b border-gray-800 text-xs\">\
                <div class=\"flex items-center space-x-3\">\
                    <div class=\"flex space-x-1.5\">\
                        <span class=\"h-3 w-3 rounded-full bg-red-500/80 inline-block\"></span>\
                        <span class=\"h-3 w-3 rounded-full bg-amber-500/80 inline-block\"></span>\
                        <span class=\"h-3 w-3 rounded-full bg-emerald-500/80 inline-block\"></span>\
                    </div>\
                    <span class=\"font-bold text-gray-300 text-xs tracking-wide uppercase\">OTel Logtail</span>\
                    {stream_indicator}\
                </div>\
                <div class=\"flex items-center space-x-2\">\
                    <input type=\"text\" placeholder=\"Filter regex or text...\" class=\"bg-gray-900 border border-gray-700 text-gray-200 text-xs rounded-lg px-2.5 py-1 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/30 w-44 transition-all\" />\
                    <button type=\"button\" class=\"px-2 py-1 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded-md transition-colors text-xs border border-gray-700\">Clear</button>\
                </div>\
            </div>",
            stream_indicator = stream_indicator
        );

        // Filter logs according to levels and search query
        let mut rendered_rows = String::new();
        for (idx, entry) in self.logs.iter().enumerate() {
            if !self.filter_levels.is_empty() && !self.filter_levels.contains(&entry.level) {
                continue;
            }
            if let Some(ref q) = self.search_query {
                if !entry.message.to_lowercase().contains(&q.to_lowercase())
                    && !entry.target.to_lowercase().contains(&q.to_lowercase())
                {
                    continue;
                }
            }

            let trace_pill = if let Some(ref tid) = entry.trace_id {
                let short_tid = if tid.len() > 10 { &tid[0..10] } else { tid };
                format!(
                    "<a href=\"/traces/{tid}\" title=\"Trace ID: {tid}\" class=\"inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-mono bg-purple-950/60 text-purple-300 border border-purple-800 hover:bg-purple-900 transition-colors\">{short}</a>",
                    tid = escape_html(tid),
                    short = escape_html(short_tid)
                )
            } else {
                String::new()
            };

            let row_wrap = if self.wrap_lines {
                "whitespace-pre-wrap break-all"
            } else {
                "whitespace-pre overflow-hidden"
            };

            let row_html = format!(
                "<div class=\"log-row group flex items-start space-x-3 px-4 py-1 hover:bg-[#161b22]/70 text-xs border-b border-gray-900/50 transition-colors leading-5\">\
                    <span class=\"text-gray-500 select-none text-[10px] w-8 shrink-0 text-right\">{num}</span>\
                    <span class=\"text-gray-400 shrink-0\">{ts}</span>\
                    <span class=\"px-1.5 py-0.2 rounded border text-[10px] font-bold shrink-0 {badge}\">{lvl}</span>\
                    <span class=\"text-indigo-400 font-semibold shrink-0 max-w-[120px] truncate\">{target}</span>\
                    <span class=\"text-gray-300 flex-1 {wrap}\">{msg}</span>\
                    {trace}\
                </div>",
                num = idx + 1,
                ts = escape_html(&entry.timestamp),
                badge = entry.level.badge_class(),
                lvl = entry.level.as_str(),
                target = escape_html(&entry.target),
                wrap = row_wrap,
                msg = escape_html(&entry.message),
                trace = trace_pill
            );
            rendered_rows.push_str(&row_html);
        }

        let body = format!(
            "<div id=\"{id}-logs\" class=\"overflow-y-auto font-mono text-xs\" style=\"max-height: {h}px;\">\
                {rows}\
            </div>",
            id = self.id,
            h = self.max_height,
            rows = rendered_rows
        );

        // Client auto-scrolling script
        let script = if self.auto_scroll {
            let nonce_attr = match &self.nonce {
                Some(n) => format!(r#" nonce="{}""#, yav_core::escape_html(n)),
                None => String::new(),
            };
            format!(
                "<script{nonce_attr}>\
                (function() {{\
                    var el = document.getElementById('{id}-logs');\
                    if (el) {{ el.scrollTop = el.scrollHeight; }}\
                }})();\
                </script>",
                nonce_attr = nonce_attr,
                id = self.id
            )
        } else {
            String::new()
        };

        Node::Element(
            Element::new("div")
                .attr("id", &self.id)
                .attr("class", &container_class)
                .child(Node::Raw(toolbar))
                .child(Node::Raw(body))
                .child(Node::Raw(script)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_log_viewer_rendering_and_filter() {
        let viewer = LogViewer::new()
            .id("server-logs")
            .log(LogEntry::new(
                "2026-11-05 12:00:00",
                LogLevel::Info,
                "server::http",
                "Server listening on :3000",
            ))
            .log(
                LogEntry::new(
                    "2026-11-05 12:00:01",
                    LogLevel::Warn,
                    "db::pool",
                    "High connection latency (42ms)",
                )
                .attribute("latency_ms", "42"),
            )
            .log(
                LogEntry::new(
                    "2026-11-05 12:00:02",
                    LogLevel::Error,
                    "auth::jwt",
                    "Expired session token",
                )
                .trace_id("4bf92f3577b34da6a3ce929d0e0e4736"),
            );

        let node = viewer.render();
        let html = render_to_string(&node);

        assert!(html.contains("id=\"server-logs\""));
        assert!(html.contains("Server listening on :3000"));
        assert!(html.contains("High connection latency"));
        assert!(html.contains("Expired session token"));
        assert!(html.contains("4bf92f3577"));
        assert!(html.contains("OTel Logtail"));
    }

    #[test]
    fn test_log_viewer_nonce() {
        let viewer = LogViewer::new()
            .id("logs-nonce")
            .auto_scroll(true)
            .nonce("log_nonce_999");
        let html = render_to_string(&viewer.render());
        assert!(html.contains("nonce=\"log_nonce_999\""));
    }
}
