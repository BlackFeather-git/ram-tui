//! Rich colour themes with 24-bit TrueColor palettes and gradients.
//!
//! Mirrors all 13 themes from Python v0.7.0.

use core_render::color::GradientStop;

/// All available theme names in display order.
pub const THEME_NAMES: &[&str] = &[
    "default",
    "dracula",
    "catppuccin",
    "nord",
    "tokyo-night",
    "gruvbox",
    "cyberpunk",
    "rose-pine",
    "everforest",
    "kanagawa",
    "monokai",
    "solarized",
    "monochrome",
];

/// Display modes.
pub const DISPLAY_MODES: &[&str] = &["hero", "compact", "mini", "tiny"];
/// Cycling modes (tiny is excluded from interactive cycling).
pub const CYCLING_MODES: &[&str] = &["hero", "compact", "mini"];

/// A resolved colour palette with ANSI escape codes.
#[derive(Debug, Clone)]
pub struct Palette {
    pub reset: &'static str,
    pub bold: &'static str,
    pub dim: String,
    pub header: String,
    pub accent: String,
    pub good: String,
    pub warning: String,
    pub critical: String,
    pub muted: String,
    pub track: String,
    pub text: String,
    pub stops: Vec<GradientStop>,
}

impl Palette {
    pub fn monochrome() -> Self {
        Self {
            reset: "",
            bold: "",
            dim: String::new(),
            header: String::new(),
            accent: String::new(),
            good: String::new(),
            warning: String::new(),
            critical: String::new(),
            muted: String::new(),
            track: String::new(),
            text: String::new(),
            stops: Vec::new(),
        }
    }
}

