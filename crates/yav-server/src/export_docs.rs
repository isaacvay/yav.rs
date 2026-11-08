//! Vector PDF 1.4 Document Generator & RFC 4180 CSV/TSV Exporter.
//!
//! Provides `@react-pdf/renderer` and `PapaParse` parity in pure Rust with zero external
//! binary dependencies:
//! - [`PdfDocument`] generates standards-compliant `%PDF-1.4` binary files with vector
//!   typography (Helvetica / Helvetica-Bold), headers, tables, invoice line items, and
//!   cross-reference (`xref`) tables.
//! - [`CsvBuilder`] generates RFC 4180 compliant CSV and TSV streams with automatic
//!   spreadsheet formula-injection sanitization (`=`, `+`, `-`, `@` prefix escaping).
//! - [`pdf_response`] and [`csv_response`] return ready-to-download Axum [`Response`]s
//!   with `Content-Disposition: attachment` headers.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

/// A line item row for invoices or tabular PDF reports.
#[derive(Debug, Clone)]
pub struct PdfTableRow {
    /// Description or label column.
    pub label: String,
    /// Quantity or secondary column.
    pub quantity: String,
    /// Unit price or tertiary column.
    pub unit_price: String,
    /// Line total or right-aligned value.
    pub total: String,
}

/// Pure-Rust PDF 1.4 vector document builder (A4 portrait: 595 x 842 pt).
#[derive(Debug, Clone, Default)]
pub struct PdfDocument {
    pub title: String,
    pub subtitle: Option<String>,
    pub metadata_lines: Vec<(String, String)>,
    pub paragraphs: Vec<String>,
    pub table_headers: Option<[String; 4]>,
    pub table_rows: Vec<PdfTableRow>,
    pub footer_total: Option<(String, String)>,
}

