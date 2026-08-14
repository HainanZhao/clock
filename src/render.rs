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
    l.iter().map(|s| s.text.chars().count()).sum()
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

