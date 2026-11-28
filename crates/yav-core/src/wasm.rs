//! Client-Side WebAssembly Islands (`WasmIsland`).
//!
//! Provides zero-overhead WebAssembly island orchestration, allowing heavy
//! client-side computation (image filters, cryptography, audio synthesis)
//! to run in pure compiled Rust with zero JavaScript build toolchain friction.
//!
//! ## WebAssembly Calling Conventions & Linear Memory ABI
//!
//! Raw WebAssembly modules compiled without high-level bindings (such as `wasm-bindgen`)
//! only accept numeric primitives (`i32`, `i64`, `f32`, `f64`) at export boundaries.
//! Complex types (JSON strings, vectors, structs) must either:
//! - Pass scalar arguments (e.g. numeric iteration counts, seeds, or buffer pointers),
//! - Utilize `wasm-bindgen` glue if targeting rich DOM/JS interop, or
//! - Manage linear memory directly via `exports.memory.buffer` and exported `malloc`/`free` routines.
//!
//! If [`WasmIslandProps::params`] is omitted or `None`, the exported function is invoked
//! with zero arguments (`exports[name]()`). If provided, the expression is evaluated and
//! passed directly to the exported function.

use crate::escape::{escape_attribute, escape_js_str, escape_script_json};
use crate::{Component, Element, Node};

/// Properties for mounting a client-side WebAssembly Island.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmIslandProps {
    /// Container DOM element identifier.
    pub id: String,
    /// URL to the `.wasm` binary asset.
    pub module_url: String,
    /// Exported WebAssembly function name to invoke upon instantiation.
    pub export_name: String,
    /// Optional arguments passed to the WASM entrypoint (e.g. numeric scalar, pointer, or expression).
    pub params: Option<String>,
    /// Fallback server-rendered HTML while WASM compiles and streams.
    pub fallback: Option<String>,
    /// Optional CSP nonce attribute applied to the hydration script.
    pub nonce: Option<String>,
    /// Whether to auto-instantiate the module on page load (`true`) or on user interaction (`false`).
    pub auto_start: bool,
}

/// Declarative WebAssembly Client Island Component.
#[derive(Debug, Clone)]
pub struct WasmIsland {
    props: WasmIslandProps,
}

impl WasmIsland {
    /// Create a new WebAssembly Island bound to a `.wasm` file and export entrypoint.
    pub fn new(
        id: impl Into<String>,
        module_url: impl Into<String>,
        export_name: impl Into<String>,
    ) -> Self {
        Self {
            props: WasmIslandProps {
                id: id.into(),
                module_url: module_url.into(),
                export_name: export_name.into(),
                params: None,
                fallback: None,
                nonce: None,
                auto_start: true,
            },
        }
    }

    /// Provide a CSP nonce attribute for the embedded hydration `<script>` tag.
    #[must_use]
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.props.nonce = Some(nonce.into());
        self
    }

    /// Provide JSON arguments passed to the WASM exported function.
    #[must_use]
    pub fn with_params(mut self, params: impl Into<String>) -> Self {
        self.props.params = Some(params.into());
        self
    }

    /// Provide placeholder HTML shown while the `.wasm` binary streams and compiles.
    #[must_use]
    pub fn with_fallback(mut self, fallback: impl Into<String>) -> Self {
        self.props.fallback = Some(fallback.into());
        self
    }

    /// Control whether the module starts automatically on load or lazily upon interaction.
    #[must_use]
    pub fn auto_start(mut self, auto: bool) -> Self {
        self.props.auto_start = auto;
        self
    }

    /// Generate inline hydration script for compiling and streaming the WASM island.
    pub fn hydration_script(&self) -> String {
        let id = escape_js_str(&self.props.id);
        let url = escape_js_str(&self.props.module_url);
        let export = escape_js_str(&self.props.export_name);
        let nonce_attr = match &self.props.nonce {
            Some(n) if !n.is_empty() => format!(" nonce=\"{}\"", escape_attribute(n)),
            _ => String::new(),
        };
        let call_expr = match &self.props.params {
            Some(raw) => {
                // Only embed params verbatim when they parse as valid JSON.
                // JSON is a safe data-only subset of JavaScript: it cannot
                // contain function expressions, statement separators, or
                // other control-flow constructs that could break out of the
                // call expression. If the value is not valid JSON, fall back
                // to a properly escaped string literal.
                let safe_arg = if serde_json::from_str::<serde_json::Value>(raw).is_ok() {
                    escape_script_json(raw)
                } else {
                    format!("\"{}\"", escape_js_str(raw))
                };
                format!("exports[\"{export}\"]({safe_arg})")
            }
            None => format!("exports[\"{export}\"]()"),
        };

        format!(
            r#"<script{nonce_attr}>(function(){{
  var el = document.getElementById("{id}");
  if (!el || !window.WebAssembly) return;

  function loadWasm() {{
    WebAssembly.instantiateStreaming(fetch("{url}"), {{
      env: {{
        memory: new WebAssembly.Memory({{ initial: 256, maximum: 512 }}),
        abort: function() {{ console.error("WASM island aborted"); }}
      }}
    }}).then(function(res) {{
      var exports = res.instance.exports;
      if (typeof exports["{export}"] === "function") {{
        var result = {call_expr};
        var event = new CustomEvent("yav:wasm-ready", {{ detail: {{ result: result, exports: exports }} }});
        el.dispatchEvent(event);
      }}
    }}).catch(function(err) {{
      console.warn("WASM Island load error:", err);
    }});
  }}

  if ({auto}) {{
    loadWasm();
  }} else {{
    el.addEventListener("click", loadWasm, {{ once: true }});
  }}
}})();</script>"#,
            id = id,
            url = url,
            export = export,
            call_expr = call_expr,
            auto = self.props.auto_start
        )
    }
}

