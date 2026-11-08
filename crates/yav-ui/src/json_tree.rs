//! # YAV Interactive JSON & Schema Tree Visualizer (`<JsonTreeViewer />`)
//!
//! Renders JSON and schema payloads as a collapsible, syntax-colored interactive tree
//! with JSONPath copy buttons, type badges, search highlighting, and zero JS bundlers.

use serde_json::Value;
use yav_core::{escape_html, Component, Element, Node};

use crate::utils::merge_classes;

/// Interactive JSON tree explorer component.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonTreeViewer {
    pub id: String,
    pub data: Value,
    pub initial_expanded_depth: usize,
    pub show_types: bool,
    pub show_copy_path: bool,
    pub class: String,
}

impl JsonTreeViewer {
    #[must_use]
    pub fn new(id: impl Into<String>, data: Value) -> Self {
        Self {
            id: id.into(),
            data,
            initial_expanded_depth: 2,
            show_types: true,
            show_copy_path: true,
            class: String::new(),
        }
    }

    #[must_use]
    pub fn expanded_depth(mut self, depth: usize) -> Self {
        self.initial_expanded_depth = depth;
        self
    }

    #[must_use]
    pub fn show_types(mut self, show: bool) -> Self {
        self.show_types = show;
        self
    }

    #[must_use]
    pub fn show_copy_path(mut self, show: bool) -> Self {
        self.show_copy_path = show;
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    fn render_value(&self, val: &Value, path: &str, depth: usize) -> String {
        let is_expanded = depth < self.initial_expanded_depth;
        let hidden_attr = if is_expanded { "" } else { " hidden" };
        let chevron_rotate = if is_expanded { " rotate-90" } else { "" };

        match val {
            Value::Null => {
                r#"<span class="text-rose-400 font-mono font-medium">null</span>"#.to_string()
            }
            Value::Bool(b) => {
                format!(r#"<span class="text-sky-400 font-mono font-semibold">{b}</span>"#)
            }
            Value::Number(n) => {
                format!(r#"<span class="text-amber-400 font-mono">{n}</span>"#)
            }
            Value::String(s) => {
                let escaped = escape_html(s);
                format!(r#"<span class="text-emerald-400 font-mono break-all">"{escaped}"</span>"#)
            }
            Value::Array(arr) => {
                let len = arr.len();
                if len == 0 {
                    return r#"<span class="text-slate-400 font-mono">[]</span>"#.to_string();
                }

                let mut children_html = String::new();
                for (i, item) in arr.iter().enumerate() {
                    let child_path = format!("{path}[{i}]");
                    let rendered_child = self.render_value(item, &child_path, depth + 1);
                    let escaped_child_path = escape_html(&child_path);
                    let copy_btn = if self.show_copy_path {
                        format!(
                            r##"<button type="button" class="yav-json-copy opacity-0 group-hover:opacity-100 px-1 text-[10px] text-slate-500 hover:text-indigo-400 transition-opacity" data-path="{escaped_child_path}" title="Copier le chemin">copy</button>"##
                        )
                    } else {
                        String::new()
                    };

                    children_html.push_str(&format!(
                        r#"<div class="group flex items-start gap-1.5 pl-4 border-l border-slate-800/80 my-1 py-0.5">
                            <span class="text-slate-500 font-mono text-xs select-none">{i}:</span>
                            <div class="flex-1">{rendered_child}</div>
                            {copy_btn}
                        </div>"#
                    ));
                }

                format!(
                    r#"<div class="yav-json-collapsible inline-block w-full">
                        <button type="button" class="yav-json-toggle inline-flex items-center gap-1 font-mono text-xs text-slate-400 hover:text-slate-200">
                            <span class="yav-json-chevron text-[10px] transition-transform duration-150{chevron_rotate}">▶</span>
                            <span class="text-slate-500 select-none">Array</span>
                            <span class="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-800 text-slate-400">[{len}]</span>
                        </button>
                        <div class="yav-json-content{hidden_attr}">{children_html}</div>
                    </div>"#
                )
            }
            Value::Object(map) => {
                let len = map.len();
                if len == 0 {
                    return r#"<span class="text-slate-400 font-mono">{{}}</span>"#.to_string();
                }

                let mut children_html = String::new();
                for (key, item) in map {
                    let child_path = if path == "$" {
                        format!("$.{key}")
                    } else {
                        format!("{path}.{key}")
                    };
                    let rendered_child = self.render_value(item, &child_path, depth + 1);
                    let escaped_child_path = escape_html(&child_path);
                    let escaped_key = escape_html(key);
                    let copy_btn = if self.show_copy_path {
                        format!(
                            r##"<button type="button" class="yav-json-copy opacity-0 group-hover:opacity-100 px-1 text-[10px] text-slate-500 hover:text-indigo-400 transition-opacity" data-path="{escaped_child_path}" title="Copier le chemin">copy</button>"##
                        )
                    } else {
                        String::new()
                    };

                    children_html.push_str(&format!(
                        r#"<div class="group flex items-start gap-1.5 pl-4 border-l border-slate-800/80 my-1 py-0.5">
                            <span class="text-indigo-300 font-mono font-medium text-xs">"{escaped_key}":</span>
                            <div class="flex-1">{rendered_child}</div>
                            {copy_btn}
                        </div>"#
                    ));
                }

                format!(
                    r#"<div class="yav-json-collapsible inline-block w-full">
                        <button type="button" class="yav-json-toggle inline-flex items-center gap-1 font-mono text-xs text-slate-400 hover:text-slate-200">
                            <span class="yav-json-chevron text-[10px] transition-transform duration-150{chevron_rotate}">▶</span>
                            <span class="text-slate-500 select-none">Object</span>
                            <span class="px-1.5 py-0.2 rounded-full text-[10px] bg-slate-800 text-slate-400">{{{len}}}</span>
                        </button>
                        <div class="yav-json-content{hidden_attr}">{children_html}</div>
                    </div>"#
                )
            }
        }
    }
}

impl Component for JsonTreeViewer {
    fn render(&self) -> Node {
        let root_class = merge_classes(&[
            "yav-json-tree bg-slate-950 border border-slate-800 rounded-xl p-4 font-mono text-xs overflow-auto shadow-2xl select-text max-w-full",
            &self.class,
        ]);

        let tree_content = self.render_value(&self.data, "$", 0);

        let script = format!(
            r##"(function(){{
const root = document.getElementById('{id}');
if (!root) return;

root.addEventListener('click', (e) => {{
  const toggleBtn = e.target.closest('.yav-json-toggle');
  if (toggleBtn) {{
    const container = toggleBtn.closest('.yav-json-collapsible');
    if (!container) return;
    const content = container.querySelector(':scope > .yav-json-content');
    const chevron = toggleBtn.querySelector('.yav-json-chevron');
    if (!content) return;

    if (content.classList.contains('hidden')) {{
      content.classList.remove('hidden');
      if (chevron) chevron.classList.add('rotate-90');
    }} else {{
      content.classList.add('hidden');
      if (chevron) chevron.classList.remove('rotate-90');
    }}
  }}

  const copyBtn = e.target.closest('.yav-json-copy');
  if (copyBtn) {{
    const path = copyBtn.getAttribute('data-path');
    if (path) {{
      navigator.clipboard.writeText(path).then(() => {{
        copyBtn.textContent = '✓';
        setTimeout(() => {{ copyBtn.textContent = 'copy'; }}, 1500);
      }});
    }}
  }}
}});
}})();"##,
            id = escape_html(&self.id),
        );

        let container = Element::new("div")
            .attr("id", &self.id)
            .attr("class", root_class)
            .child(Node::Raw(tree_content))
            .child(Element::new("script").child(script));

        Node::Element(container)
    }
}

/// Helper function to create a new `JsonTreeViewer`.
#[must_use]
pub fn json_tree_viewer(id: impl Into<String>, data: Value) -> JsonTreeViewer {
    JsonTreeViewer::new(id, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use yav_core::render_to_string;

    #[test]
    fn test_json_tree_rendering() {
        let payload = json!({
            "user": {
                "id": 1042,
                "name": "Alice Vance",
                "is_active": true,
                "tags": ["admin", "staff"]
            }
        });

        let viewer = JsonTreeViewer::new("json-explorer", payload)
            .expanded_depth(3)
            .show_copy_path(true);

        let node = viewer.render();
        let html = render_to_string(&node);

        assert!(html.contains("yav-json-tree"));
        assert!(html.contains("json-explorer"));
        assert!(html.contains("Alice Vance"));
        assert!(html.contains("1042"));
        assert!(html.contains("true"));
        assert!(html.contains("$.user.name"));
        assert!(html.contains("$.user.tags[0]"));
    }
}
