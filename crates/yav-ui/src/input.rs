//! Form control components: Input, Textarea, Select, Checkbox, and Radio.
//!
//! Provides accessible, styled input elements following standard HTML semantics
//! and ARIA accessibility guidelines.

use yav_core::{Component, Element, IntoNode, Node};

use crate::utils::merge_classes;
use crate::Size;

/// A text-based input component.
///
/// Supports standard input types (`text`, `email`, `password`, `number`, etc.)
/// and validation states.
///
/// # Examples
/// ```
/// use yav_ui::Input;
/// use yav_core::render_to_string;
///
/// let input = Input::text()
///     .name("email")
///     .placeholder("you@example.com")
///     .required(true)
///     .render();
///
/// let html = render_to_string(&input);
/// assert!(html.contains("name=\"email\""));
/// assert!(html.contains("placeholder=\"you@example.com\""));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    input_type: String,
    name: Option<String>,
    value: Option<String>,
    placeholder: Option<String>,
    id: Option<String>,
    disabled: bool,
    required: bool,
    readonly: bool,
    size: Size,
    invalid: bool,
    class: String,
    aria_label: Option<String>,
    aria_describedby: Option<String>,
    autocomplete: Option<String>,
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
    pattern: Option<String>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            input_type: "text".to_string(),
            name: None,
            value: None,
            placeholder: None,
            id: None,
            disabled: false,
            required: false,
            readonly: false,
            size: Size::Md,
            invalid: false,
            class: String::new(),
            aria_label: None,
            aria_describedby: None,
            autocomplete: None,
            min: None,
            max: None,
            step: None,
            pattern: None,
        }
    }
}

impl Input {
    /// Create a new input of default type `"text"`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a text input.
    pub fn text() -> Self {
        Self::new().input_type("text")
    }

    /// Create an email input.
    pub fn email() -> Self {
        Self::new().input_type("email")
    }

    /// Create a password input.
    pub fn password() -> Self {
        Self::new().input_type("password")
    }

    /// Create a number input.
    pub fn number() -> Self {
        Self::new().input_type("number")
    }

    /// Create a search input.
    pub fn search() -> Self {
        Self::new().input_type("search")
    }

    /// Set the input `type` attribute (e.g. `"text"`, `"email"`, `"tel"`).
    pub fn input_type(mut self, input_type: impl Into<String>) -> Self {
        self.input_type = input_type.into();
        self
    }

    /// Set the form `name` attribute.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the current input `value`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set whether the input is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the input is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set whether the input is read-only.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Set the sizing scale of the input.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Mark the input as invalid (adds error styling and `aria-invalid`).
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set the `aria-label` attribute.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the `aria-describedby` attribute.
    pub fn aria_describedby(mut self, id: impl Into<String>) -> Self {
        self.aria_describedby = Some(id.into());
        self
    }

    /// Set the `autocomplete` attribute.
    pub fn autocomplete(mut self, autocomplete: impl Into<String>) -> Self {
        self.autocomplete = Some(autocomplete.into());
        self
    }

    /// Set the `min` attribute.
    pub fn min(mut self, min: impl Into<String>) -> Self {
        self.min = Some(min.into());
        self
    }

    /// Set the `max` attribute.
    pub fn max(mut self, max: impl Into<String>) -> Self {
        self.max = Some(max.into());
        self
    }

    /// Set the `step` attribute.
    pub fn step(mut self, step: impl Into<String>) -> Self {
        self.step = Some(step.into());
        self
    }

    /// Set a regex `pattern` attribute.
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Render the input into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "block w-full rounded-md border text-sm shadow-sm placeholder-gray-400 focus:outline-none focus:ring-1 disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400";

        let state_class = if self.invalid {
            "border-red-500 text-red-900 focus:border-red-500 focus:ring-red-500 dark:border-red-500"
        } else {
            "border-gray-300 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600"
        };

        let size_class = match self.size {
            Size::Sm => "px-2.5 py-1.5 text-xs",
            Size::Md => "px-3 py-2 text-sm",
            Size::Lg => "px-4 py-2.5 text-base",
            Size::Xl => "px-5 py-3 text-lg",
        };

