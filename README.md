# clock

A beautiful, configurable clock for your terminal with 5 faces — written in
Rust, ships as a single lightweight binary, idles at ~0% CPU.

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

## Faces

| Face      | Look                                                    |
|-----------|----------------------------------------------------------|
| `digital` | Big blocky LED-style digits                              |
| `analog`  | Round clock face with hands, drawn in braille sub-pixels  |
| `binary`  | Classic binary-coded-decimal dot grid                     |
| `word`    | Natural language — "TEN PAST FOUR"                         |
| `matrix`  | Sharper, smaller 7-segment digits, also drawn in braille   |

Switch faces live with the Left/Right arrow keys, or press `Tab` for a
picker grid showing a live preview of every face at once.

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

| Key         | Action                                  |
|-------------|------------------------------------------|
| `q` / Esc   | Quit                                     |
| `←` / `→`   | Cycle to the previous / next face        |
| `Tab`       | Open a grid picker with a live preview of every face |
| `t`         | Toggle 12h / 24h                         |
| `s`         | Toggle seconds                           |
| `+` / `-`   | Grow / shrink digits (digital/matrix)    |

In the picker: arrow keys move the selection, `Enter` confirms, `Esc`
cancels.

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
face = "digital"        # digital, analog, binary, word, or matrix
hour12 = true            # 12h with am/pm, or false for 24h
show_seconds = true
show_date = true
blink_colon = true       # digital/matrix: blink the ':' once a second
tick_marks = true         # analog: hour tick marks around the rim
scale = 2                 # digital/matrix: digit size, 1-4
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
