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

const HELP: &str = "q quit  d digital  a analog  t 12/24h  s seconds  +/- size";

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
    let period_ms: i64 = if cfg.blink_colon && cfg.face == Face::Digital {
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

fn event_loop(out: &mut Stdout, cfg: &mut Config) -> Result<()> {
    let mut needs_clear = true;
    loop {
        if needs_clear {
            queue!(out, Clear(ClearType::All))?;
            needs_clear = false;
        }
        draw(out, cfg)?;
        out.flush()?;

        let wait = next_wake(cfg, Local::now());
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('d') => {
                        cfg.face = Face::Digital;
                        needs_clear = true;
                    }
                    KeyCode::Char('a') => {
                        cfg.face = Face::Analog;
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
                },
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
