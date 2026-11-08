//! Zero-JS server-side syntax-highlighted CodeBlock component.
//!
//! Provides SSR-compiled code presentation with syntax coloring, language badges,
//! optional line numbers, and a zero-JS clipboard copy button.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// Supported languages for SSR syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeLanguage {
    #[default]
    Rust,
    Html,
    Css,
    JavaScript,
    TypeScript,
    Toml,
    Json,
    Bash,
    PlainText,
}

impl std::str::FromStr for CodeLanguage {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_name(s))
    }
}

impl CodeLanguage {
    /// Resolve language enum from a language name or extension.
    pub fn from_name(lang: &str) -> Self {
        match lang.to_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "html" | "rsx" | "yav" => Self::Html,
            "css" => Self::Css,
            "javascript" | "js" => Self::JavaScript,
            "typescript" | "ts" => Self::TypeScript,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "bash" | "sh" | "shell" | "zsh" => Self::Bash,
            _ => Self::PlainText,
        }
    }

    /// Backwards-compatible parser alias.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(lang: &str) -> Self {
        Self::from_name(lang)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rust => "RUST",
            Self::Html => "HTML / RSX",
            Self::Css => "CSS",
            Self::JavaScript => "JS",
            Self::TypeScript => "TS",
            Self::Toml => "TOML",
            Self::Json => "JSON",
            Self::Bash => "BASH",
            Self::PlainText => "TEXT",
        }
    }
}

/// A pre-styled, accessible code presentation card with server-side syntax highlighting.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CodeBlock {
    code: String,
    language: CodeLanguage,
    title: Option<String>,
    line_numbers: bool,
    class: String,
}

impl CodeBlock {
    /// Create a new code block with the given source text.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            language: CodeLanguage::Rust,
            title: None,
            line_numbers: false,
            class: String::new(),
        }
    }

    /// Set language by enum.
    pub fn language(mut self, language: CodeLanguage) -> Self {
        self.language = language;
        self
    }

    /// Set language by string (e.g. `"rust"`, `"html"`, `"toml"`, `"js"`).
    pub fn lang(mut self, lang: impl AsRef<str>) -> Self {
        self.language = CodeLanguage::from_str(lang.as_ref());
        self
    }

    /// Optional file name or title for the code block header.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Toggle line number gutter.
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Add custom classes to the outer container.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let container_base = "yav-codeblock overflow-hidden rounded-xl border border-gray-800 bg-[#0d1117] text-gray-200 shadow-xl my-4 font-mono text-[13px]";
        let container_class = merge_classes(&[container_base, &self.class]);

        let mut root = Element::new("div").attr("class", container_class);

        // Header bar
        let header_base = "flex items-center justify-between border-b border-gray-800/80 bg-[#161b22] px-4 py-2.5 text-xs text-gray-400 select-none";
        let mut header = Element::new("div").attr("class", header_base);

        let title_text = self
            .title
            .unwrap_or_else(|| self.language.display_name().to_string());
        let title_node = Element::new("span")
            .attr("class", "flex items-center gap-2 font-medium text-gray-300")
            .child(Node::text(title_text));
        header = header.child(Node::Element(title_node));

        // Copy button that strips line-number spans before copying to clipboard
        let copy_btn = Element::new("button")
            .attr("type", "button")
            .attr(
                "class",
                "rounded px-2 py-1 text-[11px] font-medium text-gray-400 hover:text-white hover:bg-gray-700/50 transition-colors",
            )
            .attr("onclick", "var c=this.closest('.yav-codeblock').querySelector('code').cloneNode(true);c.querySelectorAll('[data-line-num]').forEach(function(n){n.remove()});navigator.clipboard.writeText(c.innerText);this.innerText='Copied!';setTimeout(()=>this.innerText='Copy',2000)")
            .child(Node::text("Copy"));
        header = header.child(Node::Element(copy_btn));

        root = root.child(Node::Element(header));

        // Code body with highlighted tokens
        let highlighted = highlight_code(&self.code, self.language, self.line_numbers);

        let code_wrapper = Element::new("pre")
            .attr("class", "overflow-x-auto p-4 leading-relaxed")
            .child(Node::Element(
                Element::new("code")
                    .attr("class", "block")
                    .child(Node::raw(highlighted)),
            ));

        root = root.child(Node::Element(code_wrapper));
        Node::Element(root)
    }
}

