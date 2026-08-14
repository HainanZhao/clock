//! A geometric vector font: glyphs are described as strokes — straight
//! segments and elliptical arcs — of constant width with round caps, in the
//! spirit of a light geometric sans.
//!
//! Nothing here is a pixel grid, so the letterforms stay true at any size:
//! the strokes are rasterized crisply at sub-cell resolution and drawn with
//! quadrant block characters — all sixteen combinations of a 2x2 split exist
//! in Unicode, so each terminal cell carries four sub-pixels. Half-blocks
//! alone would subdivide only vertically, leaving curves visibly stepped
//! along the horizontal axis.
//!
//! Coverage is hard-edged rather than anti-aliased: blending partial coverage
//! toward the background reads as a grey halo around the strokes.
//!
//! Coordinates are in *cell widths*. A terminal cell is one unit wide and two
//! units tall, so a sub-pixel is 0.5 x 1.0 units — distances are computed in
//! these units, which keeps circles round.

use crate::render::{span, Line as OutLine};
use crossterm::style::Color;
use std::f64::consts::PI;

/// Glyph width and inter-glyph tracking, both as a fraction of cap height.
const GLYPH_W: f64 = 0.62;
const TRACKING: f64 = 0.17;
/// Stroke thickness as a fraction of cap height, per style.
const STROKE_W: f64 = 0.125;
const SEGMENT_STROKE_W: f64 = 0.15;

/// Which letterform set to draw with.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Curved geometric sans — bowls are real ellipses.
    Geometric,
    /// Seven-segment bars only, as on an LCD panel. No curves, thicker
    /// strokes, and a small gap left between neighbouring segments.
    Segment,
}

fn stroke_w(style: Style) -> f64 {
    match style {
        Style::Geometric => STROKE_W,
        Style::Segment => SEGMENT_STROKE_W,
    }
}

/// A stroke segment in pixel space: (x0, y0, x1, y1).
type Seg = (f64, f64, f64, f64);
/// A dot centre in pixel space.
type DotPt = (f64, f64);
/// An axis-aligned box in pixel space: (x0, y0, x1, y1).
type Box2 = (f64, f64, f64, f64);
/// A flattened glyph: the column range it can touch, plus its geometry.
type Shape = (usize, usize, Vec<Seg>, Vec<DotPt>, Vec<Box2>);

#[derive(Clone, Copy)]
enum Prim {
    /// Straight stroke between two points.
    Line(f64, f64, f64, f64),
    /// Elliptical arc. Angles in degrees, 0 = right, 90 = down, 180 = left,
    /// 270 = up. The sweep runs monotonically from `a0` to `a1`, so the
    /// direction is whichever way that interval points — to arc over the top
    /// from the left, write 195 -> 375, not 195 -> 15.
    Arc {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        a0: f64,
        a1: f64,
    },
    /// A filled round dot (the colon, the full stop).
    Dot(f64, f64),
    /// A filled axis-aligned rectangle, snapped to whole terminal cells when
    /// rasterized so its edges and corners come out as solid blocks rather
    /// than partial quadrants.
    Rect(f64, f64, f64, f64),
}

use Prim::{Arc, Dot, Line, Rect};


/// Centre line and bowl radii shared by most glyphs.
const CX: f64 = GLYPH_W / 2.0;
const RX: f64 = GLYPH_W / 2.0 - STROKE_W / 2.0;
const TOP: f64 = STROKE_W / 2.0;
const BOT: f64 = 1.0 - STROKE_W / 2.0;

fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Prim {
    Arc {
        cx,
        cy,
        rx,
        ry,
        a0: 0.0,
        a1: 360.0,
    }
}