impl Component for WasmIsland {
    fn render(&self) -> Node {
        let fallback_node = match &self.props.fallback {
            Some(fb) => Node::raw(fb.clone()),
            None => Node::Element(
                Element::new("div")
                    .attr("class", "text-xs text-slate-500 animate-pulse")
                    .child(Node::text("Loading WebAssembly module...")),
            ),
        };

        Node::Element(
            Element::new("div")
                .attr("id", self.props.id.clone())
                .attr("class", "yav-wasm-island")
                .attr("data-wasm-url", self.props.module_url.clone())
                .attr("data-wasm-export", self.props.export_name.clone())
                .child(fallback_node)
                .child(Node::raw(self.hydration_script())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::render_to_string;

    #[test]
    fn test_wasm_island_rendering() {
        let island = WasmIsland::new("crypto-island", "/wasm/crypto.wasm", "hash_payload")
            .with_params(r#"{"iterations": 1000}"#)
            .with_fallback("<span>Calcul cryptographique en cours...</span>");

        let html = render_to_string(&island.render());
        assert!(html.contains("yav-wasm-island"));
        assert!(html.contains("crypto-island"));
        assert!(html.contains("/wasm/crypto.wasm"));
        assert!(html.contains("Calcul cryptographique en cours..."));
        assert!(html.contains("WebAssembly.instantiateStreaming"));
        assert!(html.contains(r#"exports["hash_payload"]({"iterations": 1000})"#));
    }

    #[test]
    fn test_wasm_island_no_params() {
        let island = WasmIsland::new("synth-island", "/wasm/synth.wasm", "start_audio");
        let html = render_to_string(&island.render());
        assert!(html.contains(r#"exports["start_audio"]()"#));
    }

    #[test]
    fn test_wasm_island_json_params_embedded_verbatim() {
        let island =
            WasmIsland::new("island", "/wasm/m.wasm", "fn").with_params(r#"{"iterations": 1000}"#);
        let html = render_to_string(&island.render());
        assert!(html.contains(r#"exports["fn"]({"iterations": 1000})"#));
    }

    #[test]
    fn test_wasm_island_non_json_params_escaped() {
        // This is not valid JSON (function call syntax), so it gets wrapped
        // in a string literal. This is safe but the caller should prefer JSON.
        let island = WasmIsland::new("island", "/wasm/m.wasm", "fn").with_params("hello world");
        let html = render_to_string(&island.render());
        // "hello world" is not valid JSON, so it becomes a string literal
        assert!(html.contains(r#"exports["fn"]("hello world")"#));
    }

    #[test]
    fn test_wasm_island_xss_prevention() {
        // This payload would break out of the call expression if embedded raw.
        // With JSON validation, it fails to parse as JSON and is wrapped in
        // an escaped string literal instead.
        let payload = r#"1); alert(document.cookie);//"#;
        let island = WasmIsland::new("island", "/wasm/m.wasm", "fn").with_params(payload);
        let html = render_to_string(&island.render());
        // The dangerous payload must NOT appear verbatim inside the call expression.
        // It should be inside a string literal.
        assert!(
            !html.contains(r#"exports["fn"](1); alert(document.cookie);//)"#),
            "XSS payload was embedded verbatim — vulnerability!"
        );
        // The alert call must not appear as executable code (it's inside a string).
        assert!(html.contains(r#"exports["fn"]("1); alert(document.cookie);//")"#));
    }

    #[test]
    fn test_wasm_island_script_tag_in_params() {
        // JSON string values can contain "</script>" which would close the
        // outer <script> tag. Verify it gets escaped.
        let payload = r#""</script><script>alert(1)</script>""#;
        let island = WasmIsland::new("island", "/wasm/m.wasm", "fn").with_params(payload);
        let html = render_to_string(&island.render());
        // The </script> must be escaped to prevent HTML breakout.
        assert!(!html.contains("</script><script>alert(1)</script>"));
        assert!(html.contains("\\u003C/script") || html.contains("<\\/script"));
    }

    #[test]
    fn test_wasm_island_nonce_support() {
        let island = WasmIsland::new("island", "/wasm/m.wasm", "fn").with_nonce("rAnd0mN0nc3");
        let html = render_to_string(&island.render());
        assert!(html.contains(r#"<script nonce="rAnd0mN0nc3">"#));
    }
}