        let full_class = merge_classes(&[base_class, state_class, size_class, &self.class]);

        let mut el = Element::new("input")
            .attr("type", &self.input_type)
            .attr("class", full_class);

        if let Some(name) = &self.name {
            el = el.attr("name", name);
        }
        if let Some(value) = &self.value {
            el = el.attr("value", value);
        }
        if let Some(placeholder) = &self.placeholder {
            el = el.attr("placeholder", placeholder);
        }
        if let Some(id) = &self.id {
            el = el.attr("id", id);
        }
        if self.disabled {
            el = el.bool_attr("disabled", true);
        }
        if self.required {
            el = el.bool_attr("required", true);
        }
        if self.readonly {
            el = el.bool_attr("readonly", true);
        }
        if self.invalid {
            el = el.attr("aria-invalid", "true");
        }
        if let Some(aria_label) = &self.aria_label {
            el = el.attr("aria-label", aria_label);
        }
        if let Some(aria_describedby) = &self.aria_describedby {
            el = el.attr("aria-describedby", aria_describedby);
        }
        if let Some(autocomplete) = &self.autocomplete {
            el = el.attr("autocomplete", autocomplete);
        }
        if let Some(min) = &self.min {
            el = el.attr("min", min);
        }
        if let Some(max) = &self.max {
            el = el.attr("max", max);
        }
        if let Some(step) = &self.step {
            el = el.attr("step", step);
        }
        if let Some(pattern) = &self.pattern {
            el = el.attr("pattern", pattern);
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Input`].
pub fn input() -> Input {
    Input::new()
}

impl IntoNode for Input {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Input> for Node {
    fn from(i: Input) -> Self {
        i.render()
    }
}

impl Component for Input {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Multi-line text input component.
#[derive(Debug, Clone, PartialEq)]
pub struct Textarea {
    name: Option<String>,
    value: Option<String>,
    placeholder: Option<String>,
    id: Option<String>,
    rows: Option<u32>,
    cols: Option<u32>,
    disabled: bool,
    required: bool,
    readonly: bool,
    invalid: bool,
    class: String,
    aria_label: Option<String>,
    aria_describedby: Option<String>,
}

impl Default for Textarea {
    fn default() -> Self {
        Self {
            name: None,
            value: None,
            placeholder: None,
            id: None,
            rows: Some(3),
            cols: None,
            disabled: false,
            required: false,
            readonly: false,
            invalid: false,
            class: String::new(),
            aria_label: None,
            aria_describedby: None,
        }
    }
}

impl Textarea {
    /// Create a new textarea.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the form `name` attribute.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the initial text value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set the DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the number of visible text rows (default is 3).
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = Some(rows);
        self
    }

    /// Set the visible width in average character widths.
    pub fn cols(mut self, cols: u32) -> Self {
        self.cols = Some(cols);
        self
    }

    /// Set whether the textarea is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the textarea is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set whether the textarea is read-only.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Mark the textarea as invalid.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set `aria-label`.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set `aria-describedby`.
    pub fn aria_describedby(mut self, id: impl Into<String>) -> Self {
        self.aria_describedby = Some(id.into());
        self
    }

    /// Render the textarea into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "block w-full rounded-md border text-sm shadow-sm placeholder-gray-400 focus:outline-none focus:ring-1 disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 px-3 py-2";

        let state_class = if self.invalid {
            "border-red-500 text-red-900 focus:border-red-500 focus:ring-red-500 dark:border-red-500"
        } else {
            "border-gray-300 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600"
        };

        let full_class = merge_classes(&[base_class, state_class, &self.class]);

        let mut el = Element::new("textarea").attr("class", full_class);

        if let Some(name) = &self.name {
            el = el.attr("name", name);
        }
        if let Some(placeholder) = &self.placeholder {
            el = el.attr("placeholder", placeholder);
        }
        if let Some(id) = &self.id {
            el = el.attr("id", id);
        }
        if let Some(rows) = self.rows {
            el = el.attr("rows", rows.to_string());
        }
        if let Some(cols) = self.cols {
            el = el.attr("cols", cols.to_string());
        }
        if self.disabled {
            el = el.bool_attr("disabled", true);
        }
        if self.required {
            el = el.bool_attr("required", true);
        }
        if self.readonly {
            el = el.bool_attr("readonly", true);
        }
        if self.invalid {
            el = el.attr("aria-invalid", "true");
        }
        if let Some(aria_label) = &self.aria_label {
            el = el.attr("aria-label", aria_label);
        }
        if let Some(aria_describedby) = &self.aria_describedby {
            el = el.attr("aria-describedby", aria_describedby);
        }

        if let Some(val) = self.value {
            el = el.child(Node::text(val));
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Textarea`].
pub fn textarea() -> Textarea {
    Textarea::new()
}

impl IntoNode for Textarea {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Textarea> for Node {
    fn from(t: Textarea) -> Self {
        t.render()
    }
}

impl Component for Textarea {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An individual option in a [`Select`] dropdown.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    value: String,
    label: String,
    selected: bool,
    disabled: bool,
}

impl SelectOption {
    /// Create a new select option with value and label.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            selected: false,
            disabled: false,
        }
    }

