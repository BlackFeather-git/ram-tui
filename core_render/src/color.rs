//! Gradient colour interpolation and TrueColor ANSI helpers.

/// A single gradient stop: (position 0.0–1.0, (r, g, b)).
pub type GradientStop = (f64, (u8, u8, u8));

/// Interpolate an RGB colour from a list of gradient stops at `pos` ∈ [0.0, 1.0].
pub fn interpolate_color(stops: &[GradientStop], pos: f64) -> Option<(u8, u8, u8)> {
    if stops.is_empty() {
        return None;
    }
    let pos = pos.clamp(0.0, 1.0);
    if stops.len() == 1 || pos <= stops[0].0 {
        return Some(stops[0].1);
    }
    if pos >= stops[stops.len() - 1].0 {
        return Some(stops[stops.len() - 1].1);
    }
    for i in 0..stops.len() - 1 {
        let (p1, c1) = stops[i];
        let (p2, c2) = stops[i + 1];
        if p1 <= pos && pos <= p2 {
            let span = p2 - p1;
            if span <= 0.0 {
                return Some(c1);
            }
            let t = (pos - p1) / span;
            let r = (c1.0 as f64 + (c2.0 as f64 - c1.0 as f64) * t).round() as u8;
            let g = (c1.1 as f64 + (c2.1 as f64 - c1.1 as f64) * t).round() as u8;
            let b = (c1.2 as f64 + (c2.2 as f64 - c1.2 as f64) * t).round() as u8;
            return Some((r, g, b));
        }
    }
    Some(stops[stops.len() - 1].1)
}

/// Format an RGB tuple as a 24-bit TrueColor ANSI foreground sequence.
pub fn rgb_to_ansi(rgb: (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2)
}

/// Severity-based colour selection for meters or sparklines without gradients.
pub fn severity_color<'a>(
    ratio: f64,
    good: &'a str,
    warning: &'a str,
    critical: &'a str,
) -> &'a str {
    if ratio < 0.60 {
        good
    } else if ratio < 0.85 {
        warning
    } else {
        critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_empty() {
        assert_eq!(interpolate_color(&[], 0.5), None);
    }

    #[test]
    fn test_interpolate_single() {
        let stops = vec![(0.5, (100u8, 200u8, 50u8))];
        assert_eq!(interpolate_color(&stops, 0.0), Some((100, 200, 50)));
        assert_eq!(interpolate_color(&stops, 1.0), Some((100, 200, 50)));
    }

    #[test]
    fn test_interpolate_bounds() {
        let stops = vec![(0.0, (0u8, 0u8, 0u8)), (1.0, (100u8, 200u8, 50u8))];
        assert_eq!(interpolate_color(&stops, -0.5), Some((0, 0, 0)));
        assert_eq!(interpolate_color(&stops, 0.5), Some((50, 100, 25)));
        assert_eq!(interpolate_color(&stops, 1.5), Some((100, 200, 50)));
    }

    #[test]
    fn test_rgb_to_ansi() {
        assert_eq!(rgb_to_ansi((255, 0, 128)), "\x1b[38;2;255;0;128m");
    }
}
