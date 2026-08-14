//! Terminal setup, the render loop, and live keybindings.

use crate::color;
use crate::config::{Config, Face, MAX_SCALE};
use crate::faces;
use crate::render::{self, Line};
use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use std::io::{Stdout, Write};
use std::time::Duration;

const HELP_ITEMS: &[&str] = &[
    "\u{2190}\u{2192} face",
    "tab picker",
    "t 12/24h",
    "s seconds",
    "+/- size",
    "0 auto",
    "q quit",
];
const PICKER_COLS: usize = 3;
/// Rows reserved at the bottom for the status line.
const CHROME_H: u16 = 2;

pub fn run(mut cfg: Config) -> Result<()> {
    let started_with = cfg.clone();

    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen, Hide)?;

    let result = event_loop(&mut out, &mut cfg);

    execute!(out, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    // Whatever the user switched to during the session becomes the default
    // for next time, so restarting resumes the face they were last looking
    // at. Written onto the *stored* config so one-off CLI overrides don't
    // leak into it, and only when something actually changed on screen.
    persist_session(&started_with, &cfg);

    result
}

/// Saves the settings the user can change from the keyboard. Failures are
/// deliberately silent: a read-only config directory shouldn't take the
/// clock down after it has already exited cleanly.
fn persist_session(before: &Config, after: &Config) {
    let changed = before.face != after.face
        || before.hour12 != after.hour12
        || before.show_seconds != after.show_seconds
        || before.scale != after.scale;
    if !changed {
        return;
    }
    if let Ok(mut stored) = Config::load() {
        stored.face = after.face;
        stored.hour12 = after.hour12;
        stored.show_seconds = after.show_seconds;
        stored.scale = after.scale;
        let _ = stored.save();
    }
}

/// How long we can sleep before the on-screen display would go stale.
///
/// The clock does no polling or busy-waiting: `event::poll` parks the thread
/// (backed by kqueue/epoll/IOCP under crossterm) until either a key arrives
/// or this deadline passes, so idle CPU usage is effectively zero. We only
/// wake as often as the display can actually change: every 500ms to blink
/// a colon, every second to advance a seconds readout, or — with seconds
/// hidden — only once a minute.
fn next_wake(cfg: &Config, now: DateTime<Local>) -> Duration {
    let blinks = cfg.blink_colon && matches!(cfg.face, Face::Digital | Face::Matrix | Face::Flip);
    let period_ms: i64 = if blinks {
        500
    } else if cfg.show_seconds {
        1000
    } else {
        60_000
    };
    let ms = now.timestamp_millis();
    let remainder = period_ms - ms.rem_euclid(period_ms);
    Duration::from_millis(remainder.clamp(10, period_ms) as u64)
}

/// Moves the picker's grid selection by (dcol, drow), clamping at the grid
/// edges and refusing to land on a trailing empty cell (the face count
/// doesn't evenly divide the column count).
fn move_selection(selected: usize, dcol: i32, drow: i32) -> usize {
    let n = Face::ALL.len();
    let rows = n.div_ceil(PICKER_COLS);
    let row = selected / PICKER_COLS;
    let col = selected % PICKER_COLS;
    let new_col = (col as i32 + dcol).clamp(0, PICKER_COLS as i32 - 1) as usize;
    let new_row = (row as i32 + drow).clamp(0, rows as i32 - 1) as usize;
    let idx = new_row * PICKER_COLS + new_col;
    if idx < n {
        idx
    } else {
        selected
    }
}

fn event_loop(out: &mut Stdout, cfg: &mut Config) -> Result<()> {
    let mut needs_clear = true;
    let mut picker: Option<usize> = None;

    loop {
        if needs_clear {
            queue!(out, Clear(ClearType::All))?;
            needs_clear = false;
        }
        match picker {
            Some(selected) => draw_picker(out, cfg, selected)?,
            None => draw(out, cfg)?,
        }
        out.flush()?;

        let wait = next_wake(cfg, Local::now());
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if let Some(selected) = picker {
                        match k.code {
                            KeyCode::Esc => {
                                picker = None;
                                needs_clear = true;
                            }
                            KeyCode::Char('q') => break,
                            KeyCode::Enter => {
                                cfg.face = Face::ALL[selected];
                                picker = None;
                                needs_clear = true;
                            }
                            KeyCode::Left => picker = Some(move_selection(selected, -1, 0)),
                            KeyCode::Right => picker = Some(move_selection(selected, 1, 0)),
                            KeyCode::Up => picker = Some(move_selection(selected, 0, -1)),
                            KeyCode::Down => picker = Some(move_selection(selected, 0, 1)),
                            _ => {}
                        }
                    } else {
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                break
                            }
                            KeyCode::Left => {
                                cfg.face = cfg.face.prev();
                                needs_clear = true;
                            }
                            KeyCode::Right => {
                                cfg.face = cfg.face.next();
                                needs_clear = true;
                            }
                            KeyCode::Tab => {
                                let idx = Face::ALL.iter().position(|f| *f == cfg.face).unwrap_or(0);
                                picker = Some(idx);
                                needs_clear = true;
                            }
                            KeyCode::Char('t') => cfg.hour12 = !cfg.hour12,
                            KeyCode::Char('s') => {
                                cfg.show_seconds = !cfg.show_seconds;
                                needs_clear = true;
                            }
                            KeyCode::Char('0') => {
                                cfg.scale = 0;
                                needs_clear = true;
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                // Leaving auto starts from the size on screen.
                                let cur = current_scale(cfg)?;
                                cfg.scale = (cur + 1).min(MAX_SCALE);
                                needs_clear = true;
                            }
                            KeyCode::Char('-') => {
                                let cur = current_scale(cfg)?;
                                cfg.scale = cur.saturating_sub(1).max(1);
                                needs_clear = true;
                            }
                            _ => {}
                        }
                    }
                }
                Event::Resize(_, _) => needs_clear = true,
                _ => {}
            }
        }
    }
    Ok(())
}

