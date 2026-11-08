//! # YAV Interactive Code Playground & REPL (`<CodePlayground />`)
//!
//! Split-pane live code editor and sandbox evaluation runner.
//! Includes synchronized line numbers gutter, execution console output / iframe preview,
//! syntax language pills, Ctrl+Enter runner, and remote execution API binding.

use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Theme for the code playground editor and terminal output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaygroundTheme {
    #[default]
    Dark,
    Light,
    Monokai,
}

impl PlaygroundTheme {
    #[must_use]
    pub fn bg_class(&self) -> &'static str {
        match self {
            Self::Dark => "bg-slate-950 border-slate-800 text-slate-100",
            Self::Light => "bg-white border-slate-200 text-slate-900",
            Self::Monokai => "bg-[#272822] border-[#3e3d32] text-[#f8f8f2]",
        }
    }
}

/// Interactive Code Sandbox and Execution Playground Component.
#[derive(Debug, Clone, PartialEq)]
pub struct CodePlayground {
    pub id: String,
    pub title: String,
    pub language: String,
    pub initial_code: String,
    pub run_endpoint: Option<String>,
    pub read_only: bool,
    pub theme: PlaygroundTheme,
    pub height_px: u32,
    pub output_placeholder: String,
    pub class: String,
}

impl CodePlayground {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: "Yav Playground".to_string(),
            language: "rust".to_string(),
            initial_code: String::new(),
            run_endpoint: None,
            read_only: false,
            theme: PlaygroundTheme::default(),
            height_px: 380,
            output_placeholder: "Cliquez sur 'Exécuter' pour compiler et tester votre code..."
                .to_string(),
            class: String::new(),
        }
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    #[must_use]
    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }

    #[must_use]
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.initial_code = code.into();
        self
    }

    #[must_use]
    pub fn run_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.run_endpoint = Some(endpoint.into());
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: PlaygroundTheme) -> Self {
        self.theme = theme;
        self
    }

    #[must_use]
    pub fn height(mut self, height_px: u32) -> Self {
        self.height_px = height_px.max(200);
        self
    }

    #[must_use]
    pub fn output_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.output_placeholder = placeholder.into();
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

impl Component for CodePlayground {
    fn render(&self) -> Node {
        let theme_bg = self.theme.bg_class();
        let base_cls = format!(
            "yav-code-playground border rounded-xl overflow-hidden shadow-2xl flex flex-col font-sans {theme_bg}"
        );
        let root_class = merge_classes(&[&base_cls, &self.class]);

        let line_count = self.initial_code.lines().count().max(1);
        let mut gutter_text = String::new();
        for i in 1..=line_count {
            gutter_text.push_str(&format!("{i}\n"));
        }

        // Header Toolbar
        let mut header = Element::new("div").attr(
            "class",
            "flex items-center justify-between px-4 py-2.5 border-b border-inherit bg-black/20 text-xs",
        );

        let left_header = Element::new("div")
            .attr("class", "flex items-center gap-3")
            .child(
                Element::new("div")
                    .attr("class", "flex items-center gap-1.5")
                    .child(
                        Element::new("span")
                            .attr("class", "w-3 h-3 rounded-full bg-rose-500/80 inline-block"),
                    )
                    .child(
                        Element::new("span")
                            .attr("class", "w-3 h-3 rounded-full bg-amber-500/80 inline-block"),
                    )
                    .child(
                        Element::new("span")
                            .attr("class", "w-3 h-3 rounded-full bg-emerald-500/80 inline-block"),
                    ),
            )
            .child(
                Element::new("span")
                    .attr("class", "font-medium text-slate-200")
                    .child(self.title.as_str()),
            )
            .child(
                Element::new("span")
                    .attr(
                        "class",
                        "px-2 py-0.5 text-[11px] font-mono font-semibold uppercase rounded bg-indigo-500/20 text-indigo-400 border border-indigo-500/30",
                    )
                    .child(self.language.as_str()),
            );

        let mut right_header = Element::new("div").attr("class", "flex items-center gap-2");

        // Copy button
        let copy_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-play-copy px-2.5 py-1 text-xs font-medium rounded bg-white/5 hover:bg-white/10 text-slate-300 transition-colors border border-white/10",
            )
            .attr("title", "Copier le code")
            .child("Copier");
        right_header = right_header.child(copy_btn);

        // Run button
        let run_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "yav-play-run flex items-center gap-1.5 px-3 py-1 text-xs font-semibold rounded bg-emerald-600 hover:bg-emerald-500 text-white transition-all shadow-sm active:scale-95",
            )
            .attr("title", "Exécuter (Ctrl+Entrée)")
            .child("▶ Exécuter");
        right_header = right_header.child(run_btn);

        header = header.child(left_header).child(right_header);

        // Split Editor & Console Pane
        let content_style = format!("height: {}px;", self.height_px);
        let mut split_pane = Element::new("div")
            .attr(
                "class",
                "grid grid-cols-1 md:grid-cols-2 divide-y md:divide-y-0 md:divide-x divide-inherit",
            )
            .attr("style", content_style);

        // Left / Top Editor Pane
        let mut editor_pane =
            Element::new("div").attr("class", "flex h-full overflow-hidden relative");

        // Line numbers gutter
        let gutter = Element::new("div")
            .attr(
                "class",
                "yav-play-gutter select-none text-right py-3 px-3 font-mono text-xs text-slate-500 bg-black/10 border-r border-inherit whitespace-pre leading-6 min-w-[2.5rem]",
            )
            .child(gutter_text.as_str());

        // Code Textarea
        let mut textarea = Element::new("textarea")
            .attr(
                "class",
                "yav-play-code flex-1 bg-transparent p-3 font-mono text-xs outline-none resize-none leading-6 text-slate-200 selection:bg-indigo-500/30 whitespace-pre font-normal",
            )
            .attr("spellcheck", "false")
            .attr("autocomplete", "off")
            .attr("autocapitalize", "off");

        if self.read_only {
            textarea = textarea.attr("readonly", "true");
        }
        textarea = textarea.child(self.initial_code.as_str());

        editor_pane = editor_pane.child(gutter).child(textarea);

        // Right / Bottom Console & Output Pane
        let console_pane = Element::new("div")
            .attr("class", "flex flex-col h-full bg-black/30 overflow-hidden")
            .child(
                Element::new("div")
                    .attr(
                        "class",
                        "flex items-center justify-between px-3 py-1.5 border-b border-inherit bg-black/20 text-[11px] text-slate-400 font-mono",
                    )
                    .child(Element::new("span").child("CONSOLE OUTPUT"))
                    .child(
                        Element::new("span")
                            .attr("class", "yav-play-status text-emerald-400 font-medium")
                            .child("Prêt"),
                    ),
            )
            .child(
                Element::new("pre")
                    .attr(
                        "class",
                        "yav-play-output flex-1 p-3 font-mono text-xs overflow-auto text-slate-300 leading-relaxed whitespace-pre-wrap select-text",
                    )
                    .child(self.output_placeholder.as_str()),
            );

        split_pane = split_pane.child(editor_pane).child(console_pane);

        // Client-side Runner Script
        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

