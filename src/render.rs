//! A tiny styled-text model shared by every face.
//!
//! Faces build `Vec<Line>` instead of plain strings so they can color parts
//! of the clock independently — a gradient across the digits, a different hue
//! per bar, an accent on the colons — and the drawing code stays generic.

use crossterm::style::Color;

#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub color: Color,
}

pub type Line = Vec<Span>;

pub fn span(text: impl Into<String>, color: Color) -> Span {
    Span {
        text: text.into(),
        color,
    }
}

pub fn line(text: impl Into<String>, color: Color) -> Line {
    vec![span(text, color)]
}

pub fn blank() -> Line {
    Vec::new()
}

pub fn line_width(l: &Line) -> usize {
    l.iter()
        .map(|s| {
            s.text.chars().map(|c| if c == '🐱' { 2 } else { 1 }).sum::<usize>()
        })
        .sum()
}

pub fn block_width(lines: &[Line]) -> usize {
    lines.iter().map(line_width).max().unwrap_or(0)
}

/// Colors plain text lines with a left-to-right gradient from `from` to `to`,
/// so a block of big digits fades across its width. Runs of identical color
/// are merged into one span to keep the emitted escape sequences small.
pub fn gradient_block(lines: &[String], from: Color, to: Color) -> Vec<Line> {
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    lines
        .iter()
        .map(|text| {
            let mut out: Line = Vec::new();
            for (i, ch) in text.chars().enumerate() {
                let t = if width <= 1 {
                    0.0
                } else {
                    i as f64 / (width - 1) as f64
                };
                let c = crate::color::lerp(from, to, t);
                match out.last_mut() {
                    Some(last) if last.color == c => last.text.push(ch),
                    _ => out.push(span(ch.to_string(), c)),
                }
            }
            out
        })
        .collect()
}


/// Pads every line out to `width` with equal space either side.
///
/// Blocks are centered when drawn, so a block whose width changes with its
/// content visibly shifts on screen. Pinning the width to the widest the
/// content can ever be holds it still.
pub fn pad_to_width(lines: Vec<Line>, width: usize) -> Vec<Line> {
    lines
        .into_iter()
        .map(|l| {
            let w = line_width(&l);
            if w >= width {
                return l;
            }
            let left = (width - w) / 2;
            let right = width - w - left;
            let color = l.first().map(|s| s.color).unwrap_or(Color::Reset);
            let mut out: Line = Vec::new();
            if left > 0 {
                out.push(span(" ".repeat(left), color));
            }
            out.extend(l);
            if right > 0 {
                out.push(span(" ".repeat(right), color));
            }
            out
        })
        .collect()
}