impl IntoNode for CodeBlock {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<CodeBlock> for Node {
    fn from(c: CodeBlock) -> Self {
        c.render()
    }
}

impl Component for CodeBlock {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`CodeBlock`].
pub fn code_block(code: impl Into<String>) -> CodeBlock {
    CodeBlock::new(code)
}

/// Properties for constructing a [`CodeBlock`] via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CodeBlockProps {
    pub code: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub filename: Option<String>,
    pub line_numbers: bool,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for [`CodeBlock`], enabling `<CodeBlock ...>...</CodeBlock>` in `rsx!`.
#[allow(non_snake_case)]
pub fn CodeBlock(props: CodeBlockProps) -> Node {
    let code = props.code.unwrap_or_else(|| {
        props
            .children
            .iter()
            .map(|n| match n {
                Node::Text(t) => t.clone(),
                Node::Raw(r) => r.clone(),
                Node::Static(s) => (*s).to_string(),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("")
    });
    let mut cb = CodeBlock::new(code);
    if let Some(lang) = props.language {
        cb = cb.lang(lang);
    }
    if let Some(title) = props.title.or(props.filename) {
        cb = cb.title(title);
    }
    cb.line_numbers(props.line_numbers)
        .class(props.class)
        .render()
}

/// Highlight source code using lightweight tokenizer and return pre-rendered HTML string.
pub fn highlight_code(code: &str, lang: CodeLanguage, show_lines: bool) -> String {
    let mut out = String::with_capacity(code.len() * 3);
    let lines: Vec<&str> = code.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if show_lines {
            let line_num = idx + 1;
            out.push_str(&format!(
                r#"<span data-line-num="{line_num}" class="inline-block w-8 mr-4 text-right text-gray-600 select-none">{line_num}</span>"#
            ));
        }
        highlight_line(line, lang, &mut out);
        if idx + 1 < lines.len() {
            out.push('\n');
        }
    }

    out
}

fn highlight_line(line: &str, lang: CodeLanguage, out: &mut String) {
    let trimmed = line.trim_start();
    // Full-line comments
    if (lang == CodeLanguage::Rust
        || lang == CodeLanguage::JavaScript
        || lang == CodeLanguage::TypeScript)
        && trimmed.starts_with("//")
    {
        let indent = &line[..line.len() - trimmed.len()];
        out.push_str(indent);
        out.push_str(r#"<span class="text-gray-500 italic">"#);
        out.push_str(&yav_core::escape_html(trimmed));
        out.push_str("</span>");
        return;
    }
    if (lang == CodeLanguage::Toml || lang == CodeLanguage::Bash) && trimmed.starts_with('#') {
        let indent = &line[..line.len() - trimmed.len()];
        out.push_str(indent);
        out.push_str(r#"<span class="text-gray-500 italic">"#);
        out.push_str(&yav_core::escape_html(trimmed));
        out.push_str("</span>");
        return;
    }

    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    while i < chars.len() {
        // Inline trailing comments (// for Rust/JS/TS, # for Toml/Bash)
        if (lang == CodeLanguage::Rust
            || lang == CodeLanguage::JavaScript
            || lang == CodeLanguage::TypeScript)
            && chars[i] == '/'
            && i + 1 < chars.len()
            && chars[i + 1] == '/'
        {
            let rest: String = chars[i..].iter().collect();
            out.push_str(r#"<span class="text-gray-500 italic">"#);
            out.push_str(&yav_core::escape_html(&rest));
            out.push_str("</span>");
            break;
        }
        if (lang == CodeLanguage::Toml || lang == CodeLanguage::Bash) && chars[i] == '#' {
            let rest: String = chars[i..].iter().collect();
            out.push_str(r#"<span class="text-gray-500 italic">"#);
            out.push_str(&yav_core::escape_html(&rest));
            out.push_str("</span>");
            break;
        }

        // Rust Lifetimes ('a, 'static, '_) vs Character Literals ('x', '\n')
        if lang == CodeLanguage::Rust
            && chars[i] == '\''
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
        {
            let is_char_lit = i + 2 < chars.len() && chars[i + 2] == '\'';
            if !is_char_lit {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let lt: String = chars[start..i].iter().collect();
                out.push_str(r#"<span class="text-amber-300 italic">"#);
                out.push_str(&yav_core::escape_html(&lt));
                out.push_str("</span>");
                continue;
            }
        }

        // Strings and character literals
        if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < chars.len() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push_str(r#"<span class="text-emerald-400">"#);
            out.push_str(&yav_core::escape_html(&s));
            out.push_str("</span>");
            continue;
        }

        // Rust attributes #[...]
        if lang == CodeLanguage::Rust
            && chars[i] == '#'
            && i + 1 < chars.len()
            && chars[i + 1] == '['
        {
            let start = i;
            while i < chars.len() && chars[i] != ']' {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let attr: String = chars[start..i].iter().collect();
            out.push_str(r#"<span class="text-amber-400">"#);
            out.push_str(&yav_core::escape_html(&attr));
            out.push_str("</span>");
            continue;
        }

        // HTML tags
        if (lang == CodeLanguage::Html || lang == CodeLanguage::Rust)
            && chars[i] == '<'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '/')
        {
            out.push_str(r#"<span class="text-sky-400">&lt;"#);
            i += 1;
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric()
                    || chars[i] == '/'
                    || chars[i] == '-'
                    || chars[i] == ':')
            {
                i += 1;
            }
            let tag: String = chars[start..i].iter().collect();
            out.push_str(&yav_core::escape_html(&tag));
            out.push_str("</span>");
            continue;
        }

        // Identifiers / Keywords
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();

            // Check if macro call (e.g. `println!`, `rsx!`)
            if i < chars.len()
                && chars[i] == '!'
                && (lang == CodeLanguage::Rust || lang == CodeLanguage::Html)
            {
                i += 1;
                out.push_str(r#"<span class="text-purple-400 font-semibold">"#);
                out.push_str(&yav_core::escape_html(&word));
                out.push('!');
                out.push_str("</span>");
                continue;
            }

            if is_keyword(&word, lang) {
                out.push_str(r#"<span class="text-rose-400 font-semibold">"#);
                out.push_str(&yav_core::escape_html(&word));
                out.push_str("</span>");
            } else if is_type_name(&word) {
                out.push_str(r#"<span class="text-teal-300">"#);
                out.push_str(&yav_core::escape_html(&word));
                out.push_str("</span>");
            } else if i < chars.len() && chars[i] == '(' {
                out.push_str(r#"<span class="text-blue-400">"#);
                out.push_str(&yav_core::escape_html(&word));
                out.push_str("</span>");
            } else {
                out.push_str(&yav_core::escape_html(&word));
            }
            continue;
        }

        // Numbers
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_hexdigit()
                    || chars[i] == '.'
                    || chars[i] == 'x'
                    || chars[i] == '_')
            {
                i += 1;
            }
            let num: String = chars[start..i].iter().collect();
            out.push_str(r#"<span class="text-amber-300">"#);
            out.push_str(&yav_core::escape_html(&num));
            out.push_str("</span>");
            continue;
        }

        // Special punctuation
        match chars[i] {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
        i += 1;
    }
}

fn is_keyword(word: &str, lang: CodeLanguage) -> bool {
    match lang {
        CodeLanguage::Rust => matches!(
            word,
            "as" | "async"
                | "await"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "Self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "unsafe"
                | "use"
                | "where"
                | "while"
        ),
        CodeLanguage::JavaScript | CodeLanguage::TypeScript => matches!(
            word,
            "async"
                | "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "function"
                | "if"
                | "import"
                | "in"
                | "instanceof"
                | "let"
                | "new"
                | "null"
                | "return"
                | "static"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
        ),
        _ => false,
    }
}

fn is_type_name(word: &str) -> bool {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => matches!(
            word,
            "u8" | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
        ),
    }
}