const codeArea = root.querySelector('.yav-play-code');
const gutter = root.querySelector('.yav-play-gutter');
const runBtn = root.querySelector('.yav-play-run');
const copyBtn = root.querySelector('.yav-play-copy');
const outputPre = root.querySelector('.yav-play-output');
const statusSpan = root.querySelector('.yav-play-status');
const runEndpoint = {run_endpoint_json};

function updateGutter() {{
  const lines = codeArea.value.split('\n').length;
  let s = '';
  for (let i = 1; i <= lines; i++) {{
    s += i + '\n';
  }}
  gutter.textContent = s;
}}

codeArea.addEventListener('input', updateGutter);
codeArea.addEventListener('scroll', () => {{
  gutter.scrollTop = codeArea.scrollTop;
}});

codeArea.addEventListener('keydown', (e) => {{
  if (e.key === 'Tab') {{
    e.preventDefault();
    const start = codeArea.selectionStart;
    const end = codeArea.selectionEnd;
    codeArea.value = codeArea.value.substring(0, start) + '    ' + codeArea.value.substring(end);
    codeArea.selectionStart = codeArea.selectionEnd = start + 4;
    updateGutter();
  }} else if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {{
    e.preventDefault();
    executeCode();
  }}
}});

copyBtn.addEventListener('click', () => {{
  navigator.clipboard.writeText(codeArea.value).then(() => {{
    copyBtn.textContent = '✓ Copié !';
    setTimeout(() => {{ copyBtn.textContent = 'Copier'; }}, 2000);
  }});
}});

function executeCode() {{
  const code = codeArea.value;
  statusSpan.textContent = 'Exécution...';
  statusSpan.className = 'yav-play-status text-amber-400 font-medium animate-pulse';

  const t0 = performance.now();

  if (runEndpoint) {{
    fetch(runEndpoint, {{
      method: 'POST',
      headers: {{ 'Content-Type': 'application/json' }},
      body: JSON.stringify({{ code: code, language: '{lang}' }})
    }})
    .then(r => r.json())
    .then(data => {{
      const elapsed = Math.round(performance.now() - t0);
      statusSpan.textContent = `Succès (${{elapsed}}ms)`;
      statusSpan.className = 'yav-play-status text-emerald-400 font-medium';
      outputPre.textContent = data.output || data.result || 'Programme terminé sans sortie standard.';
    }})
    .catch(err => {{
      statusSpan.textContent = 'Erreur';
      statusSpan.className = 'yav-play-status text-rose-400 font-medium';
      outputPre.textContent = 'Échec de connexion au serveur d\'exécution: ' + err.message;
    }});
  }} else {{
    setTimeout(() => {{
      const elapsed = Math.round(performance.now() - t0);
      statusSpan.textContent = `Exécuté (${{elapsed}}ms)`;
      statusSpan.className = 'yav-play-status text-emerald-400 font-medium';
      outputPre.textContent = `[Simulation Yav REPL] Code soumis (${{code.length}} caractères, ${{code.split('\\n').length}} lignes)\n---\nSortie simulée: Exécution réussie sans erreurs.`;
    }}, 120);
  }}
}}

runBtn.addEventListener('click', executeCode);
}})();"##,
            id = escape_html(&self.id),
            run_endpoint_json = self
                .run_endpoint
                .as_ref()
                .map_or("null".to_string(), |u| format!(r#""{}""#, escape_html(u))),
            lang = escape_html(&self.language),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(header)
            .child(split_pane)
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `CodePlayground`.
#[must_use]
pub fn code_playground(id: impl Into<String>) -> CodePlayground {
    CodePlayground::new(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_code_playground_rendering() {
        let pg = CodePlayground::new("demo_repl")
            .title("Yav Rust Playground")
            .language("rust")
            .code("fn main() {\n    println!(\"Hello, Yav!\");\n}")
            .run_endpoint("/api/run-code")
            .height(350);

        let node = pg.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-code-playground"));
        assert!(html.contains("demo_repl"));
        assert!(html.contains("Yav Rust Playground"));
        assert!(html.contains("rust"));
        assert!(html.contains("fn main()"));
        assert!(html.contains("Hello, Yav!"));
        assert!(html.contains("/api/run-code"));
        assert!(html.contains("CONSOLE OUTPUT"));
        assert!(html.contains("yav-play-run"));
        assert!(html.contains("yav-play-gutter"));
    }
}