    /// Mark this option as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Mark this option as disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Render into an `<option>` element [`Node`].
    pub fn render(self) -> Node {
        let mut el = Element::new("option")
            .attr("value", self.value)
            .child(Node::text(self.label));

        if self.selected {
            el = el.bool_attr("selected", true);
        }
        if self.disabled {
            el = el.bool_attr("disabled", true);
        }

        Node::Element(el)
    }
}

/// Dropdown selection component (`<select>`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Select {
    name: Option<String>,
    id: Option<String>,
    options: Vec<SelectOption>,
    placeholder: Option<String>,
    disabled: bool,
    required: bool,
    multiple: bool,
    size_attr: Option<u32>,
    class: String,
    aria_label: Option<String>,
}

impl Select {
    /// Create a new select component.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the form `name` attribute.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add an option to the select element.
    pub fn option(mut self, opt: SelectOption) -> Self {
        self.options.push(opt);
        self
    }

    /// Add multiple options to the select element.
    pub fn options(mut self, opts: impl IntoIterator<Item = SelectOption>) -> Self {
        self.options.extend(opts);
        self
    }

    /// Convenience method to add a basic option with value, label, and selected state.
    pub fn add_option(
        mut self,
        value: impl Into<String>,
        label: impl Into<String>,
        selected: bool,
    ) -> Self {
        self.options
            .push(SelectOption::new(value, label).selected(selected));
        self
    }

    /// Set placeholder text shown as an unselected first option.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set whether the select is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the select is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Allow multiple selections.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set visible row count (HTML `size` attribute).
    pub fn size_attr(mut self, size: u32) -> Self {
        self.size_attr = Some(size);
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Set `aria-label`.
    pub fn aria_label(mut self, label: impl Into<String>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let base_class =
            "block w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm shadow-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:bg-gray-50 dark:border-gray-600 dark:bg-gray-700 dark:text-white";
        let full_class = merge_classes(&[base_class, &self.class]);

        let mut el = Element::new("select").attr("class", full_class);

        if let Some(name) = &self.name {
            el = el.attr("name", name);
        }
        if let Some(id) = &self.id {
            el = el.attr("id", id);
        }
        if self.disabled {
            el = el.bool_attr("disabled", true);
        }
        if self.required {
            el = el.bool_attr("required", true);
        }
        if self.multiple {
            el = el.bool_attr("multiple", true);
        }
        if let Some(size) = self.size_attr {
            el = el.attr("size", size.to_string());
        }
        if let Some(aria_label) = &self.aria_label {
            el = el.attr("aria-label", aria_label);
        }

        // Render placeholder option if set
        if let Some(placeholder) = self.placeholder {
            let placeholder_opt = Element::new("option")
                .attr("value", "")
                .bool_attr("disabled", true)
                .bool_attr("selected", true)
                .child(Node::text(placeholder));
            el = el.child(Node::Element(placeholder_opt));
        }

        for opt in self.options {
            el = el.child(opt.render());
        }

        Node::Element(el)
    }
}

/// Convenience function to construct a new [`Select`].
pub fn select() -> Select {
    Select::new()
}

impl IntoNode for Select {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Select> for Node {
    fn from(s: Select) -> Self {
        s.render()
    }
}

impl Component for Select {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Checkbox input component with optional label and description.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checkbox {
    name: Option<String>,
    value: Option<String>,
    id: Option<String>,
    checked: bool,
    disabled: bool,
    required: bool,
    label: Option<String>,
    description: Option<String>,
    class: String,
}

impl Checkbox {
    /// Create a new checkbox.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the form `name` attribute.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the input `value`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set the DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set whether the checkbox is checked.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set whether the checkbox is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the checkbox is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set a visible label text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set optional descriptive helper text below the label.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add custom CSS classes to the input element.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let input_base =
            "h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-700";
        let input_class = merge_classes(&[input_base, &self.class]);