fn glyph(c: char) -> Vec<Prim> {
    // Bowl geometry used by 0/8/9/6 and the round letters.
    let ry = (1.0 - STROKE_W) / 2.0;
    match c.to_ascii_uppercase() {
        '0' => vec![ellipse(CX, 0.5, RX, ry)],
        '1' => vec![
            Line(CX - RX * 0.5, TOP + 0.13, CX, TOP),
            Line(CX, TOP, CX, BOT),
        ],
        '2' => vec![
            Arc {
                cx: CX,
                cy: TOP + RX,
                rx: RX,
                ry: RX,
                a0: 195.0,
                a1: 375.0,
            },
            Line(CX + RX * 0.97, TOP + RX + RX * 0.26, TOP, BOT),
            Line(TOP, BOT, GLYPH_W - TOP, BOT),
        ],
        '3' => vec![
            Arc {
                cx: CX,
                cy: TOP + ry * 0.52,
                rx: RX * 0.92,
                ry: ry * 0.52,
                a0: 200.0,
                a1: 430.0,
            },
            Arc {
                cx: CX,
                cy: BOT - ry * 0.52,
                rx: RX,
                ry: ry * 0.52,
                a0: -70.0,
                a1: 160.0,
            },
        ],
        '4' => vec![
            Line(CX + RX * 0.55, TOP, TOP, BOT - ry * 0.55),
            Line(TOP, BOT - ry * 0.55, GLYPH_W - TOP, BOT - ry * 0.55),
            Line(CX + RX * 0.55, TOP, CX + RX * 0.55, BOT),
        ],
        // Top bar, left stem, a middle bar into the bowl, then the bowl
        // itself sweeping top -> right -> bottom -> lower left.
        '5' => vec![
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(TOP, TOP, TOP, 0.46),
            Line(TOP, 0.46, CX, 0.46),
            Arc {
                cx: CX,
                cy: 0.70,
                rx: RX,
                ry: BOT - 0.70,
                a0: 270.0,
                a1: 500.0,
            },
        ],
        '6' => vec![
            ellipse(CX, BOT - ry * 0.5, RX, ry * 0.5),
            Arc {
                cx: CX,
                cy: 0.5,
                rx: RX,
                ry,
                a0: 300.0,
                a1: 150.0,
            },
        ],
        '7' => vec![
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(GLYPH_W - TOP, TOP, CX - RX * 0.35, BOT),
        ],
        '8' => vec![
            ellipse(CX, TOP + ry * 0.47, RX * 0.86, ry * 0.47),
            ellipse(CX, BOT - ry * 0.53, RX, ry * 0.53),
        ],
        '9' => vec![
            ellipse(CX, TOP + ry * 0.5, RX, ry * 0.5),
            Arc {
                cx: CX,
                cy: 0.5,
                rx: RX,
                ry,
                a0: 120.0,
                a1: -30.0,
            },
        ],
        ':' => vec![Dot(CX, 0.34), Dot(CX, 0.72)],
        '.' => vec![Dot(CX, BOT)],
        '-' => vec![Line(TOP + 0.06, 0.5, GLYPH_W - TOP - 0.06, 0.5)],
        '\'' => vec![Line(CX, TOP, CX, TOP + 0.16)],
        ' ' => vec![],

        'A' => vec![
            Line(TOP, BOT, CX, TOP),
            Line(CX, TOP, GLYPH_W - TOP, BOT),
            Line(TOP + RX * 0.32, BOT - ry * 0.5, GLYPH_W - TOP - RX * 0.32, BOT - ry * 0.5),
        ],
        'B' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, CX, TOP),
            Line(TOP, 0.5, CX, 0.5),
            Line(TOP, BOT, CX, BOT),
            Arc {
                cx: CX,
                cy: TOP + ry * 0.5,
                rx: RX * 0.8,
                ry: ry * 0.5,
                a0: -90.0,
                a1: 90.0,
            },
            Arc {
                cx: CX,
                cy: BOT - ry * 0.5,
                rx: RX * 0.9,
                ry: ry * 0.5,
                a0: -90.0,
                a1: 90.0,
            },
        ],
        'C' => vec![Arc {
            cx: CX,
            cy: 0.5,
            rx: RX,
            ry,
            a0: 55.0,
            a1: 305.0,
        }],
        'D' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, CX * 0.9, TOP),
            Line(TOP, BOT, CX * 0.9, BOT),
            Arc {
                cx: CX * 0.9,
                cy: 0.5,
                rx: RX,
                ry,
                a0: -90.0,
                a1: 90.0,
            },
        ],
        'E' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(TOP, 0.5, GLYPH_W - TOP - 0.06, 0.5),
            Line(TOP, BOT, GLYPH_W - TOP, BOT),
        ],
        'F' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(TOP, 0.5, GLYPH_W - TOP - 0.06, 0.5),
        ],
        'G' => vec![
            Arc {
                cx: CX,
                cy: 0.5,
                rx: RX,
                ry,
                a0: 55.0,
                a1: 305.0,
            },
            Line(GLYPH_W - TOP, 0.5, GLYPH_W - TOP, 0.5 + ry * 0.55),
            Line(CX, 0.5, GLYPH_W - TOP, 0.5),
        ],
        'H' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(GLYPH_W - TOP, TOP, GLYPH_W - TOP, BOT),
            Line(TOP, 0.5, GLYPH_W - TOP, 0.5),
        ],
        'I' => vec![Line(CX, TOP, CX, BOT)],
        'J' => vec![
            Line(GLYPH_W - TOP, TOP, GLYPH_W - TOP, BOT - ry * 0.45),
            Arc {
                cx: CX - RX * 0.1,
                cy: BOT - ry * 0.45,
                rx: RX * 0.9,
                ry: ry * 0.45,
                a0: 0.0,
                a1: 180.0,
            },
        ],
        'K' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(GLYPH_W - TOP, TOP, TOP, 0.55),
            Line(TOP + RX * 0.35, 0.42, GLYPH_W - TOP, BOT),
        ],
        'L' => vec![Line(TOP, TOP, TOP, BOT), Line(TOP, BOT, GLYPH_W - TOP, BOT)],
        'M' => vec![
            Line(TOP, BOT, TOP, TOP),
            Line(TOP, TOP, CX, 0.55),
            Line(CX, 0.55, GLYPH_W - TOP, TOP),
            Line(GLYPH_W - TOP, TOP, GLYPH_W - TOP, BOT),
        ],
        'N' => vec![
            Line(TOP, BOT, TOP, TOP),
            Line(TOP, TOP, GLYPH_W - TOP, BOT),
            Line(GLYPH_W - TOP, BOT, GLYPH_W - TOP, TOP),
        ],
        'O' => vec![ellipse(CX, 0.5, RX, ry)],
        'P' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, CX, TOP),
            Line(TOP, 0.5, CX, 0.5),
            Arc {
                cx: CX,
                cy: TOP + ry * 0.5,
                rx: RX * 0.9,
                ry: ry * 0.5,
                a0: -90.0,
                a1: 90.0,
            },
        ],
        'Q' => vec![
            ellipse(CX, 0.5, RX, ry),
            Line(CX + RX * 0.2, 0.5 + ry * 0.35, GLYPH_W - TOP, BOT),
        ],
        'R' => vec![
            Line(TOP, TOP, TOP, BOT),
            Line(TOP, TOP, CX, TOP),
            Line(TOP, 0.5, CX, 0.5),
            Arc {
                cx: CX,
                cy: TOP + ry * 0.5,
                rx: RX * 0.9,
                ry: ry * 0.5,
                a0: -90.0,
                a1: 90.0,
            },
            Line(CX, 0.5, GLYPH_W - TOP, BOT),
        ],
        // Two bowls that meet at the spine: the upper one wraps from the
        // right, over the top and down the left to its own bottom; the lower
        // one picks up at its top and wraps right, under, and out to the left.
        'S' => vec![
            Arc {
                cx: CX,
                cy: TOP + ry * 0.52,
                rx: RX,
                ry: ry * 0.52,
                a0: 20.0,
                a1: -270.0,
            },
            Arc {
                cx: CX,
                cy: BOT - ry * 0.52,
                rx: RX,
                ry: ry * 0.52,
                a0: -90.0,
                a1: 160.0,
            },
        ],
        'T' => vec![
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(CX, TOP, CX, BOT),
        ],
        'U' => vec![
            Line(TOP, TOP, TOP, BOT - ry * 0.45),
            Line(GLYPH_W - TOP, TOP, GLYPH_W - TOP, BOT - ry * 0.45),
            Arc {
                cx: CX,
                cy: BOT - ry * 0.45,
                rx: RX,
                ry: ry * 0.45,
                a0: 0.0,
                a1: 180.0,
            },
        ],
        'V' => vec![
            Line(TOP, TOP, CX, BOT),
            Line(CX, BOT, GLYPH_W - TOP, TOP),
        ],
        'W' => vec![
            Line(TOP, TOP, TOP + RX * 0.42, BOT),
            Line(TOP + RX * 0.42, BOT, CX, 0.45),
            Line(CX, 0.45, GLYPH_W - TOP - RX * 0.42, BOT),
            Line(GLYPH_W - TOP - RX * 0.42, BOT, GLYPH_W - TOP, TOP),
        ],
        'X' => vec![
            Line(TOP, TOP, GLYPH_W - TOP, BOT),
            Line(GLYPH_W - TOP, TOP, TOP, BOT),
        ],
        'Y' => vec![
            Line(TOP, TOP, CX, 0.52),
            Line(GLYPH_W - TOP, TOP, CX, 0.52),
            Line(CX, 0.52, CX, BOT),
        ],
        'Z' => vec![
            Line(TOP, TOP, GLYPH_W - TOP, TOP),
            Line(GLYPH_W - TOP, TOP, TOP, BOT),
            Line(TOP, BOT, GLYPH_W - TOP, BOT),
        ],
        _ => vec![],
    }
}