fn fg(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

/// Build palette for a given theme name.
pub fn get_palette(theme: &str, enable_color: bool) -> Palette {
    if !enable_color || theme == "monochrome" {
        return Palette::monochrome();
    }

    let (header, accent, good, warning, critical, muted, track, text, stops) = match theme {
        "default" => (
            fg(219, 31, 255),
            fg(112, 48, 239),
            fg(112, 48, 239),
            fg(179, 107, 255),
            fg(255, 75, 160),
            fg(140, 130, 175),
            fg(35, 25, 58),
            fg(240, 235, 255),
            vec![
                (0.0, (112, 48, 239)),
                (0.35, (168, 70, 255)),
                (0.70, (219, 31, 255)),
                (1.0, (224, 179, 255)),
            ],
        ),
        "dracula" => (
            fg(189, 147, 249),
            fg(255, 121, 198),
            fg(80, 250, 123),
            fg(255, 184, 108),
            fg(255, 85, 85),
            fg(98, 114, 164),
            fg(68, 71, 90),
            fg(248, 248, 242),
            vec![
                (0.0, (139, 233, 253)),
                (0.30, (189, 147, 249)),
                (0.60, (255, 121, 198)),
                (0.85, (255, 184, 108)),
                (1.0, (255, 85, 85)),
            ],
        ),
        "catppuccin" => (
            fg(203, 166, 247),
            fg(116, 199, 236),
            fg(166, 227, 161),
            fg(249, 226, 175),
            fg(243, 139, 168),
            fg(108, 112, 134),
            fg(49, 50, 68),
            fg(205, 214, 244),
            vec![
                (0.0, (116, 199, 236)),
                (0.30, (148, 226, 213)),
                (0.60, (203, 166, 247)),
                (0.85, (245, 194, 231)),
                (1.0, (243, 139, 168)),
            ],
        ),
        "nord" => (
            fg(136, 192, 208),
            fg(129, 161, 193),
            fg(163, 190, 140),
            fg(235, 203, 139),
            fg(191, 97, 106),
            fg(123, 136, 161),
            fg(59, 66, 82),
            fg(236, 239, 244),
            vec![
                (0.0, (143, 188, 187)),
                (0.30, (136, 192, 208)),
                (0.60, (129, 161, 193)),
                (0.80, (180, 142, 173)),
                (1.0, (191, 97, 106)),
            ],
        ),
        "tokyo-night" => (
            fg(125, 207, 255),
            fg(187, 154, 247),
            fg(158, 206, 106),
            fg(224, 175, 104),
            fg(247, 118, 142),
            fg(86, 95, 137),
            fg(41, 46, 66),
            fg(192, 202, 245),
            vec![
                (0.0, (125, 207, 255)),
                (0.35, (122, 162, 247)),
                (0.65, (187, 154, 247)),
                (0.85, (255, 158, 100)),
                (1.0, (247, 118, 142)),
            ],
        ),
        "gruvbox" => (
            fg(254, 128, 25),
            fg(142, 192, 124),
            fg(184, 187, 38),
            fg(250, 189, 47),
            fg(251, 73, 52),
            fg(146, 131, 116),
            fg(60, 56, 54),
            fg(235, 219, 178),
            vec![
                (0.0, (142, 192, 124)),
                (0.30, (184, 187, 38)),
                (0.60, (250, 189, 47)),
                (0.85, (254, 128, 25)),
                (1.0, (251, 73, 52)),
            ],
        ),
        "cyberpunk" => (
            fg(0, 240, 255),
            fg(255, 0, 127),
            fg(0, 240, 255),
            fg(254, 232, 1),
            fg(255, 0, 60),
            fg(153, 0, 255),
            fg(45, 18, 77),
            fg(240, 246, 252),
            vec![
                (0.0, (0, 240, 255)),
                (0.30, (254, 232, 1)),
                (0.60, (255, 0, 127)),
                (0.85, (153, 0, 255)),
                (1.0, (255, 0, 60)),
            ],
        ),
        "rose-pine" => (
            fg(196, 167, 231),
            fg(156, 207, 216),
            fg(156, 207, 216),
            fg(246, 193, 119),
            fg(235, 111, 146),
            fg(110, 106, 134),
            fg(38, 35, 58),
            fg(224, 222, 244),
            vec![
                (0.0, (156, 207, 216)),
                (0.30, (196, 167, 231)),
                (0.60, (246, 193, 119)),
                (0.85, (235, 188, 186)),
                (1.0, (235, 111, 146)),
            ],
        ),
        "everforest" => (
            fg(167, 192, 128),
            fg(135, 192, 149),
            fg(167, 192, 128),
            fg(219, 188, 127),
            fg(230, 126, 128),
            fg(133, 146, 137),
            fg(55, 65, 69),
            fg(211, 198, 170),
            vec![
                (0.0, (135, 192, 149)),
                (0.30, (167, 192, 128)),
                (0.60, (219, 188, 127)),
                (0.85, (230, 152, 117)),
                (1.0, (230, 126, 128)),
            ],
        ),
        "kanagawa" => (
            fg(126, 156, 216),
            fg(210, 126, 153),
            fg(152, 187, 108),
            fg(230, 195, 132),
            fg(195, 64, 67),
            fg(114, 113, 105),
            fg(34, 50, 73),
            fg(220, 215, 186),
            vec![
                (0.0, (126, 156, 216)),
                (0.30, (152, 187, 108)),
                (0.60, (230, 195, 132)),
                (0.85, (210, 126, 153)),
                (1.0, (195, 64, 67)),
            ],
        ),
        "monokai" => (
            fg(255, 97, 136),
            fg(120, 220, 232),
            fg(169, 220, 118),
            fg(255, 216, 102),
            fg(255, 97, 136),
            fg(114, 112, 114),
            fg(64, 62, 65),
            fg(252, 252, 250),
            vec![
                (0.0, (120, 220, 232)),
                (0.30, (169, 220, 118)),
                (0.60, (255, 216, 102)),
                (0.85, (252, 152, 103)),
                (1.0, (255, 97, 136)),
            ],
        ),
        "solarized" => (
            fg(42, 161, 152),
            fg(38, 139, 210),
            fg(133, 153, 0),
            fg(181, 137, 0),
            fg(220, 50, 47),
            fg(101, 123, 131),
            fg(7, 54, 66),
            fg(147, 161, 161),
            vec![
                (0.0, (42, 161, 152)),
                (0.30, (38, 139, 210)),
                (0.60, (108, 113, 196)),
                (0.85, (181, 137, 0)),
                (1.0, (220, 50, 47)),
            ],
        ),
        _ => return Palette::monochrome(),
    };

    Palette {
        reset: "\x1b[0m",
        bold: "\x1b[1m",
        dim: "\x1b[2m".to_string(),
        header,
        accent,
        good,
        warning,
        critical,
        muted,
        track,
        text,
        stops,
    }
}

/// Cycle to the next theme name.
pub fn next_theme(current: &str) -> &'static str {
    let idx = THEME_NAMES.iter().position(|&t| t == current).unwrap_or(0);
    THEME_NAMES[(idx + 1) % THEME_NAMES.len()]
}

/// Cycle to the next display mode.
pub fn next_cycling_mode(current: &str) -> &'static str {
    let idx = CYCLING_MODES
        .iter()
        .position(|&m| m == current)
        .unwrap_or(0);
    CYCLING_MODES[(idx + 1) % CYCLING_MODES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_themes_have_palettes() {
        for &name in THEME_NAMES {
            let pal = get_palette(name, true);
            if name == "monochrome" {
                assert!(pal.stops.is_empty());
            } else {
                assert!(!pal.stops.is_empty(), "theme {name} has no gradient stops");
            }
        }
    }

    #[test]
    fn test_theme_cycling() {
        assert_eq!(next_theme("default"), "dracula");
        assert_eq!(next_theme("monochrome"), "default");
    }

    #[test]
    fn test_mode_cycling() {
        assert_eq!(next_cycling_mode("hero"), "compact");
        assert_eq!(next_cycling_mode("mini"), "hero");
    }

    #[test]
    fn test_monochrome_returns_empty_escapes() {
        let pal = get_palette("monochrome", true);
        assert_eq!(pal.header, "");
        assert_eq!(pal.reset, "");
    }

    #[test]
    fn test_color_disabled_returns_monochrome() {
        let pal = get_palette("dracula", false);
        assert_eq!(pal.header, "");
    }
}
