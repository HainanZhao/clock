//! "TEN PAST FOUR" word clock, rounded to the nearest 5 minutes and drawn in
//! big block letters.

use crate::color;
use crate::config::{Config, MAX_CAP_PX};
use crate::render::{self, Line};
use crate::vector;
use chrono::{DateTime, Local, Timelike};

const HOURS: [&str; 12] = [
    "TWELVE", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE", "TEN",
    "ELEVEN",
];

/// The time in words, as a list of lines (already broken at sensible points).
fn phrase(now: DateTime<Local>) -> Vec<String> {
    let base_hour = (now.hour() % 12) as i64;
    let total = base_hour * 60 + now.minute() as i64;
    let rounded = ((total + 2) / 5 * 5).rem_euclid(12 * 60);
    let hour_idx = (rounded / 60) as usize % 12;
    let next_idx = (hour_idx + 1) % 12;

    let (prefix, hour) = match rounded % 60 {
        0 => (vec!["", ""], HOURS[hour_idx]),
        5 => (vec!["FIVE", "PAST"], HOURS[hour_idx]),
        10 => (vec!["TEN", "PAST"], HOURS[hour_idx]),
        15 => (vec!["QUARTER", "PAST"], HOURS[hour_idx]),
        20 => (vec!["TWENTY", "PAST"], HOURS[hour_idx]),
        25 => (vec!["TWENTYFIVE", "PAST"], HOURS[hour_idx]),
        30 => (vec!["HALF", "PAST"], HOURS[hour_idx]),
        35 => (vec!["TWENTYFIVE", "TO"], HOURS[next_idx]),
        40 => (vec!["TWENTY", "TO"], HOURS[next_idx]),
        45 => (vec!["QUARTER", "TO"], HOURS[next_idx]),
        50 => (vec!["TEN", "TO"], HOURS[next_idx]),
        55 => (vec!["FIVE", "TO"], HOURS[next_idx]),
        _ => unreachable!("rounded to a multiple of 5"),
    };

    let mut lines: Vec<String> = prefix
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    lines.push(hour.to_string());
    if lines.len() == 1 {
        lines.push("OCLOCK".to_string());
    }
    lines
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let words = phrase(now);
    let longest = words.iter().map(|w| w.chars().count()).max().unwrap_or(1);

    let reserved = if cfg.show_date { 2 } else { 0 } + if cfg.show_seconds { 2 } else { 0 };
    let usable_h = avail_h.saturating_sub(reserved);
    // Each word is one block-font line plus a blank row between words.
    let per_word_h = usable_h.saturating_sub(words.len() - 1) / words.len().max(1);
    let h = cfg.resolve_height(vector::fit_height(longest, avail_w, per_word_h, MAX_CAP_PX));

    let mut lines: Vec<Line> = Vec::new();
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            lines.push(render::blank());
        }
        // Last line is the hour itself — give it the accent end of the ramp.
        let (from, to) = if i + 1 == words.len() {
            (accent, color::lerp(accent, primary, 0.45))
        } else {
            (primary, color::lerp(primary, accent, 0.45))
        };
        lines.extend(vector::render(word, h, &[], &|t| color::lerp(from, to, t)));
    }

    if cfg.show_seconds {
        lines.push(render::blank());
        let fmt = if cfg.hour12 {
            "%I:%M:%S %p"
        } else {
            "%H:%M:%S"
        };
        lines.push(render::line(
            now.format(fmt).to_string(),
            color::dim(primary, 0.8),
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
