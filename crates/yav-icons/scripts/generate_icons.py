#!/usr/bin/env python3
"""
Icon Catalog Generator for YAV Icons.

Extracts, cleans, and categorizes SVG icons from:
- Lucide (`classic`, `regular`)
- MynaUI (`minimal`, `regular` & `solid`)
- Phosphor (`system`, `regular`, `bold`, `fill`/`solid`, `duotone`, `light`, `thin`)

Generates clean, pre-sorted, modular Rust source files in `crates/yav-icons/src/catalog/`.
"""

import os
import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor

COMMON_ALIASES = {
    'home': 'house',
    'house': 'home',
    'alert-circle': 'circle-alert',
    'circle-alert': 'alert-circle',
    'alert-triangle': 'triangle-alert',
    'triangle-alert': 'alert-triangle',
    'help-circle': 'circle-question-mark',
    'circle-question-mark': 'help-circle',
    'check-circle': 'circle-check',
    'circle-check': 'check-circle',
    'x-circle': 'circle-x',
    'circle-x': 'x-circle',
    'close': 'x',
    'x': 'close',
}

def clean_svg(content: str) -> str:
    """Extract inner SVG elements and clean redundant attributes."""
    m = re.search(r'<svg[^>]*>(.*?)</svg>', content, re.DOTALL)
    if not m:
        return ''
    inner = m.group(1).strip()
    # Remove Phosphor transparent bounding rectangle
    inner = re.sub(r'<rect\s+width="256"\s+height="256"\s+fill="none"\/?>', '', inner).strip()
    # Normalize whitespaces
    inner = re.sub(r'\s+', ' ', inner)
    return inner

def escape_rust_str(s: str) -> str:
    """Escape backslashes and double quotes for Rust string literals."""
    return s.replace('\\', '\\\\').replace('"', '\\"')

