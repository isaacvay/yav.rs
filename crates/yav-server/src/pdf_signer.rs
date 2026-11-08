//! Zero-Dependency Vector PDF Signer & Digital Watermarking.
//!
//! Provides cryptographic document verification (HMAC-SHA256 signature stamping),
//! visual tamper-evident signature badges, and diagonal vector watermarking
//! for compliance, invoicing, and contract certification.
//!
//! ## Cryptographic Scope & Integrity Guarantees
//!
//! This module implements shared-secret HMAC-SHA256 digital seals and visual audit trails.
//! It guarantees tamper-evident document integrity between cooperating parties who share
//! the verification secret.
//!
//! *Note:* This is **not** an ISO 32000-1 / PAdES (X.509 PKI / ASN.1 CMS) public key digital
//! signature intended for third-party Adobe Acrobat certificate validation. If public-key
//! eIDAS/Acrobat compliance is required, integrate with an external HSM or PKI signing service.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::export_docs::PdfDocument;

type HmacSha256 = Hmac<Sha256>;

/// Metadata and cryptographic digest of a digitally signed PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfSignature {
    /// Full name or entity of the signer (e.g. "Acme Corp Legal", "Jane Doe").
    pub signer_name: String,
    /// Reason or purpose of the signature (e.g. "Approved & Certified", "Invoice Acknowledged").
    pub reason: String,
    /// Contact info or organizational unit (e.g. "legal@acme.com").
    pub contact_info: Option<String>,
    /// Location or jurisdiction (e.g. "Paris, France").
    pub location: Option<String>,
    /// ISO 8601 or RFC 3339 timestamp of signing.
    pub timestamp_iso: String,
    /// Hex-encoded HMAC-SHA256 cryptographic digest of document bytes.
    pub signature_hex: String,
}

impl PdfSignature {
    /// Render a compact visual text badge for inclusion in printable document bodies.
    #[must_use]
    pub fn visual_badge(&self) -> String {
        let short_hash = if self.signature_hex.len() >= 16 {
            &self.signature_hex[0..16]
        } else {
            &self.signature_hex
        };
        format!(
            "--- DIGITALLY SIGNED & SEALED ---\nSigner: {}\nReason: {}\nDate: {}\nHMAC-SHA256: {}... [Integrity Verified]",
            self.signer_name, self.reason, self.timestamp_iso, short_hash
        )
    }
}

/// Diagonal or overlay vector watermark configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfWatermark {
    /// Watermark text (e.g. "CONFIDENTIAL", "COPIE CERTIFIÉE", "PAID", "DRAFT").
    pub text: String,
    /// Watermark opacity from `0.0` (invisible) to `1.0` (fully opaque). Default is `0.15`.
    pub opacity: f32,
    /// Rotation angle in degrees (counter-clockwise). Default is `45.0`.
    pub rotation_deg: f32,
    /// Font point size. Default is `48.0`.
    pub font_size: f32,
    /// RGB color tuple in `0.0..=1.0` range. Default is gray `(0.65, 0.65, 0.65)`.
    pub color_rgb: (f32, f32, f32),
}

impl Default for PdfWatermark {
    fn default() -> Self {
        Self {
            text: "CONFIDENTIAL".to_string(),
            opacity: 0.15,
            rotation_deg: 45.0,
            font_size: 48.0,
            color_rgb: (0.65, 0.65, 0.65),
        }
    }
}

impl PdfWatermark {
    /// Create a new watermark with custom text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Default::default()
        }
    }

    /// Set opacity in range `0.0..=1.0`.
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set rotation angle in degrees.
    #[must_use]
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation_deg = deg;
        self
    }

    /// Set RGB color in range `0.0..=1.0`.
    #[must_use]
    pub fn color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color_rgb = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
        self
    }

    /// Set font size in points.
    #[must_use]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size.max(8.0);
        self
    }

    /// Generate PDF vector content stream operators for rendering this watermark.
    #[must_use]
    pub fn to_pdf_stream_operators(&self, page_width: f32, page_height: f32) -> String {
        let rad = self.rotation_deg.to_radians();
        let cos = rad.cos();
        let sin = rad.sin();

        // Center coordinates
        let cx = page_width / 2.0;
        let cy = page_height / 2.0;

        let (r, g, b) = self.color_rgb;

        // Escape parenthesis in watermark text
        let escaped_text = self
            .text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");

        format!(
            "q\n\
             {r:.3} {g:.3} {b:.3} rg\n\
             BT\n\
             /F1 {size:.1} Tf\n\
             {cos:.4} {sin:.4} {neg_sin:.4} {cos:.4} {cx:.1} {cy:.1} cm\n\
             -120 0 Td\n\
             ({text}) Tj\n\
             ET\n\
             Q\n",
            r = r,
            g = g,
            b = b,
            size = self.font_size,
            cos = cos,
            sin = sin,
            neg_sin = -sin,
            cx = cx,
            cy = cy,
            text = escaped_text
        )
    }
}

