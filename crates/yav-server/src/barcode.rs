//! Pure Rust Vector Barcode & QR Code Generator.
//!
//! Generates scannable SVG vector QR Codes (full ISO/IEC 18004 with Reed–Solomon
//! error correction, format/version information and mask optimisation via the
//! `qrcode` crate) and Code 128 barcodes for invoices, 2FA pairing (TOTP),
//! event tickets, and logistics tracking.

/// Code 128 Barcode Generator (Subset B).
pub struct Barcode128;

impl Barcode128 {
    /// Generate a pure vector SVG barcode string from ASCII text.
    pub fn svg(text: &str, height: u32, bar_width: u32) -> String {
        let patterns = Self::encode_code128_b(text);
        let total_modules: u32 = patterns.len() as u32;
        let width = total_modules * bar_width + 20; // 10px quiet zone on each side

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" shape-rendering="crispEdges">
  <rect width="100%" height="100%" fill="#ffffff" />
  <g fill="#000000">
"##
        );

        let mut x = 10;
        let bar_h = height.saturating_sub(8).max(1);
        for is_bar in patterns {
            if is_bar {
                svg.push_str(&format!(
                    r#"    <rect x="{x}" y="4" width="{bar_width}" height="{bar_h}" />
"#
                ));
            }
            x += bar_width;
        }

        svg.push_str("  </g>\n</svg>");
        svg
    }

    /// Internal encoder returning boolean modules (`true` = black bar, `false` = white space).
    fn encode_code128_b(text: &str) -> Vec<bool> {
        // Code 128 B start code pattern = 104
        let mut codes = vec![104];
        let mut checksum_sum: usize = 104;

        for (i, b) in text.as_bytes().iter().enumerate() {
            let val = if *b >= 32 && *b <= 127 {
                (*b - 32) as usize
            } else {
                0
            };
            codes.push(val);
            checksum_sum += val * (i + 1);
        }

        // Checksum modulo 103
        let check_val = checksum_sum % 103;
        codes.push(check_val);
        // Stop code = 106
        codes.push(106);

        let mut modules = Vec::new();
        for &code in &codes {
            let pattern_bits = get_code128_pattern(code);
            for bit in pattern_bits {
                modules.push(bit);
            }
        }
        // Termination bar for stop code
        modules.push(true);
        modules.push(true);

        modules
    }
}