/// The scale currently on screen, so `+`/`-` continue from what the user sees
/// rather than jumping when leaving auto-scale.
fn current_scale(cfg: &Config) -> Result<u8> {
    if !cfg.is_auto_scale() {
        return Ok(cfg.scale);
    }
    let (w, h) = terminal::size()?;
    let (text, _, suffix) = faces::digital::time_text(Local::now(), cfg);
    let mut reserved = 0;
    if !suffix.is_empty() {
        reserved += 2;
    }
    if cfg.show_date {
        reserved += 2;
    }
    let cap = crate::vector::fit_height(
        text.chars().count(),
        w as usize,
        (h.saturating_sub(CHROME_H) as usize).saturating_sub(reserved),
        crate::config::MAX_CAP_PX,
    );
    // `scale` counts in 6px cap-height steps; round to the nearest one.
    Ok(((cap / 6.0).round() as u8).clamp(1, MAX_SCALE))
}

fn render_face(face: Face, now: DateTime<Local>, cfg: &Config, w: usize, h: usize) -> Vec<Line> {
    match face {
        Face::Digital => faces::digital::render(now, cfg, w, h),
        Face::Analog => faces::analog::render(now, cfg, w, h),
        Face::Binary => faces::binary::render(now, cfg, w, h),
        Face::Word => faces::word::render(now, cfg, w, h),
        Face::Matrix => faces::matrix::render(now, cfg, w, h),
        Face::Flip => faces::flip::render(now, cfg, w, h),
        Face::Bars => faces::bars::render(now, cfg, w, h),
        Face::Rings => faces::rings::render(now, cfg, w, h),
        Face::Roman => faces::roman::render(now, cfg, w, h),
    }
}

fn draw(out: &mut Stdout, cfg: &Config) -> Result<()> {
    let (term_w, term_h) = terminal::size()?;
    if term_w < 20 || term_h < 6 {
        queue!(out, Clear(ClearType::All), MoveTo(0, 0))?;
        queue!(out, Print("terminal too small"))?;
        return Ok(());
    }

    let now = Local::now();
    let avail_h = term_h.saturating_sub(CHROME_H) as usize;
    let lines = render_face(cfg.face, now, cfg, term_w as usize, avail_h);
    draw_block(out, term_w, avail_h as u16, &lines)?;

    draw_status(out, term_w, term_h)?;
    Ok(())
}

/// Centered hint bar on the bottom row. The row is cleared first so a wider
/// previous frame can't leave characters stranded past the new text.
fn draw_status(out: &mut Stdout, term_w: u16, term_h: u16) -> Result<()> {
    let sep = "  \u{00b7}  ";
    let mut text = HELP_ITEMS.join(sep);
    if text.chars().count() > term_w as usize {
        // Narrow terminal: drop the separators' padding, then trailing items.
        text = HELP_ITEMS.join(" \u{00b7} ");
        while text.chars().count() > term_w as usize && text.contains('\u{00b7}') {
            let cut = text.rfind('\u{00b7}').unwrap();
            text.truncate(cut);
            text = text.trim_end().to_string();
        }
    }
    let pad = (term_w as usize).saturating_sub(text.chars().count()) / 2;
    queue!(
        out,
        MoveTo(0, term_h.saturating_sub(1)),
        Clear(ClearType::CurrentLine),
        MoveTo(pad as u16, term_h.saturating_sub(1)),
        SetForegroundColor(Color::DarkGrey),
        Print(&text),
        ResetColor
    )?;
    Ok(())
}

/// Centers a block of styled lines in the given area and prints it.
fn draw_block(out: &mut Stdout, area_w: u16, area_h: u16, lines: &[Line]) -> Result<()> {
    let block_w = render::block_width(lines);
    let block_h = lines.len();
    let start_row = area_h.saturating_sub(block_h as u16) / 2;
    let start_col = area_w.saturating_sub(block_w as u16) / 2;

    for (i, l) in lines.iter().enumerate() {
        if i as u16 >= area_h {
            break;
        }
        let pad = block_w.saturating_sub(render::line_width(l)) / 2;
        queue!(out, MoveTo(start_col + pad as u16, start_row + i as u16))?;
        for s in l {
            queue!(out, SetForegroundColor(s.color), Print(&s.text))?;
        }
        queue!(out, ResetColor)?;
    }
    Ok(())
}

