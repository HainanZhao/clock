//! A tiny sub-pixel canvas that renders to Unicode braille characters.
//!
//! Each terminal cell holds a 2x4 grid of on/off dots (the braille standard),
//! which gives roughly double the horizontal and quadruple the vertical
//! resolution of plain characters — enough to draw a smooth-looking circle
//! and clock hands.

const DOT_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

pub struct Canvas {
    /// Size in terminal cells.
    pub cols: usize,
    pub rows: usize,
    /// Size in sub-pixel dots.
    px_w: usize,
    px_h: usize,
    bits: Vec<u8>,
}

impl Canvas {
    pub fn new(cols: usize, rows: usize) -> Self {
        Canvas {
            cols,
            rows,
            px_w: cols * 2,
            px_h: rows * 4,
            bits: vec![0u8; cols * rows],
        }
    }

    pub fn width_px(&self) -> f64 {
        self.px_w as f64
    }
    pub fn height_px(&self) -> f64 {
        self.px_h as f64
    }

    /// Sets the sub-pixel at (x, y), if in bounds. Coordinates may be negative
    /// or fractional; out-of-range points are silently dropped so callers
    /// don't need to clip circles/lines themselves.
    pub fn set(&mut self, x: f64, y: f64) {
        if x < 0.0 || y < 0.0 {
            return;
        }
        let (x, y) = (x.round() as isize, y.round() as isize);
        if x < 0 || y < 0 || x as usize >= self.px_w || y as usize >= self.px_h {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        let (cell_x, sub_x) = (x / 2, x % 2);
        let (cell_y, sub_y) = (y / 4, y % 4);
        let idx = cell_y * self.cols + cell_x;
        self.bits[idx] |= DOT_BITS[sub_y][sub_x];
    }

    /// Midpoint circle algorithm, plotted in sub-pixel space.
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64) {
        let steps = ((r * 6.0).max(64.0)) as u32;
        for i in 0..steps {
            let theta = (i as f64) / (steps as f64) * std::f64::consts::TAU;
            self.set(cx + r * theta.cos(), cy + r * theta.sin());
        }
    }

    /// Draws a line from (x0,y0) to (x1,y1) by sampling along its length —
    /// simpler than Bresenham and fine at this resolution.
    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let steps = (dist * 2.0).max(1.0) as u32;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.set(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
        }
    }

    /// Renders to text lines.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(self.rows);
        for r in 0..self.rows {
            let mut line = String::with_capacity(self.cols);
            for c in 0..self.cols {
                let bits = self.bits[r * self.cols + c];
                let ch = if bits == 0 {
                    ' '
                } else {
                    char::from_u32(0x2800 + bits as u32).unwrap_or(' ')
                };
                line.push(ch);
            }
            out.push(line);
        }
        out
    }
}