/// 11-bit patterns for Code 128 characters (0-106).
fn get_code128_pattern(code: usize) -> [bool; 11] {
    const PATTERNS: &[[u8; 6]] = &[
        [2, 1, 2, 2, 2, 2], // 0
        [2, 2, 2, 1, 2, 2], // 1
        [2, 2, 2, 2, 2, 1], // 2
        [1, 2, 1, 2, 2, 3], // 3
        [1, 2, 1, 3, 2, 2], // 4
        [1, 3, 1, 2, 2, 2], // 5
        [1, 2, 2, 2, 1, 3], // 6
        [1, 2, 2, 3, 1, 2], // 7
        [1, 3, 2, 2, 1, 2], // 8
        [2, 2, 1, 2, 1, 3], // 9
        [2, 2, 1, 3, 1, 2], // 10
        [2, 3, 1, 2, 1, 2], // 11
        [1, 1, 2, 2, 3, 2], // 12
        [1, 2, 2, 1, 3, 2], // 13
        [1, 2, 2, 2, 3, 1], // 14
        [1, 1, 3, 2, 2, 2], // 15
        [1, 2, 3, 1, 2, 2], // 16
        [1, 2, 3, 2, 2, 1], // 17
        [2, 2, 3, 2, 1, 1], // 18
        [2, 2, 1, 1, 3, 2], // 19
        [2, 2, 1, 2, 3, 1], // 20
        [2, 1, 3, 2, 1, 2], // 21
        [2, 2, 3, 1, 1, 2], // 22
        [3, 1, 2, 1, 3, 1], // 23
        [3, 1, 1, 2, 2, 2], // 24
        [3, 2, 1, 1, 2, 2], // 25
        [3, 2, 1, 2, 2, 1], // 26
        [3, 1, 2, 2, 1, 2], // 27
        [3, 2, 2, 1, 1, 2], // 28
        [3, 2, 2, 2, 1, 1], // 29
        [2, 1, 2, 1, 2, 3], // 30
        [2, 1, 2, 3, 2, 1], // 31
        [2, 3, 2, 1, 2, 1], // 32
        [1, 1, 1, 3, 2, 3], // 33
        [1, 3, 1, 1, 2, 3], // 34
        [1, 3, 1, 3, 2, 1], // 35
        [1, 1, 2, 3, 1, 3], // 36
        [1, 3, 2, 1, 1, 3], // 37
        [1, 3, 2, 3, 1, 1], // 38
        [2, 1, 1, 3, 1, 3], // 39
        [2, 3, 1, 1, 1, 3], // 40
        [2, 3, 1, 3, 1, 1], // 41
        [1, 1, 2, 1, 3, 3], // 42
        [1, 1, 2, 3, 3, 1], // 43
        [1, 3, 2, 1, 3, 1], // 44
        [1, 1, 3, 1, 2, 3], // 45
        [1, 1, 3, 3, 2, 1], // 46
        [1, 3, 3, 1, 2, 1], // 47
        [3, 1, 3, 1, 2, 1], // 48
        [2, 1, 1, 3, 3, 1], // 49
        [2, 3, 1, 1, 3, 1], // 50
        [2, 1, 3, 1, 1, 3], // 51
        [2, 1, 3, 3, 1, 1], // 52
        [2, 1, 3, 1, 3, 1], // 53
        [3, 1, 1, 1, 2, 3], // 54
        [3, 1, 1, 3, 2, 1], // 55
        [3, 3, 1, 1, 2, 1], // 56
        [3, 1, 2, 1, 1, 3], // 57
        [3, 1, 2, 3, 1, 1], // 58
        [3, 3, 2, 1, 1, 1], // 59
        [3, 1, 4, 1, 1, 1], // 60
        [2, 2, 1, 4, 1, 1], // 61
        [4, 3, 1, 1, 1, 1], // 62
        [1, 1, 1, 2, 2, 4], // 63
        [1, 1, 1, 4, 2, 2], // 64
        [1, 2, 1, 1, 2, 4], // 65
        [1, 2, 1, 4, 2, 1], // 66
        [1, 4, 1, 1, 2, 2], // 67
        [1, 4, 1, 2, 2, 1], // 68
        [1, 1, 2, 2, 1, 4], // 69
        [1, 1, 2, 4, 1, 2], // 70
        [1, 2, 2, 1, 1, 4], // 71
        [1, 2, 2, 4, 1, 1], // 72
        [1, 4, 2, 1, 1, 2], // 73
        [1, 4, 2, 2, 1, 1], // 74
        [2, 4, 1, 2, 1, 1], // 75
        [2, 2, 1, 1, 1, 4], // 76
        [4, 1, 3, 1, 1, 1], // 77
        [2, 4, 1, 1, 1, 2], // 78
        [1, 3, 4, 1, 1, 1], // 79
        [1, 1, 1, 2, 4, 2], // 80
        [1, 2, 1, 1, 4, 2], // 81
        [1, 2, 1, 2, 4, 1], // 82
        [1, 1, 4, 2, 1, 2], // 83
        [1, 2, 4, 1, 1, 2], // 84
        [1, 2, 4, 2, 1, 1], // 85
        [4, 1, 1, 2, 1, 2], // 86
        [4, 2, 1, 1, 1, 2], // 87
        [4, 2, 1, 2, 1, 1], // 88
        [2, 1, 2, 1, 4, 1], // 89
        [2, 1, 4, 1, 2, 1], // 90
        [4, 1, 2, 1, 2, 1], // 91
        [1, 1, 1, 1, 4, 3], // 92
        [1, 1, 1, 3, 4, 1], // 93
        [1, 3, 1, 1, 4, 1], // 94
        [1, 1, 4, 1, 1, 3], // 95
        [1, 1, 4, 3, 1, 1], // 96
        [4, 1, 1, 1, 1, 3], // 97
        [4, 1, 1, 3, 1, 1], // 98
        [1, 1, 3, 1, 4, 1], // 99
        [1, 1, 4, 1, 3, 1], // 100
        [3, 1, 1, 1, 4, 1], // 101
        [4, 1, 1, 1, 3, 1], // 102
        [2, 1, 1, 4, 1, 2], // 103
        [2, 1, 1, 2, 1, 4], // 104 (Start B)
        [2, 1, 1, 2, 3, 2], // 105
        [2, 3, 3, 1, 1, 1], // 106 (Stop)
    ];

    let p = if code < PATTERNS.len() {
        PATTERNS[code]
    } else {
        PATTERNS[0]
    };

    let mut bits = [false; 11];
    let mut idx = 0;
    let mut current_state = true;
    for &width in &p {
        for _ in 0..width {
            if idx < 11 {
                bits[idx] = current_state;
                idx += 1;
            }
        }
        current_state = !current_state;
    }
    bits
}