/// A small preview of `face` for the picker grid: no date clutter, and forced
/// to a size that fits inside one grid cell.
fn mini_render(face: Face, now: DateTime<Local>, cfg: &Config, w: usize, h: usize) -> Vec<Line> {
    let mut preview = cfg.clone();
    preview.scale = 0;
    preview.show_date = false;
    preview.show_seconds = false;
    preview.blink_colon = false;
    render_face(face, now, &preview, w, h)
}

fn draw_box(out: &mut Stdout, x0: u16, y0: u16, w: u16, h: u16, color: Color) -> Result<()> {
    let inner = w.saturating_sub(2) as usize;
    let mut top = String::from("\u{250c}");
    top.extend(std::iter::repeat_n('\u{2500}', inner));
    top.push('\u{2510}');
    let mut bottom = String::from("\u{2514}");
    bottom.extend(std::iter::repeat_n('\u{2500}', inner));
    bottom.push('\u{2518}');

    queue!(out, MoveTo(x0, y0), SetForegroundColor(color), Print(&top))?;
    for r in 1..h.saturating_sub(1) {
        queue!(
            out,
            MoveTo(x0, y0 + r),
            Print('\u{2502}'),
            MoveTo(x0 + w.saturating_sub(1), y0 + r),
            Print('\u{2502}')
        )?;
    }
    queue!(
        out,
        MoveTo(x0, y0 + h.saturating_sub(1)),
        Print(&bottom),
        ResetColor
    )?;
    Ok(())
}

fn draw_picker(out: &mut Stdout, cfg: &Config, selected: usize) -> Result<()> {
    let (term_w, term_h) = terminal::size()?;
    let now = Local::now();
    let accent = color::parse(&cfg.accent_color);

    let n = Face::ALL.len();
    let grid_rows = n.div_ceil(PICKER_COLS);
    let gap_x: u16 = 2;
    let gap_y: u16 = 0;
    let label_h: u16 = 1;

    // Size cells to the terminal so the grid fills the screen too.
    let box_w = ((term_w.saturating_sub(4) - gap_x * (PICKER_COLS as u16 - 1))
        / PICKER_COLS as u16)
        .clamp(16, 46);
    let box_h = ((term_h.saturating_sub(4)) / grid_rows as u16)
        .saturating_sub(label_h + gap_y)
        .clamp(6, 14);
    let cell_w = box_w.saturating_sub(2);
    let cell_h = box_h.saturating_sub(2);

    let total_w = PICKER_COLS as u16 * box_w + (PICKER_COLS as u16 - 1) * gap_x;
    let total_h = grid_rows as u16 * (box_h + label_h + gap_y);
    let start_col = term_w.saturating_sub(total_w) / 2;
    let start_row = term_h.saturating_sub(total_h + 1) / 2;

    for (i, face) in Face::ALL.iter().enumerate() {
        let col = (i % PICKER_COLS) as u16;
        let row = (i / PICKER_COLS) as u16;
        let x0 = start_col + col * (box_w + gap_x);
        let y0 = start_row + row * (box_h + label_h + gap_y);
        let is_selected = i == selected;
        let border = if is_selected { accent } else { Color::DarkGrey };

        draw_box(out, x0, y0, box_w, box_h, border)?;

        let lines = mini_render(*face, now, cfg, cell_w as usize, cell_h as usize);
        let shown = lines.len().min(cell_h as usize);
        let top_pad = (cell_h as usize).saturating_sub(shown) / 2;
        let inner_w = render::block_width(&lines).min(cell_w as usize);

        for (ri, l) in lines.iter().take(shown).enumerate() {
            let pad = (cell_w as usize).saturating_sub(inner_w) / 2
                + inner_w.saturating_sub(render::line_width(l)) / 2;
            queue!(
                out,
                MoveTo(x0 + 1 + pad as u16, y0 + 1 + (top_pad + ri) as u16)
            )?;
            let mut used = 0usize;
            for s in l {
                let room = (cell_w as usize).saturating_sub(pad + used);
                if room == 0 {
                    break;
                }
                let text: String = s.text.chars().take(room).collect();
                used += text.chars().count();
                queue!(out, SetForegroundColor(s.color), Print(&text))?;
            }
            queue!(out, ResetColor)?;
        }

        let label = face.to_string().to_uppercase();
        let lpad = (box_w as usize).saturating_sub(label.chars().count()) / 2;
        queue!(
            out,
            MoveTo(x0 + lpad as u16, y0 + box_h),
            SetForegroundColor(border),
            Print(&label),
            ResetColor
        )?;
    }

    let hint = "\u{2190}\u{2192}\u{2191}\u{2193} move   enter select   esc cancel";
    let hint_len = hint.chars().count() as u16;
    queue!(
        out,
        MoveTo(
            term_w.saturating_sub(hint_len) / 2,
            (start_row + total_h).min(term_h.saturating_sub(1))
        ),
        SetForegroundColor(Color::DarkGrey),
        Print(hint),
        ResetColor
    )?;
    Ok(())
}
