//! Deterministic scene-to-SVG serialization (SPEC-001 section 4).
//!
//! Output is byte-for-byte stable for the same scene: no timestamps, no
//! environment data, fixed attribute order, trimmed numbers, escaped text.
//! The writer is owned (no third-party SVG crate) per SPEC-001 section 7.

pub mod serialize;

pub use serialize::to_svg;

/// Formats a number with up to three decimal places, trimming trailing
/// zeros and avoiding negative zero.
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let mut text = format!("{value:.3}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

/// XML-escapes text content.
pub fn escape_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

/// XML-escapes attribute values (includes quotes).
pub fn escape_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_trimmed_and_stable() {
        assert_eq!(format_number(10.0), "10");
        assert_eq!(format_number(10.500), "10.5");
        assert_eq!(format_number(-0.00001), "0");
        assert_eq!(format_number(-12.25), "-12.25");
        assert_eq!(format_number(f64::NAN), "0");
        assert_eq!(format_number(160.0), "160");
    }

    #[test]
    fn text_and_attributes_are_escaped() {
        assert_eq!(escape_text("a & b < c"), "a &amp; b &lt; c");
        assert_eq!(
            escape_attribute("say \"hi\" & 'bye'"),
            "say &quot;hi&quot; &amp; &#39;bye&#39;"
        );
    }
}
