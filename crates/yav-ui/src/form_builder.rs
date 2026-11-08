//! Dynamic Form Generator from JSON Schema.
//!
//! Automatically inspects standard JSON Schemas or OpenAPI schemas and renders accessible,
//! responsive Tailwind form fields, inputs, selects, textareas, and submit buttons.

use serde_json::Value;
use yav_core::{Component, Element, Node};

/// Configurable dynamic form component derived from a JSON Schema.
#[derive(Debug, Clone)]
pub struct DynamicForm {
    pub schema: Value,
    pub action: String,
    pub method: String,
    pub submit_label: String,
    pub form_id: Option<String>,
}

impl DynamicForm {
    /// Create a new dynamic form from a JSON Schema object.
    pub fn new(schema: Value, action: impl Into<String>) -> Self {
        Self {
            schema,
            action: action.into(),
            method: "POST".to_string(),
            submit_label: "Enregistrer".to_string(),
            form_id: None,
        }
    }

    #[must_use]
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into();
        self
    }

    #[must_use]
    pub fn submit_label(mut self, label: impl Into<String>) -> Self {
        self.submit_label = label.into();
        self
    }

    #[must_use]
    pub fn form_id(mut self, id: impl Into<String>) -> Self {
        self.form_id = Some(id.into());
        self
    }
}

impl Component for DynamicForm {
    fn render(&self) -> Node {
        let mut form_el = Element::new("form")
            .attr("action", self.action.clone())
            .attr("method", self.method.clone())
            .attr("class", "yav-dynamic-form space-y-4 max-w-xl");

        if let Some(id) = &self.form_id {
            form_el = form_el.attr("id", id);
        }

        let required_fields: Vec<String> = self
            .schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(props) = self.schema.get("properties").and_then(|v| v.as_object()) {
            for (field_name, field_spec) in props {
                let is_required = required_fields.contains(field_name);
                let title = field_spec
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or(field_name);
                let description = field_spec.get("description").and_then(|v| v.as_str());
                let field_type = field_spec
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("string");

                let mut field_group = Element::new("div").attr("class", "space-y-1.5");

                // Label
                let mut label_el = Element::new("label")
                    .attr("for", field_name)
                    .attr("class", "block text-sm font-medium text-slate-200")
                    .child(Node::text(title.to_string()));

                if is_required {
                    label_el = label_el.child(
                        Element::new("span")
                            .attr("class", "text-rose-500 ml-1")
                            .child(Node::text("*")),
                    );
                }
                field_group = field_group.child(label_el);

                // Input control based on type and format
                if let Some(enum_vals) = field_spec.get("enum").and_then(|v| v.as_array()) {
                    let mut select_el = Element::new("select")
                        .attr("id", field_name)
                        .attr("name", field_name)
                        .attr(
                            "class",
                            "w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none",
                        );
                    if is_required {
                        select_el = select_el.bool_attr("required", true);
                    }
                    for ev in enum_vals {
                        if let Some(s) = ev.as_str() {
                            select_el = select_el.child(
                                Element::new("option").attr("value", s).child(Node::text(s)),
                            );
                        }
                    }
                    field_group = field_group.child(select_el);
                } else if field_type == "boolean" {
                    let checkbox_el = Element::new("div")
                        .attr("class", "flex items-center gap-2 pt-1")
                        .child(
                            Element::new("input")
                                .attr("type", "checkbox")
                                .attr("id", field_name)
                                .attr("name", field_name)
                                .attr(
                                    "class",
                                    "h-4 w-4 rounded border-slate-700 bg-slate-900 text-indigo-600 focus:ring-indigo-500",
                                ),
                        )
                        .child(
                            Element::new("span")
                                .attr("class", "text-xs text-slate-400")
                                .child(Node::text(description.unwrap_or(""))),
                        );
                    field_group = field_group.child(checkbox_el);
                } else if field_type == "integer" || field_type == "number" {
                    let mut num_inp = Element::new("input")
                        .attr("type", "number")
                        .attr("id", field_name)
                        .attr("name", field_name)
                        .attr(
                            "class",
                            "w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none",
                        );
                    if is_required {
                        num_inp = num_inp.bool_attr("required", true);
                    }
                    if let Some(min) = field_spec.get("minimum").and_then(|v| v.as_f64()) {
                        num_inp = num_inp.attr("min", min.to_string());
                    }
                    if let Some(max) = field_spec.get("maximum").and_then(|v| v.as_f64()) {
                        num_inp = num_inp.attr("max", max.to_string());
                    }
                    field_group = field_group.child(num_inp);
                } else {
                    let format = field_spec.get("format").and_then(|v| v.as_str());
                    let input_type = match format {
                        Some("email") => "email",
                        Some("date") => "date",
                        Some("password") => "password",
                        Some("uri") => "url",
                        _ => "text",
                    };
                    let mut text_inp = Element::new("input")
                        .attr("type", input_type)
                        .attr("id", field_name)
                        .attr("name", field_name)
                        .attr(
                            "class",
                            "w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-white focus:border-indigo-500 focus:outline-none",
                        );
                    if is_required {
                        text_inp = text_inp.bool_attr("required", true);
                    }
                    field_group = field_group.child(text_inp);
                }

                // Description help text
                if field_type != "boolean" {
                    if let Some(desc) = description {
                        field_group = field_group.child(
                            Element::new("p")
                                .attr("class", "text-xs text-slate-400")
                                .child(Node::text(desc)),
                        );
                    }
                }

                form_el = form_el.child(field_group);
            }
        }

        // Submit Button
        let submit_btn = Element::new("button")
            .attr("type", "submit")
            .attr(
                "class",
                "inline-flex items-center justify-center rounded-lg bg-indigo-600 px-4 py-2.5 text-sm font-semibold text-white shadow hover:bg-indigo-500 focus:outline-none focus:ring-2 focus:ring-indigo-500",
            )
            .child(Node::text(self.submit_label.clone()));

        form_el = form_el.child(submit_btn);

        Node::Element(form_el)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use yav_core::render_to_string;

    #[test]
    fn test_dynamic_form_generation() {
        let schema = json!({
            "type": "object",
            "required": ["email", "age"],
            "properties": {
                "email": {
                    "type": "string",
                    "format": "email",
                    "title": "Adresse Email"
                },
                "age": {
                    "type": "integer",
                    "minimum": 18,
                    "title": "Âge"
                },
                "role": {
                    "type": "string",
                    "enum": ["Admin", "User", "Guest"],
                    "title": "Rôle"
                },
                "subscribe": {
                    "type": "boolean",
                    "description": "Recevoir la newsletter"
                }
            }
        });

        let form = DynamicForm::new(schema, "/api/register").submit_label("Créer le compte");

        let html = render_to_string(&form.render());
        assert!(html.contains("action=\"/api/register\""));
        assert!(html.contains("type=\"email\""));
        assert!(html.contains("type=\"number\""));
        assert!(html.contains("<select"));
        assert!(html.contains("Admin"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("Créer le compte"));
    }
}