/// Which of the seven segments are lit, in the usual a..g order:
/// a top, b upper right, c lower right, d bottom, e lower left, f upper
/// left, g middle.
fn segments(c: char) -> Option<[bool; 7]> {
    Some(match c {
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
        'A' => [true, true, true, false, true, true, true],
        'P' => [true, true, false, false, true, true, true],
        'M' => [true, true, true, false, true, true, false],
        _ => return None,
    })
}

/// Seven-segment form of `c`, or `None` when this style has no glyph for it
/// (letters outside A/P/M, punctuation other than the colon).
///
/// Bars are rectangles rather than stroked lines: a stroke gets round caps,
/// which land as partial quadrants and make the corners look frayed.
fn segment_glyph(c: char) -> Option<Vec<Prim>> {
    let sw = SEGMENT_STROKE_W;
    let (x0, x1) = (0.0, GLYPH_W);
    let mid = 0.5;
    let (m0, m1) = (mid - sw / 2.0, mid + sw / 2.0);

    if c == ':' {
        let hw = sw / 2.0;
        return Some(vec![
            Rect(CX - hw, 0.30 - hw, CX + hw, 0.30 + hw),
            Rect(CX - hw, 0.68 - hw, CX + hw, 0.68 + hw),
        ]);
    }
    if c == ' ' {
        return Some(Vec::new());
    }
    let s = segments(c)?;

    // Every bar runs its full length, so neighbouring segments overlap at the
    // corners and merge into one solid shape. Insetting them to leave the
    // gaps of a real LCD panel breaks the digits into loose blocks at the
    // sizes a terminal actually gives us.
    let mut out = Vec::new();
    let mut push = |on: bool, p: Prim| {
        if on {
            out.push(p);
        }
    };
    push(s[0], Rect(x0, 0.0, x1, sw));
    push(s[1], Rect(x1 - sw, 0.0, x1, m1));
    push(s[2], Rect(x1 - sw, m0, x1, 1.0));
    push(s[3], Rect(x0, 1.0 - sw, x1, 1.0));
    push(s[4], Rect(x0, m0, x0 + sw, 1.0));
    push(s[5], Rect(x0, 0.0, x0 + sw, m1));
    push(s[6], Rect(x0, m0, x1, m1));
    Some(out)
}