        let mut input_el = Element::new("input")
            .attr("type", "checkbox")
            .attr("class", input_class);

        if let Some(name) = &self.name {
            input_el = input_el.attr("name", name);
        }
        if let Some(value) = &self.value {
            input_el = input_el.attr("value", value);
        }
        if let Some(id) = &self.id {
            input_el = input_el.attr("id", id);
        }
        if self.checked {
            input_el = input_el.bool_attr("checked", true);
        }
        if self.disabled {
            input_el = input_el.bool_attr("disabled", true);
        }
        if self.required {
            input_el = input_el.bool_attr("required", true);
        }

        if let Some(label_text) = self.label {
            let mut label_el = Element::new("label")
                .attr("class", "inline-flex items-start gap-2 cursor-pointer")
                .child(Node::Element(input_el));

            let mut text_container =
                Element::new("span").attr("class", "text-sm text-gray-700 dark:text-gray-300");
            text_container = text_container.child(Node::text(label_text));

            if let Some(desc) = self.description {
                let desc_el = Element::new("p")
                    .attr("class", "text-xs text-gray-500 dark:text-gray-400 mt-0.5")
                    .child(Node::text(desc));
                text_container = text_container.child(Node::Element(desc_el));
            }

            label_el = label_el.child(Node::Element(text_container));
            Node::Element(label_el)
        } else {
            Node::Element(input_el)
        }
    }
}

/// Convenience function to construct a new [`Checkbox`].
pub fn checkbox() -> Checkbox {
    Checkbox::new()
}

impl IntoNode for Checkbox {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Checkbox> for Node {
    fn from(c: Checkbox) -> Self {
        c.render()
    }
}

impl Component for Checkbox {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Radio input component with optional label.
#[derive(Debug, Clone, PartialEq)]
pub struct Radio {
    name: String,
    value: String,
    id: Option<String>,
    checked: bool,
    disabled: bool,
    required: bool,
    label: Option<String>,
    class: String,
}

impl Radio {
    /// Create a new radio button with radio group name and option value.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            id: None,
            checked: false,
            disabled: false,
            required: false,
            label: None,
            class: String::new(),
        }
    }

    /// Set the DOM `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set whether this radio is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set whether this radio is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether this radio is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set a visible label text.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add custom CSS classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let input_base =
            "h-4 w-4 border-gray-300 text-blue-600 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-700";
        let input_class = merge_classes(&[input_base, &self.class]);

        let mut input_el = Element::new("input")
            .attr("type", "radio")
            .attr("name", &self.name)
            .attr("value", &self.value)
            .attr("class", input_class);

        if let Some(id) = &self.id {
            input_el = input_el.attr("id", id);
        }
        if self.checked {
            input_el = input_el.bool_attr("checked", true);
        }
        if self.disabled {
            input_el = input_el.bool_attr("disabled", true);
        }
        if self.required {
            input_el = input_el.bool_attr("required", true);
        }

