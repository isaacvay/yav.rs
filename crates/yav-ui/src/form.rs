//! Form components: Form, Label, and FormGroup.
//!
//! Provides accessible form layouts, HTTP method spoofing (PUT, DELETE, PATCH),
//! field labels with required indicators, and helper/error messages.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;

/// HTTP method for a [`Form`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormMethod {
    Get,
    #[default]
    Post,
    Put,
    Delete,
    Patch,
}

impl FormMethod {
    /// Returns the HTML-native form method attribute (`"get"` or `"post"`).
    pub fn html_method(&self) -> &'static str {
        match self {
            FormMethod::Get => "get",
            _ => "post",
        }
    }

    /// Returns the simulated HTTP method string if method overriding is needed.
    pub fn override_value(&self) -> Option<&'static str> {
        match self {
            FormMethod::Put => Some("PUT"),
            FormMethod::Delete => Some("DELETE"),
            FormMethod::Patch => Some("PATCH"),
            _ => None,
        }
    }
}

/// Typed CSRF token container for SSR context injection via [`yav_core::provide_context`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CsrfToken(pub String);

impl From<String> for CsrfToken {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for CsrfToken {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// A semantic HTML `<form>` component.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Form {
    action: Option<String>,
    method: FormMethod,
    enctype: Option<String>,
    csrf: Option<String>,
    novalidate: bool,
    id: Option<String>,
    children: Vec<Node>,
    class: String,
}

impl Form {
    /// Create a new form with default POST method.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set destination action URL.
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Set HTTP submission method.
    pub fn method(mut self, method: FormMethod) -> Self {
        self.method = method;
        self
    }

    /// Set an anti-CSRF token that will be automatically embedded as a hidden `_csrf` input.
    pub fn csrf(mut self, token: impl Into<String>) -> Self {
        self.csrf = Some(token.into());
        self
    }

    /// Set form encoding type (e.g. `"multipart/form-data"`).
    pub fn enctype(mut self, enctype: impl Into<String>) -> Self {
        self.enctype = Some(enctype.into());
        self
    }

    /// Convenience helper to set multipart file upload enctype.
    pub fn multipart(mut self) -> Self {
        self.enctype = Some("multipart/form-data".to_string());
        self
    }

    /// Set `novalidate` to disable browser native validation.
    pub fn novalidate(mut self, novalidate: bool) -> Self {
        self.novalidate = novalidate;
        self
    }

    /// Set DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a child node (e.g. [`FormGroup`] or [`crate::Button`]).
    pub fn child(mut self, node: impl IntoNode) -> Self {
        self.children.push(node.into_node());
        self
    }

    /// Add multiple children.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let mut el = Element::new("form").attr("method", self.method.html_method());

        if let Some(action) = &self.action {
            el = el.attr("action", action);
        }

        if let Some(enctype) = &self.enctype {
            el = el.attr("enctype", enctype);
        }

        if let Some(id) = &self.id {
            el = el.attr("id", id);
        }

        if self.novalidate {
            el = el.bool_attr("novalidate", true);
        }

        if !self.class.is_empty() {
            el = el.attr("class", &self.class);
        }

        // Method override input for PUT, DELETE, PATCH
        if let Some(override_method) = self.method.override_value() {
            let hidden_input = Element::new("input")
                .attr("type", "hidden")
                .attr("name", "_method")
                .attr("value", override_method);
            el = el.child(Node::Element(hidden_input));
        }

        // Anti-CSRF protection token hidden input (explicit or context-provided)
        let csrf_token = self
            .csrf
            .or_else(|| yav_core::use_context::<CsrfToken>().map(|t| t.0));
        if let Some(ref token) = csrf_token {
            let csrf_input = Element::new("input")
                .attr("type", "hidden")
                .attr("name", "_csrf")
                .attr("value", token);
            el = el.child(Node::Element(csrf_input));
        }

        for child in self.children {
            el = el.child(child);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Form`].
pub fn form() -> Form {
    Form::new()
}

impl IntoNode for Form {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Form> for Node {
    fn from(f: Form) -> Self {
        f.render()
    }
}

impl Component for Form {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Accessible field label with optional required indicator.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    text: String,
    for_id: Option<String>,
    required: bool,
    class: String,
}

impl Label {
    /// Create a new label with text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            for_id: None,
            required: false,
            class: String::new(),
        }
    }

    /// Link the label to a target input `id` via `for="..."`.
    pub fn for_id(mut self, for_id: impl Into<String>) -> Self {
        self.for_id = Some(for_id.into());
        self
    }

    /// Mark as required (renders red asterisk indicator).
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut el = Element::new("label").attr("class", full_class);

        if let Some(for_id) = &self.for_id {
            el = el.attr("for", for_id);
        }

        el = el.child(Node::text(self.text));

        if self.required {
            let req_indicator = Element::new("span")
                .attr("class", "text-red-500 ml-0.5")
                .attr("aria-hidden", "true")
                .child(Node::text("*"));
            el = el.child(Node::Element(req_indicator));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Label`].
pub fn label(text: impl Into<String>) -> Label {
    Label::new(text)
}

impl IntoNode for Label {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Label> for Node {
    fn from(l: Label) -> Self {
        l.render()
    }
}

impl Component for Label {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// A wrapper grouping a field [`Label`], control element, helper text, and validation error.
#[derive(Debug, Clone, PartialEq)]
pub struct FormGroup {
    label: Option<Label>,
    control: Node,
    help_text: Option<String>,
    error_text: Option<String>,
    id: Option<String>,
    class: String,
}

impl FormGroup {
    /// Create a new form group wrapping an input control node.
    pub fn new(control: impl IntoNode) -> Self {
        Self {
            label: None,
            control: control.into_node(),
            help_text: None,
            error_text: None,
            id: None,
            class: String::new(),
        }
    }