/// The glyph for `c` in `style`, falling back to the geometric form when the
/// segment set has nothing for that character.
fn glyph_for(style: Style, c: char) -> Vec<Prim> {
    match style {
        Style::Geometric => glyph(c),
        Style::Segment => {
            segment_glyph(c.to_ascii_uppercase()).unwrap_or_else(|| glyph(c))
        }
    }
}

/// Flattens a glyph's primitives into line segments and dots in pixel space.
/// `h` is the cap height in pixels; x is offset by `x0`.
fn flatten(prims: &[Prim], x0: f64, h: f64) -> (Vec<Seg>, Vec<DotPt>, Vec<Box2>) {
    let mut segs = Vec::new();
    let mut dots = Vec::new();
    let mut rects = Vec::new();
    let map = |x: f64, y: f64| (x0 + x * h, y * h);
    // Cell boundaries sit at integer x and even y in this unit space, so
    // snapping there guarantees fully covered cells.
    let snap_x = |v: f64| v.round();
    let snap_y = |v: f64| (v / 2.0).round() * 2.0;

    for p in prims {
        match *p {
            Line(ax, ay, bx, by) => {
                let (x1, y1) = map(ax, ay);
                let (x2, y2) = map(bx, by);
                segs.push((x1, y1, x2, y2));
            }
            Dot(x, y) => {
                let (px, py) = map(x, y);
                dots.push((px, py));
            }
            Rect(ax, ay, bx, by) => {
                let (rx0, ry0) = map(ax, ay);
                let (rx1, ry1) = map(bx, by);
                let sx0 = snap_x(rx0);
                let sy0 = snap_y(ry0);
                let sx1 = snap_x(rx1).max(sx0 + 1.0);
                let sy1 = snap_y(ry1).max(sy0 + 2.0);
                rects.push((sx0, sy0, sx1, sy1));
            }
            Arc {
                cx,
                cy,
                rx,
                ry,
                a0,
                a1,
            } => {
                // Enough steps that the chord error stays well under a pixel.
                let arc_px = (rx.max(ry) * h * (a1 - a0).abs() * PI / 180.0).abs();
                let steps = (arc_px * 0.9).clamp(8.0, 400.0) as usize;
                let mut prev: Option<(f64, f64)> = None;
                for i in 0..=steps {
                    let t = i as f64 / steps as f64;
                    let ang = (a0 + (a1 - a0) * t).to_radians();
                    let (px, py) = map(cx + rx * ang.cos(), cy + ry * ang.sin());
                    if let Some((qx, qy)) = prev {
                        segs.push((qx, qy, px, py));
                    }
                    prev = Some((px, py));
                }
            }
        }
    }
    (segs, dots, rects)
}

