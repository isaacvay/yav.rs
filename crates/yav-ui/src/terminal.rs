//! Interactive Web ANSI Terminal & Live Log Shell Component.
//!
//! Provides a server-rendered terminal emulator supporting ANSI color codes,
//! command prompt simulation, and live Server-Sent Events (SSE) streaming hooks.

use yav_core::{Component, Element, Node};

/// Line styling attributes parsed from ANSI escape sequences.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnsiSpan {
    pub text: String,
    pub fg_color: Option<&'static str>,
    pub bg_color: Option<&'static str>,
    pub bold: bool,
    pub dim: bool,
    pub underline: bool,
}

/// Parse a raw terminal string containing ANSI escape codes into styled spans.
pub fn parse_ansi(input: &str) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut current_fg = None;
    let mut current_bg = None;
    let mut bold = false;
    let mut dim = false;
    let mut underline = false;

    let mut cursor = 0;
    while let Some(esc_pos) = input[cursor..].find("\x1b[") {
        let abs_esc = cursor + esc_pos;
        if abs_esc > cursor {
            spans.push(AnsiSpan {
                text: input[cursor..abs_esc].to_string(),
                fg_color: current_fg,
                bg_color: current_bg,
                bold,
                dim,
                underline,
            });
        }

        if let Some(m_pos) = input[abs_esc..].find('m') {
            let code_str = &input[abs_esc + 2..abs_esc + m_pos];
            for part in code_str.split(';') {
                match part.trim() {
                    "0" | "" => {
                        current_fg = None;
                        current_bg = None;
                        bold = false;
                        dim = false;
                        underline = false;
                    }
                    "1" => bold = true,
                    "2" => dim = true,
                    "4" => underline = true,
                    "30" => current_fg = Some("#475569"), // Black/Slate
                    "31" => current_fg = Some("#ef4444"), // Red
                    "32" => current_fg = Some("#10b981"), // Green
                    "33" => current_fg = Some("#f59e0b"), // Yellow
                    "34" => current_fg = Some("#3b82f6"), // Blue
                    "35" => current_fg = Some("#a855f7"), // Magenta
                    "36" => current_fg = Some("#06b6d4"), // Cyan
                    "37" => current_fg = Some("#f8fafc"), // White
                    "90" => current_fg = Some("#64748b"), // Bright Black
                    "91" => current_fg = Some("#f87171"), // Bright Red
                    "92" => current_fg = Some("#34d399"), // Bright Green
                    "93" => current_fg = Some("#fbbf24"), // Bright Yellow
                    "94" => current_fg = Some("#60a5fa"), // Bright Blue
                    "95" => current_fg = Some("#c084fc"), // Bright Magenta
                    "96" => current_fg = Some("#22d3ee"), // Bright Cyan
                    "97" => current_fg = Some("#ffffff"), // Bright White
                    _ => {}
                }
            }
            cursor = abs_esc + m_pos + 1;
        } else {
            cursor = abs_esc + 2;
        }
    }

    if cursor < input.len() {
        spans.push(AnsiSpan {
            text: input[cursor..].to_string(),
            fg_color: current_fg,
            bg_color: current_bg,
            bold,
            dim,
            underline,
        });
    }

    spans
}

/// Properties for configuring the `<Terminal />` component.
#[derive(Debug, Clone, Default)]
pub struct TerminalProps {
    /// Window title displayed in the terminal titlebar (e.g. `"yav-worker-01.log"`).
    pub title: String,
    /// Static log output or startup banner lines.
    pub lines: Vec<String>,
    /// Command line prompt string (e.g. `"admin@yav-app:~$ "`).
    pub prompt: String,
    /// Optional SSE endpoint URL streaming real-time live log lines.
    pub stream_url: Option<String>,
    /// Whether user can type commands into the terminal.
    pub allow_input: bool,
    /// Terminal container height in pixels.
    pub height_px: u32,
}

/// Server-rendered interactive Web Terminal emulator.
#[derive(Debug, Clone)]
pub struct Terminal {
    props: TerminalProps,
}

impl Terminal {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            props: TerminalProps {
                title: title.into(),
                lines: Vec::new(),
                prompt: "user@yav:~$ ".to_string(),
                stream_url: None,
                allow_input: true,
                height_px: 360,
            },
        }
    }

    #[must_use]
    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.props.lines.push(line.into());
        self
    }

    #[must_use]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.props.prompt = prompt.into();
        self
    }

    #[must_use]
    pub fn stream_url(mut self, url: impl Into<String>) -> Self {
        self.props.stream_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn height_px(mut self, height: u32) -> Self {
        self.props.height_px = height;
        self
    }
}

