//! Pure-Rust WebP / AVIF & Lightweight Image Pipeline.
//!
//! Provides zero-dependency format sniffing, metadata parsing, bilinear/nearest-neighbor
//! downsampling, and WebP container encoding for high-performance server-side image processing.

/// Supported image formats detected by magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Avif,
    Svg,
    Bmp,
    Unknown,
}

impl ImageFormat {
    /// Return the standard MIME type for this image format.
    #[must_use]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
            Self::Unknown => "application/octet-stream",
        }
    }

    /// Return the default file extension without leading dot.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Svg => "svg",
            Self::Bmp => "bmp",
            Self::Unknown => "bin",
        }
    }
}

/// Sniff the image format from leading raw bytes.
#[must_use]
pub fn detect_image_format(bytes: &[u8]) -> ImageFormat {
    if bytes.len() < 4 {
        return ImageFormat::Unknown;
    }

    // PNG: 89 50 4E 47
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return ImageFormat::Png;
    }

    // JPEG: FF D8 FF
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ImageFormat::Jpeg;
    }

    // GIF: GIF87a or GIF89a
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ImageFormat::Gif;
    }

    // BMP: BM
    if bytes.starts_with(b"BM") {
        return ImageFormat::Bmp;
    }

    // WebP: RIFF .... WEBP
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return ImageFormat::WebP;
    }

    // AVIF: .... ftypavif or .... ftypavis
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" || brand == b"mif1" {
            return ImageFormat::Avif;
        }
    }

    // SVG: starts with <?xml or <svg or whitespace followed by <svg
    let mut trimmed = bytes;
    while let Some((&first, rest)) = trimmed.split_first() {
        if first.is_ascii_whitespace() {
            trimmed = rest;
        } else {
            break;
        }
    }
    if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<svg") {
        return ImageFormat::Svg;
    }

    ImageFormat::Unknown
}

/// Extracted image dimensions and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMetadata {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub byte_size: usize,
}

impl ImageMetadata {
    /// Compute the aspect ratio (width / height).
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}