fn dist2_seg(px: f64, py: f64, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len2 = dx * dx + dy * dy;
    let t = if len2 <= f64::EPSILON {
        0.0
    } else {
        (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0)
    };
    let (cx, cy) = (x0 + t * dx, y0 + t * dy);
    (px - cx).powi(2) + (py - cy).powi(2)
}

/// Advance from one glyph's origin to the next, in pixels.
fn advance_px(h: f64) -> f64 {
    (GLYPH_W + TRACKING) * h
}

/// Rendered width in terminal columns for `n` glyphs at cap height `h` px.
pub fn width_of(n: usize, h: f64) -> usize {
    if n == 0 {
        return 0;
    }
    (advance_px(h) * (n - 1) as f64 + GLYPH_W * h).ceil() as usize
}

/// Rendered height in terminal rows — two square pixels per cell.
pub fn height_of(h: f64) -> usize {
    (h.ceil() as usize).div_ceil(2)
}

/// Largest cap height (in sub-cell pixels) that fits `n` glyphs in the area.
pub fn fit_height(n: usize, avail_w: usize, avail_h: usize, max_h: f64) -> f64 {
    let by_h = (avail_h * 2) as f64;
    let by_w = if n == 0 {
        max_h
    } else {
        avail_w as f64 / ((n - 1) as f64 * (GLYPH_W + TRACKING) + GLYPH_W)
    };
    by_h.min(by_w).min(max_h).max(5.0)
}

/// The sixteen quadrant glyphs, indexed by `ul<<3 | ur<<2 | ll<<1 | lr`.
const QUADRANTS: [char; 16] = [
    ' ', '\u{2597}', '\u{2596}', '\u{2584}', '\u{259D}', '\u{2590}', '\u{259E}', '\u{259F}',
    '\u{2598}', '\u{259A}', '\u{258C}', '\u{2599}', '\u{2580}', '\u{259C}', '\u{259B}', '\u{2588}',
];

/// Renders `text` as crisp quadrant-block glyphs.
///
/// `color_at(t)` gives the color for horizontal position `t` in 0..=1 across
/// the block, so gradients come for free. Glyph indices listed in
/// `blink_mask` are skipped.
pub fn render(
    text: &str,
    h: f64,
    blink_mask: &[bool],
    color_at: &dyn Fn(f64) -> Color,
) -> Vec<OutLine> {
    render_styled(text, h, blink_mask, color_at, Style::Geometric)
}

