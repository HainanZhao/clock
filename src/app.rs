//! Terminal setup, the render loop, and live keybindings.

use crate::color;
use crate::config::{Config, Face};
use crate::faces;
use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use std::io::{Stdout, Write};
use std::time::Duration;

const HELP: &str = "q quit  \u{2190}/\u{2192} face  tab picker  t 12/24h  s seconds  +/- size";
const PICKER_COLS: usize = 2;

pub fn run(mut cfg: Config) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    execute!(out, EnterAlternateScreen, Hide)?;

    let result = event_loop(&mut out, &mut cfg);

    execute!(out, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

/// How long we can sleep before the on-screen display would go stale.
///
/// The clock does no polling or busy-waiting: `event::poll` parks the thread
/// (backed by kqueue/epoll/IOCP under crossterm) until either a key arrives
/// or this deadline passes, so idle CPU usage is effectively zero. We only
/// wake as often as the display can actually change: every 500ms to blink
/// the digital colon, every second to advance a seconds readout, or — with
/// seconds hidden — only once a minute.
fn next_wake(cfg: &Config, now: DateTime<Local>) -> Duration {
    let period_ms: i64 = if cfg.blink_colon && matches!(cfg.face, Face::Digital | Face::Matrix) {
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
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                cfg.scale = (cfg.scale + 1).min(4);
                                needs_clear = true;
                            }
                            KeyCode::Char('-') => {
                                cfg.scale = cfg.scale.saturating_sub(1).max(1);
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

fn draw(out: &mut Stdout, cfg: &Config) -> Result<()> {
    let (term_w, term_h) = terminal::size()?;
    if term_w < 24 || term_h < 8 {
        queue!(out, Clear(ClearType::All), MoveTo(0, 0))?;
        queue!(out, Print("terminal too small"))?;
        return Ok(());
    }

    let now = Local::now();
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    match cfg.face {
        Face::Digital => {
            let lines = faces::digital::render(now, cfg);
            draw_plain_block(out, term_w, term_h, &lines, primary)?;
        }
        Face::Binary => {
            let lines = faces::binary::render(now, cfg);
            draw_plain_block(out, term_w, term_h, &lines, primary)?;
        }
        Face::Word => {
            let lines = faces::word::render(now, cfg);
            draw_plain_block(out, term_w, term_h, &lines, primary)?;
        }
        Face::Matrix => {
            let lines = faces::matrix::render(now, cfg);
            draw_plain_block(out, term_w, term_h, &lines, primary)?;
        }
        Face::Analog => {
            let radius = ((term_h as f64 - 6.0) / 2.0)
                .min(term_w as f64 / 4.5)
                .max(4.0);
            let rendered = faces::analog::render(now, cfg, radius);
            draw_analog(out, term_w, term_h, &rendered, primary, accent, cfg)?;
        }
    }

    // Status line, dim, bottom-left.
    queue!(
        out,
        MoveTo(0, term_h.saturating_sub(1)),
        SetForegroundColor(Color::DarkGrey),
        Print(HELP),
        ResetColor
    )?;
    Ok(())
}

fn draw_plain_block(
    out: &mut Stdout,
    term_w: u16,
    term_h: u16,
    lines: &[String],
    color: Color,
) -> Result<()> {
    let block_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let block_height = lines.len();
    let start_row = term_h.saturating_sub(block_height as u16) / 2;
    let start_col = term_w.saturating_sub(block_width as u16) / 2;

    for (i, line) in lines.iter().enumerate() {
        let pad = (block_width.saturating_sub(line.chars().count())) / 2;
        queue!(
            out,
            MoveTo(start_col + pad as u16, start_row + i as u16),
            SetForegroundColor(color),
            Print(line),
            ResetColor
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_analog(
    out: &mut Stdout,
    term_w: u16,
    term_h: u16,
    rendered: &faces::analog::Rendered,
    primary: Color,
    accent: Color,
    cfg: &Config,
) -> Result<()> {
    let now = Local::now();
    let cols = rendered.face.first().map(|l| l.chars().count()).unwrap_or(0);
    let rows = rendered.face.len();

    let mut extra_lines: Vec<String> = vec![String::new()];
    let time_fmt = if cfg.hour12 { "%I:%M:%S %p" } else { "%H:%M:%S" };
    extra_lines.push(now.format(time_fmt).to_string());
    if cfg.show_date {
        extra_lines.push(now.format("%A, %B %-d %Y").to_string());
    }

    let extra_width = extra_lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let block_width = cols.max(extra_width);
    let block_height = rows + extra_lines.len();
    let start_row = term_h.saturating_sub(block_height as u16) / 2;
    let start_col = term_w.saturating_sub(block_width as u16) / 2;
    let face_pad = (block_width.saturating_sub(cols)) / 2;

    for r in 0..rows {
        let face_line: Vec<char> = rendered.face[r].chars().collect();
        let hand_line: Vec<char> = rendered.hands[r].chars().collect();
        queue!(out, MoveTo(start_col + face_pad as u16, start_row + r as u16))?;

        let mut cur_color: Option<Color> = None;
        let mut buf = String::new();
        for i in 0..cols {
            let hc = hand_line.get(i).copied().unwrap_or(' ');
            let fc = face_line.get(i).copied().unwrap_or(' ');
            let (ch, col) = if hc != ' ' {
                (hc, accent)
            } else {
                (fc, primary)
            };
            match cur_color {
                Some(c) if c == col => buf.push(ch),
                Some(c) => {
                    queue!(out, SetForegroundColor(c), Print(&buf))?;
                    buf.clear();
                    buf.push(ch);
                    cur_color = Some(col);
                }
                None => {
                    cur_color = Some(col);
                    buf.push(ch);
                }
            }
        }
        if let Some(c) = cur_color {
            queue!(out, SetForegroundColor(c), Print(&buf), ResetColor)?;
        }
    }

    for (i, line) in extra_lines.iter().enumerate() {
        let pad = (block_width.saturating_sub(line.chars().count())) / 2;
        queue!(
            out,
            MoveTo(start_col + pad as u16, start_row + rows as u16 + i as u16),
            SetForegroundColor(primary),
            Print(line),
            ResetColor
        )?;
    }
    Ok(())
}

/// A small, single-color rendering of `face` used as a thumbnail in the
/// face-picker grid: no date/seconds clutter, fixed minimum scale.
fn mini_render(face: Face, now: DateTime<Local>, cfg: &Config) -> Vec<String> {
    let mut preview = cfg.clone();
    preview.scale = 1;
    preview.show_date = false;
    preview.show_seconds = false;
    preview.blink_colon = false;

    match face {
        Face::Digital => faces::digital::render(now, &preview),
        Face::Analog => faces::analog::render_mono(now, &preview, 2.5),
        Face::Binary => faces::binary::render(now, &preview),
        Face::Word => faces::word::render(now, &preview),
        Face::Matrix => faces::matrix::render(now, &preview),
    }
}

fn draw_box(out: &mut Stdout, x0: u16, y0: u16, w: u16, h: u16, color: Color) -> Result<()> {
    let inner = w.saturating_sub(2) as usize;
    let mut top = String::from("\u{250c}");
    top.extend(std::iter::repeat_n('\u{2500}', inner));
    top.push('\u{2510}');
    let mut bottom = String::from("\u{2514}");
    bottom.extend(std::iter::repeat_n('\u{2500}', inner));
    bottom.push('\u{2518}');

    queue!(
        out,
        MoveTo(x0, y0),
        SetForegroundColor(color),
        Print(&top)
    )?;
    for r in 1..h.saturating_sub(1) {
        queue!(
            out,
            MoveTo(x0, y0 + r),
            Print('\u{2502}'),
            MoveTo(x0 + w.saturating_sub(1), y0 + r),
            Print('\u{2502}')
        )?;
    }
    queue!(out, MoveTo(x0, y0 + h.saturating_sub(1)), Print(&bottom), ResetColor)?;
    Ok(())
}

fn draw_picker(out: &mut Stdout, cfg: &Config, selected: usize) -> Result<()> {
    let (term_w, term_h) = terminal::size()?;
    let now = Local::now();
    let primary = color::parse(&cfg.color);
    let accent = color::parse(&cfg.accent_color);

    let n = Face::ALL.len();
    let rows = n.div_ceil(PICKER_COLS);
    let cell_w: u16 = 30;
    let cell_h: u16 = 9;
    let box_w = cell_w + 2;
    let box_h = cell_h + 2;
    let gap_x: u16 = 3;
    let gap_y: u16 = 1;
    let label_h: u16 = 1;

    let total_w = PICKER_COLS as u16 * box_w + (PICKER_COLS as u16 - 1) * gap_x;
    let total_h = rows as u16 * (box_h + label_h) + (rows as u16 - 1) * gap_y;
    let start_col = term_w.saturating_sub(total_w) / 2;
    let start_row = term_h.saturating_sub(total_h + 2) / 2;

    for (i, face) in Face::ALL.iter().enumerate() {
        let col = (i % PICKER_COLS) as u16;
        let row = (i / PICKER_COLS) as u16;
        let x0 = start_col + col * (box_w + gap_x);
        let y0 = start_row + row * (box_h + label_h + gap_y);
        let is_selected = i == selected;
        let border = if is_selected { accent } else { Color::DarkGrey };

        draw_box(out, x0, y0, box_w, box_h, border)?;

        let lines = mini_render(*face, now, cfg);
        let top_pad = (cell_h as usize).saturating_sub(lines.len()) / 2;
        for (ri, line) in lines.iter().take(cell_h as usize).enumerate() {
            let text: String = line.chars().take(cell_w as usize).collect();
            let pad = (cell_w as usize).saturating_sub(text.chars().count()) / 2;
            queue!(
                out,
                MoveTo(x0 + 1 + pad as u16, y0 + 1 + (top_pad + ri) as u16),
                SetForegroundColor(primary),
                Print(&text),
                ResetColor
            )?;
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
        MoveTo(term_w.saturating_sub(hint_len) / 2, start_row + total_h + 1),
        SetForegroundColor(Color::DarkGrey),
        Print(hint),
        ResetColor
    )?;
    Ok(())
}
