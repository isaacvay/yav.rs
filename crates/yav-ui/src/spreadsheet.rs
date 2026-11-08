//! Reactive Spreadsheet Component with Formula Evaluator (`=SUM(A1:B4)`).
//!
//! Provides Handsontable / AG Grid Spreadsheet parity in pure Rust:
//! - Server-side formula engine evaluates `=SUM(A1:A4)`, `=AVG(B1:B3)`, `=MIN(...)`,
//!   `=MAX(...)`, and binary expressions (`=A1+B1`, `=A1*B2`) before HTML emission.
//! - Renders column letters (`A`, `B`, `C`...) and row numbers (`1`, `2`, `3`...)
//!   with `data-cell="A1"` and `data-formula="=SUM(A1:A2)"` attributes for live client
//!   editing.

use std::collections::HashMap;
use yav_core::{Component, Element, IntoNode, Node};

/// Interactive spreadsheet grid with formula evaluation.
#[derive(Debug, Clone)]
pub struct Spreadsheet {
    pub id: String,
    pub cols: usize,
    pub rows: usize,
    cells: HashMap<String, String>,
}

impl Spreadsheet {
    /// Create a new spreadsheet with `cols` columns (`A..`) and `rows` rows (`1..`).
    #[must_use]
    pub fn new(id: impl Into<String>, cols: usize, rows: usize) -> Self {
        Self {
            id: id.into(),
            cols: cols.clamp(1, 26),
            rows: rows.clamp(1, 500),
            cells: HashMap::new(),
        }
    }

    /// Set the raw value or formula of a cell (e.g. `("A1", "120")` or `("A3", "=SUM(A1:A2)")`).
    #[must_use]
    pub fn set_cell(mut self, coord: impl Into<String>, value: impl Into<String>) -> Self {
        self.cells
            .insert(coord.into().to_ascii_uppercase(), value.into());
        self
    }

    /// Evaluate the value of a cell coordinate (e.g. `"A3"`), resolving formulas recursively.
    #[must_use]
    pub fn evaluate_cell(&self, coord: &str) -> String {
        self.eval_recursive(&coord.to_ascii_uppercase(), 0)
    }

    fn eval_recursive(&self, coord: &str, depth: u8) -> String {
        if depth > 16 {
            return "#CYCLE!".to_string();
        }
        let raw = match self.cells.get(coord) {
            Some(v) => v.trim(),
            None => return String::new(),
        };
        if !raw.starts_with('=') {
            return raw.to_string();
        }
        let expr = raw[1..].trim().to_ascii_uppercase();

        // Range functions: SUM(A1:B2), AVG(A1:B2), MIN(A1:B2), MAX(A1:B2)
        for func in ["SUM", "AVG", "MIN", "MAX"] {
            if let Some(rest) = expr.strip_prefix(func) {
                let inner = rest.trim().trim_start_matches('(').trim_end_matches(')');
                if let Some((start_ref, end_ref)) = inner.split_once(':') {
                    let values =
                        self.expand_range_numbers(start_ref.trim(), end_ref.trim(), depth + 1);
                    if values.is_empty() {
                        return "0".to_string();
                    }
                    let res = match func {
                        "SUM" => values.iter().sum::<f64>(),
                        "AVG" => values.iter().sum::<f64>() / (values.len() as f64),
                        "MIN" => values.iter().copied().fold(f64::INFINITY, f64::min),
                        "MAX" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        _ => 0.0,
                    };
                    return format_number(res);
                }
            }
        }

        // Binary arithmetic with proper precedence (*, / over +, -) and left-associativity
        if let Some((idx, op)) = find_split_op(&expr) {
            let left = &expr[..idx];
            let right = &expr[idx + 1..];
            let l_val = self.resolve_operand(left.trim(), depth + 1);
            let r_val = self.resolve_operand(right.trim(), depth + 1);
            let res = match op {
                '+' => l_val + r_val,
                '-' => l_val - r_val,
                '*' => l_val * r_val,
                '/' => {
                    if r_val.abs() < f64::EPSILON {
                        return "#DIV/0!".to_string();
                    }
                    l_val / r_val
                }
                _ => 0.0,
            };
            return format_number(res);
        }

        // Direct cell reference =A1
        let val = self.resolve_operand(&expr, depth + 1);
        format_number(val)
    }

    fn resolve_operand(&self, token: &str, depth: u8) -> f64 {
        let t = token.trim();
        if let Ok(n) = t.parse::<f64>() {
            return n;
        }
        if let Some((idx, op)) = find_split_op(t) {
            let l_val = self.resolve_operand(&t[..idx], depth + 1);
            let r_val = self.resolve_operand(&t[idx + 1..], depth + 1);
            return match op {
                '+' => l_val + r_val,
                '-' => l_val - r_val,
                '*' => l_val * r_val,
                '/' => {
                    if r_val.abs() < f64::EPSILON {
                        0.0
                    } else {
                        l_val / r_val
                    }
                }
                _ => 0.0,
            };
        }
        self.eval_recursive(t, depth).parse::<f64>().unwrap_or(0.0)
    }

