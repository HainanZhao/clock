//! Snake clock face: a self-playing game of classic Snake fills the screen
//! while the time reads as a header above it.
//!
//! The whole game is a deterministic function of the wall clock rather than
//! stored state: the current 15-minute window seeds the run, and every frame
//! replays it from tick zero up to "now". That keeps this face a pure
//! function like every other one (no persisted state, resize-safe, and a
//! restarted process resumes the same run other instances would show).

use crate::color;
use crate::config::Config;
use crate::render::{self, span, Line};
use chrono::{DateTime, Local};
use std::collections::{HashSet, VecDeque};

type Point = (i32, i32);

/// Milliseconds per snake step — classic arcade pace.
pub const TICK_MS: i64 = 130;
/// The snake (and the RNG driving it) fully restarts every 15 minutes, even
/// if it never grows long enough or corners itself first.
const WINDOW_MS: i64 = 15 * 60 * 1000;

/// A tiny deterministic PRNG (SplitMix64) — good enough for food placement,
/// and lets the whole run be replayed byte-for-byte from a seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Starts a fresh run: a length-3 snake in the middle of the board, heading
/// right, with one food cell placed somewhere it isn't.
fn spawn(cols: usize, rows: usize, rng: &mut Rng) -> (VecDeque<Point>, HashSet<Point>, Point) {
    let cx = (cols / 2) as i32;
    let cy = (rows / 2) as i32;
    let body: VecDeque<Point> = (0..3).map(|i| (cx - i, cy)).collect();
    let occupied: HashSet<Point> = body.iter().copied().collect();
    let food = spawn_food(cols, rows, &occupied, rng);
    (body, occupied, food)
}

/// Picks a random empty cell for the next food. Falls back to a full scan if
/// the board is nearly packed, so it always terminates.
fn spawn_food(cols: usize, rows: usize, occupied: &HashSet<Point>, rng: &mut Rng) -> Point {
    for _ in 0..200 {
        let p = (rng.below(cols) as i32, rng.below(rows) as i32);
        if !occupied.contains(&p) {
            return p;
        }
    }
    (0..rows as i32)
        .flat_map(|y| (0..cols as i32).map(move |x| (x, y)))
        .find(|p| !occupied.contains(p))
        .unwrap_or((0, 0))
}

/// Advances the game by one step: the snake heads toward the food, preferring
/// whichever axis closes the bigger gap, and steers around its own body and
/// the walls. Cornering itself — or growing past `max_len` — restarts the run
/// in place, which is exactly the "reset" behavior asked for.
fn step(
    cols: usize,
    rows: usize,
    body: &mut VecDeque<Point>,
    occupied: &mut HashSet<Point>,
    food: &mut Point,
    rng: &mut Rng,
    max_len: usize,
) {
    let head = *body.front().expect("snake body is never empty");
    let (dx, dy) = (food.0 - head.0, food.1 - head.1);
    let horiz = (dx != 0).then_some((dx.signum(), 0));
    let vert = (dy != 0).then_some((0, dy.signum()));

    let mut prefs: Vec<Point> = Vec::with_capacity(4);
    let (first, second) = if dx.abs() >= dy.abs() {
        (horiz, vert)
    } else {
        (vert, horiz)
    };
    prefs.extend(first);
    prefs.extend(second);
    for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        if !prefs.contains(&d) {
            prefs.push(d);
        }
    }

    let safe = prefs.into_iter().find(|d| {
        let n = (head.0 + d.0, head.1 + d.1);
        n.0 >= 0 && n.1 >= 0 && n.0 < cols as i32 && n.1 < rows as i32 && !occupied.contains(&n)
    });

    let Some(d) = safe else {
        let (b, o, f) = spawn(cols, rows, rng);
        *body = b;
        *occupied = o;
        *food = f;
        return;
    };

    let next = (head.0 + d.0, head.1 + d.1);
    body.push_front(next);
    occupied.insert(next);

    if next == *food {
        if body.len() >= max_len {
            let (b, o, f) = spawn(cols, rows, rng);
            *body = b;
            *occupied = o;
            *food = f;
        } else {
            *food = spawn_food(cols, rows, occupied, rng);
        }
    } else if let Some(tail) = body.pop_back() {
        occupied.remove(&tail);
    }
}

/// Replays the current 15-minute window from tick zero up to "now", so the
/// board is always a pure function of the wall clock and the terminal size.
fn simulate(now: DateTime<Local>, cols: usize, rows: usize) -> (VecDeque<Point>, Point) {
    let now_ms = now.timestamp_millis();
    let anchor = now_ms.div_euclid(WINDOW_MS) * WINDOW_MS;
    let target_tick = (now_ms - anchor) / TICK_MS;

    let mut rng = Rng::new(anchor as u64 ^ 0x2545_F491_4F6C_DD1D);
    let max_len = (cols * rows / 3).clamp(8, 60).min(cols * rows - 4);
    let (mut body, mut occupied, mut food) = spawn(cols, rows, &mut rng);

    for _ in 0..target_tick {
        step(
            cols,
            rows,
            &mut body,
            &mut occupied,
            &mut food,
            &mut rng,
            max_len,
        );
    }
    (body, food)
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let (time_text, _, suffix) = crate::faces::digital::time_text(now, cfg);
    let header = if suffix.is_empty() {
        time_text
    } else {
        format!("{time_text} {suffix}")
    };

    let mut extra: Vec<Line> = Vec::new();
    if cfg.show_date {
        extra.push(render::blank());
        extra.push(render::line(
            now.format("%A, %B %-d %Y").to_string(),
            color::dim(primary, 0.75),
        ));
    }

    let mut lines: Vec<Line> = Vec::with_capacity(1 + avail_h + extra.len());
    lines.push(render::line(header, primary));

    let cols = avail_w;
    let rows = avail_h.saturating_sub(1 + extra.len());
    if cols < 6 || rows < 4 {
        lines.extend(extra);
        return lines;
    }

    let (body, food) = simulate(now, cols, rows);
    let head = body[0];
    let body_set: HashSet<Point> = body.iter().skip(1).copied().collect();

    for y in 0..rows {
        let mut line: Line = Vec::new();
        for x in 0..cols {
            let p = (x as i32, y as i32);
            let (ch, color) = if p == head {
                ('\u{25C6}', accent)
            } else if body_set.contains(&p) {
                ('\u{2588}', color::dim(primary, 0.85))
            } else if p == food {
                ('\u{2022}', accent)
            } else {
                (' ', primary)
            };
            match line.last_mut() {
                Some(last) if last.color == color => last.text.push(ch),
                _ => line.push(span(ch.to_string(), color)),
            }
        }
        lines.push(line);
    }

    lines.extend(extra);
    lines
}