    /// Attach a configured [`Label`].
    pub fn label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Convenience helper to attach a label by text.
    pub fn label_text(mut self, text: impl Into<String>) -> Self {
        self.label = Some(Label::new(text));
        self
    }

    /// Set descriptive help text displayed below the control.
    pub fn help_text(mut self, help: impl Into<String>) -> Self {
        self.help_text = Some(help.into());
        self
    }

    /// Set an error validation message displayed below the control with `role="alert"`.
    pub fn error_text(mut self, error: impl Into<String>) -> Self {
        self.error_text = Some(error.into());
        self
    }

    /// Set DOM `id` on the group container.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class = "mb-4";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut el = Element::new("div").attr("class", full_class);

        if let Some(id) = &self.id {
            el = el.attr("id", id);
        }

        if let Some(label) = self.label {
            el = el.child(label.render());
        }

        el = el.child(self.control);

        if let Some(error) = self.error_text {
            let err_el = Element::new("p")
                .attr("role", "alert")
                .attr(
                    "class",
                    "mt-1 text-xs text-red-600 dark:text-red-400 font-medium",
                )
                .child(Node::text(error));
            el = el.child(Node::Element(err_el));
        } else if let Some(help) = self.help_text {
            let help_el = Element::new("p")
                .attr("class", "mt-1 text-xs text-gray-500 dark:text-gray-400")
                .child(Node::text(help));
            el = el.child(Node::Element(help_el));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`FormGroup`].
pub fn form_group(control: impl IntoNode) -> FormGroup {
    FormGroup::new(control)
}

impl IntoNode for FormGroup {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<FormGroup> for Node {
    fn from(g: FormGroup) -> Self {
        g.render()
    }
}

impl Component for FormGroup {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

// ── Functional Component Adapters for RSX / JSX ─────────────────────────────

/// Properties for constructing a `Form` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormProps {
    pub action: Option<String>,
    pub method: Option<String>,
    pub enctype: Option<String>,
    pub novalidate: bool,
    pub id: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Form`, enabling `<Form action="..." method="...">...</Form>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Form(props: FormProps) -> Node {
    let mut f = Form::new();
    if let Some(act) = props.action {
        f = f.action(act);
    }
    if let Some(m) = props.method {
        match m.to_lowercase().as_str() {
            "get" => f = f.method(FormMethod::Get),
            "post" => f = f.method(FormMethod::Post),
            "put" => f = f.method(FormMethod::Put),
            "delete" => f = f.method(FormMethod::Delete),
            "patch" => f = f.method(FormMethod::Patch),
            _ => {}
        }
    }
    if let Some(enc) = props.enctype {
        f = f.enctype(enc);
    }
    if props.novalidate {
        f = f.novalidate(true);
    }
    if let Some(id) = props.id {
        f = f.id(id);
    }
    if !props.class.is_empty() {
        f = f.class(props.class);
    }
    for child in props.children {
        f = f.child(child);
    }
    f.render()
}

/// Properties for constructing a `Label` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LabelProps {
    pub text: Option<String>,
    pub for_id: Option<String>,
    pub required: bool,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `Label`, enabling `<Label for_id="...">...</Label>` in `rsx!`.
#[allow(non_snake_case)]
pub fn Label(props: LabelProps) -> Node {
    let mut l = if let Some(t) = props.text {
        Label::new(t)
    } else {
        Label::new("")
    };
    if let Some(f) = props.for_id {
        l = l.for_id(f);
    }
    if props.required {
        l = l.required(true);
    }
    if !props.class.is_empty() {
        l = l.class(props.class);
    }
    let mut node = l.render();
    if !props.children.is_empty() {
        if let Node::Element(ref mut el) = node {
            for child in props.children {
                el.children.push(child);
            }
        }
    }
    node
}

/// Properties for constructing a `FormGroup` via `rsx!`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormGroupProps {
    pub help_text: Option<String>,
    pub error_text: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Functional component adapter for `FormGroup`, enabling `<FormGroup>...</FormGroup>` in `rsx!`.
#[allow(non_snake_case)]
pub fn FormGroup(props: FormGroupProps) -> Node {
    let mut g = FormGroup::new(Node::raw(""));
    if let Some(h) = props.help_text {
        g = g.help_text(h);
    }
    if let Some(e) = props.error_text {
        g = g.error_text(e);
    }
    if !props.class.is_empty() {
        g = g.class(props.class);
    }
    let mut node = g.render();
    if !props.children.is_empty() {
        if let Node::Element(ref mut el) = node {
            for child in props.children {
                el.children.push(child);
            }
        }
    }
    node
}

/// Combined form field with label, input control, and automatic error feedback.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormFieldProps {
    pub label: String,
    pub name: String,
    pub input_type: Option<String>,
    pub value: Option<String>,
    pub placeholder: Option<String>,
    pub error: Option<String>,
    pub help: Option<String>,
    pub required: bool,
    pub class: String,
}

/// High-level ergonomic form field component: `<FormField label="Email" name="email" required=true />`.
#[allow(non_snake_case)]
pub fn FormField(props: FormFieldProps) -> Node {
    let id = format!("field-{}", props.name);
    let mut inp = crate::input::Input::new()
        .name(&props.name)
        .id(&id)
        .input_type(props.input_type.as_deref().unwrap_or("text"));
    if let Some(ref val) = props.value {
        inp = inp.value(val);
    }
    if let Some(ref ph) = props.placeholder {
        inp = inp.placeholder(ph);
    }
    if props.required {
        inp = inp.required(true);
    }
    if let Some(ref _err) = props.error {
        inp = inp.invalid(true).aria_describedby(format!("{id}-error"));
    }

    let mut fg = FormGroup::new(inp.render()).label(
        Label::new(&props.label)
            .for_id(&id)
            .required(props.required),
    );
    if let Some(ref err) = props.error {
        fg = fg.error_text(err);
    }
    if let Some(ref help) = props.help {
        fg = fg.help_text(help);
    }
    if !props.class.is_empty() {
        fg = fg.class(props.class);
    }
    fg.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn form_renders_with_method_and_action() {
        let node = Form::new()
            .action("/login")
            .method(FormMethod::Post)
            .child(Node::text("Fields here"))
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<form"));
        assert!(html.contains("method=\"post\""));
        assert!(html.contains("action=\"/login\""));
        assert!(html.contains("Fields here"));
        assert!(html.ends_with("</form>"));
    }

    #[test]
    fn form_method_override_for_put_delete() {
        let put_form = Form::new().method(FormMethod::Put).render();
        let html_put = render_to_string(&put_form);
        assert!(html_put.contains("method=\"post\""));
        assert!(html_put.contains("type=\"hidden\""));
        assert!(html_put.contains("name=\"_method\""));
        assert!(html_put.contains("value=\"PUT\""));

        let del_form = Form::new().method(FormMethod::Delete).render();
        let html_del = render_to_string(&del_form);
        assert!(html_del.contains("value=\"DELETE\""));
    }

    #[test]
    fn form_csrf_hidden_input_rendering() {
        let form = Form::new()
            .action("/posts")
            .csrf("secure_csrf_token_12345")
            .render();
        let html = render_to_string(&form);
        assert!(html.contains("type=\"hidden\""));
        assert!(html.contains("name=\"_csrf\""));
        assert!(html.contains("value=\"secure_csrf_token_12345\""));
    }

    #[test]
    fn form_csrf_auto_context_injection() {
        let form_node =
            yav_core::provide_context(CsrfToken::from("auto_context_csrf_abc999"), || {
                let form = Form::new().action("/login").render();
                render_to_string(&form)
            });
        assert!(form_node.contains("name=\"_csrf\""));
        assert!(form_node.contains("value=\"auto_context_csrf_abc999\""));
    }

    #[test]
    fn label_renders_for_attribute_and_required_indicator() {
        let node = Label::new("Email Address")
            .for_id("email-input")
            .required(true)
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<label"));
        assert!(html.contains("for=\"email-input\""));
        assert!(html.contains("Email Address"));
        assert!(html.contains("*"));
        assert!(html.contains("text-red-500"));
        assert!(html.ends_with("</label>"));
    }

    #[test]
    fn form_group_renders_label_control_help_and_error() {
        let dummy_input = Node::Element(Element::new("input").attr("id", "username"));
        let node = FormGroup::new(dummy_input)
            .label(Label::new("Username").for_id("username"))
            .help_text("Pick a unique handle.")
            .error_text("Username is already taken.")
            .render();

        let html = render_to_string(&node);
        assert!(html.contains("for=\"username\""));
        assert!(html.contains("Username"));
        assert!(html.contains("<input id=\"username\""));
        assert!(html.contains("role=\"alert\""));
        assert!(html.contains("Username is already taken."));
    }
}