def process_file(args):
    filepath, name, family, style, view_box, stroke_width, is_fill, category = args
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            raw = f.read()
        cleaned = clean_svg(raw)
        return {
            'name': name,
            'family': family,
            'style': style,
            'svg_content': cleaned,
            'category': category,
            'view_box': view_box,
            'stroke_width': stroke_width,
            'is_fill': is_fill,
        }
    except Exception as e:
        print(f"Error processing {filepath}: {e}", file=sys.stderr)
        return None

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
    icons_dir = os.path.join(repo_root, 'crates', 'yav-icons')
    output_dir = os.path.join(icons_dir, 'src', 'catalog')
    os.makedirs(output_dir, exist_ok=True)

    print(f"[yav-icons] Generating icon catalogs into: {output_dir}")
    t0 = time.time()

    configs = [
        # (family, style, rel_folder, view_box, stroke_width, is_fill, pattern, suffix)
        ('Classic', 'Regular', 'lucide/icons', '0 0 24 24', '2', False, r'\.svg$', r'\.svg$'),
        ('Minimal', 'Regular', 'mynaui-icons-main/icons', '0 0 24 24', '1.5', False, r'\.svg$', r'\.svg$'),
        ('Minimal', 'Solid', 'mynaui-icons-main/icons-solid', '0 0 24 24', '', True, r'\.svg$', r'\.svg$'),
        ('System', 'Regular', 'phosphor-icons/SVGs/regular', '0 0 256 256', '16', False, r'\.svg$', r'\.svg$'),
        ('System', 'Bold', 'phosphor-icons/SVGs/bold', '0 0 256 256', '24', False, r'-bold\.svg$', r'-bold\.svg$'),
        ('System', 'Solid', 'phosphor-icons/SVGs/fill', '0 0 256 256', '', True, r'-fill\.svg$', r'-fill\.svg$'),
        ('System', 'Duotone', 'phosphor-icons/SVGs/duotone', '0 0 256 256', '16', False, r'-duotone\.svg$', r'-duotone\.svg$'),
        ('System', 'Light', 'phosphor-icons/SVGs/light', '0 0 256 256', '12', False, r'-light\.svg$', r'-light\.svg$'),
        ('System', 'Thin', 'phosphor-icons/SVGs/thin', '0 0 256 256', '8', False, r'-thin\.svg$', r'-thin\.svg$'),
    ]

    all_tasks = []

    for fam, sty, rel_folder, vb, sw, is_fill, pattern, suffix in configs:
        folder = os.path.join(icons_dir, rel_folder)
        if not os.path.exists(folder):
            print(f"[!] Warning: Folder {folder} not found, skipping...")
            continue

        for fname in sorted(os.listdir(folder)):
            if re.search(pattern, fname):
                name = re.sub(suffix, '', fname)
                filepath = os.path.join(folder, fname)
                category = "general"
                # If Lucide, check for json file category
                if fam == 'Classic':
                    json_path = os.path.join(folder, f"{name}.json")
                    if os.path.exists(json_path):
                        try:
                            import json
                            with open(json_path, 'r', encoding='utf-8') as jf:
                                jdata = json.load(jf)
                                cats = jdata.get('categories', [])
                                if cats:
                                    category = cats[0]
                        except Exception:
                            pass

                all_tasks.append((filepath, name, fam, sty, vb, sw, is_fill, category))

    print(f"[yav-icons] Parsing {len(all_tasks)} SVG icons with thread pool...")
    with ThreadPoolExecutor(max_workers=8) as executor:
        results = list(executor.map(process_file, all_tasks))

    # Group results by (family, style)
    groups = {}
    for item in results:
        if not item:
            continue
        key = (item['family'], item['style'])
        if key not in groups:
            groups[key] = []
        groups[key].append(item)

    # Inject common aliases in each group if alias target exists but alias name doesn't
    for key, items in groups.items():
        existing_names = {x['name'] for x in items}
        aliases_to_add = []
        for alias_name, target_name in COMMON_ALIASES.items():
            if alias_name not in existing_names and target_name in existing_names:
                target_item = next(x for x in items if x['name'] == target_name)
                alias_item = dict(target_item)
                alias_item['name'] = alias_name
                aliases_to_add.append(alias_item)
        items.extend(aliases_to_add)

    # Sort each group alphabetically by name for O(log N) binary search lookup
    for key in groups:
        groups[key].sort(key=lambda x: x['name'])
        print(f"  - {key[0]} {key[1]}: {len(groups[key])} icons")

    # Generate catalog/classic.rs
    classic_path = os.path.join(output_dir, 'classic.rs')
    with open(classic_path, 'w', encoding='utf-8') as f:
        f.write("//! Classic icon collection (derived from Lucide).\n\n")
        f.write("use super::{IconData, IconFamily, IconStyle};\n\n")
        write_icon_slice(f, "CLASSIC_REGULAR", groups.get(('Classic', 'Regular'), []))

    # Generate catalog/minimal.rs
    minimal_path = os.path.join(output_dir, 'minimal.rs')
    with open(minimal_path, 'w', encoding='utf-8') as f:
        f.write("//! Minimal icon collection (derived from MynaUI).\n\n")
        f.write("use super::{IconData, IconFamily, IconStyle};\n\n")
        write_icon_slice(f, "MINIMAL_REGULAR", groups.get(('Minimal', 'Regular'), []))
        write_icon_slice(f, "MINIMAL_SOLID", groups.get(('Minimal', 'Solid'), []))

    # Generate catalog/system.rs
    system_path = os.path.join(output_dir, 'system.rs')
    with open(system_path, 'w', encoding='utf-8') as f:
        f.write("//! System icon collection (derived from Phosphor).\n\n")
        f.write("use super::{IconData, IconFamily, IconStyle};\n\n")
        write_icon_slice(f, "SYSTEM_REGULAR", groups.get(('System', 'Regular'), []))
        write_icon_slice(f, "SYSTEM_BOLD", groups.get(('System', 'Bold'), []))
        write_icon_slice(f, "SYSTEM_SOLID", groups.get(('System', 'Solid'), []))
        write_icon_slice(f, "SYSTEM_DUOTONE", groups.get(('System', 'Duotone'), []))
        write_icon_slice(f, "SYSTEM_LIGHT", groups.get(('System', 'Light'), []))
        write_icon_slice(f, "SYSTEM_THIN", groups.get(('System', 'Thin'), []))

    total_icons = sum(len(g) for g in groups.values())
    print(f"[yav-icons] Successfully generated {total_icons} icons in {time.time() - t0:.2f}s!")

def write_icon_slice(f, const_name, icon_list):
    f.write(f"/// Curated {const_name} icons (sorted by name).\n")
    f.write(f"pub static {const_name}: &[IconData] = &[\n")
    for icon in icon_list:
        esc_svg = escape_rust_str(icon['svg_content'])
        is_fill_str = "true" if icon['is_fill'] else "false"
        f.write(f'    IconData {{\n')
        f.write(f'        name: "{icon["name"]}",\n')
        f.write(f'        family: IconFamily::{icon["family"]},\n')
        f.write(f'        style: IconStyle::{icon["style"]},\n')
        f.write(f'        svg_content: "{esc_svg}",\n')
        f.write(f'        category: "{icon["category"]}",\n')
        f.write(f'        view_box: "{icon["view_box"]}",\n')
        f.write(f'        stroke_width: "{icon["stroke_width"]}",\n')
        f.write(f'        is_fill: {is_fill_str},\n')
        f.write(f'    }},\n')
    f.write("];\n\n")

if __name__ == '__main__':
    main()