        if let Some(label_text) = self.label {
            let label_el = Element::new("label")
                .attr("class", "inline-flex items-center gap-2 cursor-pointer")
                .child(Node::Element(input_el))
                .child(Node::Element(
                    Element::new("span")
                        .attr("class", "text-sm text-gray-700 dark:text-gray-300")
                        .child(Node::text(label_text)),
                ));
            Node::Element(label_el)
        } else {
            Node::Element(input_el)
        }
    }
}

/// Convenience function to construct a new [`Radio`].
pub fn radio(name: impl Into<String>, value: impl Into<String>) -> Radio {
    Radio::new(name, value)
}

impl IntoNode for Radio {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Radio> for Node {
    fn from(r: Radio) -> Self {
        r.render()
    }
}

impl Component for Radio {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// An accessible toggle switch component.
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    name: Option<String>,
    id: Option<String>,
    label: Option<String>,
    description: Option<String>,
    checked: bool,
    disabled: bool,
    size: Size,
    class: String,
}

impl Default for Switch {
    fn default() -> Self {
        Self {
            name: None,
            id: None,
            label: None,
            description: None,
            checked: false,
            disabled: false,
            size: Size::Md,
            class: String::new(),
        }
    }
}

impl Switch {
    /// Create a new switch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the input ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set an accompanying label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set an optional secondary description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set whether the switch is toggled on.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set whether the switch is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the switch size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Add custom classes to the container.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the switch into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let (track_size, thumb_size, thumb_translate) = match self.size {
            Size::Sm => ("h-4 w-7", "h-3 w-3", "peer-checked:translate-x-3"),
            Size::Md => ("h-6 w-11", "h-5 w-5", "peer-checked:translate-x-5"),
            Size::Lg | Size::Xl => ("h-7 w-14", "h-6 w-6", "peer-checked:translate-x-7"),
        };

        let mut label_el = Element::new("label").attr(
            "class",
            merge_classes(&[
                "inline-flex items-center cursor-pointer select-none",
                if self.disabled {
                    "opacity-50 cursor-not-allowed"
                } else {
                    ""
                },
                &self.class,
            ]),
        );

        // Hidden native checkbox input
        let mut input = Element::new("input")
            .attr("type", "checkbox")
            .attr("role", "switch")
            .attr("class", "sr-only peer");

        if let Some(name) = self.name {
            input = input.attr("name", name);
        }
        if let Some(id) = self.id {
            input = input.attr("id", id);
        }
        if self.checked {
            input = input.attr("checked", "checked");
        }
        if self.disabled {
            input = input.attr("disabled", "disabled");
        }
        label_el = label_el.child(Node::Element(input));

        // Switch Track
        let track_class = format!("relative inline-flex {track_size} shrink-0 rounded-full border-2 border-transparent bg-gray-200 transition-colors duration-200 ease-in-out peer-checked:bg-primary-600 peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-primary-600 dark:bg-gray-700");
        let thumb_class = format!("pointer-events-none inline-block {thumb_size} transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {thumb_translate}");

        let thumb = Element::new("span").attr("class", thumb_class);
        let track = Element::new("span")
            .attr("class", track_class)
            .child(Node::Element(thumb));
        label_el = label_el.child(Node::Element(track));

        // Label & Description
        if self.label.is_some() || self.description.is_some() {
            let mut text_wrap = Element::new("span").attr("class", "ml-3 flex flex-col");
            if let Some(lbl) = self.label {
                let lbl_el = Element::new("span")
                    .attr(
                        "class",
                        "text-sm font-medium text-gray-900 dark:text-gray-100",
                    )
                    .child(Node::text(lbl));
                text_wrap = text_wrap.child(Node::Element(lbl_el));
            }
            if let Some(desc) = self.description {
                let desc_el = Element::new("span")
                    .attr("class", "text-xs text-gray-500 dark:text-gray-400")
                    .child(Node::text(desc));
                text_wrap = text_wrap.child(Node::Element(desc_el));
            }
            label_el = label_el.child(Node::Element(text_wrap));
        }

