//! Seven-segment digits composed directly in terminal cells.
//!
//! A seven-segment digit is already discrete — three horizontal bars and four
//! vertical ones on a small integer grid — so there is nothing to rasterize.
//! Every cell here is placed deliberately from integer bar thicknesses and
//! lengths, which means the output is exact at any size: no partial coverage,
//! no stray single-cell spikes, no gaps that round away to nothing.
//!
//! All measurements are multiples of a unit `u`, chosen to fill the space.
//! A terminal cell is about twice as tall as it is wide, so vertical bars are
//! made twice as thick (in columns) as horizontal ones (in rows) to keep the
//! stroke weight even.

use crate::render::{span, Line};
use crossterm::style::Color;

/// Horizontal bar thickness is `u` rows; vertical bar thickness `2u` columns.
/// A digit is then `8u` columns wide and `7u` rows tall.
pub fn digit_w(u: usize) -> usize {
    8 * u
}
pub fn digit_h(u: usize) -> usize {
    7 * u
}
/// Cells are twice as tall as they are wide, so a `2u x u` dot is square.
fn colon_w(u: usize) -> usize {
    2 * u
}
fn gap(u: usize) -> usize {
    u
}

fn advance(c: char, u: usize) -> usize {
    if c == ':' {
        colon_w(u)
    } else {
        digit_w(u)
    }
}

/// Total width of `text` at unit `u`, including the gaps between glyphs.
pub fn width_of(text: &str, u: usize) -> usize {
    let n = text.chars().count();
    if n == 0 {
        return 0;
    }
    text.chars().map(|c| advance(c, u)).sum::<usize>() + gap(u) * (n - 1)
}

/// The largest unit whose rendering of `text` fits the given area.
pub fn fit_unit(text: &str, avail_w: usize, avail_h: usize, max_u: usize) -> usize {
    (1..=max_u.max(1))
        .rev()
        .find(|&u| width_of(text, u) <= avail_w && digit_h(u) <= avail_h)
        .unwrap_or(1)
}

/// Which of the seven segments are lit, in the usual a..g order: a top,
/// b upper right, c lower right, d bottom, e lower left, f upper left,
/// g middle.
fn segments(c: char) -> Option<[bool; 7]> {
    Some(match c.to_ascii_uppercase() {
        '0' => [true, true, true, true, true, true, false],
        '1' => [false, true, true, false, false, false, false],
        '2' => [true, true, false, true, true, false, true],
        '3' => [true, true, true, true, false, false, true],
        '4' => [false, true, true, false, false, true, true],
        '5' => [true, false, true, true, false, true, true],
        '6' => [true, false, true, true, true, true, true],
        '7' => [true, true, true, false, false, false, false],
        '8' => [true, true, true, true, true, true, true],
        '9' => [true, true, true, true, false, true, true],
        'A' => [true, true, true, false, true, true, true],
        'P' => [true, true, false, false, true, true, true],
        'M' => [true, true, true, false, true, true, false],
        _ => return None,
    })
}

/// Paints one glyph into `grid` (row-major, `stride` wide) at column `x0`.
fn draw_glyph(grid: &mut [bool], stride: usize, x0: usize, c: char, u: usize) {
    let (w, h) = (digit_w(u), digit_h(u));
    let mut set = |r: usize, cx: usize| {
        let col = x0 + cx;
        if col < stride {
            grid[r * stride + col] = true;
        }
    };

    if c == ':' {
        // Two square dots at the thirds, sized to the bar thickness.
        let dot_w = colon_w(u).max(1);
        for (top, _) in [(2 * u, 0), (4 * u + u, 0)] {
            for r in top..(top + u).min(h) {
                for cx in 0..dot_w {
                    set(r, cx);
                }
            }
        }
        return;
    }

    let Some(s) = segments(c) else { return };
    let tv = 2 * u; // vertical bar thickness, in columns
    let th = u; // horizontal bar thickness, in rows

    // Horizontal bars span the full width; vertical bars the full half-height.
    // They meet flush, so corners are solid.
    let bars: [(bool, usize, usize, usize, usize); 7] = [
        (s[0], 0, th, 0, w),                    // a
        (s[1], th, 3 * u, w - tv, w),           // b
        (s[2], 4 * u, 6 * u, w - tv, w),        // c
        (s[3], 6 * u, 7 * u, 0, w),             // d
        (s[4], 4 * u, 6 * u, 0, tv),            // e
        (s[5], th, 3 * u, 0, tv),               // f
        (s[6], 3 * u, 4 * u, 0, w),             // g
    ];
    for (on, r0, r1, c0, c1) in bars {
        if !on {
            continue;
        }
        for r in r0..r1.min(h) {
            for cx in c0..c1 {
                set(r, cx);
            }
        }
    }

    // Chamfer the four outer corners, counting rows double since a cell is
    // twice as tall as it is wide. Only worth doing once the bars are thick
    // enough to spare the cells — at small units it eats the digit.
    let k = u / 3;
    if k == 0 {
        return;
    }
    for r in 0..h {
        for cx in 0..w {
            let (rt, rb) = (r, h - 1 - r);
            let (cl, cr) = (cx, w - 1 - cx);
            let cut = (2 * rt + cl < 2 * k)
                || (2 * rt + cr < 2 * k)
                || (2 * rb + cl < 2 * k)
                || (2 * rb + cr < 2 * k);
            if cut {
                let col = x0 + cx;
                if col < stride {
                    grid[r * stride + col] = false;
                }
            }
        }
    }
}

/// Renders `text` as seven-segment digits at unit `u`.
///
/// `color_at(t)` gives the color for horizontal position `t` in 0..=1 across
/// the block. Glyph indices listed in `blink_mask` are left blank.
pub fn render(text: &str, u: usize, blink_mask: &[bool], color_at: &dyn Fn(f64) -> Color) -> Vec<Line> {
    let u = u.max(1);
    let w = width_of(text, u);
    let h = digit_h(u);
    if w == 0 {
        return Vec::new();
    }
    let mut grid = vec![false; w * h];

    let mut x = 0usize;
    for (i, c) in text.chars().enumerate() {
        if !blink_mask.get(i).copied().unwrap_or(false) {
            draw_glyph(&mut grid, w, x, c, u);
        }
        x += advance(c, u) + gap(u);
    }

    let denom = (w.max(2) - 1) as f64;
    (0..h)
        .map(|r| {
            let mut line: Line = Vec::new();
            for cx in 0..w {
                let ch = if grid[r * w + cx] { '\u{2588}' } else { ' ' };
                let c = color_at(cx as f64 / denom);
                match line.last_mut() {
                    Some(last) if last.color == c => last.text.push(ch),
                    _ => line.push(span(ch.to_string(), c)),
                }
            }
            line
        })
        .collect()
}