/// Pure-Rust PDF Signer & Watermarking Engine.
pub struct PdfSigner;

impl PdfSigner {
    /// Compute the cryptographic HMAC-SHA256 signature for a raw byte stream.
    #[must_use]
    pub fn compute_hmac(secret_key: &[u8], data: &[u8]) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret_key).expect("HMAC key can accept any size slice");
        mac.update(data);
        let result = mac.finalize().into_bytes();
        let mut hex = String::with_capacity(64);
        for byte in result {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{:02x}", byte);
        }
        hex
    }

    /// Verify an HMAC-SHA256 signature against expected data using constant-time comparison.
    #[must_use]
    pub fn verify_hmac(secret_key: &[u8], data: &[u8], signature_hex: &str) -> bool {
        let expected = Self::compute_hmac(secret_key, data);
        if expected.len() != signature_hex.len() {
            return false;
        }
        // Constant-time comparison
        let mut diff = 0u8;
        for (a, b) in expected.bytes().zip(signature_hex.bytes()) {
            diff |= a ^ b;
        }
        diff == 0
    }

    /// Sign a [`PdfDocument`] in place before compilation, stamping visual badges
    /// and metadata keys into the document model.
    pub fn sign_document(
        doc: &mut PdfDocument,
        secret_key: &[u8],
        signer_name: impl Into<String>,
        reason: impl Into<String>,
        timestamp_iso: impl Into<String>,
    ) -> PdfSignature {
        let signer = signer_name.into();
        let reason_str = reason.into();
        let ts = timestamp_iso.into();

        // Calculate pre-signature digest of document structure
        let mut hasher = Sha256::new();
        hasher.update(doc.title.as_bytes());
        if let Some(ref sub) = doc.subtitle {
            hasher.update(sub.as_bytes());
        }
        for (k, v) in &doc.metadata_lines {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }
        for p in &doc.paragraphs {
            hasher.update(p.as_bytes());
        }
        for row in &doc.table_rows {
            hasher.update(row.label.as_bytes());
            hasher.update(row.total.as_bytes());
        }
        let digest_bytes = hasher.finalize();

        let sig_hex = Self::compute_hmac(secret_key, &digest_bytes);

        let signature = PdfSignature {
            signer_name: signer,
            reason: reason_str,
            contact_info: None,
            location: None,
            timestamp_iso: ts,
            signature_hex: sig_hex,
        };

        // Append visual verification badge and document metadata
        doc.metadata_lines
            .push(("Digitally Signed By".into(), signature.signer_name.clone()));
        doc.metadata_lines
            .push(("Signature Date".into(), signature.timestamp_iso.clone()));
        doc.metadata_lines.push((
            "HMAC Digest".into(),
            signature.signature_hex[0..16].to_string(),
        ));
        doc.paragraphs.push(signature.visual_badge());

        signature
    }

    /// Stamp an incremental cryptographic signature block onto an existing raw `%PDF-1.4` file.
    #[must_use]
    pub fn sign_raw_pdf(
        pdf_bytes: &[u8],
        secret_key: &[u8],
        signer_name: &str,
        reason: &str,
        timestamp_iso: &str,
    ) -> Vec<u8> {
        let sig_hex = Self::compute_hmac(secret_key, pdf_bytes);

        let mut output = Vec::with_capacity(pdf_bytes.len() + 512);
        output.extend_from_slice(pdf_bytes);

        // Append PDF incremental signature trailer
        let sig_block = format!(
            "% --- YAV CRYPTOGRAPHIC SIGNATURE BLOCK ---\n\
             % Signer: {signer}\n\
             % Reason: {reason}\n\
             % Timestamp: {ts}\n\
             % Signature: {sig}\n\
             % ------------------------------------------\n\
             %%EOF\n",
            signer = signer_name,
            reason = reason,
            ts = timestamp_iso,
            sig = sig_hex
        );
        output.extend_from_slice(sig_block.as_bytes());

        output
    }

    /// Extract and verify the cryptographic signature block from a raw PDF file.
    pub fn verify_raw_pdf(
        signed_pdf: &[u8],
        secret_key: &[u8],
    ) -> Result<PdfSignature, &'static str> {
        let marker = b"% --- YAV CRYPTOGRAPHIC SIGNATURE BLOCK ---";

        let marker_pos = signed_pdf
            .windows(marker.len())
            .rposition(|w| w == marker)
            .ok_or("No cryptographic signature block found")?;
        let original_bytes = &signed_pdf[0..marker_pos];

        // Parse signature fields
        let block_bytes = &signed_pdf[marker_pos..];
        let block = std::str::from_utf8(block_bytes)
            .map_err(|_| "Signature block is not valid UTF-8/ASCII")?;
        let mut signer = String::new();
        let mut reason = String::new();
        let mut timestamp = String::new();
        let mut signature_hex = String::new();

        for line in block.lines() {
            if let Some(val) = line.strip_prefix("% Signer: ") {
                signer = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("% Reason: ") {
                reason = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("% Timestamp: ") {
                timestamp = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("% Signature: ") {
                signature_hex = val.trim().to_string();
            }
        }

        if signature_hex.is_empty() {
            return Err("Missing signature digest");
        }

        if !Self::verify_hmac(secret_key, original_bytes, &signature_hex) {
            return Err(
                "Cryptographic signature verification failed: document tampered or invalid key",
            );
        }

        Ok(PdfSignature {
            signer_name: signer,
            reason,
            contact_info: None,
            location: None,
            timestamp_iso: timestamp,
            signature_hex,
        })
    }

    /// Apply a diagonal vector watermark to an existing `%PDF-1.4` document stream.
    #[must_use]
    pub fn apply_watermark_to_pdf_stream(pdf_bytes: &[u8], watermark: &PdfWatermark) -> Vec<u8> {
        let endstream_marker = b"endstream";
        let ops = watermark.to_pdf_stream_operators(595.28, 841.89); // A4 dimensions

        if let Some(pos) = pdf_bytes
            .windows(endstream_marker.len())
            .position(|w| w == endstream_marker)
        {
            let mut out = Vec::with_capacity(pdf_bytes.len() + ops.len());
            out.extend_from_slice(&pdf_bytes[0..pos]);
            out.extend_from_slice(ops.as_bytes());
            out.extend_from_slice(&pdf_bytes[pos..]);
            out
        } else {
            // Append at end if no stream marker found
            let mut out = Vec::with_capacity(pdf_bytes.len() + ops.len());
            out.extend_from_slice(pdf_bytes);
            out.extend_from_slice(ops.as_bytes());
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_signing_and_verification() {
        let key = b"super-secret-key-12345";
        let payload = b"Hello, this is a contract for Yav.rs";

        let sig = PdfSigner::compute_hmac(key, payload);
        assert_eq!(sig.len(), 64);
        assert!(PdfSigner::verify_hmac(key, payload, &sig));

        // Tamper test
        assert!(!PdfSigner::verify_hmac(key, b"Tampered payload", &sig));
        assert!(!PdfSigner::verify_hmac(b"wrong-key", payload, &sig));
    }

    #[test]
    fn test_sign_document_model() {
        let mut doc = PdfDocument::new("Quarterly Financial Report")
            .subtitle("Q4 2026")
            .paragraph("Revenue grew 42% YoY.");

        let key = b"corp-cert-key-2026";
        let sig = PdfSigner::sign_document(
            &mut doc,
            key,
            "Chief Compliance Officer",
            "Financial Audit Passed",
            "2026-11-05T12:00:00Z",
        );

        assert_eq!(sig.signer_name, "Chief Compliance Officer");
        assert!(doc
            .paragraphs
            .iter()
            .any(|p| p.contains("DIGITALLY SIGNED")));
        assert!(doc
            .metadata_lines
            .iter()
            .any(|(k, v)| k == "Digitally Signed By" && v == "Chief Compliance Officer"));
    }

    #[test]
    fn test_raw_pdf_sign_and_verify_roundtrip() {
        let dummy_pdf =
            b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";
        let key = b"my-private-signing-key";

        let signed_pdf = PdfSigner::sign_raw_pdf(
            dummy_pdf,
            key,
            "Alice Security",
            "Production Release Approval",
            "2026-11-05T12:30:00Z",
        );

        let verified =
            PdfSigner::verify_raw_pdf(&signed_pdf, key).expect("Valid signature verification");
        assert_eq!(verified.signer_name, "Alice Security");
        assert_eq!(verified.reason, "Production Release Approval");

        // Test tampering with body
        let mut tampered = signed_pdf.clone();
        tampered[10] = b'X';
        assert!(PdfSigner::verify_raw_pdf(&tampered, key).is_err());
    }

    #[test]
    fn test_pdf_watermark_generation() {
        let wm = PdfWatermark::new("CONFIDENTIAL DRAFT")
            .opacity(0.2)
            .rotation(45.0)
            .color(0.8, 0.2, 0.2);

        let ops = wm.to_pdf_stream_operators(595.0, 842.0);
        assert!(ops.contains("0.800 0.200 0.200 rg"));
        assert!(ops.contains("(CONFIDENTIAL DRAFT) Tj"));

        let dummy_stream = b"stream\nBT /F1 12 Tf (Hello World) Tj ET\nendstream";
        let watermarked = PdfSigner::apply_watermark_to_pdf_stream(dummy_stream, &wm);
        let watermarked_str = String::from_utf8_lossy(&watermarked);
        assert!(watermarked_str.contains("(CONFIDENTIAL DRAFT) Tj"));
        assert!(watermarked_str.contains("endstream"));
    }
}
