//! Rolling historical memory sparkline rendering.
//!
//! Converts a sequence of historical utilization ratios into a compact,
//! colorized Unicode sparkline (using ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█').

use crate::color::{interpolate_color, rgb_to_ansi, severity_color, GradientStop};

const SPARK_GLYPHS: [char; 8] = ['·', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sequence of values in [0.0, 1.0] into a colorized sparkline string.
///
/// `history` contains recent samples (oldest to newest).
/// `width` specifies how many columns to render (samples are truncated or resampled).
pub fn render_sparkline(
    history: &[f64],
    width: usize,
    stops: &[GradientStop],
    good: &str,
    warning: &str,
    critical: &str,
    reset: &str,
) -> String {
    if width == 0 || history.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(width * 20);

    // Take the most recent `width` samples (or pad with initial value if fewer)
    let visible_samples: Vec<f64> = if history.len() >= width {
        history[history.len() - width..].to_vec()
    } else {
        let padding_needed = width - history.len();
        let mut v = vec![history[0]; padding_needed];
        v.extend_from_slice(history);
        v
    };

    // Calculate dynamic range with generous top and bottom padding
    let min_val = visible_samples
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max_val = visible_samples
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let mid = (min_val + max_val) / 2.0;
    let half_span = ((max_val - min_val) / 2.0).max(0.035);
    let pad_margin = half_span * 0.65;

    let floor = (mid - half_span - pad_margin).max(0.0);
    let ceil = (mid + half_span + pad_margin).min(1.0);
    let range = (ceil - floor).max(0.04);

    for (i, &ratio) in visible_samples.iter().enumerate() {
        let ratio = ratio.clamp(0.0, 1.0);
        let norm = ((ratio - floor) / range).clamp(0.0, 1.0);
        let glyph_idx = ((norm * 7.0).round() as usize).clamp(0, 7);
        let glyph = SPARK_GLYPHS[glyph_idx];

        let color = if !stops.is_empty() {
            if let Some(rgb) = interpolate_color(stops, (i as f64 + 0.5) / width as f64) {
                rgb_to_ansi(rgb)
            } else {
                severity_color(ratio, good, warning, critical).to_string()
            }
        } else {
            severity_color(ratio, good, warning, critical).to_string()
        };

        out.push_str(&color);
        out.push(glyph);
    }

    out.push_str(reset);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_sparkline() {
        let s = render_sparkline(&[], 10, &[], "", "", "", "");
        assert!(s.is_empty());
    }

    #[test]
    fn test_zero_width_sparkline() {
        let s = render_sparkline(&[0.5], 0, &[], "", "", "", "");
        assert!(s.is_empty());
    }

    #[test]
    fn test_sparkline_glyphs() {
        let history = vec![0.0, 0.14, 0.28, 0.42, 0.57, 0.71, 0.85, 1.0];
        let s = render_sparkline(&history, 8, &[], "", "", "", "");
        assert!(s.contains('·'));
        assert!(s.contains('█'));
    }

    #[test]
    fn test_sparkline_padding() {
        let history = vec![0.5];
        let s = render_sparkline(&history, 5, &[], "", "", "", "");
        // Should produce 5 glyphs plus reset
        assert_eq!(s.chars().filter(|&c| SPARK_GLYPHS.contains(&c)).count(), 5);
    }
}