        Node::Element(label_el)
    }
}

impl IntoNode for Switch {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Switch> for Node {
    fn from(s: Switch) -> Self {
        s.render()
    }
}

impl Component for Switch {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`Switch`].
pub fn switch() -> Switch {
    Switch::new()
}

/// A numeric range slider component.
#[derive(Debug, Clone, PartialEq)]
pub struct Slider {
    name: Option<String>,
    id: Option<String>,
    label: Option<String>,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    unit: Option<String>,
    disabled: bool,
    class: String,
}

impl Default for Slider {
    fn default() -> Self {
        Self {
            name: None,
            id: None,
            label: None,
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            unit: None,
            disabled: false,
            class: String::new(),
        }
    }
}

impl Slider {
    /// Create a new slider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the input name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the input ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the slider label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the slider value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Set the slider minimum bound.
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Set the slider maximum bound.
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Set the slider step increment.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set an optional unit prefix/suffix (e.g. `"€"`, `"km"`).
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set whether the slider is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the slider into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let mut wrapper =
            Element::new("div").attr("class", merge_classes(&["w-full space-y-1.5", &self.class]));

        // Header: label + value
        if self.label.is_some() || self.unit.is_some() {
            let mut header =
                Element::new("div").attr("class", "flex items-center justify-between text-sm");
            if let Some(lbl) = self.label {
                let l_el = Element::new("span")
                    .attr("class", "font-medium text-gray-700 dark:text-gray-300")
                    .child(Node::text(lbl));
                header = header.child(Node::Element(l_el));
            }
            let display_val = if let Some(ref u) = self.unit {
                format!("{}{}", self.value, u)
            } else {
                format!("{}", self.value)
            };
            let v_el = Element::new("span")
                .attr(
                    "class",
                    "font-semibold text-gray-900 dark:text-gray-100 font-mono",
                )
                .child(Node::text(display_val));
            header = header.child(Node::Element(v_el));
            wrapper = wrapper.child(Node::Element(header));
        }

        // Native range input
        let mut input = Element::new("input")
            .attr("type", "range")
            .attr("min", self.min.to_string())
            .attr("max", self.max.to_string())
            .attr("step", self.step.to_string())
            .attr("value", self.value.to_string())
            .attr(
                "class",
                "w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-primary-600 dark:bg-gray-700",
            );

        if let Some(name) = self.name {
            input = input.attr("name", name);
        }
        if let Some(id) = self.id {
            input = input.attr("id", id);
        }
        if self.disabled {
            input = input.attr("disabled", "disabled");
        }
        wrapper = wrapper.child(Node::Element(input));

        // Min and Max scale markers
        let scale = Element::new("div")
            .attr(
                "class",
                "flex justify-between text-xs text-gray-400 dark:text-gray-500 font-mono",
            )
            .child(Node::Element(
                Element::new("span").child(Node::text(self.min.to_string())),
            ))
            .child(Node::Element(
                Element::new("span").child(Node::text(self.max.to_string())),
            ));
        wrapper = wrapper.child(Node::Element(scale));

        Node::Element(wrapper)
    }
}

impl IntoNode for Slider {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<Slider> for Node {
    fn from(s: Slider) -> Self {
        s.render()
    }
}

impl Component for Slider {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`Slider`].
pub fn slider() -> Slider {
    Slider::new()
}

/// Dual min/max range slider component.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeSlider {
    min_name: String,
    max_name: String,
    label: Option<String>,
    min_value: f64,
    max_value: f64,
    min: f64,
    max: f64,
    step: f64,
    unit: Option<String>,
    class: String,
}

impl Default for RangeSlider {
    fn default() -> Self {
        Self {
            min_name: "min".to_string(),
            max_name: "max".to_string(),
            label: None,
            min_value: 0.0,
            max_value: 100.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            unit: None,
            class: String::new(),
        }
    }
}

