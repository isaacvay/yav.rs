//! Enterprise DataGrid Component (`yav::ui::datagrid` — TanStack Table / AG Grid equivalent).
//!
//! Provides a rich, accessible, server-rendered data table with:
//! - Instant client-side search filtering (`yav:filter`)
//! - Server-driven or client-driven column sorting (`yav:get` + `yav:swap="morph"`)
//! - Optional row selection checkboxes
//! - Built-in pagination toolbar and empty-state placeholder

use crate::utils::merge_classes;
use yav_core::{Component, Element, IntoNode, Node};

/// Column definition for [`struct@DataGrid`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataGridColumn {
    pub key: String,
    pub header: String,
    pub sortable: bool,
}

impl DataGridColumn {
    /// Create a new column `(key, header)`.
    pub fn new(key: impl Into<String>, header: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            header: header.into(),
            sortable: true,
        }
    }

    /// Enable or disable sorting on this column.
    #[must_use]
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
}

/// Interactive server-rendered DataGrid with filtering, sorting, selection, and pagination.
#[derive(Debug, Clone, PartialEq)]
pub struct DataGrid {
    pub id: String,
    pub columns: Vec<DataGridColumn>,
    pub rows: Vec<Vec<String>>,
    pub searchable: bool,
    pub selectable: bool,
    pub sort_endpoint: Option<String>,
    pub page: usize,
    pub total_pages: usize,
    pub class: String,
}

impl Default for DataGrid {
    fn default() -> Self {
        Self {
            id: "yav-datagrid".to_string(),
            columns: Vec::new(),
            rows: Vec::new(),
            searchable: true,
            selectable: false,
            sort_endpoint: None,
            page: 1,
            total_pages: 1,
            class: String::new(),
        }
    }
}