impl PdfDocument {
    /// Create a new PDF document with a main heading title.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Add a subtitle below the main heading.
    #[must_use]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Add a key-value metadata pair (e.g. `("Invoice #", "INV-2026-0042")`).
    #[must_use]
    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata_lines.push((key.into(), value.into()));
        self
    }

    /// Append a body text paragraph.
    #[must_use]
    pub fn paragraph(mut self, text: impl Into<String>) -> Self {
        self.paragraphs.push(text.into());
        self
    }

    /// Set the 4 column headers for the tabular section.
    #[must_use]
    pub fn table_headers(
        mut self,
        col1: impl Into<String>,
        col2: impl Into<String>,
        col3: impl Into<String>,
        col4: impl Into<String>,
    ) -> Self {
        self.table_headers = Some([col1.into(), col2.into(), col3.into(), col4.into()]);
        self
    }

    /// Add a row to the table.
    #[must_use]
    pub fn add_row(
        mut self,
        label: impl Into<String>,
        quantity: impl Into<String>,
        unit_price: impl Into<String>,
        total: impl Into<String>,
    ) -> Self {
        self.table_rows.push(PdfTableRow {
            label: label.into(),
            quantity: quantity.into(),
            unit_price: unit_price.into(),
            total: total.into(),
        });
        self
    }

    /// Add a highlighted grand-total line at the bottom of the table.
    #[must_use]
    pub fn total(mut self, label: impl Into<String>, amount: impl Into<String>) -> Self {
        self.footer_total = Some((label.into(), amount.into()));
        self
    }

    /// Render the document into a valid `%PDF-1.4` binary byte vector.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut stream = String::with_capacity(2048);
        let mut y: f32 = 780.0;

        // Title (Helvetica-Bold 20pt)
        stream.push_str(&format!(
            "BT /F2 20 Tf 50 {y:.1} Td ({}) Tj ET\n",
            escape_pdf_text(&self.title)
        ));
        y -= 24.0;

        if let Some(ref sub) = self.subtitle {
            stream.push_str(&format!(
                "BT /F1 11 Tf 50 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(sub)
            ));
            y -= 20.0;
        }

        // Horizontal divider line
        stream.push_str(&format!("0.85 G 1 w 50 {y:.1} m 545 {y:.1} l S\n"));
        y -= 22.0;

        // Metadata key-value pairs
        for (k, v) in &self.metadata_lines {
            stream.push_str(&format!(
                "BT /F2 10 Tf 50 {y:.1} Td ({}:) Tj ET\n",
                escape_pdf_text(k)
            ));
            stream.push_str(&format!(
                "BT /F1 10 Tf 150 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(v)
            ));
            y -= 16.0;
        }

        if !self.metadata_lines.is_empty() {
            y -= 10.0;
        }

        // Body paragraphs
        for p in &self.paragraphs {
            stream.push_str(&format!(
                "BT /F1 10 Tf 50 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(p)
            ));
            y -= 18.0;
        }

        // Table headers
        if let Some(ref hdrs) = self.table_headers {
            y -= 8.0;
            stream.push_str(&format!("0.94 g 50 {:.1} 495 20 re f 0 g\n", y - 5.0));
            let xs = [55.0_f32, 300.0, 390.0, 480.0];
            for (idx, h) in hdrs.iter().enumerate() {
                stream.push_str(&format!(
                    "BT /F2 10 Tf {:.1} {y:.1} Td ({}) Tj ET\n",
                    xs[idx],
                    escape_pdf_text(h)
                ));
            }
            y -= 22.0;
        }

        // Table rows
        for row in &self.table_rows {
            stream.push_str(&format!(
                "BT /F1 10 Tf 55 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(&row.label)
            ));
            stream.push_str(&format!(
                "BT /F1 10 Tf 300 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(&row.quantity)
            ));
            stream.push_str(&format!(
                "BT /F1 10 Tf 390 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(&row.unit_price)
            ));
            stream.push_str(&format!(
                "BT /F1 10 Tf 480 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(&row.total)
            ));
            y -= 18.0;
        }

        if let Some((ref tot_label, ref tot_val)) = self.footer_total {
            y -= 6.0;
            stream.push_str(&format!("0.80 G 1 w 350 {y:.1} m 545 {y:.1} l S\n"));
            y -= 18.0;
            stream.push_str(&format!(
                "BT /F2 12 Tf 355 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(tot_label)
            ));
            stream.push_str(&format!(
                "BT /F2 12 Tf 480 {y:.1} Td ({}) Tj ET\n",
                escape_pdf_text(tot_val)
            ));
        }

        // Assemble PDF 1.4 objects with byte offsets for xref table
        let mut pdf = Vec::with_capacity(stream.len() + 1024);
        pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

        let mut offsets = Vec::with_capacity(6);

        // Object 1: Catalog
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        offsets.push(pdf.len());
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // Object 3: Page (A4 595x842)
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R /F2 5 0 R >> >> /Contents 6 0 R >>\nendobj\n",
        );

        // Object 4: Font F1 (Helvetica)
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );

        // Object 5: Font F2 (Helvetica-Bold)
        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n",
        );

        // Object 6: Content stream
        offsets.push(pdf.len());
        let content_bytes = stream.as_bytes();
        pdf.extend_from_slice(
            format!("6 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len()).as_bytes(),
        );
        pdf.extend_from_slice(content_bytes);
        pdf.extend_from_slice(b"endstream\nendobj\n");

        // Xref table
        let xref_offset = pdf.len();
        pdf.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes(),
        );
        for off in &offsets {
            pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }

        // Trailer
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                offsets.len() + 1
            )
            .as_bytes(),
        );

        pdf
    }
}

fn escape_pdf_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() && !c.is_ascii_control() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}

/// Build an HTTP download response for a [`PdfDocument`].
#[must_use]
pub fn pdf_response(filename: &str, doc: &PdfDocument) -> Response {
    let bytes = doc.to_bytes();
    let safe_name: String = filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    let disposition = format!("attachment; filename=\"{safe_name}\"");

    let mut resp = (StatusCode::OK, bytes).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    if let Ok(hv) = HeaderValue::from_str(&disposition) {
        resp.headers_mut().insert(header::CONTENT_DISPOSITION, hv);
    }
    resp
}

