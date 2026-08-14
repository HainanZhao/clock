//! Maps human-friendly color names (used in the config file / CLI) to crossterm colors.

use crossterm::style::Color;

/// Parses a color name into a `crossterm::style::Color`.
///
/// Accepts the standard ANSI names (plus "grey" as an alias for "gray") and
/// `#rrggbb` / `rgb(r,g,b)` for truecolor terminals. Falls back to `White`
/// for anything unrecognized so a typo in the config never crashes the app.
pub fn parse(name: &str) -> Color {
    let n = name.trim().to_ascii_lowercase();

    if let Some(hex) = n.strip_prefix('#') {
        if hex.len() == 6 {
            if let Ok(v) = u32::from_str_radix(hex, 16) {
                return Color::Rgb {
                    r: ((v >> 16) & 0xff) as u8,
                    g: ((v >> 8) & 0xff) as u8,
                    b: (v & 0xff) as u8,
                };
            }
        }
    }
    if let Some(inner) = n.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
            ) {
                return Color::Rgb { r, g, b };
            }
        }
    }

    match n.as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::DarkGrey,
        "dark_red" => Color::DarkRed,
        "dark_green" => Color::DarkGreen,
        "dark_yellow" | "orange" => Color::DarkYellow,
        "dark_blue" => Color::DarkBlue,
        "dark_magenta" => Color::DarkMagenta,
        "dark_cyan" => Color::DarkCyan,
        _ => Color::White,
    }
}

/// The list of built-in color names, shown in `clock config colors`.
pub const NAMES: &[&str] = &[
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "gray",
    "dark_red",
    "dark_green",
    "dark_yellow",
    "dark_blue",
    "dark_magenta",
    "dark_cyan",
    "#rrggbb (truecolor)",
];
