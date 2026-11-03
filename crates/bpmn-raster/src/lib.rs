//! SVG-to-PNG rasterization via resvg (SPEC-001 section 3 P0.5).
//!
//! PNG export takes explicit scale and background configuration and refuses
//! to allocate beyond a pixel budget (RISK-005).

use bpmn_model::diagnostic::{Diagnostic, DiagnosticCode};
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

/// PNG output plus non-fatal rasterization diagnostics.
#[derive(Debug, Clone)]
pub struct PngOutput {
    /// Encoded PNG bytes.
    pub bytes: Vec<u8>,
    /// Font availability and substitution warnings (see [`configure_fonts`]).
    pub diagnostics: Vec<Diagnostic>,
}

/// Preferred `sans-serif` families in priority order. ADR-0004 commits no font
/// assets, so output maps `sans-serif` to the best installed family; the order
/// keeps reference-toolchain output stable (EXP-002).
const SANS_FALLBACKS: [&str; 4] = ["DejaVu Sans", "Liberation Sans", "Noto Sans", "Arial"];

/// Preferred `serif` families in priority order.
const SERIF_FALLBACKS: [&str; 4] = [
    "DejaVu Serif",
    "Liberation Serif",
    "Noto Serif",
    "Times New Roman",
];

/// Loads system fonts and maps the SVG generic families to installed ones.
///
/// Without this, a missing preferred family makes usvg drop text nodes
/// silently (no glyphs are ever drawn and no error is raised). The chosen
/// family is reported as a [`DiagnosticCode::RasterFontSubstituted`] warning
/// when it is not the top preference, and an empty font database warns
/// [`DiagnosticCode::RasterMissingFonts`].
fn configure_fonts(options: &mut usvg::Options) -> Vec<Diagnostic> {
    let fontdb = options.fontdb_mut();
    fontdb.load_system_fonts();
    configure_loaded_fontdb(fontdb)
}

/// Maps generic families on an already-loaded database; separated from
/// [`configure_fonts`] so the empty-database behavior stays unit-testable.
fn configure_loaded_fontdb(fontdb: &mut usvg::fontdb::Database) -> Vec<Diagnostic> {
    let families = available_families(fontdb);
    let mut diagnostics = Vec::new();
    if families.is_empty() {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::RasterMissingFonts,
            "no fonts found on this system; text will not be rasterized",
        ));
        return diagnostics;
    }
    for (generic, preferred) in [("sans-serif", &SANS_FALLBACKS), ("serif", &SERIF_FALLBACKS)] {
        let Some(family) = pick_family(families.iter().map(String::as_str), preferred) else {
            continue;
        };
        if generic == "sans-serif" {
            fontdb.set_sans_serif_family(family);
        } else {
            fontdb.set_serif_family(family);
        }
        if !preferred
            .first()
            .is_some_and(|name| name.eq_ignore_ascii_case(family))
        {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::RasterFontSubstituted,
                format!(
                    "{generic} resolved to {family:?}; none of the preferred fonts ({}) is installed",
                    preferred.join(", ")
                ),
            ));
        }
    }
    diagnostics
}

/// Installed family names, sorted case-insensitively and deduplicated so that
/// last-resort selection is deterministic.
fn available_families(fontdb: &usvg::fontdb::Database) -> Vec<String> {
    let mut families: Vec<String> = fontdb
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();
    families.sort_unstable_by_key(|family| family.to_lowercase());
    families.dedup();
    families
}

/// Returns the first preferred family present in `available`; otherwise the
/// smallest available family (emoji faces excluded); `None` when nothing
/// usable is installed.
fn pick_family<'a>(
    available: impl IntoIterator<Item = &'a str>,
    preferred: &[&str],
) -> Option<&'a str> {
    let mut families: Vec<&str> = available
        .into_iter()
        .filter(|family| !family.to_ascii_lowercase().contains("emoji"))
        .collect();
    families.sort_unstable_by_key(|family| family.to_ascii_lowercase());
    families.dedup();
    for candidate in preferred {
        if let Some(found) = families
            .iter()
            .find(|family| family.eq_ignore_ascii_case(candidate))
        {
            return Some(found);
        }
    }
    families.first().copied()
}

/// Renders SVG text into a PNG byte buffer.
///
/// Font diagnostics are discarded; use [`render_png_with_diagnostics`] when
/// they must reach the caller.
pub fn render_png(svg: &str, options: &PngOptions) -> Result<Vec<u8>, RasterError> {
    render_png_with_diagnostics(svg, options).map(|output| output.bytes)
}

/// Renders SVG text into a PNG byte buffer plus non-fatal font diagnostics.
pub fn render_png_with_diagnostics(
    svg: &str,
    options: &PngOptions,
) -> Result<PngOutput, RasterError> {
    let (pixmap, diagnostics) = render_pixmap_with_diagnostics(svg, options)?;
    let bytes = pixmap.encode_png().map_err(|_| RasterError::Encoding)?;
    Ok(PngOutput { bytes, diagnostics })
}

/// Renders SVG text into a pixel buffer, discarding font diagnostics.
pub fn render_pixmap(svg: &str, options: &PngOptions) -> Result<tiny_skia::Pixmap, RasterError> {
    render_pixmap_with_diagnostics(svg, options).map(|(pixmap, _)| pixmap)
}

/// Renders SVG text into a pixel buffer, reporting font availability and
/// substitution as warnings alongside the pixmap.
pub fn render_pixmap_with_diagnostics(
    svg: &str,
    options: &PngOptions,
) -> Result<(tiny_skia::Pixmap, Vec<Diagnostic>), RasterError> {
    if !options.scale.is_finite() || options.scale <= 0.0 {
        return Err(RasterError::InvalidScale {
            scale: options.scale,
        });
    }

    let mut usvg_options = usvg::Options {
        font_family: "sans-serif".to_string(),
        ..usvg::Options::default()
    };
    let diagnostics = configure_fonts(&mut usvg_options);
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
    Ok((pixmap, diagnostics))
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
    use bpmn_model::diagnostic::Severity;

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

    #[test]
    fn pick_family_follows_preferred_order() {
        let available = ["Noto Sans", "DejaVu Sans", "Arial"];
        assert_eq!(pick_family(available, &SANS_FALLBACKS), Some("DejaVu Sans"));
    }

    #[test]
    fn pick_family_falls_back_to_smallest_installed() {
        let available = ["iA Writer Duo", "Noto Color Emoji", "Noto Sans"];
        assert_eq!(pick_family(available, &SANS_FALLBACKS), Some("Noto Sans"));
    }

    #[test]
    fn pick_family_is_none_without_fonts() {
        let available: [&str; 0] = [];
        assert_eq!(pick_family(available, &SANS_FALLBACKS), None);
    }

    #[test]
    fn empty_font_database_warns_instead_of_dropping_text() {
        let mut database = usvg::fontdb::Database::new();
        let diagnostics = configure_loaded_fontdb(&mut database);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_str(), "raster.missing_fonts");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn png_with_diagnostics_reports_only_warnings_and_valid_png() {
        let output = render_png_with_diagnostics(SVG, &PngOptions::default()).expect("png");
        assert_eq!(png_dimensions(&output.bytes), Some((100, 50)));
        assert!(
            output
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.severity == Severity::Warning)
        );
        assert!(output.diagnostics.iter().all(|diagnostic| matches!(
            diagnostic.code,
            DiagnosticCode::RasterMissingFonts | DiagnosticCode::RasterFontSubstituted
        )));
    }
}