/// Standalone QR Code Matrix & SVG Generator.
///
/// Backed by the `qrcode` crate: a full ISO/IEC 18004 encoder with automatic
/// version selection, byte/kanji/alphanumeric mode selection, Reed–Solomon error
/// correction, format/version information and optimal mask selection. The matrix
/// produced here is genuinely scannable by any QR reader.
#[derive(Debug, Clone)]
pub struct QrCode {
    size: usize,
    modules: Vec<bool>,
}

impl QrCode {
    /// Encode a text payload into a real QR code matrix (error-correction level M).
    ///
    /// The version (matrix size) is chosen automatically by the encoder. If the
    /// payload exceeds the maximum QR capacity, a scannable QR encoding an
    /// explicit error marker is returned rather than an invalid matrix; use
    /// [`QrCode::try_encode`] to handle the error explicitly.
    pub fn encode(text: &str) -> Self {
        match Self::try_encode_with_ec_level(text, qrcode::EcLevel::M) {
            Ok(code) => code,
            Err(err) => {
                tracing::warn!(error = %err, "QR payload too large; emitting error marker QR");
                Self::try_encode_with_ec_level("ERROR: QR payload too large", qrcode::EcLevel::M)
                    .unwrap_or(Self {
                        size: 21,
                        modules: vec![false; 21 * 21],
                    })
            }
        }
    }

    /// Encode with an explicit ISO/IEC 18004 error-correction level
    /// (`L` 7%, `M` 15%, `Q` 25%, `H` 30%).
    pub fn encode_with_ec_level(text: &str, ec_level: qrcode::EcLevel) -> Self {
        Self::try_encode_with_ec_level(text, ec_level).unwrap_or_else(|err| {
            tracing::warn!(error = %err, "QR payload too large; emitting error marker QR");
            Self::try_encode_with_ec_level("ERROR: QR payload too large", qrcode::EcLevel::M)
                .unwrap_or(Self {
                    size: 21,
                    modules: vec![false; 21 * 21],
                })
        })
    }

    /// Fallible QR encoding. Returns the encoder error when the payload cannot
    /// fit into any standard QR version at the requested correction level.
    pub fn try_encode(text: &str) -> Result<Self, String> {
        Self::try_encode_with_ec_level(text, qrcode::EcLevel::M)
    }

    /// Fallible QR encoding with an explicit error-correction level.
    pub fn try_encode_with_ec_level(text: &str, ec_level: qrcode::EcLevel) -> Result<Self, String> {
        let code = qrcode::QrCode::with_error_correction_level(text, ec_level)
            .map_err(|err| err.to_string())?;
        let size = code.width();
        let modules = code
            .to_colors()
            .into_iter()
            .map(|color| color == qrcode::types::Color::Dark)
            .collect::<Vec<bool>>();
        debug_assert_eq!(modules.len(), size * size);
        Ok(Self { size, modules })
    }

    /// Matrix width in modules (21 for Version 1, up to 177 for Version 40).
    pub fn size(&self) -> usize {
        self.size
    }

    /// Raw row-major module matrix (`true` = dark module).
    pub fn modules(&self) -> &[bool] {
        &self.modules
    }

    /// Reference 7x7 finder pattern, kept as a public helper for tests and
    /// custom renderers.
    pub fn draw_finder_pattern(modules: &mut [bool], size: usize, r0: usize, c0: usize) {
        for dr in 0..7 {
            for dc in 0..7 {
                let is_border = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                let is_center = (2..=4).contains(&dr) && (2..=4).contains(&dc);
                modules[(r0 + dr) * size + (c0 + dc)] = is_border || is_center;
            }
        }
    }

    /// Reference 5x5 alignment pattern, kept as a public helper for tests and
    /// custom renderers.
    pub fn draw_alignment_pattern(
        modules: &mut [bool],
        size: usize,
        center_r: usize,
        center_c: usize,
    ) {
        for dr in 0..5 {
            for dc in 0..5 {
                let r = center_r - 2 + dr;
                let c = center_c - 2 + dc;
                let is_border = dr == 0 || dr == 4 || dc == 0 || dc == 4;
                let is_center = dr == 2 && dc == 2;
                modules[r * size + c] = is_border || is_center;
            }
        }
    }

