//! Cuckoo Clock face: an ornate wooden cuckoo clock house, complete with
//! a ticking swinging pendulum, cuckoo bird door, and central analog dial,
//! drawn with braille sub-pixels.

use crate::braille::Canvas;
use crate::color;
use crate::config::Config;
use crate::render::{self, span, Line};
use chrono::{DateTime, Local, Timelike};
use std::f64::consts::{PI, TAU};

fn radius_for(avail_w: usize, avail_h: usize, reserved: usize) -> f64 {
    let h = avail_h.saturating_sub(reserved) as f64;
    ((h - 2.0).min(avail_w as f64 / 2.0 - 1.5)).max(5.0)
}

fn disk(canvas: &mut Canvas, cx: f64, cy: f64, r: f64) {
    let steps = (r * 2.0) as usize;
    for i in 0..=steps {
        canvas.circle(cx, cy, i as f64 * 0.5);
    }
}

fn pinecone(canvas: &mut Canvas, cx: f64, cy: f64, r_dial: f64) {
    // Draw a textured cast-iron pinecone weight using horizontal bands
    canvas.line(cx - r_dial * 0.12, cy, cx + r_dial * 0.12, cy);
    canvas.line(cx - r_dial * 0.16, cy + r_dial * 0.18, cx + r_dial * 0.16, cy + r_dial * 0.18);
    canvas.line(cx - r_dial * 0.12, cy + r_dial * 0.36, cx + r_dial * 0.12, cy + r_dial * 0.36);
    canvas.line(cx - r_dial * 0.08, cy + r_dial * 0.54, cx + r_dial * 0.08, cy + r_dial * 0.54);
    canvas.set(cx, cy + r_dial * 0.72);
}

fn pine_tree(canvas: &mut Canvas, cx: f64, base_y: f64, r_dial: f64) {
    // Draw a small pine tree silhouette
    let h_tree = r_dial * 0.45;
    canvas.line(cx, base_y, cx, base_y - h_tree); // trunk
    // Layers of branches
    canvas.line(cx - r_dial * 0.20, base_y - h_tree * 0.25, cx + r_dial * 0.20, base_y - h_tree * 0.25);
    canvas.line(cx - r_dial * 0.15, base_y - h_tree * 0.55, cx + r_dial * 0.15, base_y - h_tree * 0.55);
    canvas.line(cx - r_dial * 0.08, base_y - h_tree * 0.85, cx + r_dial * 0.08, base_y - h_tree * 0.85);
}

fn draw_window(canvas: &mut Canvas, cx: f64, cy: f64, w_size: f64) {
    // Draw a small square decorative window with a crossbar
    canvas.line(cx - w_size, cy - w_size, cx + w_size, cy - w_size);
    canvas.line(cx - w_size, cy + w_size, cx + w_size, cy + w_size);
    canvas.line(cx - w_size, cy - w_size, cx - w_size, cy + w_size);
    canvas.line(cx + w_size, cy - w_size, cx + w_size, cy + w_size);
    canvas.line(cx, cy - w_size, cx, cy + w_size);
    canvas.line(cx - w_size, cy, cx + w_size, cy);
}