impl DataGrid {
    pub fn new(columns: Vec<DataGridColumn>, rows: Vec<Vec<String>>) -> Self {
        Self {
            columns,
            rows,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn searchable(mut self, enabled: bool) -> Self {
        self.searchable = enabled;
        self
    }

    #[must_use]
    pub fn selectable(mut self, enabled: bool) -> Self {
        self.selectable = enabled;
        self
    }

    #[must_use]
    pub fn sort_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.sort_endpoint = Some(endpoint.into());
        self
    }

    #[must_use]
    pub fn pagination(mut self, page: usize, total_pages: usize) -> Self {
        self.page = page.max(1);
        self.total_pages = total_pages.max(1);
        self
    }

    #[must_use]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn render(self) -> Node {
        let body_id = format!("{}-tbody", self.id);
        let wrapper_cls = merge_classes(&[
            "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-xs",
            &self.class,
        ]);

        let mut root = Element::new("div")
            .attr("id", &self.id)
            .attr("class", wrapper_cls);

        // Search & filter toolbar
        if self.searchable {
            let filter_sel = format!("#{body_id} tr");
            root = root.child(Node::Element(
                Element::new("div")
                    .attr(
                        "class",
                        "flex items-center justify-between gap-4 px-4 py-3 border-b border-slate-200 dark:border-slate-800",
                    )
                    .child(Node::Element(
                        Element::new("input")
                            .attr("type", "search")
                            .attr("placeholder", "Filter rows...")
                            .attr("yav:filter", filter_sel)
                            .attr(
                                "class",
                                "w-full max-w-xs rounded-lg border border-slate-300 dark:border-slate-700 bg-transparent px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                            ),
                    ))
                    .child(Node::Element(
                        Element::new("span")
                            .attr("class", "text-xs text-slate-500")
                            .child(Node::Text(format!("{} rows", self.rows.len()))),
                    )),
            ));
        }

        // Table header
        let mut head_row = Element::new("tr").attr(
            "class",
            "border-b border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/50 text-left text-xs font-semibold uppercase tracking-wider text-slate-600 dark:text-slate-300",
        );

        if self.selectable {
            head_row = head_row.child(Node::Element(
                Element::new("th")
                    .attr("class", "w-10 px-4 py-3")
                    .child(Node::Element(
                        Element::new("input")
                            .attr("type", "checkbox")
                            .attr("aria-label", "Select all rows"),
                    )),
            ));
        }

        for col in &self.columns {
            let mut th = Element::new("th").attr("class", "px-4 py-3 select-none");
            if col.sortable {
                if let Some(ref ep) = self.sort_endpoint {
                    th = th
                        .attr("yav:get", format!("{ep}?sort={}", col.key))
                        .attr("yav:target", format!("#{}", self.id))
                        .attr("yav:swap", "morph")
                        .attr("class", "px-4 py-3 cursor-pointer hover:text-blue-600");
                }
                th = th.child(Node::Text(format!("{} ↕", col.header)));
            } else {
                th = th.child(Node::Text(col.header.clone()));
            }
            head_row = head_row.child(Node::Element(th));
        }

        // Table body
        let mut tbody = Element::new("tbody").attr("id", &body_id).attr(
            "class",
            "divide-y divide-slate-200 dark:divide-slate-800 text-sm",
        );

        if self.rows.is_empty() {
            let colspan = self.columns.len() + usize::from(self.selectable);
            tbody = tbody.child(Node::Element(
                Element::new("tr").child(Node::Element(
                    Element::new("td")
                        .attr("colspan", colspan.to_string())
                        .attr("class", "px-4 py-8 text-center text-slate-500")
                        .child(Node::Text("No matching records found.".to_string())),
                )),
            ));
        } else {
            for (r_idx, row) in self.rows.iter().enumerate() {
                let mut tr = Element::new("tr").attr(
                    "class",
                    "hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors",
                );
                if self.selectable {
                    tr = tr.child(Node::Element(
                        Element::new("td")
                            .attr("class", "px-4 py-3")
                            .child(Node::Element(
                                Element::new("input")
                                    .attr("type", "checkbox")
                                    .attr("name", "selected_row")
                                    .attr("value", r_idx.to_string()),
                            )),
                    ));
                }
                for cell in row {
                    tr = tr.child(Node::Element(
                        Element::new("td")
                            .attr("class", "px-4 py-3 text-slate-700 dark:text-slate-200")
                            .child(Node::Text(cell.clone())),
                    ));
                }
                tbody = tbody.child(Node::Element(tr));
            }
        }

        let table = Node::Element(
            Element::new("div")
                .attr("class", "overflow-x-auto")
                .child(Node::Element(
                    Element::new("table")
                        .attr("class", "w-full border-collapse")
                        .child(Node::Element(
                            Element::new("thead").child(Node::Element(head_row)),
                        ))
                        .child(Node::Element(tbody)),
                )),
        );

        root = root.child(table);

        // Pagination footer
        Node::Element(
            root.child(Node::Element(
                Element::new("div")
                    .attr(
                        "class",
                        "flex items-center justify-between px-4 py-3 border-t border-slate-200 dark:border-slate-800 text-xs text-slate-500",
                    )
                    .child(Node::Text(format!(
                        "Page {} of {}",
                        self.page, self.total_pages
                    ))),
            )),
        )
    }
}

impl IntoNode for DataGrid {
    fn into_node(self) -> Node {
        self.render()
    }
}

impl Component for DataGrid {
    fn render(&self) -> Node {
        self.clone().render()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DataGridProps {
    pub id: Option<String>,
    pub columns: Vec<DataGridColumn>,
    pub rows: Vec<Vec<String>>,
    pub searchable: Option<bool>,
    pub selectable: Option<bool>,
    pub sort_endpoint: Option<String>,
    pub page: Option<usize>,
    pub total_pages: Option<usize>,
    pub class: String,
}

#[allow(non_snake_case)]
pub fn DataGrid(props: DataGridProps) -> Node {
    let mut dg = DataGrid::new(props.columns, props.rows);
    if let Some(id) = props.id {
        dg = dg.id(id);
    }
    if let Some(s) = props.searchable {
        dg = dg.searchable(s);
    }
    if let Some(sel) = props.selectable {
        dg = dg.selectable(sel);
    }
    if let Some(ep) = props.sort_endpoint {
        dg = dg.sort_endpoint(ep);
    }
    dg = dg.pagination(props.page.unwrap_or(1), props.total_pages.unwrap_or(1));
    if !props.class.is_empty() {
        dg = dg.class(props.class);
    }
    dg.render()
}

pub fn data_grid(columns: Vec<DataGridColumn>, rows: Vec<Vec<String>>) -> DataGrid {
    DataGrid::new(columns, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_datagrid_renders_search_and_sortable_columns() {
        let cols = vec![
            DataGridColumn::new("name", "Name"),
            DataGridColumn::new("role", "Role"),
        ];
        let rows = vec![
            vec!["Alice".to_string(), "Admin".to_string()],
            vec!["Bob".to_string(), "Developer".to_string()],
        ];
        let node = data_grid(cols, rows)
            .selectable(true)
            .sort_endpoint("/api/users")
            .pagination(1, 5)
            .render();
        let html = render_to_string(&node);
        assert!(html.contains("yav:filter=\"#yav-datagrid-tbody tr\""));
        assert!(html.contains("yav:get=\"/api/users?sort=name\""));
        assert!(html.contains("Page 1 of 5"));
        assert!(html.contains("Alice"));
    }
}
