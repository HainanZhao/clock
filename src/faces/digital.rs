//! Big blocky digits, LED-clock style.

use crate::config::Config;
use chrono::{DateTime, Local, Timelike};

const GLYPH_H: usize = 5;

/// 5x5 bitmap glyphs for 0-9, ':' and space. `#` = lit pixel, anything else = dark.
fn glyph(c: char) -> [&'static str; GLYPH_H] {
    match c {
        '0' => ["#####", "#   #", "#   #", "#   #", "#####"],
        '1' => ["   # ", "  ## ", "   # ", "   # ", " ####"],
        '2' => ["#####", "    #", "#####", "#    ", "#####"],
        '3' => ["#####", "    #", " ####", "    #", "#####"],
        '4' => ["#   #", "#   #", "#####", "    #", "    #"],
        '5' => ["#####", "#    ", "#####", "    #", "#####"],
        '6' => ["#####", "#    ", "#####", "#   #", "#####"],
        '7' => ["#####", "    #", "   # ", "  #  ", "  #  "],
        '8' => ["#####", "#   #", "#####", "#   #", "#####"],
        '9' => ["#####", "#   #", "#####", "    #", "#####"],
        ':' => ["     ", "  #  ", "     ", "  #  ", "     "],
        _ => ["     ", "     ", "     ", "     ", "     "],
    }
}

/// Renders `text` (digits, ':' and spaces only) as big block glyphs, scaled up
/// by `scale` in both directions, and blank instead of lit wherever `blink_mask`
/// (indexed by input character position) is true — used to blink the colons.
fn render_text(text: &str, scale: u8, blink_mask: &[bool]) -> Vec<String> {
    let scale = scale.max(1) as usize;
    let mut rows = vec![String::new(); GLYPH_H * scale];

    for (i, c) in text.chars().enumerate() {
        let blank = blink_mask.get(i).copied().unwrap_or(false);
        let g = glyph(c);
        for (row_idx, row) in g.iter().enumerate() {
            let mut cell = String::new();
            for pixel in row.chars() {
                let lit = pixel == '#' && !blank;
                let ch = if lit { '█' } else { ' ' };
                for _ in 0..scale {
                    cell.push(ch);
                }
            }
            // one blank column of spacing between glyphs
            cell.push(' ');
            for s in 0..scale {
                rows[row_idx * scale + s].push_str(&cell);
            }
        }
    }
    rows
}

/// Renders the full digital face: HH:MM(:SS) plus an optional date line.
/// Returns plain (uncolored) lines; the caller decides styling per line.
pub fn render(now: DateTime<Local>, cfg: &Config) -> Vec<String> {
    let (hour, suffix) = if cfg.hour12 {
        let h = now.hour12().1;
        let h = if h == 0 { 12 } else { h };
        (h, if now.hour() < 12 { " AM" } else { " PM" })
    } else {
        (now.hour(), "")
    };

    let mut text = format!("{hour:02}:{:02}", now.minute());
    // Character index of each ':' so we know which glyph slots to blink.
    let mut colon_positions = vec![2];
    if cfg.show_seconds {
        text.push_str(&format!(":{:02}", now.second()));
        colon_positions.push(5);
    }

    let blink_on = cfg.blink_colon && now.timestamp_millis() / 500 % 2 == 0;
    let mut blink_mask = vec![false; text.chars().count()];
    if blink_on {
        for pos in colon_positions {
            if pos < blink_mask.len() {
                blink_mask[pos] = true;
            }
        }
    }

    let mut lines = render_text(&text, cfg.scale_clamped(), &blink_mask);

    if !suffix.is_empty() {
        lines.push(String::new());
        lines.push(suffix.trim().to_string());
    }

    if cfg.show_date {
        lines.push(String::new());
        lines.push(now.format("%A, %B %-d %Y").to_string());
    }

    lines
}
