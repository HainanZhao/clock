//! Horizontal progress bars: how far through the hour, minute, and second
//! we are. Stretches to the full terminal width.

use crate::color;
use crate::config::Config;
use crate::render::{self, span, Line};
use chrono::{DateTime, Local, Timelike};

struct Row {
    label: &'static str,
    value: u32,
    max: u32,
    /// Fraction filled, kept separate from value/max so the hour bar can
    /// advance smoothly through its minutes rather than jumping on the hour.
    frac: f64,
    hue: f64,
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);

    let hour_max: u32 = if cfg.hour12 { 12 } else { 24 };
    let hour_val = if cfg.hour12 {
        let h = now.hour12().1;
        if h == 0 {
            12
        } else {
            h
        }
    } else {
        now.hour()
    };

    let mut rows = vec![
        Row {
            label: "HOURS",
            value: hour_val,
            max: hour_max,
            frac: (now.hour() % hour_max) as f64 / hour_max as f64
                + now.minute() as f64 / (60.0 * hour_max as f64),
            hue: color::HOUR_HUE,
        },
        Row {
            label: "MINUTES",
            value: now.minute(),
            max: 60,
            frac: now.minute() as f64 / 60.0 + now.second() as f64 / 3600.0,
            hue: color::MINUTE_HUE,
        },
    ];
    if cfg.show_seconds {
        rows.push(Row {
            label: "SECONDS",
            value: now.second(),
            max: 60,
            frac: now.second() as f64 / 60.0,
            hue: color::SECOND_HUE,
        });
    }

    let label_w = rows.iter().map(|r| r.label.len()).max().unwrap_or(7);
    let value_w = 6; // " 12/24"
    let bar_w = avail_w
        .saturating_sub(label_w + value_w + 4)
        .clamp(10, 160);

    // Thicker bars on taller terminals, so the face doesn't look sparse.
    let reserved = if cfg.show_date { 3 } else { 1 };
    let per_row = avail_h.saturating_sub(reserved) / rows.len().max(1);
    let thickness = cfg.resolve_scale(per_row.saturating_sub(1).clamp(1, 5));

    let mut lines: Vec<Line> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            lines.push(render::blank());
        }
        let filled = ((row.frac * bar_w as f64).round() as usize).min(bar_w);
        let base = color::hue(row.hue);

        for t in 0..thickness {
            let mut l: Line = Vec::new();
            // Only the middle stripe carries the label and numbers.
            let is_mid = t == thickness / 2;
            if is_mid {
                l.push(span(
                    format!("{:>label_w$}  ", row.label, label_w = label_w),
                    color::dim(base, 0.85),
                ));
            } else {
                l.push(span(" ".repeat(label_w + 2), base));
            }

            for x in 0..bar_w {
                let t_pos = x as f64 / (bar_w.max(2) - 1) as f64;
                if x < filled {
                    let c = color::lerp(base, color::lerp(base, primary, 0.6), t_pos);
                    match l.last_mut() {
                        Some(last) if last.color == c => last.text.push('\u{2588}'),
                        _ => l.push(span("\u{2588}".to_string(), c)),
                    }
                } else {
                    let c = color::dim(base, 0.22);
                    match l.last_mut() {
                        Some(last) if last.color == c => last.text.push('\u{2591}'),
                        _ => l.push(span("\u{2591}".to_string(), c)),
                    }
                }
            }

            // Every stripe carries the same total width, so the block-centering
            // in the drawing code lines all of them up.
            let value_text = format!(" {:>2}/{}", row.value, row.max);
            if is_mid {
                l.push(span(value_text, color::dim(base, 0.9)));
            } else {
                l.push(span(" ".repeat(value_text.chars().count()), base));
            }
            lines.push(l);
        }
    }

    if cfg.show_date {
        lines.push(render::blank());
        lines.push(render::line(
            now.format("%A, %B %-d %Y  %H:%M:%S").to_string(),
            color::dim(primary, 0.75),
        ));
    }
    lines
}