pub fn render(now: DateTime<Local>, cfg: &Config, avail_w: usize, avail_h: usize) -> Vec<Line> {
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let mut extra: Vec<Line> = Vec::new();
    if cfg.show_date {
        extra.push(render::blank());
        extra.push(render::line(
            now.format("%A, %B %-d %Y").to_string(),
            color::dim(primary, 0.75),
        ));
    }

    let radius = radius_for(avail_w, avail_h, extra.len());
    let cols = (radius * 2.0 + 3.0).ceil() as usize;
    let rows = (radius + 2.0).ceil() as usize;

    let mut house = Canvas::new(cols, rows);
    let mut dial = Canvas::new(cols, rows);
    let mut hands = Canvas::new(cols, rows);
    let mut pendulum = Canvas::new(cols, rows);

    let cx = house.width_px() / 2.0;
    // Shift dial center up slightly to leave room for the swinging pendulum below
    let cy = house.height_px() * 0.44;
    let r = radius * 2.0;

    let r_dial = r * 0.42;

    // 1. Draw the Cuckoo House silhouette
    let roof_peak_y = cy - r_dial * 1.8;
    let roof_left_x = cx - r_dial * 1.6;
    let roof_right_x = cx + r_dial * 1.6;
    let roof_side_y = cy - r_dial * 0.8;
    let floor_y = cy + r_dial * 1.3;

    // Outer Timber Roof Line (Double Roof Shingles Look)
    house.line(cx, roof_peak_y - 2.5, roof_left_x - 1.5, roof_side_y - 1.5);
    house.line(cx, roof_peak_y - 2.5, roof_right_x + 1.5, roof_side_y - 1.5);

    // Inner Roof peak lines
    house.line(cx, roof_peak_y, roof_left_x, roof_side_y);
    house.line(cx, roof_peak_y, roof_right_x, roof_side_y);

    // Side walls
    house.line(roof_left_x, roof_side_y, roof_left_x, floor_y);
    house.line(roof_right_x, roof_side_y, roof_right_x, floor_y);
    // Base Floor
    house.line(roof_left_x, floor_y, roof_right_x, floor_y);

    // Add a cute chimney on the left roof slope with rising smoke puffs!
    let chim_x = cx - r_dial * 0.9;
    let chim_y = roof_peak_y + (roof_side_y - roof_peak_y) * 0.45;
    house.line(chim_x - 1.5, chim_y, chim_x - 1.5, chim_y - 4.5);
    house.line(chim_x + 1.5, chim_y, chim_x + 1.5, chim_y - 4.5);
    house.line(chim_x - 1.5, chim_y - 4.5, chim_x + 1.5, chim_y - 4.5);
    
    // Smoke puffs
    house.set(chim_x, chim_y - 6.5);
    house.set(chim_x + 1.5, chim_y - 8.5);

    // Add decorative left/right windows on the chalet walls
    draw_window(&mut house, cx - r_dial * 1.25, cy, r_dial * 0.16);
    draw_window(&mut house, cx + r_dial * 1.25, cy, r_dial * 0.16);

    // Add decorative Pine Trees on the outer corners of the floor
    pine_tree(&mut house, cx - r_dial * 1.45, floor_y, r_dial);
    pine_tree(&mut house, cx + r_dial * 1.45, floor_y, r_dial);

    // Cute little cuckoo bird door above the dial
    let door_cx = cx;
    let door_cy = cy - r_dial * 1.35;
    house.circle(door_cx, door_cy, r_dial * 0.25);
    house.line(door_cx - r_dial * 0.25, door_cy, door_cx + r_dial * 0.25, door_cy);

    // If it's near the top of the hour or seconds are even, show a cuckoo bird popping out!
    if now.minute() == 0 || now.second() % 2 == 0 {
        // Draw a tiny bird head popping out of the door
        house.line(door_cx, door_cy - 1.0, door_cx + 4.0, door_cy - 1.0); // beak
        house.set(door_cx - 1.0, door_cy - 1.0); // body
        house.set(door_cx, door_cy - 2.0); // wing
    }

    // 2. Draw the ticking swinging Pendulum
    let base_y = floor_y;
    // Pendulum swings left and right second-by-second
    let swing_offset = if now.second() % 2 == 0 { -r_dial * 0.40 } else { r_dial * 0.40 };
    let pend_rod_len = r_dial * 0.85;
    let pend_weight_y = base_y + pend_rod_len;
    let pend_weight_x = cx + swing_offset;
    // Pendulum rod
    pendulum.line(cx, base_y, pend_weight_x, pend_weight_y);
    // Pendulum weight (brass disk)
    disk(&mut pendulum, pend_weight_x, pend_weight_y, r_dial * 0.20);

    // 2B. Draw the two traditional Pinecone Weights hanging on chains below the house!
    // Realistic touch: draw them at different winding heights!
    let chain_l_len = r_dial * 0.70;
    let chain_r_len = r_dial * 0.95;
    
    // Left chain and weight
    for r_offset in 0..=(chain_l_len as usize) {
        if r_offset % 2 == 0 {
            house.set(cx - r_dial * 0.65, base_y + r_offset as f64);
        }
    }
    pinecone(&mut house, cx - r_dial * 0.65, base_y + chain_l_len, r_dial);

    // Right chain and weight
    for r_offset in 0..=(chain_r_len as usize) {
        if r_offset % 2 == 0 {
            house.set(cx + r_dial * 0.65, base_y + r_offset as f64);
        }
    }
    pinecone(&mut house, cx + r_dial * 0.65, base_y + chain_r_len, r_dial);

    // 3. Draw the Clock Dial (rim and simple tick marks)
    dial.circle(cx, cy, r_dial);
    for h_idx in 0..12 {
        let theta = (h_idx as f64) / 12.0 * TAU - PI / 2.0;
        let inner = if h_idx % 3 == 0 { r_dial * 0.82 } else { r_dial * 0.90 };
        dial.line(
            cx + inner * theta.cos(),
            cy + inner * theta.sin(),
            cx + r_dial * theta.cos(),
            cy + r_dial * theta.sin(),
        );
    }

    // 4. Draw Analog Hands
    let hand = |canvas: &mut Canvas, len: f64, units: f64, per_rev: f64| {
        let theta = units / per_rev * TAU - PI / 2.0;
        canvas.line(cx, cy, cx + len * theta.cos(), cy + len * theta.sin());
    };

    let hour_units = (now.hour() % 12) as f64 + now.minute() as f64 / 60.0;
    let min_units = now.minute() as f64 + now.second() as f64 / 60.0;
    hand(&mut hands, r_dial * 0.52, hour_units, 12.0);
    hand(&mut hands, r_dial * 0.80, min_units, 60.0);

    let house_l = house.lines();
    let dial_l = dial.lines();
    let hands_l = hands.lines();
    let pend_l = pendulum.lines();

    // Wood/Chalet colors: weathered wood (primary), brass pendulum/bird (accent).
    let house_c = primary;
    let dial_c = color::dim(primary, 0.40);
    let hands_c = accent;
    let pend_c = accent;

    let mut lines: Vec<Line> = Vec::with_capacity(rows + extra.len());
    for r_idx in 0..rows {
        let h_char: Vec<char> = house_l[r_idx].chars().collect();
        let d_char: Vec<char> = dial_l[r_idx].chars().collect();
        let hands_char: Vec<char> = hands_l[r_idx].chars().collect();
        let p_char: Vec<char> = pend_l[r_idx].chars().collect();

        let mut out: Line = Vec::new();
        for i in 0..cols {
            let at = |v: &Vec<char>| v.get(i).copied().unwrap_or(' ');
            // Priority: hands -> pendulum -> dial -> wooden house.
            let (ch, c) = if at(&hands_char) != ' ' {
                (at(&hands_char), hands_c)
            } else if at(&p_char) != ' ' {
                (at(&p_char), pend_c)
            } else if at(&d_char) != ' ' {
                (at(&d_char), dial_c)
            } else {
                (at(&h_char), house_c)
            };
            match out.last_mut() {
                Some(last) if last.color == c => last.text.push(ch),
                _ => out.push(span(ch.to_string(), c)),
            }
        }
        lines.push(out);
    }

    lines.extend(extra);
    lines
}
