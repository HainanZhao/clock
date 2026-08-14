//! Roman numeral clock — the time as IX : XLV, in big block letters.

use crate::vector;
use crate::color;
use crate::config::{Config, MAX_CAP_PX};
use crate::render::{self, Line};
use chrono::{DateTime, Local, Timelike};

fn to_roman(mut n: u32) -> String {
    if n == 0 {
        return "N".to_string(); // Roman numerals have no zero; "nulla".
    }
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let hour = if cfg.hour12 {
        let h = now.hour12().1;
        if h == 0 {
            12
        } else {
            h
        }
    } else {
        now.hour()
    };

    // Roman numerals get long (XXXVIII), so each unit goes on its own line —
    // stacked they can be drawn much larger than one wide row would allow.
    // Roman numerals for seconds change length constantly (VII, VIII, IX),
    // so never step faster than five seconds here however `second_step` is
    // set — otherwise the face churns.
    let second = {
        let step = cfg.second_step.max(5);
        now.second() / step * step
    };

    let mut parts = vec![to_roman(hour), to_roman(now.minute())];
    if cfg.show_seconds {
        parts.push(to_roman(second));
    }

    // Size to the longest numeral any of these fields could ever produce, not
    // to the current one. Sizing to the current value makes the cap height —
    // and so the whole block — change from second to second, and a centered
    // block that changes size jumps around the screen.
    let hour_range = if cfg.hour12 { 1..=12 } else { 0..=23 };
    let longest = hour_range
        .map(|v| to_roman(v).chars().count())
        .chain((0..60).map(|v| to_roman(v).chars().count()))
        .max()
        .unwrap_or(1);

    let mut reserved = 0;
    if cfg.hour12 {
        reserved += 2;
    }
    if cfg.show_date {
        reserved += 2;
    }
    let usable_h = avail_h.saturating_sub(reserved);
    let per_part_h = usable_h.saturating_sub(parts.len() - 1) / parts.len().max(1);
    let h = cfg.resolve_height(vector::fit_height(
        longest,
        avail_w,
        per_part_h,
        MAX_CAP_PX,
    ));

    let mut lines: Vec<Line> = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            lines.push(render::blank());
        }
        let ramp = i as f64 / parts.len().max(2) as f64;
        let from = color::lerp(accent, primary, ramp);
        let to = color::lerp(accent, primary, ramp + 0.4);
        lines.extend(vector::render(part, h, &[], &|t| color::lerp(from, to, t)));
    }

    // Hold the block at the worst-case width so it stays put as the numerals
    // change length.
    lines = render::pad_to_width(lines, vector::width_of(longest, h));

    if cfg.hour12 {
        lines.push(render::blank());
        lines.push(render::line(
            if now.hour() < 12 { "ANTE MERIDIEM" } else { "POST MERIDIEM" },
            accent,
        ));
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
