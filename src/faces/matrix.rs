//! A sharper, smaller 7-segment digital face drawn with braille sub-pixels
//! (same technique as the analog face) instead of full block characters.

use crate::braille::Canvas;
use crate::config::Config;
use chrono::{DateTime, Local, Timelike};

/// Which of the 7 segments (a=top, b=upper-right, c=lower-right, d=bottom,
/// e=lower-left, f=upper-left, g=middle) are lit for a digit.
fn segments(c: char) -> [bool; 7] {
    match c {
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
        _ => [false; 7],
    }
}

/// Renders `text` (digits, ':' and spaces) using braille 7-segment glyphs,
/// `scale` sub-pixel units per digit, blanking positions in `blink_mask`.
pub fn render_text(text: &str, scale: u8, blink_mask: &[bool]) -> Vec<String> {
    let w = 8.0 * scale.max(1) as f64;
    let h = 16.0 * scale.max(1) as f64;
    let gutter = 4.0 * scale.max(1) as f64;
    let colon_w = 4.0 * scale.max(1) as f64;

    let mut cursor_x = 0.0;
    let widths: Vec<f64> = text
        .chars()
        .map(|c| if c == ':' { colon_w } else { w })
        .collect();
    let total_w: f64 = widths.iter().sum::<f64>() + gutter * (widths.len().max(1) - 1) as f64;

    let cols = (total_w / 2.0).ceil() as usize + 1;
    let rows = (h / 4.0).ceil() as usize + 1;
    let mut canvas = Canvas::new(cols, rows);

    let (tl, tr, ml, mr, bl, br) = ((0.0, 0.0), (w, 0.0), (0.0, h / 2.0), (w, h / 2.0), (0.0, h), (w, h));

    for (i, c) in text.chars().enumerate() {
        let x0 = cursor_x;
        let blank = blink_mask.get(i).copied().unwrap_or(false);
        if !blank {
            if c == ':' {
                let cx = x0 + colon_w / 2.0;
                canvas.line(cx, h * 0.28, cx, h * 0.32);
                canvas.line(cx, h * 0.68, cx, h * 0.72);
            } else {
                let seg = segments(c);
                let shifted = |p: (f64, f64)| (x0 + p.0, p.1);
                let edges = [
                    (seg[0], tl, tr), // a
                    (seg[1], tr, mr), // b
                    (seg[2], mr, br), // c
                    (seg[3], bl, br), // d
                    (seg[4], ml, bl), // e
                    (seg[5], tl, ml), // f
                    (seg[6], ml, mr), // g
                ];
                for (on, p0, p1) in edges {
                    if on {
                        let (x0p, y0p) = shifted(p0);
                        let (x1p, y1p) = shifted(p1);
                        canvas.line(x0p, y0p, x1p, y1p);
                    }
                }
            }
        }
        cursor_x += widths[i] + gutter;
    }

    canvas.lines()
}

pub fn render(now: DateTime<Local>, cfg: &Config) -> Vec<String> {
    let (hour, suffix) = if cfg.hour12 {
        let h = now.hour12().1;
        let h = if h == 0 { 12 } else { h };
        (h, if now.hour() < 12 { " AM" } else { " PM" })
    } else {
        (now.hour(), "")
    };

    let mut text = format!("{hour:02}:{:02}", now.minute());
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
