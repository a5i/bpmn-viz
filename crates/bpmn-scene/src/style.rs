//! Paints, styles, text styles, viewports and marker definitions.

/// An sRGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
    /// Alpha channel (0–255).
    pub a: u8,
}

impl Rgba {
    /// Creates an opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Creates a color with alpha.
    #[allow(clippy::same_name_method)]
    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque black.
    pub const BLACK: Rgba = Rgba::rgb(0, 0, 0);

    /// Opaque white.
    pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);

    /// CSS serialization: `#rrggbb` when opaque, `rgba(...)` otherwise.
    pub fn css(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({},{},{},{})",
                self.r,
                self.g,
                self.b,
                self.a as f64 / 255.0
            )
        }
    }
}

/// Fill or stroke paint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Paint {
    /// Solid color.
    Color(Rgba),
    /// Explicitly no paint.
    None,
}

impl Paint {
    /// CSS value: `none` or the color.
    pub fn css(&self) -> String {
        match self {
            Paint::Color(color) => color.css(),
            Paint::None => "none".to_string(),
        }
    }
}

/// Marker orientation rule (SVG `orient`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orient {
    /// Auto-rotate to path direction.
    Auto,
}

impl Orient {
    /// CSS value.
    pub fn css(&self) -> &'static str {
        match self {
            Orient::Auto => "auto",
        }
    }
}

/// A reusable `<marker>` definition (for example an arrowhead).
#[derive(Debug, Clone)]
pub struct Marker {
    /// Stable definition id.
    pub id: String,
    /// Marker coordinate box side (square).
    pub view_box_size: f64,
    /// Reference point x inside the marker box.
    pub ref_x: f64,
    /// Reference point y inside the marker box.
    pub ref_y: f64,
    /// Marker width in stroke units.
    pub marker_width: f64,
    /// Marker height in stroke units.
    pub marker_height: f64,
    /// Orientation rule.
    pub orient: Orient,
    /// Path drawn inside the marker.
    pub data: kurbo::BezPath,
    /// Marker fill style.
    pub fill: Paint,
}

/// Shape paint style.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    /// Fill paint; `None` value means explicitly no fill.
    pub fill: Option<Paint>,
    /// Stroke paint; `None` value means explicitly no stroke.
    pub stroke: Option<Paint>,
    /// Stroke width in user units.
    pub stroke_width: f64,
    /// Dash pattern in user units.
    pub dash_array: Option<Vec<f64>>,
    /// Marker id drawn at the end of the path.
    pub marker_end: Option<String>,
    /// Marker id drawn at the start of the path.
    pub marker_start: Option<String>,
}

/// Font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// Normal weight.
    #[default]
    Normal,
    /// Bold weight.
    Bold,
}

impl FontWeight {
    /// CSS value.
    pub fn css(&self) -> &'static str {
        match self {
            FontWeight::Normal => "normal",
            FontWeight::Bold => "bold",
        }
    }
}

/// Text style.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font family list (CSS-like, comma separated).
    pub family: String,
    /// Font size in user units.
    pub size: f64,
    /// Font weight.
    pub weight: FontWeight,
    /// Text fill color.
    pub fill: Rgba,
    /// Italic flag.
    pub italic: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            family: "sans-serif".to_string(),
            size: 12.0,
            weight: FontWeight::Normal,
            fill: Rgba::BLACK,
            italic: false,
        }
    }
}

/// The coordinate box a scene is drawn into.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Viewport {
    /// Viewport min x.
    pub x: f64,
    /// Viewport min y.
    pub y: f64,
    /// Viewport width.
    pub width: f64,
    /// Viewport height.
    pub height: f64,
}

impl Viewport {
    /// Creates a viewport.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}
