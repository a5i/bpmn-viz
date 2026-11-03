//! Text anchoring and approximate line wrapping.
//!
//! The prototype deliberately avoids font-metric dependencies (SPEC-001
//! section 2, RISK-002): wrapping uses a fixed average character width
//! ratio, which is deterministic across platforms. EXP-002 tracks the
//! experiment and its limits.

/// Horizontal text anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAnchor {
    /// Left edge at `x`.
    #[default]
    Start,
    /// Centered on `x`.
    Middle,
    /// Right edge at `x`.
    End,
}

impl TextAnchor {
    /// CSS value.
    pub fn css(&self) -> &'static str {
        match self {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        }
    }
}

/// Approximate width of `text` at `font_size` using the fixed ratio.
pub fn estimated_width(text: &str, font_size: f64) -> f64 {
    // Average glyph width for a proportional sans face; deterministic.
    const AVG_CHAR_WIDTH_RATIO: f64 = 0.6;
    let chars = text.chars().count() as f64;
    chars * font_size * AVG_CHAR_WIDTH_RATIO
}

/// Greedy word-wrap into lines each at most `max_width` wide. Long words
/// that do not fit on any line are kept whole (never split).
pub fn wrap_text(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let candidate_width = if current.is_empty() {
                estimated_width(word, font_size)
            } else {
                estimated_width(&format!("{current} {word}"), font_size)
            };
            if candidate_width <= max_width || current.is_empty() {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            }
        }
        if paragraph.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_respects_width() {
        let lines = wrap_text("one two three four", 70.0, 12.0);
        assert!(lines.len() >= 2);
        for line in &lines {
            assert!(estimated_width(line, 12.0) <= 70.0 + 1e-9 || line.split(' ').count() == 1);
        }
    }

    #[test]
    fn long_words_are_kept_whole() {
        let lines = wrap_text("extraordinarilylongidentifier", 10.0, 12.0);
        assert_eq!(lines, vec!["extraordinarilylongidentifier"]);
    }

    #[test]
    fn empty_text_wraps_to_one_empty_line() {
        assert_eq!(wrap_text("", 100.0, 12.0), vec![String::new()]);
    }

    #[test]
    fn explicit_newlines_are_respected() {
        let lines = wrap_text("a\nb", 100.0, 12.0);
        assert_eq!(lines, vec!["a", "b"]);
    }
}
