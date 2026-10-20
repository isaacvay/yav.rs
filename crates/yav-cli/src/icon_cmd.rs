//! Icon catalog search and exploration command (`yav icon`).

use clap::Subcommand;
use yav_icons::{all_categories, all_icons, get_icon, icons_in_category, search_icons};

/// Icon subcommand options.
#[derive(Debug, Clone, Subcommand)]
pub enum IconAction {
    /// Search for icons by name or keyword
    Search {
        /// Search term (e.g. "arrow", "user", "file", "media")
        query: String,
    },
    /// List all available icons or filter by category
    List {
        /// Optional category filter (e.g. "navigation", "actions", "media", "ui")
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Inspect details and usage for a specific icon
    Info {
        /// Icon name (e.g. "home", "search", "check")
        name: String,
    },
}

/// Run icon action.
pub fn run(action: &IconAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        IconAction::Search { query } => {
            let results = search_icons(query);
            if results.is_empty() {
                println!("\x1b[33mNo icons found matching \"{query}\".\x1b[0m");
                println!("Try browsing all icons with: \x1b[36myav icon list\x1b[0m");
            } else {
                println!(
                    "\x1b[32m✔ Found {} icon(s) matching \"{query}\":\x1b[0m\n",
                    results.len()
                );
                for icon in results {
                    println!(
                        "  \x1b[1m{:<18}\x1b[0m \x1b[90m[{}]\x1b[0m",
                        icon.name, icon.category
                    );
                    println!(
                        "    \x1b[36mRSX:  <Icon name=\"{}\" size={{IconSize::Md}} />\x1b[0m",
                        icon.name
                    );
                    println!(
                        "    \x1b[90mRust: render_icon(\"{}\", IconSize::Md, None, None)\x1b[0m\n",
                        icon.name
                    );
                }
            }
        }
        IconAction::List { category } => {
            if let Some(cat) = category {
                let icons = icons_in_category(cat);
                if icons.is_empty() {
                    println!("\x1b[33mNo icons in category \"{cat}\".\x1b[0m");
                    println!("Available categories: {}", all_categories().join(", "));
                } else {
                    println!(
                        "\x1b[32m✔ {} icon(s) in category \"{cat}\":\x1b[0m\n",
                        icons.len()
                    );
                    for icon in icons {
                        println!(
                            "  • {:<18} \x1b[90m(category: {})\x1b[0m",
                            icon.name, icon.category
                        );
                    }
                }
            } else {
                let all = all_icons();
                println!(
                    "\x1b[32m✔ Total icons: {}\x1b[0m (Categories: {})\n",
                    all.len(),
                    all_categories().join(", ")
                );
                for cat in all_categories() {
                    let in_cat = icons_in_category(cat);
                    println!(
                        "\x1b[1;34m── {} ({}) ──\x1b[0m",
                        cat.to_uppercase(),
                        in_cat.len()
                    );
                    let names: Vec<&str> = in_cat.iter().map(|i| i.name).collect();
                    println!("  {}\n", names.join(", "));
                }
            }
        }
        IconAction::Info { name } => {
            if let Some(icon) = get_icon(name) {
                println!("\x1b[1;32mIcon: {}\x1b[0m", icon.name);
                println!("  Category: {}", icon.category);
                println!("\n\x1b[1mUsage in RSX:\x1b[0m");
                println!(
                    "  \x1b[36m<Icon name=\"{}\" class=\"text-indigo-600\" />\x1b[0m",
                    icon.name
                );
                println!("\n\x1b[1mUsage in Rust:\x1b[0m");
                println!("  \x1b[36mrender_icon(\"{}\", IconSize::Md, Some(\"text-indigo-600\"), None)\x1b[0m", icon.name);
            } else {
                eprintln!("\x1b[31m❌ Icon \"{name}\" not found in catalog.\x1b[0m");
                println!("Run \x1b[36myav icon search <keyword>\x1b[0m to find similar icons.");
            }
        }
    }
    Ok(())
}