impl RangeSlider {
    /// Create a new range slider with field names for minimum and maximum values.
    pub fn new(min_name: impl Into<String>, max_name: impl Into<String>) -> Self {
        Self {
            min_name: min_name.into(),
            max_name: max_name.into(),
            ..Default::default()
        }
    }

    /// Set an optional label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set current min and max values.
    pub fn values(mut self, min: f64, max: f64) -> Self {
        self.min_value = min;
        self.max_value = max;
        self
    }

    /// Set global slider bounds.
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set step increment.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set unit prefix/suffix.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Add custom classes.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    /// Render the range slider into a virtual DOM [`Node`].
    pub fn render(self) -> Node {
        let mut wrapper = Element::new("div")
            .attr("class", merge_classes(&["w-full space-y-2", &self.class]))
            .attr("data-range-slider", "true");

        // Header: label + readout
        if self.label.is_some() || self.unit.is_some() {
            let mut header =
                Element::new("div").attr("class", "flex items-center justify-between text-sm");
            if let Some(lbl) = self.label {
                let l_el = Element::new("span")
                    .attr("class", "font-medium text-gray-700 dark:text-gray-300")
                    .child(Node::text(lbl));
                header = header.child(Node::Element(l_el));
            }
            let u = self.unit.as_deref().unwrap_or("");
            let range_text = format!("{}{u} – {}{u}", self.min_value, self.max_value);
            let r_el = Element::new("span")
                .attr(
                    "class",
                    "font-semibold text-gray-900 dark:text-gray-100 font-mono",
                )
                .child(Node::text(range_text));
            header = header.child(Node::Element(r_el));
            wrapper = wrapper.child(Node::Element(header));
        }

        // Pair of number / slider inputs for forms
        let mut grid = Element::new("div").attr("class", "grid grid-cols-2 gap-3");

        let min_box = Element::new("div")
            .attr("class", "space-y-1")
            .child(
                Node::Element(
                    Element::new("label")
                        .attr("class", "text-xs text-gray-500 dark:text-gray-400")
                        .child(Node::text("Min")),
                ),
            )
            .child(
                Node::Element(
                    Element::new("input")
                        .attr("type", "number")
                        .attr("name", self.min_name)
                        .attr("min", self.min.to_string())
                        .attr("max", self.max.to_string())
                        .attr("step", self.step.to_string())
                        .attr("value", self.min_value.to_string())
                        .attr(
                            "class",
                            "w-full rounded-md border border-gray-300 px-3 py-1.5 text-sm dark:border-gray-700 dark:bg-gray-800 dark:text-white font-mono",
                        ),
                ),
            );

        let max_box = Element::new("div")
            .attr("class", "space-y-1")
            .child(
                Node::Element(
                    Element::new("label")
                        .attr("class", "text-xs text-gray-500 dark:text-gray-400")
                        .child(Node::text("Max")),
                ),
            )
            .child(
                Node::Element(
                    Element::new("input")
                        .attr("type", "number")
                        .attr("name", self.max_name)
                        .attr("min", self.min.to_string())
                        .attr("max", self.max.to_string())
                        .attr("step", self.step.to_string())
                        .attr("value", self.max_value.to_string())
                        .attr(
                            "class",
                            "w-full rounded-md border border-gray-300 px-3 py-1.5 text-sm dark:border-gray-700 dark:bg-gray-800 dark:text-white font-mono",
                        ),
                ),
            );

        grid = grid.child(Node::Element(min_box));
        grid = grid.child(Node::Element(max_box));
        wrapper = wrapper.child(Node::Element(grid));

        Node::Element(wrapper)
    }
}

impl IntoNode for RangeSlider {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl From<RangeSlider> for Node {
    fn from(r: RangeSlider) -> Self {
        r.render()
    }
}

impl Component for RangeSlider {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

/// Convenience function to construct a [`RangeSlider`].
pub fn range_slider(min_name: impl Into<String>, max_name: impl Into<String>) -> RangeSlider {
    RangeSlider::new(min_name, max_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn input_renders_with_type_name_placeholder() {
        let node = Input::text()
            .name("username")
            .placeholder("Enter username")
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<input"));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("name=\"username\""));
        assert!(html.contains("placeholder=\"Enter username\""));
        assert!(html.ends_with("/>"));
    }

