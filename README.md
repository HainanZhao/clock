# clock

A beautiful, configurable clock for your terminal with 9 faces — written in
Rust, ships as a single lightweight binary, idles at ~0% CPU.

```
      ██       ██████                    ██       ██                      ██     ██████
      ██       ██████                    ██       ██                      ██     ██████
    ████     ██      ██     ██         ████     ████         ██         ████   ██      ██
    ████     ██      ██     ██         ████     ████         ██         ████   ██      ██
      ██             ██     ██       ██  ██       ██         ██       ██  ██   ██      ██
      ██             ██     ██       ██  ██       ██         ██       ██  ██   ██      ██
      ██           ██              ██    ██       ██                ██    ██     ██████
      ██           ██              ██    ██       ██                ██    ██     ██████
      ██         ██         ██     ██████████     ██         ██     ██████████ ██      ██
      ██         ██         ██     ██████████     ██         ██     ██████████ ██      ██
      ██       ██           ██           ██       ██         ██           ██   ██      ██
      ██       ██           ██           ██       ██         ██           ██   ██      ██
    ██████   ██████████                  ██     ██████                    ██     ██████
    ██████   ██████████                  ██     ██████                    ██     ██████
                                   Friday, August 14 2026
```

## Faces

| Face      | Look                                                       |
|-----------|-------------------------------------------------------------|
| `digital` | Big blocky LED-style digits                                 |
| `analog`  | Round clock face with hands, drawn in braille sub-pixels     |
| `binary`  | Binary-coded-decimal dot grid, one column per digit          |
| `word`    | Natural language — "TWENTY PAST FOUR" in big letters          |
| `matrix`  | Sharp 7-segment digits drawn in braille sub-pixels           |
| `flip`    | Retro split-flap board — each digit on a card with a seam     |
| `bars`    | Horizontal progress bars through the hour, minute and second |
| `rings`   | Concentric progress arcs, time in the middle                 |
| `roman`   | Roman numerals, stacked and oversized                        |

Switch faces live with the Left/Right arrow keys, or press `Tab` for a
picker grid showing a live preview of every face at once.

Every face auto-scales to fill your terminal — the bigger the window, the
bigger the clock. Press `+`/`-` to override the size, `0` to go back to auto.

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
| `+` / `-`   | Grow / shrink the clock                  |
| `0`         | Back to auto-fill sizing                 |

In the picker: arrow keys move the selection, `Enter` confirms, `Esc`
cancels.

Whatever you switch to during a session is remembered — face, 12/24h, seconds
and size are written back to the config on exit, so restarting picks up where
you left off. One-off `--flag` overrides are *not* saved.

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
face = "digital"          # digital, analog, binary, word, matrix, flip, bars, rings, roman
hour12 = true             # 12h with am/pm, or false for 24h
show_seconds = true
show_date = true
blink_colon = true        # digital/matrix/flip: blink the ':' once a second
tick_marks = true         # analog: hour tick marks around the rim
scale = 0                 # 0 = auto-fill the terminal; 1-9 to pin a size
color = "#38d9e8"         # primary color
accent_color = "#3b82f6"  # gradient end / hands / accents
```

Faces are drawn with a gradient running from `color` to `accent_color`, and
the multi-color faces (`bars`, `rings`, `analog`, `binary`) additionally give
the hour, minute and second their own hues.

Colors accept the standard ANSI names (`red`, `green`, `blue`, ...) or
`#rrggbb` for truecolor terminals. Run `clock config colors` for the full
built-in list.

Every setting also has a matching `--flag` for one-off overrides — see
`clock --help`.

## Rendering

Glyphs are not a scaled-up pixel grid. Each digit and letter is described as
vector strokes — straight segments and elliptical arcs of constant width with
round caps, in the spirit of a light geometric sans — so the letterforms stay
true at any size.

Strokes are rasterized at sub-cell resolution and drawn with quadrant block
characters. All sixteen combinations of a 2x2 split exist in Unicode, so every
terminal cell carries four sub-pixels; half-blocks alone would subdivide only
vertically and leave curves visibly stepped along the horizontal axis.

Edges are hard rather than anti-aliased: blending partial coverage toward the
background reads as a grey halo around the strokes rather than as smoothing.

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