/// As [`render`], but in the given [`Style`].
pub fn render_styled(
    text: &str,
    h: f64,
    blink_mask: &[bool],
    color_at: &dyn Fn(f64) -> Color,
    style: Style,
) -> Vec<OutLine> {
    let n = text.chars().count();
    if n == 0 {
        return Vec::new();
    }
    let cols = width_of(n, h);
    let rows = height_of(h);
    // Two sub-pixels per cell on each axis.
    let sub_cols = cols * 2;
    let sub_rows = rows * 2;
    let r = stroke_w(style) * h / 2.0;
    let r2 = r * r;

    let mut shapes: Vec<Shape> = Vec::new();
    for (i, c) in text.chars().enumerate() {
        if blink_mask.get(i).copied().unwrap_or(false) {
            continue;
        }
        let x0 = advance_px(h) * i as f64;
        let (segs, dots, rects) = flatten(&glyph_for(style, c), x0, h);
        if segs.is_empty() && dots.is_empty() && rects.is_empty() {
            continue;
        }
        // Sub-column range this glyph can possibly touch.
        let lo = (((x0 - r) / 0.5).floor().max(0.0)) as usize;
        let hi = ((((x0 + GLYPH_W * h + r) / 0.5).ceil() as usize) + 1).min(sub_cols);
        shapes.push((lo, hi, segs, dots, rects));
    }

    let mut on = vec![false; sub_cols * sub_rows];
    let dot_r2 = (r * 1.35).powi(2);
    for (lo, hi, segs, dots, rects) in &shapes {
        for iy in 0..sub_rows {
            // Sub-rows are one unit tall, sub-columns half a unit wide.
            let fy = iy as f64 + 0.5;
            for ix in *lo..*hi {
                let idx = iy * sub_cols + ix;
                if on[idx] {
                    continue;
                }
                let fx = ix as f64 * 0.5 + 0.25;
                let hit = rects
                    .iter()
                    .any(|&(x1, y1, x2, y2)| fx >= x1 && fx < x2 && fy >= y1 && fy < y2)
                    || dots
                        .iter()
                        .any(|&(dx, dy)| (fx - dx).powi(2) + (fy - dy).powi(2) <= dot_r2)
                    || segs
                        .iter()
                        .any(|&(x1, y1, x2, y2)| dist2_seg(fx, fy, x1, y1, x2, y2) <= r2);
                if hit {
                    on[idx] = true;
                }
            }
        }
    }

    let denom = (cols.max(2) - 1) as f64;
    let mut lines: Vec<OutLine> = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut line: OutLine = Vec::new();
        for col in 0..cols {
            let at = |iy: usize, ix: usize| -> usize {
                if iy < sub_rows && ix < sub_cols && on[iy * sub_cols + ix] {
                    1
                } else {
                    0
                }
            };
            let (iy, ix) = (row * 2, col * 2);
            let key = at(iy, ix) << 3 | at(iy, ix + 1) << 2 | at(iy + 1, ix) << 1 | at(iy + 1, ix + 1);
            let ch = QUADRANTS[key];
            let c = color_at(col as f64 / denom);

            match line.last_mut() {
                Some(last) if last.color == c => last.text.push(ch),
                _ => line.push(span(ch.to_string(), c)),
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints a whole word, to check spacing and that neighbouring glyphs
    /// don't collide: `cargo test word_shape -- --nocapture`
    #[test]
    fn word_shape() {
        for word in ["PAST", "QUARTER", "12:48:07"] {
            println!("=== {word} ===");
            for l in render(word, 20.0, &[], &|_| Color::White) {
                let text: String = l.iter().map(|s| s.text.as_str()).collect();
                println!("{}", text.trim_end());
            }
        }
    }

    /// Prints each glyph on its own so shapes can be eyeballed:
    /// `cargo test glyph_shapes -- --nocapture`
    #[test]
    fn glyph_shapes() {
        for c in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
            println!("=== {c} ===");
            for l in render(&c.to_string(), 28.0, &[], &|_| Color::White) {
                let text: String = l.iter().map(|s| s.text.as_str()).collect();
                println!("{}", text.trim_end());
            }
        }
    }
}
