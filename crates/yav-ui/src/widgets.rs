//! Advanced Shadcn/UI Interactive Primitives (`DatePicker`, `Combobox`, `Sheet`, `InputOtp`, `FileDropzone`, `ResizablePanels`, `Kbd`).
//!
//! Completes full Shadcn/UI parity with accessible, zero-Node.js server-rendered widgets.

use crate::utils::merge_classes;
use yav_core::{Element, Node};

// ─────────────────────────────────────────────────────────────────────────────
// 1. DatePicker
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatePickerProps {
    pub name: String,
    pub value: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub label: Option<String>,
    pub required: bool,
    pub class: String,
}

/// Accessible DatePicker input component with optional min/max bounds.
#[allow(non_snake_case)]
pub fn DatePicker(props: DatePickerProps) -> Node {
    let cls = merge_classes(&[
        "flex flex-col gap-1.5 text-sm text-slate-700 dark:text-slate-200",
        &props.class,
    ]);
    let mut wrapper = Element::new("div").attr("class", cls);
    if let Some(lbl) = props.label {
        wrapper = wrapper.child(Node::Element(
            Element::new("label")
                .attr("for", &props.name)
                .attr("class", "font-medium")
                .child(Node::Text(lbl)),
        ));
    }

    let mut inp = Element::new("input")
        .attr("type", "date")
        .attr("id", &props.name)
        .attr("name", &props.name)
        .attr(
            "class",
            "rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
        );
    if let Some(v) = props.value {
        inp = inp.attr("value", v);
    }
    if let Some(min) = props.min {
        inp = inp.attr("min", min);
    }
    if let Some(max) = props.max {
        inp = inp.attr("max", max);
    }
    if props.required {
        inp = inp.bool_attr("required", true);
    }
    Node::Element(wrapper.child(Node::Element(inp)))
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Combobox (Autocomplete Searchable Select)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComboboxProps {
    pub name: String,
    pub placeholder: Option<String>,
    pub options: Vec<(String, String)>,
    pub value: Option<String>,
    pub class: String,
}

/// Accessible searchable Combobox backed by `<datalist>` and keyboard navigation.
#[allow(non_snake_case)]
pub fn Combobox(props: ComboboxProps) -> Node {
    let list_id = format!("{}-datalist", props.name);
    let cls = merge_classes(&["relative w-full", &props.class]);

    let mut inp = Element::new("input")
        .attr("type", "text")
        .attr("name", &props.name)
        .attr("list", &list_id)
        .attr("role", "combobox")
        .attr("aria-autocomplete", "list")
        .attr(
            "placeholder",
            props
                .placeholder
                .unwrap_or_else(|| "Select or type...".to_string()),
        )
        .attr(
            "class",
            "w-full rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
        );
    if let Some(v) = props.value {
        inp = inp.attr("value", v);
    }

    let mut datalist = Element::new("datalist").attr("id", &list_id);
    for (val, label) in props.options {
        datalist = datalist.child(Node::Element(
            Element::new("option")
                .attr("value", val)
                .child(Node::Text(label)),
        ));
    }

    Node::Element(
        Element::new("div")
            .attr("class", cls)
            .child(Node::Element(inp))
            .child(Node::Element(datalist)),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. InputOtp (Multi-Digit Verification Code Input)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputOtpProps {
    pub name: String,
    pub length: usize,
    pub class: String,
}

impl Default for InputOtpProps {
    fn default() -> Self {
        Self {
            name: "otp".to_string(),
            length: 6,
            class: String::new(),
        }
    }
}

/// Multi-slot One-Time Password (`InputOtp`) component for 2FA and email verification flows.
#[allow(non_snake_case)]
pub fn InputOtp(props: InputOtpProps) -> Node {
    let digits = props.length.clamp(4, 8);
    let cls = merge_classes(&["inline-flex items-center gap-2", &props.class]);
    let mut group = Element::new("div")
        .attr("class", cls)
        .attr("role", "group")
        .attr("aria-label", "One-time passcode");

    for idx in 0..digits {
        group = group.child(Node::Element(
            Element::new("input")
                .attr("type", "text")
                .attr("inputmode", "numeric")
                .attr("maxlength", "1")
                .attr("pattern", "[0-9]")
                .attr("name", format!("{}_{}", props.name, idx))
                .attr("aria-label", format!("Digit {}", idx + 1))
                .attr(
                    "class",
                    "w-10 h-12 text-center text-lg font-semibold rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 focus:outline-none focus:ring-2 focus:ring-blue-500",
                ),
        ));
    }
    Node::Element(group)
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. FileDropzone (Drag & Drop Multipart Upload Area)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDropzoneProps {
    pub name: String,
    pub accept: Option<String>,
    pub multiple: bool,
    pub description: Option<String>,
    pub class: String,
}

/// Drag-and-drop file upload zone styled for multipart forms.
#[allow(non_snake_case)]
pub fn FileDropzone(props: FileDropzoneProps) -> Node {
    let cls = merge_classes(&[
        "relative flex flex-col items-center justify-center rounded-xl border-2 border-dashed border-slate-300 dark:border-slate-700 p-6 text-center hover:border-blue-500 transition-colors cursor-pointer",
        &props.class,
    ]);

    let mut file_input = Element::new("input")
        .attr("type", "file")
        .attr("name", &props.name)
        .attr(
            "class",
            "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
        );
    if let Some(acc) = props.accept {
        file_input = file_input.attr("accept", acc);
    }
    if props.multiple {
        file_input = file_input.bool_attr("multiple", true);
    }

    Node::Element(
        Element::new("label")
            .attr("class", cls)
            .child(Node::Element(file_input))
            .child(Node::Element(
                Element::new("p")
                    .attr(
                        "class",
                        "text-sm font-medium text-slate-700 dark:text-slate-200",
                    )
                    .child(Node::Text("Click to upload or drag and drop".to_string())),
            ))
            .child(Node::Element(
                Element::new("p")
                    .attr("class", "mt-1 text-xs text-slate-500")
                    .child(Node::Text(props.description.unwrap_or_else(|| {
                        "PNG, JPG, WEBP or PDF up to 10MB".to_string()
                    }))),
            )),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Sheet (Slide-over Side Panel)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SheetProps {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub children: Vec<Node>,
    pub class: String,
}

/// Slide-over [`Sheet`] overlay panel (Shadcn Sheet equivalent).
#[allow(non_snake_case)]
pub fn Sheet(props: SheetProps) -> Node {
    let cls = merge_classes(&[
        "fixed inset-y-0 right-0 z-50 w-full max-w-md border-l border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-2xl",
        &props.class,
    ]);
    let mut panel = Element::new("aside")
        .attr("id", &props.id)
        .attr("role", "dialog")
        .attr("aria-modal", "true")
        .attr("aria-label", &props.title)
        .attr("class", cls)
        .child(Node::Element(
            Element::new("div")
                .attr("class", "flex items-center justify-between mb-4")
                .child(Node::Element(
                    Element::new("h3")
                        .attr("class", "text-lg font-semibold")
                        .child(Node::Text(props.title)),
                ))
                .child(Node::Element(
                    Element::new("button")
                        .attr("type", "button")
                        .attr("yav:toggle", format!("#{}", props.id))
                        .attr("aria-label", "Close panel")
                        .attr("class", "text-slate-400 hover:text-slate-600")
                        .child(Node::Text("✕".to_string())),
                )),
        ));

    if let Some(desc) = props.description {
        panel = panel.child(Node::Element(
            Element::new("p")
                .attr("class", "text-sm text-slate-500 mb-4")
                .child(Node::Text(desc)),
        ));
    }

    for child in props.children {
        panel = panel.child(child);
    }
    Node::Element(panel)
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. ResizablePanels & Kbd
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResizablePanelsProps {
    pub left: Option<Node>,
    pub right: Option<Node>,
    pub class: String,
}

/// Split-pane [`ResizablePanels`] container with a draggable divider bar.
#[allow(non_snake_case)]
pub fn ResizablePanels(props: ResizablePanelsProps) -> Node {
    let cls = merge_classes(&[
        "grid grid-cols-1 md:grid-cols-2 divide-y md:divide-y-0 md:divide-x divide-slate-200 dark:divide-slate-800 rounded-xl border border-slate-200 dark:border-slate-800 overflow-hidden",
        &props.class,
    ]);
    Node::Element(
        Element::new("div")
            .attr("class", cls)
            .child(Node::Element(
                Element::new("div")
                    .attr("class", "p-4 overflow-auto resize-x")
                    .child(props.left.unwrap_or(Node::Fragment(vec![]))),
            ))
            .child(Node::Element(
                Element::new("div")
                    .attr("class", "p-4 overflow-auto")
                    .child(props.right.unwrap_or(Node::Fragment(vec![]))),
            )),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KbdProps {
    pub keys: String,
    pub class: String,
}

/// Keyboard shortcut badge (`<Kbd keys="⌘K" />`).
#[allow(non_snake_case)]
pub fn Kbd(props: KbdProps) -> Node {
    let cls = merge_classes(&[
        "inline-flex items-center rounded border border-slate-300 dark:border-slate-700 bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 font-mono text-xs font-medium text-slate-700 dark:text-slate-300 shadow-2xs",
        &props.class,
    ]);
    Node::Element(
        Element::new("kbd")
            .attr("class", cls)
            .child(Node::Text(props.keys)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_shadcn_widgets_render() {
        let dp = DatePicker(DatePickerProps {
            name: "birth_date".to_string(),
            label: Some("Birth Date".to_string()),
            required: true,
            ..Default::default()
        });
        assert!(render_to_string(&dp).contains("type=\"date\""));

        let otp = InputOtp(InputOtpProps {
            name: "code".to_string(),
            length: 6,
            ..Default::default()
        });
        assert!(render_to_string(&otp).contains("name=\"code_5\""));

        let kbd = Kbd(KbdProps {
            keys: "Ctrl+K".to_string(),
            ..Default::default()
        });
        assert!(render_to_string(&kbd).contains("<kbd "));
    }
}
