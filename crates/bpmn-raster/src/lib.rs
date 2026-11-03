//! SVG-to-PNG rasterization via resvg (SPEC-001 section 3 P0.5).
//!
//! PNG export takes explicit scale and background configuration and refuses
//! to allocate beyond a pixel budget (RISK-005).

use bpmn_scene::style::Rgba;
use resvg::tiny_skia;
use resvg::usvg;
use thiserror::Error;

/// PNG export configuration.
#[derive(Debug, Clone)]
pub struct PngOptions {
    /// Positive finite scale factor applied to the SVG size.
    pub scale: f64,
    /// Background color painted before rendering (default: opaque white).
    pub background: Rgba,
    /// Maximum output width in pixels.
    pub max_width: u32,
    /// Maximum output height in pixels.
    pub max_height: u32,
    /// Maximum total pixels (width × height).
    pub max_pixels: u64,
}

impl Default for PngOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            background: Rgba::WHITE,
            max_width: 16384,
            max_height: 16384,
            max_pixels: 64 * 1024 * 1024,
        }
    }
}

/// Rasterization failure.
#[derive(Debug, Clone, Error)]
pub enum RasterError {
    /// The SVG could not be parsed.
    #[error("SVG input is invalid: {message}")]
    InvalidSvg {
        /// Parser message.
        message: String,
    },
    /// Scale must be positive and finite.
    #[error("scale must be a positive finite number, got {scale}")]
    InvalidScale {
        /// The rejected scale.
        scale: f64,
    },
    /// Output exceeds the configured pixel budget.
    #[error("output size {width}x{height} exceeds the configured pixel budget")]
    TooLarge {
        /// Rejected width.
        width: u32,
        /// Rejected height.
        height: u32,
    },
    /// PNG encoding failed.
    #[error("PNG encoding failed")]
    Encoding,
}

/// Renders SVG text into a PNG byte buffer.
pub fn render_png(svg: &str, options: &PngOptions) -> Result<Vec<u8>, RasterError> {
    let pixmap = render_pixmap(svg, options)?;
    pixmap.encode_png().map_err(|_| RasterError::Encoding)
}

/// Renders SVG text into a pixel buffer (exposed for tests).
pub fn render_pixmap(svg: &str, options: &PngOptions) -> Result<tiny_skia::Pixmap, RasterError> {
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err(RasterError::InvalidScale {
            scale: options.scale,
        });
    }

    let mut usvg_options = usvg::Options {
        font_family: "sans-serif".to_string(),
        ..usvg::Options::default()
    };
    {
        let fontdb = usvg_options.fontdb_mut();
        fontdb.load_system_fonts();
        fontdb.set_sans_serif_family("DejaVu Sans");
        fontdb.set_serif_family("DejaVu Serif");
    }
    let tree =
        usvg::Tree::from_str(svg, &usvg_options).map_err(|error| RasterError::InvalidSvg {
            message: error.to_string(),
        })?;

    let size = tree.size();
    let width = (size.width() as f64 * options.scale).round() as u32;
    let height = (size.height() as f64 * options.scale).round() as u32;
    if width == 0 || height == 0 {
        return Err(RasterError::InvalidSvg {
            message: "SVG has zero width or height".to_string(),
        });
    }
    if width > options.max_width
        || height > options.max_height
        || (width as u64) * (height as u64) > options.max_pixels
    {
        return Err(RasterError::TooLarge { width, height });
    }

    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or(RasterError::TooLarge { width, height })?;
    let background = tiny_skia::Color::from_rgba(
        options.background.r as f32 / 255.0,
        options.background.g as f32 / 255.0,
        options.background.b as f32 / 255.0,
        options.background.a as f32 / 255.0,
    )
    .unwrap_or_else(|| tiny_skia::Color::from_rgba8(255, 255, 255, 255));
    pixmap.fill(background);

    let transform = tiny_skia::Transform::from_scale(options.scale as f32, options.scale as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(pixmap)
}

/// Reads PNG IHDR dimensions without a full decode.
pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const HEADER: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || !bytes.starts_with(HEADER) {
        return None;
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50" viewBox="0 0 100 50">
  <rect id="bpmn-background" x="0" y="0" width="100" height="50" fill="#ffffff"/>
  <rect x="10" y="10" width="40" height="30" fill="#1a1a1a" stroke="none"/>
  <text x="10" y="45" font-family="sans-serif" font-size="12">t</text>
</svg>
"##;

    #[test]
    fn scales_and_reports_dimensions() {
        let one_x = render_png(SVG, &PngOptions::default()).expect("1x");
        assert_eq!(png_dimensions(&one_x), Some((100, 50)));
        let two_x = render_png(
            SVG,
            &PngOptions {
                scale: 2.0,
                ..PngOptions::default()
            },
        )
        .expect("2x");
        assert_eq!(png_dimensions(&two_x), Some((200, 100)));
    }

    #[test]
    fn pixels_are_not_empty() {
        let pixmap = render_pixmap(SVG, &PngOptions::default()).expect("pixmap");
        let white =
            tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).expect("opaque white");
        let mut non_background = 0;
        for pixel in pixmap.pixels() {
            if *pixel != white {
                non_background += 1;
            }
        }
        assert!(
            non_background > 100,
            "expected drawn content, got {non_background} px"
        );
    }

    #[test]
    fn rejects_invalid_scale() {
        let error = render_png(
            SVG,
            &PngOptions {
                scale: 0.0,
                ..PngOptions::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("scale"));
    }

    #[test]
    fn enforces_pixel_budget() {
        let error = render_png(
            SVG,
            &PngOptions {
                scale: 1000.0,
                max_width: 100,
                ..PngOptions::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, RasterError::TooLarge { .. }));
    }

    #[test]
    fn invalid_svg_errors() {
        assert!(render_png("<not-svg/>", &PngOptions::default()).is_err());
    }
}
