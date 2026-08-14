//! Seven-segment clock, as on an LCD panel: thick straight bars with a small
//! gap at every joint, and the unlit segments left faintly visible the way a
//! real display shows them.

use crate::color;
use crate::config::{Config, MAX_CAP_PX};
use crate::faces::digital::{blink_mask, time_text};
use crate::render::{self, Line};
use crate::vector::{self, Style};
use chrono::{DateTime, Local};

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
    let h = cfg.resolve_height(vector::fit_height(
        n,
        avail_w,
        avail_h.saturating_sub(reserved),
        MAX_CAP_PX,
    ));

    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);
    let mask = blink_mask(now, cfg, n, &colons);

    // The unlit segments of every digit, drawn first and very dim: it is what
    // makes the face read as a panel rather than as floating bars.
    let mut lines = if cfg.ghost_segments {
        let all_on: String = text
            .chars()
            .map(|c| if c == ':' { ':' } else { '8' })
            .collect();
        let ghost = color::dim(primary, 0.16);
        let lit = vector::render_styled(&text, h, &mask, &|_| primary, Style::Segment);
        let panel = vector::render_styled(&all_on, h, &[], &|_| ghost, Style::Segment);
        overlay(panel, lit, primary, accent, ghost)
    } else {
        vector::render_styled(
            &text,
            h,
            &mask,
            &|t| color::lerp(primary, accent, t),
            Style::Segment,
        )
    };

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

/// Merges the dim all-segments-on panel with the lit digits on top, so a cell
/// takes the lit color wherever the digit covers it and the ghost color where
/// only the panel does. Both renders share a size, so cells line up 1:1.
fn overlay(
    panel: Vec<Line>,
    lit: Vec<Line>,
    primary: crossterm::style::Color,
    accent: crossterm::style::Color,
    ghost: crossterm::style::Color,
) -> Vec<Line> {
    let flatten = |lines: &[Line]| -> Vec<Vec<char>> {
        lines
            .iter()
            .map(|l| l.iter().flat_map(|s| s.text.chars()).collect())
            .collect()
    };
    let panel_rows = flatten(&panel);
    let lit_rows = flatten(&lit);
    let width = panel_rows.iter().map(|r| r.len()).max().unwrap_or(0);

    panel_rows
        .iter()
        .enumerate()
        .map(|(y, prow)| {
            let empty = Vec::new();
            let lrow = lit_rows.get(y).unwrap_or(&empty);
            let mut out: Line = Vec::new();
            for x in 0..width {
                let lc = lrow.get(x).copied().unwrap_or(' ');
                let pc = prow.get(x).copied().unwrap_or(' ');
                let (ch, c) = if lc != ' ' {
                    let t = x as f64 / (width.max(2) - 1) as f64;
                    (lc, color::lerp(primary, accent, t))
                } else {
                    (pc, ghost)
                };
                match out.last_mut() {
                    Some(last) if last.color == c => last.text.push(ch),
                    _ => out.push(render::span(ch.to_string(), c)),
                }
            }
            out
        })
        .collect()
}