    fn expand_range_numbers(&self, start: &str, end: &str, depth: u8) -> Vec<f64> {
        let Some((c1, r1)) = parse_coord(start) else {
            return Vec::new();
        };
        let Some((c2, r2)) = parse_coord(end) else {
            return Vec::new();
        };
        let mut nums = Vec::new();
        for col in c1.min(c2)..=c1.max(c2) {
            for row in r1.min(r2)..=r1.max(r2) {
                let coord = format!("{}{}", (b'A' + col) as char, row);
                if let Ok(v) = self.eval_recursive(&coord, depth).parse::<f64>() {
                    nums.push(v);
                }
            }
        }
        nums
    }
}

fn find_split_op(expr: &str) -> Option<(usize, char)> {
    let bytes = expr.as_bytes();
    // 1. Lowest precedence operators '+' and '-' scanned right-to-left for left-associativity
    for (i, &b) in bytes.iter().enumerate().rev() {
        if (b == b'+' || b == b'-') && i > 0 {
            let prev = bytes[i - 1];
            if !matches!(prev, b'+' | b'-' | b'*' | b'/' | b'(' | b'e' | b'E') {
                return Some((i, b as char));
            }
        }
    }
    // 2. Higher precedence operators '*' and '/' scanned right-to-left
    for (i, &b) in bytes.iter().enumerate().rev() {
        if (b == b'*' || b == b'/') && i > 0 {
            return Some((i, b as char));
        }
    }
    None
}

fn parse_coord(coord: &str) -> Option<(u8, usize)> {
    let bytes = coord.trim().as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let col_char = bytes[0].to_ascii_uppercase();
    if !col_char.is_ascii_uppercase() {
        return None;
    }
    let row_num = coord[1..].parse::<usize>().ok()?;
    Some((col_char - b'A', row_num))
}

fn format_number(n: f64) -> String {
    if (n - n.round()).abs() < 1e-9 {
        format!("{}", n.round() as i64)
    } else {
        format!("{:.2}", n)
    }
}

impl Component for Spreadsheet {
    fn render(&self) -> Node {
        let mut head_row = Element::new("tr").child(
            Element::new("th")
                .attr("class", "w-10 border border-slate-800 bg-slate-900 text-center text-xs font-mono text-slate-500 py-1.5")
                .child(Node::text("#")),
        );

        for c in 0..self.cols {
            let col_letter = ((b'A' + (c as u8)) as char).to_string();
            head_row = head_row.child(
                Element::new("th")
                    .attr("class", "min-w-[110px] border border-slate-800 bg-slate-900 px-3 py-1.5 text-center text-xs font-mono font-bold text-slate-300")
                    .child(Node::text(col_letter)),
            );
        }

        let mut tbody = Element::new("tbody");
        for r in 1..=self.rows {
            let mut tr = Element::new("tr").child(
                Element::new("td")
                    .attr("class", "border border-slate-800 bg-slate-900/70 text-center text-xs font-mono text-slate-500 py-1")
                    .child(Node::text(r.to_string())),
            );

            for c in 0..self.cols {
                let coord = format!("{}{}", (b'A' + (c as u8)) as char, r);
                let raw_val = self.cells.get(&coord).cloned().unwrap_or_default();
                let display_val = self.evaluate_cell(&coord);

                let is_formula = raw_val.starts_with('=');
                let td_class = if is_formula {
                    "border border-slate-800 bg-indigo-950/20 px-2.5 py-1.5 text-xs font-mono font-semibold text-indigo-300 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                } else {
                    "border border-slate-800 bg-slate-950 px-2.5 py-1.5 text-xs font-mono text-slate-200 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                };

                let mut td = Element::new("td")
                    .attr("data-cell", coord)
                    .attr("contenteditable", "true")
                    .attr("class", td_class);

                if is_formula {
                    td = td.attr("data-formula", raw_val);
                }

                tr = tr.child(td.child(Node::text(display_val)));
            }
            tbody = tbody.child(tr);
        }

        Element::new("div")
            .attr("id", self.id.clone())
            .attr(
                "class",
                "yav-spreadsheet overflow-x-auto rounded-xl border border-slate-800 shadow-xl",
            )
            .child(
                Element::new("table")
                    .attr("class", "w-full border-collapse")
                    .child(Element::new("thead").child(head_row))
                    .child(tbody),
            )
            .into_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::render_to_string;

    #[test]
    fn test_spreadsheet_formulas_sum_avg_and_arithmetic() {
        let sheet = Spreadsheet::new("budget-grid", 3, 4)
            .set_cell("A1", "150")
            .set_cell("A2", "250")
            .set_cell("A3", "100")
            .set_cell("A4", "=SUM(A1:A3)")
            .set_cell("B1", "=AVG(A1:A3)")
            .set_cell("C1", "=A4*2")
            .set_cell("C2", "=10-5-2")
            .set_cell("C3", "=10+2*3")
            .set_cell("C4", "=12/3/2");

        assert_eq!(sheet.evaluate_cell("A4"), "500");
        assert_eq!(sheet.evaluate_cell("B1"), "166.67");
        assert_eq!(sheet.evaluate_cell("C1"), "1000");
        assert_eq!(sheet.evaluate_cell("C2"), "3");
        assert_eq!(sheet.evaluate_cell("C3"), "16");
        assert_eq!(sheet.evaluate_cell("C4"), "2");

        let html = render_to_string(&sheet.render());
        assert!(html.contains("data-cell=\"A4\""));
        assert!(html.contains("data-formula=\"=SUM(A1:A3)\""));
        assert!(html.contains(">500</td>"));
        assert!(html.contains(">1000</td>"));
    }
}
