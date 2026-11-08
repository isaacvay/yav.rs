//! Component generator command (`yav add <component>`).
//!
//! Adds production-ready, styled UI blocks directly to the developer's project
//! in `src/components/`, allowing full customization (similar to shadcn/ui).

use std::fs;
use std::path::Path;

/// Available template components that can be added.
pub const AVAILABLE_TEMPLATES: &[(&str, &str, &str)] = &[
    (
        "auth-card",
        "auth_card.rs",
        include_str!("../templates/AuthCard.rs.txt"),
    ),
    (
        "stat-grid",
        "stat_grid.rs",
        include_str!("../templates/StatGrid.rs.txt"),
    ),
    (
        "data-table",
        "data_table.rs",
        include_str!("../templates/DataTable.rs.txt"),
    ),
    (
        "pricing-table",
        "pricing_table.rs",
        include_str!("../templates/PricingTable.rs.txt"),
    ),
    (
        "navbar",
        "navbar.rs",
        include_str!("../templates/Navbar.rs.txt"),
    ),
    (
        "drawer",
        "drawer.rs",
        include_str!("../templates/Drawer.rs.txt"),
    ),
    (
        "command-menu",
        "command_menu.rs",
        include_str!("../templates/CommandMenu.rs.txt"),
    ),
    (
        "stat-card",
        "stat_card.rs",
        include_str!("../templates/StatCard.rs.txt"),
    ),
    (
        "carousel",
        "carousel.rs",
        include_str!("../templates/Carousel.rs.txt"),
    ),
    (
        "theme-selector",
        "theme_selector.rs",
        include_str!("../templates/ThemeSelector.rs.txt"),
    ),
    (
        "motion",
        "motion.rs",
        include_str!("../templates/Motion.rs.txt"),
    ),
    (
        "cursor-spotlight",
        "cursor_spotlight.rs",
        include_str!("../templates/CursorSpotlight.rs.txt"),
    ),
    (
        "charts",
        "charts_panel.rs",
        "use yav::prelude::*;\n\npub fn ChartsPanel() -> Node {\n    let pts = vec![ChartPoint::new(\"Q1\", 140.0), ChartPoint::new(\"Q2\", 230.0), ChartPoint::new(\"Q3\", 310.0)];\n    line_chart(pts).title(\"Quarterly Growth\").render()\n}\n",
    ),
    (
        "data-grid",
        "data_grid_view.rs",
        "use yav::prelude::*;\n\npub fn DataGridView() -> Node {\n    let cols = vec![DataGridColumn::new(\"id\", \"ID\"), DataGridColumn::new(\"email\", \"Email\")];\n    let rows = vec![vec![\"1\".into(), \"alice@yav.rs\".into()]];\n    data_grid(cols, rows).searchable(true).selectable(true).render()\n}\n",
    ),
    (
        "date-picker",
        "date_picker_field.rs",
        "use yav::prelude::*;\n\npub fn DatePickerField() -> Node {\n    DatePicker(DatePickerProps { name: \"scheduled_at\".into(), label: Some(\"Schedule Date\".into()), required: true, ..Default::default() })\n}\n",
    ),
    (
        "input-otp",
        "otp_verification.rs",
        "use yav::prelude::*;\n\npub fn OtpVerification() -> Node {\n    InputOtp(InputOtpProps { name: \"totp_code\".into(), length: 6, ..Default::default() })\n}\n",
    ),
    (
        "kanban-board",
        "kanban_board_view.rs",
        "use yav::prelude::*;\n\npub fn KanbanBoardView() -> Node {\n    KanbanBoard::new(\"sprint\").column(KanbanColumn::new(\"todo\", \"To Do\").card(KanbanCard::new(\"1\", \"Ship Yav.rs\", \"B\"))).render()\n}\n",
    ),
    (
        "flow-canvas",
        "flow_canvas_view.rs",
        "use yav::prelude::*;\n\npub fn FlowCanvasView() -> Node {\n    FlowCanvas::new(\"workflow\").node(FlowNode::new(\"a\", \"Start\", 40.0, 80.0)).node(FlowNode::new(\"b\", \"End\", 260.0, 80.0)).edge(FlowEdge::new(\"a\", \"b\").animated(true)).render()\n}\n",
    ),
    (
        "virtual-list",
        "virtual_list_view.rs",
        "use yav::prelude::*;\n\npub fn VirtualListView() -> Node {\n    VirtualList::new(100_000, 36).viewport_height_px(480).render()\n}\n",
    ),
    (
        "rich-editor",
        "rich_editor_view.rs",
        "use yav::prelude::*;\n\npub fn RichEditorView() -> Node {\n    RichTextEditor::new(\"content\").initial_html(\"<p>Start writing...</p>\").render()\n}\n",
    ),
    (
        "ai-chat",
        "ai_chat_view.rs",
        "use yav::prelude::*;\n\npub fn AiChatView() -> Node {\n    AiChat::new(\"/api/ai/chat\").message(AiChatBubble::assistant(\"How can I help?\")).render()\n}\n",
    ),
];

/// Executes the `yav add <name>` command.
pub fn run(component_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let name_lower = component_name.to_lowercase();

    let matched = AVAILABLE_TEMPLATES
        .iter()
        .find(|(name, _, _)| *name == name_lower);

    let (key, filename, content) = match matched {
        Some(t) => *t,
        None => {
            eprintln!(
                "\x1b[31m❌ Unknown component template: '{}'\x1b[0m",
                component_name
            );
            println!("\x1b[36mAvailable component templates:\x1b[0m");
            for (k, f, _) in AVAILABLE_TEMPLATES {
                println!("  - \x1b[32m{}\x1b[0m (creates src/components/{})", k, f);
            }
            return Err("Unknown template".into());
        }
    };

    let target_dir = Path::new("src").join("components");
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    let target_file = target_dir.join(filename);
    if target_file.exists() {
        println!(
            "\x1b[33m⚠️ Warning: {} already exists. Overwrite? (Skipping by default)\x1b[0m",
            target_file.display()
        );
        return Ok(());
    }

    fs::write(&target_file, content)?;

    // Check if src/components/mod.rs exists, and append declaration
    let mod_file = target_dir.join("mod.rs");
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let mod_name = stem.to_lowercase();
    let mod_decl = format!("pub mod {};\npub use {}::*;\n", mod_name, mod_name);

    if mod_file.exists() {
        let existing = fs::read_to_string(&mod_file)?;
        if !existing.contains(&format!("pub mod {}", mod_name)) {
            let mut updated = existing;
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&mod_decl);
            fs::write(&mod_file, updated)?;
        }
    } else {
        fs::write(&mod_file, format!("//! Components module\n\n{}", mod_decl))?;
    }

    println!(
        "\x1b[1;32m✔ Added component '{}' at {}\x1b[0m",
        key,
        target_file.display()
    );
    println!("\x1b[90m  Usage: use crate::components::{};\x1b[0m", stem);

    Ok(())
}