/// RFC 4180 CSV & TSV builder with spreadsheet formula-injection sanitization.
#[derive(Debug, Clone)]
pub struct CsvBuilder {
    delimiter: char,
    sanitize_formulas: bool,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Default for CsvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvBuilder {
    /// Create a new comma-separated (`','`) CSV builder with formula-injection protection enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            delimiter: ',',
            sanitize_formulas: true,
            headers: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Create a tab-separated (`'\t'`) TSV builder.
    #[must_use]
    pub fn new_tsv() -> Self {
        Self {
            delimiter: '\t',
            sanitize_formulas: true,
            headers: Vec::new(),
            rows: Vec::new(),
        }
    }

    /// Set column header names.
    #[must_use]
    pub fn headers<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.headers = cols.into_iter().map(Into::into).collect();
        self
    }

    /// Append a data row.
    #[must_use]
    pub fn row<I, S>(mut self, cells: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self
    }

    /// Render the RFC 4180 CSV/TSV string.
    #[must_use]
    pub fn finish(&self) -> String {
        let mut out = String::new();
        if !self.headers.is_empty() {
            self.write_record(&self.headers, &mut out);
        }
        for row in &self.rows {
            self.write_record(row, &mut out);
        }
        out
    }

    fn write_record(&self, fields: &[String], out: &mut String) {
        for (idx, field) in fields.iter().enumerate() {
            if idx > 0 {
                out.push(self.delimiter);
            }
            let mut sanitized = field.clone();
            // Defend against OWASP CSV Formula Injection (=, +, -, @)
            if self.sanitize_formulas
                && sanitized
                    .chars()
                    .next()
                    .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
                && sanitized.parse::<f64>().is_err()
            {
                sanitized.insert(0, '\'');
            }

            let needs_quotes = sanitized.contains(self.delimiter)
                || sanitized.contains('"')
                || sanitized.contains('\n')
                || sanitized.contains('\r');

            if needs_quotes {
                out.push('"');
                for ch in sanitized.chars() {
                    if ch == '"' {
                        out.push_str("\"\"");
                    } else {
                        out.push(ch);
                    }
                }
                out.push('"');
            } else {
                out.push_str(&sanitized);
            }
        }
        out.push_str("\r\n");
    }
}

/// Build an HTTP download response for a [`CsvBuilder`].
#[must_use]
pub fn csv_response(filename: &str, csv: &CsvBuilder) -> Response {
    let body = csv.finish();
    let safe_name: String = filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    let disposition = format!("attachment; filename=\"{safe_name}\"");

    let mut resp = (StatusCode::OK, body).into_response();
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    if let Ok(hv) = HeaderValue::from_str(&disposition) {
        resp.headers_mut().insert(header::CONTENT_DISPOSITION, hv);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_invoice_generation() {
        let doc = PdfDocument::new("INVOICE #INV-2026-089")
            .subtitle("Yav Cloud Enterprise Ltd.")
            .meta("Customer", "Acme Corp")
            .meta("Date", "2026-11-05")
            .table_headers("Item", "Qty", "Unit Price", "Total")
            .add_row("Yav Pro Plan (Annual)", "1", "$290.00", "$290.00")
            .add_row("Extra Edge Bandwidth (1TB)", "2", "$25.00", "$50.00")
            .total("Total Due", "$340.00");

        let bytes = doc.to_bytes();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("INVOICE #INV-2026-089"));
        assert!(text.contains("Yav Pro Plan \\(Annual\\)"));
        assert!(text.contains("xref"));
        assert!(text.ends_with("%%EOF\n"));
    }

    #[test]
    fn test_csv_rfc4180_and_formula_injection_protection() {
        let csv = CsvBuilder::new()
            .headers(["ID", "Name", "Notes", "Payload"])
            .row(["1", "Alice, Jr.", "Said \"Hello\"", "=cmd|' /C calc'!A0"]);

        let rendered = csv.finish();
        assert!(rendered.contains("ID,Name,Notes,Payload\r\n"));
        assert!(rendered.contains("\"Alice, Jr.\""));
        assert!(rendered.contains("\"Said \"\"Hello\"\"\""));
        // Dangerous formula prefixed with single quote
        assert!(rendered.contains("'=cmd|' /C calc'!A0"));
    }
}
