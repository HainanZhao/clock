//! Seven-segment clock — a digitized number, not a typeface.
//!
//! The digits come from [`crate::seg7`], which lays the seven bars out
//! directly in terminal cells from integer thicknesses. Nothing is sampled or
//! scaled from an outline, so the bars are exact at every size.

use crate::color;
use crate::config::Config;
use crate::faces::digital::{blink_mask, time_text};
use crate::render::{self, Line};
use crate::seg7;
use chrono::{DateTime, Local};

/// Cap on the unit size, so a huge terminal doesn't give absurdly fat bars.
const MAX_UNIT: usize = 12;

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let (text, colons, suffix) = time_text(now, cfg);
    let n = text.chars().count();

    let mut reserved = 0;
    if !suffix.is_empty() {
        reserved += 2;
    }
    if cfg.show_date {
        reserved += 2;
    }

    let fit = seg7::fit_unit(&text, cfg.show_seconds, avail_w, avail_h.saturating_sub(reserved), MAX_UNIT);
    // `scale` counts in units directly here; 0 stays auto.
    let u = if cfg.is_auto_scale() {
        fit
    } else {
        (cfg.scale as usize).clamp(1, MAX_UNIT)
    };

    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);
    let mask = blink_mask(now, cfg, n, &colons);

    let mut lines = seg7::render(&text, u, &mask, cfg.ghost_segments, &|t| color::lerp(primary, accent, t));

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