impl Component for Terminal {
    fn render(&self) -> Node {
        let p = &self.props;

        let titlebar = Element::new("div")
            .attr("class", "flex items-center justify-between border-b border-slate-800 bg-slate-900/90 px-4 py-2.5 select-none")
            .child(
                Element::new("div")
                    .attr("class", "flex items-center gap-2")
                    .child(Element::new("span").attr("class", "h-3 w-3 rounded-full bg-rose-500/80 inline-block"))
                    .child(Element::new("span").attr("class", "h-3 w-3 rounded-full bg-amber-500/80 inline-block"))
                    .child(Element::new("span").attr("class", "h-3 w-3 rounded-full bg-emerald-500/80 inline-block")),
            )
            .child(
                Element::new("div")
                    .attr("class", "text-xs font-mono font-medium text-slate-400")
                    .child(Node::text(p.title.clone())),
            )
            .child(
                Element::new("div")
                    .attr("class", "text-[10px] font-mono text-slate-500")
                    .child(Node::text("bash")),
            );

        let mut body_el = Element::new("div")
            .attr(
                "class",
                "yav-terminal-body p-4 font-mono text-xs overflow-y-auto space-y-1 text-slate-200",
            )
            .attr("style", format!("height: {}px;", p.height_px));

        if let Some(stream) = &p.stream_url {
            body_el = body_el.attr("yav:sse-stream", stream);
        }

        for line in &p.lines {
            let spans = parse_ansi(line);
            let mut line_row =
                Element::new("div").attr("class", "leading-relaxed whitespace-pre-wrap");
            for span in spans {
                let mut span_el = Element::new("span").child(Node::text(span.text));
                let mut style = String::new();
                if let Some(fg) = span.fg_color {
                    style.push_str(&format!("color: {fg};"));
                }
                if let Some(bg) = span.bg_color {
                    style.push_str(&format!("background-color: {bg};"));
                }
                if span.bold {
                    style.push_str("font-weight: bold;");
                }
                if span.dim {
                    style.push_str("opacity: 0.6;");
                }
                if span.underline {
                    style.push_str("text-decoration: underline;");
                }
                if !style.is_empty() {
                    span_el = span_el.attr("style", style);
                }
                line_row = line_row.child(span_el);
            }
            body_el = body_el.child(line_row);
        }

        if p.allow_input {
            let prompt_row = Element::new("div")
                .attr("class", "flex items-center gap-2 pt-1")
                .child(
                    Element::new("span")
                        .attr("class", "text-emerald-400 font-semibold select-none")
                        .child(Node::text(p.prompt.clone())),
                )
                .child(
                    Element::new("input")
                        .attr("type", "text")
                        .attr("class", "flex-1 bg-transparent border-none outline-none text-white font-mono text-xs")
                        .attr("autofocus", "true")
                        .attr("spellcheck", "false"),
                );
            body_el = body_el.child(prompt_row);
        }

        Node::Element(
            Element::new("div")
                .attr("class", "yav-terminal rounded-xl border border-slate-800 bg-slate-950 shadow-2xl overflow-hidden")
                .child(titlebar)
                .child(body_el),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_ansi_parsing() {
        let raw = "\x1b[32mSuccess\x1b[0m \x1b[1;31mError\x1b[0m";
        let spans = parse_ansi(raw);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Success");
        assert_eq!(spans[0].fg_color, Some("#10b981"));
        assert_eq!(spans[1].text, " ");
        assert_eq!(spans[2].text, "Error");
        assert_eq!(spans[2].fg_color, Some("#ef4444"));
        assert!(spans[2].bold);
    }

    #[test]
    fn test_terminal_rendering() {
        let term = Terminal::new("deploy.log")
            .line("\x1b[32m[INFO]\x1b[0m Starting build container...")
            .line("\x1b[34m[DOCKER]\x1b[0m Image compiled in 1.4s");

        let html = render_to_string(&term.render());
        assert!(html.contains("deploy.log"));
        assert!(html.contains("Starting build container"));
        assert!(html.contains("color: #10b981;"));
    }
}