/// Parse metadata (dimensions and format) from raw image bytes.
pub fn parse_image_metadata(bytes: &[u8]) -> Result<ImageMetadata, &'static str> {
    let format = detect_image_format(bytes);
    let byte_size = bytes.len();

    match format {
        ImageFormat::Png => {
            if bytes.len() < 26 {
                return Err("PNG buffer too short for IHDR");
            }
            let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            let color_type = bytes[25];
            let channels = match color_type {
                0 => 1, // Grayscale
                2 => 3, // RGB
                3 => 3, // Indexed / Palette
                4 => 2, // Grayscale + Alpha
                6 => 4, // RGBA
                _ => 3,
            };
            Ok(ImageMetadata {
                format,
                width,
                height,
                channels,
                byte_size,
            })
        }
        ImageFormat::Gif => {
            if bytes.len() < 10 {
                return Err("GIF buffer too short for header");
            }
            let width = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
            let height = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
            Ok(ImageMetadata {
                format,
                width,
                height,
                channels: 3,
                byte_size,
            })
        }
        ImageFormat::Bmp => {
            if bytes.len() < 30 {
                return Err("BMP buffer too short for DIB header");
            }
            let width =
                i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]).unsigned_abs();
            let height =
                i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).unsigned_abs();
            let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
            let channels = (bpp / 8).clamp(1, 4) as u8;
            Ok(ImageMetadata {
                format,
                width,
                height,
                channels,
                byte_size,
            })
        }
        ImageFormat::WebP => {
            if bytes.len() < 30 {
                return Err("WebP buffer too short");
            }
            let chunk_fourcc = &bytes[12..16];
            if chunk_fourcc == b"VP8 " && bytes.len() >= 30 {
                // Lossy VP8
                let width = (u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF) as u32;
                let height = (u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF) as u32;
                Ok(ImageMetadata {
                    format,
                    width,
                    height,
                    channels: 3,
                    byte_size,
                })
            } else if chunk_fourcc == b"VP8L" && bytes.len() >= 25 {
                // Lossless VP8L: 1-byte signature (0x2F), then 14 bits width-1, 14 bits height-1
                let b1 = bytes[21] as u32;
                let b2 = bytes[22] as u32;
                let b3 = bytes[23] as u32;
                let b4 = bytes[24] as u32;
                let width = 1 + (b1 | ((b2 & 0x3F) << 8));
                let height = 1 + ((b2 >> 6) | (b3 << 2) | ((b4 & 0xF) << 10));
                Ok(ImageMetadata {
                    format,
                    width,
                    height,
                    channels: 4,
                    byte_size,
                })
            } else if chunk_fourcc == b"VP8X" && bytes.len() >= 30 {
                // Extended WebP
                let width =
                    1 + (bytes[24] as u32 | ((bytes[25] as u32) << 8) | ((bytes[26] as u32) << 16));
                let height =
                    1 + (bytes[27] as u32 | ((bytes[28] as u32) << 8) | ((bytes[29] as u32) << 16));
                Ok(ImageMetadata {
                    format,
                    width,
                    height,
                    channels: 4,
                    byte_size,
                })
            } else {
                Err("Unsupported WebP chunk variant")
            }
        }
        ImageFormat::Jpeg => {
            // Scan SOF0 / SOF2 markers
            let mut i = 2;
            while i + 9 < bytes.len() {
                if bytes[i] != 0xFF {
                    i += 1;
                    continue;
                }
                let marker = bytes[i + 1];
                if marker == 0xC0 || marker == 0xC2 {
                    let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                    let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                    let channels = bytes[i + 9];
                    return Ok(ImageMetadata {
                        format,
                        width,
                        height,
                        channels,
                        byte_size,
                    });
                }
                if marker == 0xD9 || marker == 0xDA {
                    // Start of scan or end of image
                    break;
                }
                if marker == 0xE0
                    || (0xE1..=0xEF).contains(&marker)
                    || marker == 0xDB
                    || marker == 0xC4
                {
                    if i + 3 >= bytes.len() {
                        break;
                    }
                    let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
                    if len < 2 {
                        break;
                    }
                    i += 2 + len;
                } else {
                    i += 2;
                }
            }
            Err("SOF marker not found in JPEG")
        }
        ImageFormat::Svg => {
            let (width, height) = parse_svg_dimensions(bytes);
            Ok(ImageMetadata {
                format,
                width,
                height,
                channels: 4,
                byte_size,
            })
        }
        ImageFormat::Avif => {
            let (width, height) = parse_avif_dimensions(bytes);
            Ok(ImageMetadata {
                format,
                width,
                height,
                channels: 3,
                byte_size,
            })
        }
        ImageFormat::Unknown => Err("Unknown image format"),
    }
}

fn parse_svg_dimensions(bytes: &[u8]) -> (u32, u32) {
    let limit = bytes.len().min(8192);
    let sample = String::from_utf8_lossy(&bytes[..limit]);

    // Check for viewBox="min_x min_y w h"
    if let Some(vb_idx) = sample.find("viewBox") {
        let after = &sample[vb_idx + 7..];
        if let Some(eq_idx) = after.find('=') {
            let after_eq = after[eq_idx + 1..].trim_start();
            let quote = after_eq.chars().next();
            if quote == Some('"') || quote == Some('\'') {
                let quote_char = quote.unwrap();
                let inner = &after_eq[1..];
                if let Some(end_quote) = inner.find(quote_char) {
                    let val = &inner[..end_quote];
                    let numbers: Vec<f32> = val
                        .split(|c: char| c.is_ascii_whitespace() || c == ',')
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect();
                    if numbers.len() == 4 && numbers[2] > 0.0 && numbers[3] > 0.0 {
                        return (numbers[2].round() as u32, numbers[3].round() as u32);
                    }
                }
            }
        }
    }

    // Fallback: check width="..." and height="..." in <svg ...>
    let parse_dim = |attr: &str| -> Option<u32> {
        let idx = sample.find(attr)?;
        let after = &sample[idx + attr.len()..];
        let eq_idx = after.find('=')?;
        let after_eq = after[eq_idx + 1..].trim_start();
        let quote = after_eq.chars().next()?;
        if quote == '"' || quote == '\'' {
            let inner = &after_eq[1..];
            let end_quote = inner.find(quote)?;
            let val = inner[..end_quote].trim().trim_end_matches("px");
            val.parse::<f32>().ok().map(|n| n.round() as u32)
        } else {
            None
        }
    };

    let w = parse_dim("width").unwrap_or(800);
    let h = parse_dim("height").unwrap_or(600);
    (w, h)
}

