//! Classic round clock face, drawn with braille sub-pixels.

use crate::braille::Canvas;
use crate::config::Config;
use chrono::{DateTime, Local, Timelike};
use std::f64::consts::TAU;

pub struct Rendered {
    /// Rim + hour ticks, meant to be drawn in the primary color.
    pub face: Vec<String>,
    /// Hour/minute/second hands, meant to be drawn in the accent color.
    pub hands: Vec<String>,
}

/// `radius_cells` is the radius in terminal-cell units; the canvas itself is
/// sized generously around it so hands and ticks never clip at the edges.
pub fn render(now: DateTime<Local>, cfg: &Config, radius_cells: f64) -> Rendered {
    let cols = (radius_cells * 2.0 + 3.0).ceil() as usize;
    let rows = (radius_cells * 2.0 + 3.0).ceil() as usize;
    let mut face = Canvas::new(cols, rows);
    let mut hands = Canvas::new(cols, rows);

    let cx = face.width_px() / 2.0;
    let cy = face.height_px() / 2.0;
    let r = radius_cells.min(cols.min(rows) as f64 / 2.0 - 1.0) * 2.0; // sub-pixel radius

    face.circle(cx, cy, r);

    if cfg.tick_marks {
        for h in 0..12 {
            let theta = (h as f64) / 12.0 * TAU - std::f64::consts::FRAC_PI_2;
            let outer = r;
            let inner = if h % 3 == 0 { r * 0.82 } else { r * 0.90 };
            face.line(
                cx + inner * theta.cos(),
                cy + inner * theta.sin(),
                cx + outer * theta.cos(),
                cy + outer * theta.sin(),
            );
        }
    }

    let hour = (now.hour() % 12) as f64 + now.minute() as f64 / 60.0;
    let minute = now.minute() as f64 + now.second() as f64 / 60.0;

    let hand = |len_frac: f64, units: f64, units_per_rev: f64, hands: &mut Canvas| {
        let theta = units / units_per_rev * TAU - std::f64::consts::FRAC_PI_2;
        let len = r * len_frac;
        hands.line(cx, cy, cx + len * theta.cos(), cy + len * theta.sin());
    };

    hand(0.5, hour, 12.0, &mut hands);
    hand(0.78, minute, 60.0, &mut hands);
    if cfg.show_seconds {
        hand(0.9, now.second() as f64, 60.0, &mut hands);
    }
    // center hub
    hands.set(cx, cy);

    Rendered {
        face: face.lines(),
        hands: hands.lines(),
    }
}

/// Same clock, but face + hands drawn onto one canvas for a single-color
/// preview (used by the face-picker grid, where per-cell accent coloring
/// isn't worth the complexity).
pub fn render_mono(now: DateTime<Local>, cfg: &Config, radius_cells: f64) -> Vec<String> {
    let rendered = render(now, cfg, radius_cells);
    rendered
        .face
        .iter()
        .zip(rendered.hands.iter())
        .map(|(f, h)| {
            f.chars()
                .zip(h.chars())
                .map(|(fc, hc)| if hc != ' ' { hc } else { fc })
                .collect()
        })
        .collect()
}
