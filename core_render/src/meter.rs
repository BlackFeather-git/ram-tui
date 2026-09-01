//! Progress bar / meter track rendering with sub-character accuracy.

use crate::color::{interpolate_color, rgb_to_ansi, severity_color, GradientStop};

const BLOCK_FRACTIONS: &[&str] = &["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
const BLOCK_FULL: &str = "█";
const BLOCK_TRACK: char = '░';

/// Graph symbol style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphSymbol {
    Block,
    Braille,
}

impl GraphSymbol {
    pub fn cycle(self) -> Self {
        match self {
            Self::Block => Self::Braille,
            Self::Braille => Self::Block,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Braille => "braille",
        }
    }
}

/// Render a progress bar meter track.
///
/// `ratio` in [0.0, 1.0], `width` is the character width of the bar.
#[allow(clippy::too_many_arguments)]
pub fn render_meter_track(
    ratio: f64,
    width: usize,
    symbol: GraphSymbol,
    stops: &[GradientStop],
    color_override: Option<&str>,
    good: &str,
    warning: &str,
    critical: &str,
    track_color: &str,
    reset: &str,
) -> String {
    let ratio = ratio.clamp(0.0, 1.0);
    let width = width.max(1);
    let mut out = String::with_capacity(width * 20);

    match symbol {
        GraphSymbol::Braille => {
            let total_steps = (ratio * width as f64 * 2.0).round() as usize;
            let full_blocks = total_steps / 2;
            let rem = total_steps % 2;
            let empty = width
                .saturating_sub(full_blocks)
                .saturating_sub(if rem > 0 { 1 } else { 0 });

            for i in 0..full_blocks {
                let c = if let Some(co) = color_override {
                    co.to_string()
                } else if !stops.is_empty() {
                    if let Some(rgb) = interpolate_color(stops, (i as f64 + 0.5) / width as f64) {
                        rgb_to_ansi(rgb)
                    } else {
                        severity_color(ratio, good, warning, critical).to_string()
                    }
                } else {
                    severity_color(ratio, good, warning, critical).to_string()
                };
                out.push_str(&c);
                out.push('⣿');
            }
            if rem == 1 {
                let c = if let Some(co) = color_override {
                    co.to_string()
                } else if !stops.is_empty() {
                    if let Some(rgb) =
                        interpolate_color(stops, (full_blocks as f64 + 0.5) / width as f64)
                    {
                        rgb_to_ansi(rgb)
                    } else {
                        severity_color(ratio, good, warning, critical).to_string()
                    }
                } else {
                    severity_color(ratio, good, warning, critical).to_string()
                };
                out.push_str(&c);
                out.push('⡇');
            }
            out.push_str(reset);
            out.push_str(track_color);
            for _ in 0..empty {
                out.push(BLOCK_TRACK);
            }
            out.push_str(reset);
        }
        GraphSymbol::Block => {
            let total_eighths = (ratio * width as f64 * 8.0).round() as usize;
            let full_blocks = total_eighths / 8;
            let rem_eighths = total_eighths % 8;
            let empty = width
                .saturating_sub(full_blocks)
                .saturating_sub(if rem_eighths > 0 { 1 } else { 0 });

            for i in 0..full_blocks {
                let c = if let Some(co) = color_override {
                    co.to_string()
                } else if !stops.is_empty() {
                    if let Some(rgb) = interpolate_color(stops, (i as f64 + 0.5) / width as f64) {
                        rgb_to_ansi(rgb)
                    } else {
                        severity_color(ratio, good, warning, critical).to_string()
                    }
                } else {
                    severity_color(ratio, good, warning, critical).to_string()
                };
                out.push_str(&c);
                out.push_str(BLOCK_FULL);
            }
            if rem_eighths > 0 {
                let c = if let Some(co) = color_override {
                    co.to_string()
                } else if !stops.is_empty() {
                    if let Some(rgb) =
                        interpolate_color(stops, (full_blocks as f64 + 0.5) / width as f64)
                    {
                        rgb_to_ansi(rgb)
                    } else {
                        severity_color(ratio, good, warning, critical).to_string()
                    }
                } else {
                    severity_color(ratio, good, warning, critical).to_string()
                };
                out.push_str(&c);
                out.push_str(BLOCK_FRACTIONS[rem_eighths]);
            }
            out.push_str(reset);
            out.push_str(track_color);
            for _ in 0..empty {
                out.push(BLOCK_TRACK);
            }
            out.push_str(reset);
        }
    }
    out
}