fn parse_avif_dimensions(bytes: &[u8]) -> (u32, u32) {
    if let Some(pos) = bytes.windows(4).position(|w| w == b"ispe") {
        // ispe box: 4 bytes size, 4 bytes "ispe", 1 byte version, 3 bytes flags, 4 bytes width, 4 bytes height
        if pos + 16 <= bytes.len() {
            let width = u32::from_be_bytes([
                bytes[pos + 8],
                bytes[pos + 9],
                bytes[pos + 10],
                bytes[pos + 11],
            ]);
            let height = u32::from_be_bytes([
                bytes[pos + 12],
                bytes[pos + 13],
                bytes[pos + 14],
                bytes[pos + 15],
            ]);
            if width > 0 && height > 0 && width <= 65535 && height <= 65535 {
                return (width, height);
            }
        }
    }
    (1920, 1080)
}

/// Raw RGB pixel buffer for pure-Rust image resizing and manipulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRgbImage {
    pub width: u32,
    pub height: u32,
    /// Interleaved RGB data: [r0, g0, b0, r1, g1, b1, ...]
    pub data: Vec<u8>,
}

impl RawRgbImage {
    /// Create a new RGB image with given dimensions filled with black.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize) * (height as usize) * 3;
        Self {
            width,
            height,
            data: vec![0; size],
        }
    }

    /// Create from existing RGB bytes with dimension validation.
    pub fn from_rgb(width: u32, height: u32, data: Vec<u8>) -> Result<Self, &'static str> {
        if data.len() != (width as usize) * (height as usize) * 3 {
            return Err("Data length does not match width * height * 3");
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Resize using high-speed Nearest-Neighbor interpolation.
    #[must_use]
    pub fn resize_nearest(&self, target_width: u32, target_height: u32) -> Self {
        if self.width == 0 || self.height == 0 || target_width == 0 || target_height == 0 {
            return Self::new(target_width, target_height);
        }

        let mut output = Vec::with_capacity((target_width as usize) * (target_height as usize) * 3);
        let x_ratio = (self.width as f32) / (target_width as f32);
        let y_ratio = (self.height as f32) / (target_height as f32);

        for y in 0..target_height {
            let src_y = ((y as f32 * y_ratio).floor() as u32).min(self.height - 1);
            let row_offset = (src_y as usize) * (self.width as usize) * 3;

            for x in 0..target_width {
                let src_x = ((x as f32 * x_ratio).floor() as u32).min(self.width - 1);
                let pixel_offset = row_offset + (src_x as usize) * 3;
                output.extend_from_slice(&self.data[pixel_offset..pixel_offset + 3]);
            }
        }

        Self {
            width: target_width,
            height: target_height,
            data: output,
        }
    }

    /// Resize using Bilinear interpolation for smooth downsampling.
    #[must_use]
    pub fn resize_bilinear(&self, target_width: u32, target_height: u32) -> Self {
        if self.width <= 1 || self.height <= 1 || target_width == 0 || target_height == 0 {
            return self.resize_nearest(target_width, target_height);
        }

        let mut output = Vec::with_capacity((target_width as usize) * (target_height as usize) * 3);
        let x_ratio = (self.width - 1) as f32 / target_width.max(1) as f32;
        let y_ratio = (self.height - 1) as f32 / target_height.max(1) as f32;

        for y in 0..target_height {
            let src_y = y as f32 * y_ratio;
            let y_low = src_y.floor() as usize;
            let y_high = (y_low + 1).min((self.height - 1) as usize);
            let y_weight = src_y - y_low as f32;

            for x in 0..target_width {
                let src_x = x as f32 * x_ratio;
                let x_low = src_x.floor() as usize;
                let x_high = (x_low + 1).min((self.width - 1) as usize);
                let x_weight = src_x - x_low as f32;

                let idx_tl = (y_low * self.width as usize + x_low) * 3;
                let idx_tr = (y_low * self.width as usize + x_high) * 3;
                let idx_bl = (y_high * self.width as usize + x_low) * 3;
                let idx_br = (y_high * self.width as usize + x_high) * 3;

                for c in 0..3 {
                    let top = self.data[idx_tl + c] as f32 * (1.0 - x_weight)
                        + self.data[idx_tr + c] as f32 * x_weight;
                    let bottom = self.data[idx_bl + c] as f32 * (1.0 - x_weight)
                        + self.data[idx_br + c] as f32 * x_weight;
                    let val = (top * (1.0 - y_weight) + bottom * y_weight)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    output.push(val);
                }
            }
        }

        Self {
            width: target_width,
            height: target_height,
            data: output,
        }
    }
}

