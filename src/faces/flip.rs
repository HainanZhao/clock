//! Retro split-flap / airport-board clock: each digit sits on a card with a
//! horizontal seam across the middle.

use crate::color;
use crate::config::{Config, MAX_CAP_PX};
use crate::faces::digital::time_text;
use crate::render::{self, line_width, span, Line};
use crate::vector;
use chrono::{DateTime, Local};

/// A card is one glyph plus a border and padding on each side.
fn card_w(h: f64) -> usize {
    vector::width_of(1, h) + 4
}
/// Two border rows plus the seam row, on top of the glyph itself.
fn card_h(h: f64) -> usize {
    vector::height_of(h) + 3
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let (text, _, suffix) = time_text(now, cfg);
    // Cards only for the digits; colons become a thin separator column.
    let digits: Vec<char> = text.chars().filter(|c| *c != ':').collect();
    let groups = digits.len() / 2; // HH MM (SS)

    let mut reserved = 0;
    if !suffix.is_empty() {
        reserved += 2;
    }
    if cfg.show_date {
        reserved += 2;
    }
    let usable_h = avail_h.saturating_sub(reserved);

    // Cards add fixed chrome per digit, so solve for the cap height that
    // makes the whole row of cards fit rather than reusing the plain fit.
    let sep_w = 3;
    let chrome_w = digits.len() * 4 + groups.saturating_sub(1) * sep_w;
    let per_glyph_w = avail_w.saturating_sub(chrome_w) / digits.len().max(1);
    let fit = vector::fit_height(1, per_glyph_w, usable_h.saturating_sub(3), MAX_CAP_PX);
    let h = cfg.resolve_height(fit);

    let cw = card_w(h);
    let ch = card_h(h);
    let glyph_rows = vector::height_of(h);
    let seam_row = 1 + glyph_rows / 2;
    let border = color::dim(primary, 0.55);

    // Each digit rendered once; card rows then slice into these.
    let glyphs: Vec<Vec<Line>> = digits
        .iter()
        .enumerate()
        .map(|(i, d)| {
            // Ramp the gradient across the whole row of cards, not per card.
            let t0 = i as f64 / digits.len().max(2) as f64;
            let from = color::lerp(accent, primary, t0);
            let to = color::lerp(accent, primary, t0 + 0.3);
            vector::render(&d.to_string(), h, &[], &|t| color::lerp(from, to, t))
        })
        .collect();

    let inner_w = cw - 2;
    let mut lines: Vec<Line> = Vec::new();
    for row in 0..ch {
        let mut l: Line = Vec::new();
        for (i, glyph) in glyphs.iter().enumerate() {
            if i > 0 && i % 2 == 0 {
                // Blinking colon between the HH / MM / SS groups.
                let on = !cfg.blink_colon || now.timestamp_millis() / 500 % 2 != 0;
                let dot = row == ch / 3 || row == 2 * ch / 3;
                let mark = if on && dot { "\u{25cf}" } else { " " };
                l.push(span(format!(" {mark} "), accent));
            }

            if row == 0 {
                l.push(span(
                    format!("\u{250c}{}\u{2510}", "\u{2500}".repeat(inner_w)),
                    border,
                ));
            } else if row == ch - 1 {
                l.push(span(
                    format!("\u{2514}{}\u{2518}", "\u{2500}".repeat(inner_w)),
                    border,
                ));
            } else if row == seam_row {
                l.push(span(
                    format!("\u{251c}{}\u{2524}", "\u{2504}".repeat(inner_w)),
                    color::dim(primary, 0.4),
                ));
            } else {
                // Rows past the seam shift up by one to account for it.
                let gi = if row > seam_row { row - 2 } else { row - 1 };
                let empty: Line = Vec::new();
                let g = glyph.get(gi).unwrap_or(&empty);
                let gw = line_width(g);
                let pad = inner_w.saturating_sub(gw);
                let left = pad / 2;

                l.push(span("\u{2502}".to_string(), border));
                l.push(span(" ".repeat(left), primary));
                l.extend(g.iter().cloned());
                l.push(span(" ".repeat(pad - left), primary));
                l.push(span("\u{2502}".to_string(), border));
            }
        }
        lines.push(l);
    }

    if !suffix.is_empty() {
        lines.push(render::blank());
        lines.push(render::line(suffix, accent));
    }
    if cfg.show_date {
        lines.push(render::blank());
        lines.push(render::line(
            now.format("%A, %B %-d %Y").to_string(),
            color::dim(primary, 0.75),
        ));
    }
    lines
}
