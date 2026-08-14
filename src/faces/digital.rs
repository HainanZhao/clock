//! Big blocky digits, LED-clock style, sized to fill the terminal.

use crate::color;
use crate::config::{Config, MAX_CAP_PX};
use crate::render::{self, Line};
use crate::vector;
use chrono::{DateTime, Local, Timelike};

/// The clock text plus which character positions are colons (so they can blink).
pub fn time_text(now: DateTime<Local>, cfg: &Config) -> (String, Vec<usize>, &'static str) {
    let (hour, suffix) = if cfg.hour12 {
        let h = now.hour12().1;
        let h = if h == 0 { 12 } else { h };
        (h, if now.hour() < 12 { "AM" } else { "PM" })
    } else {
        (now.hour(), "")
    };

    let mut text = format!("{hour:02}:{:02}", now.minute());
    let mut colons = vec![2];
    if cfg.show_seconds {
        text.push_str(&format!(":{:02}", cfg.step_second(now.second())));
        colons.push(5);
    }
    (text, colons, suffix)
}

pub fn blink_mask(now: DateTime<Local>, cfg: &Config, len: usize, colons: &[usize]) -> Vec<bool> {
    let mut mask = vec![false; len];
    if cfg.blink_colon && now.timestamp_millis() / 500 % 2 == 0 {
        for &pos in colons {
            if pos < mask.len() {
                mask[pos] = true;
            }
        }
    }
    mask
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let (text, colons, suffix) = time_text(now, cfg);
    let n = text.chars().count();

    // Reserve room for the am/pm tag and date lines below the digits.
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

    let mask = blink_mask(now, cfg, n, &colons);
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let mut lines = vector::render(&text, h, &mask, &|t| color::lerp(primary, accent, t));

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
