//! Persisted user configuration: which face to draw, colors, and display options.
//!
//! Lives at `$XDG_CONFIG_HOME/clock/config.toml` (or the platform equivalent
//! via the `dirs` crate). Every field has `#[serde(default)]` so old config
//! files keep loading after new fields are added.

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Face {
    #[default]
    Digital,
    Analog,
    Binary,
    Word,
    Matrix,
}

impl Face {
    /// All faces, in the order they're cycled through and shown in the picker grid.
    pub const ALL: [Face; 5] = [
        Face::Digital,
        Face::Analog,
        Face::Binary,
        Face::Word,
        Face::Matrix,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }

    pub fn next(self) -> Face {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Face {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

impl fmt::Display for Face {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Face::Digital => write!(f, "digital"),
            Face::Analog => write!(f, "analog"),
            Face::Binary => write!(f, "binary"),
            Face::Word => write!(f, "word"),
            Face::Matrix => write!(f, "matrix"),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_scale() -> u8 {
    2
}
fn default_color() -> String {
    "cyan".to_string()
}
fn default_accent() -> String {
    "magenta".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Which clock face to draw.
    pub face: Face,
    /// 12-hour clock with am/pm (digital face only) vs. 24-hour.
    #[serde(default = "default_true")]
    pub hour12: bool,
    /// Show a seconds readout / second hand.
    #[serde(default = "default_true")]
    pub show_seconds: bool,
    /// Show today's date under the clock.
    #[serde(default = "default_true")]
    pub show_date: bool,
    /// Blink the ':' separators once a second (digital face only).
    #[serde(default = "default_true")]
    pub blink_colon: bool,
    /// Draw hour tick marks around the rim (analog face only).
    #[serde(default = "default_true")]
    pub tick_marks: bool,
    /// Size multiplier for the big digits (digital face only), 1-4.
    #[serde(default = "default_scale")]
    pub scale: u8,
    /// Primary color: digit / clock-face color.
    #[serde(default = "default_color")]
    pub color: String,
    /// Accent color: blinking colon / clock hands.
    #[serde(default = "default_accent")]
    pub accent_color: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            face: Face::default(),
            hour12: true,
            show_seconds: true,
            show_date: true,
            blink_colon: true,
            tick_marks: true,
            scale: default_scale(),
            color: default_color(),
            accent_color: default_accent(),
        }
    }
}

impl Config {
    /// Where the config file lives on this platform.
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("could not determine the platform config directory")?;
        Ok(dir.join("clock").join("config.toml"))
    }

    /// Loads the config file, falling back to defaults if it doesn't exist yet.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config = toml::from_str(&text)
            .with_context(|| format!("parsing {} (bad TOML?)", path.display()))?;
        Ok(cfg)
    }

    /// Writes the config to disk, creating the parent directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    pub fn scale_clamped(&self) -> u8 {
        self.scale.clamp(1, 4)
    }
}