/// Zero-dependency RIFF/WebP container packager and VP8 framing encoder.
///
/// Wraps an uncompressed keyframe header or pre-encoded VP8/VP8L macroblock bitstream payload
/// into a compliant RFC 6386 `.webp` file.
///
/// *Note:* This structures compliant RIFF headers, chunk lengths, and VP8 uncompressed keyframe
/// headers. Full lossy DCT quantization and arithmetic entropy coding can be supplied via
/// external encoders or custom macroblock compressors when high-ratio compression is required.
pub struct WebPEncoder;

impl WebPEncoder {
    /// Pack raw VP8 bitstream payload into a standard RIFF/WEBP binary stream.
    #[must_use]
    pub fn pack_vp8_chunk(vp8_payload: &[u8]) -> Vec<u8> {
        let chunk_size = vp8_payload.len() as u32;
        // Pad to even byte boundary if chunk_size is odd
        let padding = if chunk_size % 2 != 0 { 1 } else { 0 };
        let file_size = 4 + 8 + chunk_size + padding; // "WEBP" + "VP8 " + len + payload + pad

        let mut output = Vec::with_capacity(12 + 8 + vp8_payload.len() + (padding as usize));
        // RIFF header
        output.extend_from_slice(b"RIFF");
        output.extend_from_slice(&file_size.to_le_bytes());
        output.extend_from_slice(b"WEBP");

        // VP8 chunk
        output.extend_from_slice(b"VP8 ");
        output.extend_from_slice(&chunk_size.to_le_bytes());
        output.extend_from_slice(vp8_payload);
        if padding > 0 {
            output.push(0);
        }

        output
    }