    /// Whether a module belongs to a function pattern in a fixed-size reference
    /// matrix. Kept as a public helper for tests and custom renderers.
    pub fn is_function_pattern(r: usize, c: usize, size: usize) -> bool {
        // Top-left finder + separator
        if r <= 8 && c <= 8 {
            return true;
        }
        // Top-right finder + separator
        if r <= 8 && c >= size - 8 {
            return true;
        }
        // Bottom-left finder + separator
        if r >= size - 8 && c <= 8 {
            return true;
        }
        // Alignment pattern for Version 2 (centered at 18, 18)
        if size == 25 && (16..=20).contains(&r) && (16..=20).contains(&c) {
            return true;
        }
        // Timing patterns
        if r == 6 || c == 6 {
            return true;
        }
        false
    }

    /// Render the QR matrix as a standalone, scalable SVG document.
    ///
    /// The `viewBox` uses module coordinates (one unit per module) so adjacent
    /// dark modules join exactly with no sub-pixel gaps, and a four-module quiet
    /// zone is included as required by ISO/IEC 18004. Consecutive dark modules
    /// in a row are merged into one `<rect>` to keep the output small.
    pub fn svg(&self, pixel_size: u32) -> String {
        let margin = 4usize;
        let total_modules = self.size + margin * 2;
        let pixel_size = pixel_size.max(total_modules as u32);

        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total_modules} {total_modules}" width="{pixel_size}" height="{pixel_size}" shape-rendering="crispEdges">
  <rect width="{total_modules}" height="{total_modules}" fill="#ffffff" />
  <g fill="#000000">
"##
        );

        for r in 0..self.size {
            let mut c = 0;
            while c < self.size {
                if !self.modules[r * self.size + c] {
                    c += 1;
                    continue;
                }
                let start = c;
                while c < self.size && self.modules[r * self.size + c] {
                    c += 1;
                }
                svg.push_str(&format!(
                    r#"    <rect x="{}" y="{}" width="{}" height="1" />
"#,
                    start + margin,
                    r + margin,
                    c - start
                ));
            }
        }

        svg.push_str("  </g>\n</svg>");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barcode128_svg_generation() {
        let svg = Barcode128::svg("INV-2026-001", 60, 2);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn test_qr_code_matrix_and_svg() {
        let qr = QrCode::encode("https://yav.rs");
        let svg = qr.svg(240);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("width=\"240\" height=\"240\""));
        assert!(svg.contains("<rect"));
        // Version 1 = 21x21 + 4-module quiet zone on each side.
        assert!(qr.size() >= 21);
        assert_eq!(qr.modules().len(), qr.size() * qr.size());
        assert!(svg.contains(&format!(
            "viewBox=\"0 0 {} {}\"",
            qr.size() + 8,
            qr.size() + 8
        )));
    }

    #[test]
    fn test_qr_is_scannable_not_a_decorative_matrix() {
        // The previous implementation always produced a fixed 25x25 matrix with
        // no error-correction codewords. A real encoder must choose versions
        // based on payload length and must include the three finder patterns.
        let short = QrCode::encode("A");
        let long = QrCode::encode(&"x".repeat(300));
        assert!(short.size() < long.size());

        assert!(short.modules()[0], "top-left finder border");
        assert!(!short.modules()[short.size() + 1], "finder ring");
        assert!(short.modules()[3 * short.size() + 3], "finder centre");
        assert_ne!(
            short.modules()[6 * short.size() + 8],
            short.modules()[6 * short.size() + 9],
            "timing pattern must alternate"
        );
    }

    #[test]
    fn test_qr_payloads_differ_and_error_correction_levels_work() {
        let a = QrCode::encode("PAYLOAD-A");
        let b = QrCode::encode("PAYLOAD-B");
        assert_ne!(a.modules(), b.modules());

        let h = QrCode::encode_with_ec_level("HELLO", qrcode::EcLevel::H);
        let l = QrCode::encode_with_ec_level("HELLO", qrcode::EcLevel::L);
        assert!(h.size() >= l.size());
        assert!(QrCode::try_encode("valid").is_ok());
    }

    #[test]
    fn test_svg_merges_adjacent_modules_and_keeps_quiet_zone() {
        let qr = QrCode::encode("TEST");
        let svg = qr.svg(200);
        let rect_count = svg.matches("<rect").count();
        assert!(rect_count > 1);
        // Quiet zone: no dark module may be drawn in the first 4 module rows/cols.
        assert!(!svg.contains("y=\"0\""));
        assert!(!svg.contains("y=\"1\""));
        assert!(!svg.contains("x=\"0\""));
    }
}
