mod app;
mod braille;
mod color;
mod config;
mod faces;
mod render;
mod seg7;
mod vector;

use anyhow::{bail, Result};
use clap::{Args, Parser, Subcommand};
use config::{Config, Face};

/// A beautiful, configurable clock for your terminal.
#[derive(Parser)]
#[command(name = "clock", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    overrides: Overrides,
}

#[derive(Args)]
struct Overrides {
    /// Clock face to draw for this run (doesn't change the saved config).
    #[arg(long)]
    face: Option<Face>,
    /// Show a 12-hour clock with am/pm.
    #[arg(long, conflicts_with = "hour24")]
    hour12: bool,
    /// Show a 24-hour clock.
    #[arg(long)]
    hour24: bool,
    /// Show seconds.
    #[arg(long, conflicts_with = "no_seconds")]
    seconds: bool,
    /// Hide seconds.
    #[arg(long)]
    no_seconds: bool,
    /// Show today's date.
    #[arg(long, conflicts_with = "no_date")]
    date: bool,
    /// Hide today's date.
    #[arg(long)]
    no_date: bool,
    /// Primary color (digits / clock face). See `clock config colors`.
    #[arg(long)]
    color: Option<String>,
    /// Accent color (blinking colon / clock hands).
    #[arg(long)]
    accent_color: Option<String>,
    /// Clock size: 0 auto-fills the terminal, 1-9 pins a size.
    #[arg(long)]
    scale: Option<u8>,
}

impl Overrides {
    fn apply(&self, mut cfg: Config) -> Config {
        if let Some(face) = self.face {
            cfg.face = face;
        }
        if self.hour12 {
            cfg.hour12 = true;
        }
        if self.hour24 {
            cfg.hour12 = false;
        }
        if self.seconds {
            cfg.show_seconds = true;
        }
        if self.no_seconds {
            cfg.show_seconds = false;
        }
        if self.date {
            cfg.show_date = true;
        }
        if self.no_date {
            cfg.show_date = false;
        }
        if let Some(c) = &self.color {
            cfg.color = c.clone();
        }
        if let Some(c) = &self.accent_color {
            cfg.accent_color = c.clone();
        }
        if let Some(s) = self.scale {
            cfg.scale = s;
        }
        cfg
    }
}

#[derive(Subcommand)]
enum Command {
    /// Show the clock (also the default when no subcommand is given).
    Run,
    /// Manage the saved configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print the config file path.
    Path,
    /// Print the current config (as TOML).
    Show,
    /// Set a config value and save it, e.g. `clock config set face analog`.
    Set { key: String, value: String },
    /// Reset the config file to defaults.
    Reset,
    /// List the built-in color names.
    Colors,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let base = Config::load()?;

    match cli.command {
        None | Some(Command::Run) => {
            let cfg = cli.overrides.apply(base);
            app::run(cfg)
        }
        Some(Command::Config { action }) => run_config(action, base),
    }
}

fn run_config(action: ConfigAction, mut cfg: Config) -> Result<()> {
    match action {
        ConfigAction::Path => {
            println!("{}", Config::path()?.display());
        }
        ConfigAction::Show => {
            print!("{}", toml::to_string_pretty(&cfg)?);
        }
        ConfigAction::Reset => {
            Config::default().save()?;
            println!("reset {} to defaults", Config::path()?.display());
        }
        ConfigAction::Colors => {
            for name in color::NAMES {
                println!("{name}");
            }
        }
        ConfigAction::Set { key, value } => {
            set_field(&mut cfg, &key, &value)?;
            cfg.save()?;
            println!("saved {} to {}", key, Config::path()?.display());
        }
    }
    Ok(())
}

fn set_field(cfg: &mut Config, key: &str, value: &str) -> Result<()> {
    fn parse_bool(v: &str) -> Result<bool> {
        match v.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok(true),
            "false" | "off" | "no" | "0" => Ok(false),
            other => bail!("expected true/false, got '{other}'"),
        }
    }

    match key {
        "face" => {
            cfg.face = match value.to_ascii_lowercase().as_str() {
                "digital" => Face::Digital,
                "analog" => Face::Analog,
                "binary" => Face::Binary,
                "word" => Face::Word,
                "matrix" => Face::Matrix,
                "flip" => Face::Flip,
                "bars" => Face::Bars,
                "rings" => Face::Rings,
                "roman" => Face::Roman,
                "lcd" => Face::Lcd,
                "blocks" => Face::Blocks,
                "hourglass" => Face::Hourglass,
                other => bail!(
                    "unknown face '{other}' (expected one of: digital, analog, binary, word, \
                     matrix, flip, bars, rings, roman, lcd, hourglass, blocks)"
                ),
            }
        }
        "hour12" => cfg.hour12 = parse_bool(value)?,
        "show_seconds" => cfg.show_seconds = parse_bool(value)?,
        "show_date" => cfg.show_date = parse_bool(value)?,
        "blink_colon" => cfg.blink_colon = parse_bool(value)?,
        "tick_marks" => cfg.tick_marks = parse_bool(value)?,
        "ghost_segments" => cfg.ghost_segments = parse_bool(value)?,
        "second_step" => {
            cfg.second_step = value
                .parse::<u32>()
                .map_err(|_| anyhow::anyhow!("second_step must be a number 1-60"))?
                .clamp(1, 60)
        }
        "scale" => {
            cfg.scale = value
                .parse::<u8>()
                .map_err(|_| anyhow::anyhow!("scale must be 0 (auto) or 1-{}", config::MAX_SCALE))?
                .min(config::MAX_SCALE)
        }
        "color" => cfg.color = value.to_string(),
        "accent_color" => cfg.accent_color = value.to_string(),
        other => bail!(
            "unknown key '{other}' (expected one of: face, hour12, show_seconds, show_date, \
             blink_colon, tick_marks, ghost_segments, second_step, scale, color, \
             accent_color)"
        ),
    }
    Ok(())
}