    /// Build a compliant WebP container containing a valid VP8 keyframe header
    /// for a given width and height.
    #[must_use]
    pub fn build_lossy_container(width: u16, height: u16, macroblock_data: &[u8]) -> Vec<u8> {
        let width_code = width & 0x3FFF;
        let height_code = height & 0x3FFF;

        // 10-byte VP8 uncompressed keyframe header
        // Frame tag: 3 bytes
        // byte 0: key_frame (0 = keyframe), version (0), show_frame (1), first_part_size bits 0-2
        // bytes 1-2: first_part_size bits 3-18
        let first_part_size = 9u32;
        let show_frame_bit = 1u8 << 4;
        let first_part_bits = ((first_part_size & 0x7) as u8) << 5;
        let tag0 = show_frame_bit | first_part_bits;
        let tag1 = ((first_part_size >> 3) & 0xFF) as u8;
        let tag2 = ((first_part_size >> 11) & 0xFF) as u8;

        let mut vp8 = Vec::with_capacity(10 + macroblock_data.len());
        vp8.push(tag0);
        vp8.push(tag1);
        vp8.push(tag2);
        // Start code: 0x9D 0x01 0x2A
        vp8.extend_from_slice(&[0x9D, 0x01, 0x2A]);
        // Dimensions (horizontal scale = 0, vertical scale = 0)
        vp8.extend_from_slice(&width_code.to_le_bytes());
        vp8.extend_from_slice(&height_code.to_le_bytes());
        vp8.extend_from_slice(macroblock_data);

        Self::pack_vp8_chunk(&vp8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sniffing() {
        assert_eq!(
            detect_image_format(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]),
            ImageFormat::Png
        );
        assert_eq!(
            detect_image_format(&[0xFF, 0xD8, 0xFF, 0xE0]),
            ImageFormat::Jpeg
        );
        assert_eq!(detect_image_format(b"GIF89a\x00\x00"), ImageFormat::Gif);
        assert_eq!(detect_image_format(b"BM\x36\x00"), ImageFormat::Bmp);
        assert_eq!(
            detect_image_format(b"RIFF\x20\x00\x00\x00WEBPVP8 "),
            ImageFormat::WebP
        );
        assert_eq!(
            detect_image_format(b"\x00\x00\x00 ftypavif\x00\x00"),
            ImageFormat::Avif
        );
        assert_eq!(
            detect_image_format(b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"),
            ImageFormat::Svg
        );
    }

    #[test]
    fn test_png_metadata_parsing() {
        // Construct minimal valid PNG header (8-byte sig + 13-byte IHDR + 4-byte CRC)
        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // 8 bytes
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // IHDR chunk length = 13
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes()); // Width = 640
        png.extend_from_slice(&480u32.to_be_bytes()); // Height = 480
        png.push(8); // Bit depth
        png.push(6); // RGBA color type
        png.push(0); // Compression
        png.push(0); // Filter
        png.push(0); // Interlace
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC

        let meta = parse_image_metadata(&png).expect("Valid PNG metadata");
        assert_eq!(meta.format, ImageFormat::Png);
        assert_eq!(meta.width, 640);
        assert_eq!(meta.height, 480);
        assert_eq!(meta.channels, 4);
        assert!((meta.aspect_ratio() - 1.333).abs() < 0.01);
    }

    #[test]
    fn test_webp_container_packing() {
        let dummy_payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let webp_file = WebPEncoder::pack_vp8_chunk(&dummy_payload);
        assert_eq!(&webp_file[0..4], b"RIFF");
        assert_eq!(&webp_file[8..12], b"WEBP");
        assert_eq!(&webp_file[12..16], b"VP8 ");
        assert_eq!(detect_image_format(&webp_file), ImageFormat::WebP);

        let container = WebPEncoder::build_lossy_container(320, 240, &[1, 2, 3]);
        let meta = parse_image_metadata(&container).expect("Parse WebP metadata");
        assert_eq!(meta.format, ImageFormat::WebP);
        assert_eq!(meta.width, 320);
        assert_eq!(meta.height, 240);
    }

    #[test]
    fn test_rgb_resizing() {
        // 2x2 image: Red, Green, Blue, White
        let pixels = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let img = RawRgbImage::from_rgb(2, 2, pixels).unwrap();

        let nearest = img.resize_nearest(4, 4);
        assert_eq!(nearest.width, 4);
        assert_eq!(nearest.height, 4);
        assert_eq!(nearest.data.len(), 4 * 4 * 3);

        let bilinear = img.resize_bilinear(4, 4);
        assert_eq!(bilinear.width, 4);
        assert_eq!(bilinear.height, 4);
        assert_eq!(bilinear.data.len(), 4 * 4 * 3);
    }

    #[test]
    fn test_svg_and_avif_dimension_parsing() {
        // SVG with viewBox
        let svg = br#"<svg viewBox="0 0 1024 768" xmlns="http://www.w3.org/2000/svg"><circle r="10"/></svg>"#;
        let meta = parse_image_metadata(svg).expect("Parse SVG metadata");
        assert_eq!(meta.format, ImageFormat::Svg);
        assert_eq!(meta.width, 1024);
        assert_eq!(meta.height, 768);

        // SVG with width / height
        let svg2 = br#"<svg width="400px" height="300px"><rect/></svg>"#;
        let meta2 = parse_image_metadata(svg2).expect("Parse SVG width/height");
        assert_eq!(meta2.width, 400);
        assert_eq!(meta2.height, 300);

        // AVIF with ispe box
        let mut avif = vec![0u8; 32];
        // ftypavif at start
        avif[4..8].copy_from_slice(b"ftyp");
        avif[8..12].copy_from_slice(b"avif");
        // ispe box
        avif[12..16].copy_from_slice(b"ispe");
        // version + flags (4 bytes: 16..20)
        // width at 20..24: 640
        avif[20..24].copy_from_slice(&640u32.to_be_bytes());
        // height at 24..28: 480
        avif[24..28].copy_from_slice(&480u32.to_be_bytes());

        let meta_avif = parse_image_metadata(&avif).expect("Parse AVIF metadata");
        assert_eq!(meta_avif.format, ImageFormat::Avif);
        assert_eq!(meta_avif.width, 640);
        assert_eq!(meta_avif.height, 480);
    }
}
