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
