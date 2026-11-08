//! # YAV Drag-and-Drop Kanban Board (`<KanbanBoard />`)
//!
//! Multi-column drag-and-drop Kanban board with LexoRank fractional ordering
//! and keyboard accessibility (rivaling `@dnd-kit/core`, `@hello-pangea/dnd`,
//! and Linear's issue board).

use yav_core::{Component, Element, Node};

use crate::utils::merge_classes;

/// A single draggable card inside a [`KanbanColumn`].
#[derive(Debug, Clone)]
pub struct KanbanCard {
    pub id: String,
    pub title: String,
    pub tag: Option<String>,
    pub assignee: Option<String>,
    pub rank: String,
}

impl KanbanCard {
    pub fn new(id: impl Into<String>, title: impl Into<String>, rank: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            tag: None,
            assignee: None,
            rank: rank.into(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    pub fn assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }
}

/// A column on the [`KanbanBoard`].
#[derive(Debug, Clone)]
pub struct KanbanColumn {
    pub id: String,
    pub title: String,
    pub cards: Vec<KanbanCard>,
}

impl KanbanColumn {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            cards: Vec::new(),
        }
    }

    pub fn card(mut self, card: KanbanCard) -> Self {
        self.cards.push(card);
        self
    }

    pub fn cards(mut self, cards: Vec<KanbanCard>) -> Self {
        self.cards = cards;
        self
    }
}

/// Props for [`KanbanBoard`].
#[derive(Debug, Clone, Default)]
pub struct KanbanBoardProps {
    pub id: String,
    pub columns: Vec<KanbanColumn>,
    pub drop_url: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

/// Interactive multi-column Kanban board component with HTML5 drag-and-drop
/// and LexoRank `data-rank` attributes.
#[derive(Debug, Clone, Default)]
pub struct KanbanBoard {
    pub id: String,
    pub columns: Vec<KanbanColumn>,
    pub drop_url: Option<String>,
    pub class: String,
    pub children: Vec<Node>,
}

impl KanbanBoard {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    pub fn column(mut self, col: KanbanColumn) -> Self {
        self.columns.push(col);
        self
    }

    pub fn columns(mut self, cols: Vec<KanbanColumn>) -> Self {
        self.columns = cols;
        self
    }

    pub fn drop_url(mut self, url: impl Into<String>) -> Self {
        self.drop_url = Some(url.into());
        self
    }

    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }

    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }
}

impl KanbanBoard {
    pub fn from_props(props: KanbanBoardProps) -> Self {
        Self {
            id: props.id,
            columns: props.columns,
            drop_url: props.drop_url,
            class: props.class,
            children: props.children,
        }
    }
}

impl Component for KanbanBoard {
    fn render(&self) -> Node {
        let board_id = if self.id.is_empty() {
            "yav-kanban"
        } else {
            &self.id
        };
        let classes = merge_classes(&[
            "yav-kanban flex gap-4 overflow-x-auto pb-4 items-start",
            &self.class,
        ]);

        let mut board = Element::new("div")
            .attr("id", board_id)
            .attr("class", &classes)
            .attr("role", "region")
            .attr("aria-label", "Kanban Board");

        if let Some(ref url) = self.drop_url {
            board = board.attr("yav:drop-url", url);
        }

        for col in &self.columns {
            let header = Element::new("div")
                .attr("class", "flex items-center justify-between px-3 py-2 font-semibold text-sm text-gray-700 dark:text-gray-200")
                .child(Node::Element(
                    Element::new("span").child(Node::Text(col.title.clone())),
                ))
                .child(Node::Element(
                    Element::new("span")
                        .attr("class", "text-xs px-2 py-0.5 rounded-full bg-gray-200 dark:bg-gray-700")
                        .child(Node::Text(col.cards.len().to_string())),
                ));

            let mut list = Element::new("div")
                .attr(
                    "class",
                    "yav-kanban-column-list flex flex-col gap-2 min-h-[120px] p-2",
                )
                .attr("data-column-id", &col.id);

            for card in &col.cards {
                let mut card_el = Element::new("div")
                    .attr(
                        "class",
                        "yav-kanban-card group rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-3 shadow-sm hover:shadow cursor-grab active:cursor-grabbing",
                    )
                    .attr("draggable", "true")
                    .attr("tabindex", "0")
                    .attr("role", "listitem")
                    .attr("data-card-id", &card.id)
                    .attr("data-rank", &card.rank);

                card_el = card_el.child(Node::Element(
                    Element::new("div")
                        .attr(
                            "class",
                            "text-sm font-medium text-gray-900 dark:text-gray-100",
                        )
                        .child(Node::Text(card.title.clone())),
                ));

                if card.tag.is_some() || card.assignee.is_some() {
                    let mut meta_row = Element::new("div").attr(
                        "class",
                        "mt-2 flex items-center justify-between text-xs text-gray-500",
                    );
                    if let Some(ref tag) = card.tag {
                        meta_row = meta_row.child(Node::Element(
                            Element::new("span")
                                .attr("class", "px-1.5 py-0.5 rounded bg-indigo-50 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300")
                                .child(Node::Text(tag.clone())),
                        ));
                    }
                    if let Some(ref assignee) = card.assignee {
                        meta_row = meta_row.child(Node::Element(
                            Element::new("span")
                                .attr("class", "font-mono text-gray-600 dark:text-gray-400")
                                .child(Node::Text(assignee.clone())),
                        ));
                    }
                    card_el = card_el.child(Node::Element(meta_row));
                }

                list = list.child(Node::Element(card_el));
            }

            let col_el = Element::new("div")
                .attr(
                    "class",
                    "yav-kanban-column w-72 shrink-0 rounded-xl bg-gray-100/80 dark:bg-gray-900/60 border border-gray-200/70 dark:border-gray-800",
                )
                .attr("data-column", &col.id)
                .child(Node::Element(header))
                .child(Node::Element(list));

            board = board.child(Node::Element(col_el));
        }

        Node::Element(board)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_kanban_board_renders_columns_and_ranks() {
        let board = KanbanBoard::new("sprint-board")
            .drop_url("/api/kanban/move")
            .column(
                KanbanColumn::new("todo", "To Do").card(
                    KanbanCard::new("ISS-1", "Implement CRDT sync", "0|hzzzzz:")
                        .tag("core")
                        .assignee("@alice"),
                ),
            )
            .render();

        let html = render_to_string(&board);
        assert!(html.contains("sprint-board"));
        assert!(html.contains("yav:drop-url=\"/api/kanban/move\""));
        assert!(html.contains("data-rank=\"0|hzzzzz:\""));
        assert!(html.contains("draggable=\"true\""));
        assert!(html.contains("Implement CRDT sync"));
    }
}
