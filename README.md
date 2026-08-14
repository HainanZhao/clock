# clock

A beautiful, configurable digital & analog clock for your terminal — written
in Rust, ships as a single lightweight binary, idles at ~0% CPU.

```
      ██         ██              ██      ██ ██████████            ██████████ ██████████
      ██         ██              ██      ██ ██████████            ██████████ ██████████
    ████       ████              ██      ██ ██      ██            ██      ██ ██      ██
    ████       ████              ██      ██ ██      ██            ██      ██ ██      ██
      ██         ██              ██████████ ██      ██            ██      ██ ██████████
      ██         ██              ██████████ ██      ██            ██      ██ ██████████
      ██         ██                      ██ ██      ██            ██      ██         ██
      ██         ██                      ██ ██      ██            ██      ██         ██
  ████████   ████████                    ██ ██████████            ██████████ ██████████
  ████████   ████████                    ██ ██████████            ██████████ ██████████
                              11:15:30 AM
                          Friday, August 14 2026
```

## Install

**Homebrew (macOS/Linux):**

```sh
brew install hainanzhao/tap/clock
```

**Linux/macOS (prebuilt binary):**

```sh
curl -fsSL https://raw.githubusercontent.com/hainanzhao/clock/main/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/hainanzhao/clock/main/install.ps1 | iex
```

**From source (any platform, requires Rust):**

```sh
cargo install --git https://github.com/hainanzhao/clock
```

## Usage

```sh
clock                       # show the clock using your saved config
clock --face analog         # try a face for this run only (doesn't save)
clock --face digital --color green --no-date
```

While running:

| Key       | Action                        |
|-----------|-------------------------------|
| `q` / Esc | Quit                          |
| `d`       | Switch to the digital face    |
| `a`       | Switch to the analog face     |
| `t`       | Toggle 12h / 24h               |
| `s`       | Toggle seconds                 |
| `+` / `-` | Grow / shrink digits (digital) |

## Configuring

Settings persist in a small TOML file so `clock` always starts the way you
like it, without needing flags every time.

```sh
clock config path              # print the config file location
clock config show              # print the current config
clock config set face analog   # persist a setting
clock config set color "#33ccff"
clock config colors            # list built-in color names
clock config reset             # back to defaults
```

Config file (created on first `config set`, edit by hand too):

```toml
face = "digital"        # "digital" or "analog"
hour12 = true            # 12h with am/pm, or false for 24h
show_seconds = true
show_date = true
blink_colon = true       # digital: blink the ':' once a second
tick_marks = true         # analog: hour tick marks around the rim
scale = 2                 # digital: digit size, 1-4
color = "cyan"            # digits / clock face color
accent_color = "magenta"  # blinking colon / clock hands color
```

Colors accept the standard ANSI names (`red`, `green`, `blue`, ...) or
`#rrggbb` for truecolor terminals. Run `clock config colors` for the full
built-in list.

Every setting also has a matching `--flag` for one-off overrides — see
`clock --help`.

## Why it's lightweight

`clock` does no polling or busy-waiting. It sleeps until the display would
actually change — every 500ms to blink the digital colon, every second for a
seconds readout, or once a minute with seconds hidden — and otherwise sits
at 0% CPU, backed by your OS's native event notification (kqueue/epoll/IOCP)
via [crossterm](https://github.com/crossterm-rs/crossterm).

## Building from source

```sh
git clone https://github.com/hainanzhao/clock
cd clock
cargo build --release
./target/release/clock
```

## License

MIT