    #[test]
    fn input_disabled_and_required() {
        let node = Input::email().disabled(true).required(true).render();
        let html = render_to_string(&node);
        assert!(html.contains("disabled"));
        assert!(html.contains("required"));
    }

    #[test]
    fn input_invalid_state_and_aria() {
        let node = Input::password().invalid(true).render();
        let html = render_to_string(&node);
        assert!(html.contains("aria-invalid=\"true\""));
        assert!(html.contains("border-red-500"));
    }

    #[test]
    fn textarea_renders_with_content() {
        let node = Textarea::new()
            .name("bio")
            .rows(5)
            .value("Hello Rust")
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<textarea"));
        assert!(html.contains("rows=\"5\""));
        assert!(html.contains("Hello Rust"));
        assert!(html.ends_with("</textarea>"));
    }

    #[test]
    fn select_renders_with_options() {
        let node = Select::new()
            .name("country")
            .placeholder("Choose...")
            .add_option("us", "United States", false)
            .add_option("ca", "Canada", true)
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<select"));
        assert!(html.contains("name=\"country\""));
        assert!(html.contains("Choose..."));
        assert!(html.contains("<option value=\"ca\" selected>Canada</option>"));
        assert!(html.ends_with("</select>"));
    }

    #[test]
    fn checkbox_renders_alone_and_with_label() {
        let single = Checkbox::new().name("opt").render();
        let html_single = render_to_string(&single);
        assert!(html_single.starts_with("<input type=\"checkbox\""));

        let labeled = Checkbox::new()
            .name("agree")
            .label("I agree to terms")
            .checked(true)
            .render();
        let html_labeled = render_to_string(&labeled);
        assert!(html_labeled.starts_with("<label"));
        assert!(html_labeled.contains("checked"));
        assert!(html_labeled.contains("I agree to terms"));
        assert!(html_labeled.ends_with("</label>"));
    }

    #[test]
    fn radio_renders_with_label() {
        let node = Radio::new("theme", "dark")
            .label("Dark mode")
            .checked(true)
            .render();
        let html = render_to_string(&node);
        assert!(html.starts_with("<label"));
        assert!(html.contains("type=\"radio\""));
        assert!(html.contains("name=\"theme\""));
        assert!(html.contains("value=\"dark\""));
        assert!(html.contains("checked"));
        assert!(html.contains("Dark mode"));
    }

    #[test]
    fn switch_renders_with_label_and_role() {
        let node = switch()
            .name("notifications")
            .label("Enable alerts")
            .description("Receive email notifications")
            .checked(true)
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("role=\"switch\""));
        assert!(html.contains("name=\"notifications\""));
        assert!(html.contains("checked=\"checked\""));
        assert!(html.contains("Enable alerts"));
        assert!(html.contains("Receive email notifications"));
    }

    #[test]
    fn slider_renders_with_bounds_and_value() {
        let node = slider()
            .name("price")
            .label("Max Price")
            .min(10000.0)
            .max(150000.0)
            .step(5000.0)
            .value(85000.0)
            .unit("€")
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("type=\"range\""));
        assert!(html.contains("min=\"10000\""));
        assert!(html.contains("max=\"150000\""));
        assert!(html.contains("step=\"5000\""));
        assert!(html.contains("value=\"85000\""));
        assert!(html.contains("Max Price"));
        assert!(html.contains("85000€"));
    }

    #[test]
    fn range_slider_renders_inputs() {
        let node = range_slider("min_price", "max_price")
            .label("Budget")
            .values(25000.0, 90000.0)
            .unit("$")
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("name=\"min_price\""));
        assert!(html.contains("name=\"max_price\""));
        assert!(html.contains("value=\"25000\""));
        assert!(html.contains("value=\"90000\""));
        assert!(html.contains("25000$ – 90000$"));
    }
}
