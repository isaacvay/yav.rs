# Yav Framework Extension for VS Code & Cursor

Official Visual Studio Code and Cursor extension for **Yav** (`.yav` & `.rsx` component files).

## Features

- **Syntax Highlighting**:
  - Full frontmatter support (`--- ... ---`) with ESM-style imports (`import { ... } from "..."`) and `props { ... }` blocks.
  - Rust-style frontmatter (`use ...`, `pub struct Props`).
  - Web template tags (`<div>`, `<Button>`, `<Card />`) with distinct colors for custom components vs HTML elements.
  - Dynamic expressions `{props.name}`, `{props.count}` and inline RSX conditionals.
- **Snippets**:
  - `ycomp`: Scaffold a modern web-style Yav component with frontmatter and props.
  - `ycomprust`: Scaffold a pure Rust Yav component.
  - `props`: Define a props block with default values.
  - `imp`: ESM-style import statement.
  - `yif`: Conditional RSX template block.
- **Language Configuration**:
  - Auto-closing tags, quotes, brackets, and block folding.

---

## Installation

### Method 1: Automatic via Yav CLI (Recommended for all platforms: Linux, macOS, Windows)

Simply run:
```bash
yav ext install
```
This automatically discovers all installed editors (VS Code, Cursor, Windsurf, VSCodium, VS Code Insiders) and installs the extension in one click!

To check extension status across editors:
```bash
yav ext status
```

---

### Method 2: Instant Symlink (for extension developers)

Link this folder directly into your extensions directory:

#### On Linux / macOS:
```bash
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/yav-vscode
# Or for Cursor:
ln -s "$(pwd)/editors/vscode" ~/.cursor/extensions/yav-vscode
```

#### On Windows (PowerShell):
```powershell
New-Item -ItemType SymbolicLink -Path "$HOME\.vscode\extensions\yav-vscode" -Target "$PWD\editors\vscode"
```

Restart VS Code or reload the window (`Ctrl+Shift+P` -> `Developer: Reload Window`), and any `.yav` or `.rsx` file will immediately benefit from syntax highlighting and snippets!

---

### Method 2: Package into a `.vsix` file

To distribute or install on any machine:

```bash
# 1. Install vsce if not installed
npm install -g @vscode/vsce

# 2. Package the extension
cd editors/vscode
vsce package

# 3. Install in VS Code
code --install-extension yav-vscode-0.1.0.vsix
```

---

## Example `.yav` Component

```yav
---
import { Button } from "yav/ui"

props {
    title: String,
    badge: String = "Active",
    count: u32 = 0,
}
---
<div class="p-6 bg-card rounded-xl shadow-sm border border-border">
    <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold">{props.title}</h2>
        <span class="px-2 py-1 text-xs font-medium bg-primary/10 text-primary rounded-full">
            {props.badge}
        </span>
    </div>
    <p class="mt-2 text-muted-foreground">Total: {props.count}</p>
    <div class="mt-4">
        <Button variant="primary">View Details</Button>
    </div>
</div>
```
