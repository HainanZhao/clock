# Developer & Agent Lessons Learned

This document records key lessons learned from refactoring the terminal clock faces (LCD, BCD, Analog, Rings, and Blocks), ensuring that future development maintains the same level of geometric accuracy, performance, and usability.

---

## 1. System Execution & Path Verification
* **The Symptom**: Local code changes to the clock face rendered perfectly in unit tests but did not show up when running the `clock` command in the shell.
* **The Cause**: The developer's shell was executing a globally installed binary (located in `/Users/hainan.zhao/.cargo/bin/clock`) rather than the local debug/release targets in our workspace.
* **The Lesson**: When a user reports that local updates are not reflecting in their active terminal, always check `which -a clock` to identify if a global installation is taking precedence over the local cargo workspace. If so, compile and overwrite the global installation using:
  ```sh
  cargo install --path .
  ```

---

## 2. Terminal Cell Aspect Ratios & Geometry
A standard terminal character cell is roughly twice as tall as it is wide (a 1:2 aspect ratio). When rendering graphical faces on a terminal grid, you must compensate for this to avoid squashed, stretched, or pixelated shapes:

### A. Braille Canvas Drawing (`analog.rs` and `rings.rs`)
* Unicode Braille characters occupy a single terminal cell but contain a sub-pixel grid of `2x4` dots.
* Since a cell's physical aspect ratio is `1:2` and we divide it into `2x4` dots, each individual sub-pixel dot is exactly a **100% square** (`(w/2) / (h/4) = 1.0`).
* To draw a perfectly round circle of radius `R` sub-pixels:
  * **Width in cells**: `(2 * R) / 2 = R` columns.
  * **Height in cells**: `(2 * R) / 4 = R / 2` rows.
  * **The Bug**: The original code calculated rows to be identical to columns (`rows = cols`), which made the height of the canvas twice what was physically needed. This resulted in a tall vertically stretched ellipse with massive blank gaps at the top and bottom.
  * **The Rule**: Always set the canvas height in rows to **exactly half** of the canvas width in columns:
    ```rust
    let cols = (radius * 2.0 + 3.0).ceil() as usize;
    let rows = (radius + 2.0).ceil() as usize; // exactly half
    ```

### B. Block-Based Scaled Layouts (`binary.rs` BCD Clock)
* When drawing scalable solid blocks (e.g. BCD clock dots), simply scaling their column width makes them stretch into flat horizontal lines because each block row is only 1 cell high.
* **The Rule**: Always scale the block row height dynamically in proportion to its column width using a 2:1 cell ratio:
  ```rust
  let dot_h = (dot_w + 1) / 2;
  ```
  This guarantees that at any scale (from 1 to 6), BCD dots are drawn as beautifully proportioned, square-like blocks.

### C. Multi-Layer Canvas Overlays & Blend Priorities (`cuckoo.rs`, `radar.rs`, `ship.rs`)
* When drawing complex analog/circular graphic layouts (like orbits, steering wheels, crosshairs, and swinging lines), render them onto **separate, distinct `Canvas` buffers** first.
* To merge these layers into a single console printout, iterate over the cell rows/columns, and use a strict **blend priority** to decide which character (and color) goes on top (e.g. seconds hand/radar ray -> minutes/hour hands -> background grids/rims):
  ```rust
  let (ch, c) = if at(&sc) != ' ' {
      (at(&sc), sec_c)
  } else if at(&hc) != ' ' {
      (at(&hc), hands_c)
  } else {
      (at(&wc), wheel_c)
  };
  ```
* Draw **solid circular disks** procedurally by calling `Canvas::circle` repeatedly with increasing fractional radii:
  ```rust
  fn disk(canvas: &mut Canvas, cx: f64, cy: f64, r: f64) {
      let steps = (r * 2.0) as usize;
      for i in 0..=steps {
          canvas.circle(cx, cy, i as f64 * 0.5);
      }
  }
  ```
* Draw **dotted/dashed tracks** or crosshairs procedurally by rendering an arc/line and filtering with modulo steps:
  ```rust
  fn dotted_circle(canvas: &mut Canvas, cx: f64, cy: f64, r: f64, dot_spacing: usize) {
      let arc_px = r * TAU;
      let steps = (arc_px * 2.0).max(1.0) as u32;
      for i in 0..=steps {
          if i % (dot_spacing as u32) == 0 {
              let theta = i as f64 / steps as f64 * TAU;
              canvas.set(cx + r * theta.cos(), cy + r * theta.sin());
          }
      }
  }
  ```

---

## 3. Seven-Segment LCD Digit Geometry & Continuity (`seg7.rs`)
* Snapping physical segment boundaries flush is critical to prevent gaps and ensure a uniform digit height:
  * **Original Bug**: Upper segments (`b`/`f`) stopped at `3 * u` and lower segments (`c`/`e`) started at `4 * u`. This left a 1-cell gap in the middle of digits like `0` (where segment `g` is inactive) and made digits like `1`, `4`, and `7` physically shorter than others.
  * **The Fix**: Make upper segments cover `0..4 * u` and lower segments cover `3 * u..7 * u` so they meet and overlap exactly in the middle (`3 * u..4 * u`). This guarantees all digits are exactly `7 * u` tall and remain fully contiguous.

---

## 4. Visual Simplification Over Complexity (`blocks.rs`)
* Heavy, striped hour-alternating patterns or complex shading in full-screen grids can look like a cluttered, zebra-striped mess on larger terminal screens.
* Keep full-screen layouts clean and modern:
  * Use a **smooth, continuous gradient** for elapsed segments.
  * Use a **uniform dimmed color** for remaining/unspent segments.
  * Only highlight the currently active filling block.

---

## 5. Mathematical Geometric Assertions
* Visual validation by eyeballing terminal output is fragile. Always add comprehensive unit tests with **strict geometric assertions** to verify segment layouts.
* Assert that:
  * Every single digit spans the full grid height (active pixels exist on the top row `0` and bottom row `h - 1`).
  * Continuous vertical segments have no row `r` where all their columns are `false` (gap-free verification).

---

## 6. Asynchronous Background Integration & Alert Flashing (`calendar.rs`)
* **Background Thread State-Sharing**: Never block the main clock draw/tick thread with synchronous network requests (such as querying Google Calendar API). Instead:
  * Spawn a **lightweight background worker thread** on startup when `--calendar` is enabled.
  * Share fetched events safely using an `Arc<Mutex<Vec<CalendarEvent>>>`.
  * Periodically poll (e.g. every 30 seconds) in the background and swap out the shared event vector.
* **Full-Terminal Background Flashing**: To trigger the immersive red background flash (1 minute before any event), check the current time against the loaded events list in the central `draw` function. If it falls in the 1-minute window and the seconds are even:
    * Set the drawing background color (`bg_color`) to `Color::Red`.
    * Temporarily clone the `Config` and overwrite the theme colors to `"black"`. This creates a beautiful, ultra-high-contrast alarm state (black digits/hands on a solid bright-red background) across **all 16 clock faces automatically**!
    * Update `draw_block` and `draw_status` to accept the `bg` color argument and render their spacing, margins, borders, and empty padding regions with the active background color (using `SetBackgroundColor(bg)`), ensuring the entire terminal window is filled with solid red.
* **Procedural Wave Rendering (`waves.rs`)**: Draw gorgeous, continuous sine/cosine waves by mapping standard-width Braille sub-pixel grids. To represent hours, minutes, and seconds as flowing visual waveforms:
  * Overlap multiple distinct waves of different frequencies, amplitudes, and phases on a single canvas, and overlay a beautifully bordered bottom-aligned text card to house the digital time readout cleanly at the bottom (`let card_top = rows.saturating_sub(card_h)`).
* **First-Run Mock UX**: Always provide a default `mock_mode: true` fallback in the calendar JSON configuration so the feature is **fully testable immediately out of the box** (e.g. dynamically injecting a demo event starting in 65 seconds), bypassing the friction of Google Developer API key provisioning.
* **Time-Window Unit Testing**: Write unit tests comparing mock timestamps offset from `Local::now()` by varying durations (e.g. +45s, +60s, +75s, -10s) to verify exact boundary behavior of alert triggers under test.

---

## 7. Solid Full-Block Retro Grid Rendering & LCD Split-Flap Cards (`grid.rs`, `flip.rs`, and `render.rs`)
* **Solid Full-Block Grid Layouts**: When rendering custom 3x5 block digits, avoid using Braille sub-pixel matrices if the user's terminal/font draws Braille with disconnected dots (which look like a mesh of tiny dots instead of a solid line and can be hard to read).
  * Instead, utilize the standard **solid full-block character cell `█`** (`\u{2588}`) which covers the entire character box with a 100% filled, solid block of color on any terminal in the world.
  * **Square Pixel Scaling**: Because a character cell is physically twice as tall as it is wide, to make each block look perfectly square, render each block as **`u * 2`** columns wide and **`u`** rows high (where `u` is the scale unit). For a medium block size (`u = 1`), each block is exactly `██` (2 columns by 1 row), forming a perfect visual square on your screen.
  * **Proportional Gaps**: Space the solid blocks horizontally with exactly `u * 2` spaces (`inner_gap`), vertically with exactly `u` rows (`v_gap`), and separate digits with exactly `u * 4` spaces (`char_gap`). This guarantees an incredibly bold, beautifully spaced, and 100% solid retro grid that is perfectly legible from any distance.
  * **Color Consistency**: Draw the blinking colons (`:`) in the exact same color as your main digits (`primary` color) instead of accent colors, ensuring the clock face remains perfectly unified and clean.
* **Font Consistency in Card Faces**: Always favor segment-snapped LCD fonts (`seg7::render`) over continuous geometric vectors when simulating split-flap airport flap-boards (`flip.rs`). The segmented look matches the split physical plastic cards perfectly! When drawing borders, set the internal margin and padding exactly so that the LCD grid centers cleanly inside the cards.

---

## 8. Animated / Analog Clock Face Seconds Safeguard (`app.rs`)
* **The Symptom**: When seconds are hidden (`show_seconds = false` or `--no-seconds`), the clock's wake interval naturally defaults to once a minute (`60_000` ms) to save CPU. However, on highly animated, hand-based, or mechanical clock faces (like `analog`, `rings`, `hourglass`, `cuckoo`, `radar`, `ship`), this caused the sweeping hands, pendulums, and radar rays to freeze and appear broken.
* **The Rule**: Always treat mechanical, animated, and analog-style faces as requiring seconds.
  * In the main event loop wake calculation (`next_wake` in `app.rs`), force the thread to wake up **every second** if any of these animated faces are active:
    ```rust
    let animated = matches!(cfg.face, Face::Analog | Face::Rings | Face::Hourglass | Face::Cuckoo | Face::Radar | Face::Ship);
    ```
  * In the individual face renderers, draw second hands/tracks unconditionally, bypassing the `show_seconds = false` check to preserve their vital kinetic animations.

---

## 9. Interactive CLI Integrations (`calendar.rs` and `main.rs`)
* **Interactive CLI Setups**: When launching a raw terminal application (with alternative screens and hide cursor), ensure any interactive prompts or setups (such as Google Calendar link authorization) are executed **before** raw terminal mode is initialized.
  * Run the setup checks sequentially in `main` so the user can easily see prompts, type inputs, and configure files directly in their standard terminal before the alternate screen clears their scrollback buffer.

---

## 10. Colon Blinking Wake-Up Safeguard (`app.rs`)
* **The Symptom**: When a block-based grid or text face uses blinking colons (`:` blinking on/off every 500ms), the main event loop (`next_wake` in `app.rs`) must explicitly wake up every **500 milliseconds** instead of once per second.
* **The Cause**: If the face is not in the `blinks` match block inside `next_wake`, the clock only wakes up on 1-second (1000ms) boundaries. Since 1000ms is a multiple of 1000, the clock always wakes up at the exact same sub-second phase, causing the blinking colon to appear completely frozen and permanently static on the screen.
* **The Rule**: Always add any face that supports blinking colons (like `Face::Grid`) to the `blinks` matches in `next_wake`:
  ```rust
  let blinks = cfg.blink_colon && matches!(cfg.face, Face::Digital | Face::Matrix | Face::Flip | Face::Lcd | Face::Grid);
  ```

---

## 11. Consistent Auto-Scaling Footprints (`app.rs` and face renderers)
* **The Symptom**: When seconds are hidden (`show_seconds = false`), the rendered time string becomes shorter (e.g. `12:48` instead of `12:48:07`). Under default auto-scaling (`scale = 0`), this shorter text has more room, causing the font size to balloon up enormously and make the letters look completely different (thicker, rounder, and larger) compared to when seconds are visible. Toggling seconds causes the entire clock to radically change size and jump around.
* **The Rule**: Always calculate the auto-fitting scale based on the **full 8-character string (with seconds)**, even when seconds are hidden:
  * In `app::current_scale` and `digital::render`, calculate height fitting as if 3 extra characters (the seconds and colon) are present.
  * In `matrix::render` and `seg7::fit_unit`, append a mock `":00"` suffix when determining the fit scale.
  * In `flip::render` and `grid::render`, add `2` or `3` to the character length during fit calculations.
  This ensures that toggling seconds off/on cleanly slides them in and out of view **without ever changing the size, thickness, or scale of the remaining hours and minutes**, providing a premium, unified user experience!

---

## 12. Robust Parsing Exception Fallback (`config.rs`)
* **The Symptom**: When an existing clock face variant is renamed or removed in a future release (e.g. replacing `"cats"` with `"grid"`, or `"bars"` with `"waves"`), users' saved `config.toml` files on disk will still contain the old obsolete string (e.g. `face = "cats"`). If using default derives, this results in a critical Serde deserialization crash on startup, preventing the clock from running or even launching help/reset subcommands.
* **The Rule**: Always implement manual `Deserialize` on any configuration enums (like `Face`) to gracefully intercept unknown variants.
  * If the parser encounters an unknown, obsolete, or mistyped face name, catch the exception and gracefully fallback to the first clock face (`Face::Digital` / default):
    ```rust
    impl<'de> Deserialize<'de> for Face {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            Ok(match s.to_ascii_lowercase().as_str() {
                "digital" => Face::Digital,
                "analog" => Face::Analog,
                ...
                _ => Face::Digital, // Graceful fallback
            })
        }
    }
    ```
  * Always accompany this with dedicated deserialization unit tests to verify proper fallback behavior across releases.

---

## 13. Interactive Terminal Mouse Selection Support (`app.rs`)
* **Mouse Event Capture**: Capturing mouse events (such as clicks) transforms static console logs into dynamic, desktop-grade graphical interfaces.
  * Enable/Disable mouse event streaming in the raw alternate screen block:
    ```rust
    use crossterm::event::{EnableMouseCapture, DisableMouseCapture};
    execute!(out, EnterAlternateScreen, Hide, EnableMouseCapture)?;
    ```
  * In the main render loop, map standard raw screen click coordinates (`mouse_event.column`, `mouse_event.row`) to your custom card bounding boxes (`cx >= x0 && cx < x0 + box_w && cy >= y0 && cy < y0 + box_h + label_h`) to determine exactly which card the user clicked on.
  * Establish intuitive click workflows (e.g. single-clicking an inactive card highlights/selects it, while clicking the already-selected card instantly activates it and closes the picker), providing a seamless, mouse-friendly, and visual selection experience!

---

## 14. Solid-by-Default Fallback Accent Color Resolution (`config.rs` and `app.rs`)
* **The Symptom**: When a user overrides the main clock color using the CLI (e.g., `clock --color red`), they intuitively expect the entire clock face to render in solid red. However, because `accent_color` defaults to a saved color like `#3b82f6` (blue) in their configuration file on disk, the clock's gradient engine blends `red` (left) into `blue` (right), producing a purplish/magenta clock which is highly confusing.
* **The Rule**: Always default the `accent_color` configuration setting to `"none"`, and centrally resolve it to fall back to the primary `color` automatically.
  * Define `default_accent` to return `"none"`.
  * Implement an accent resolution method `resolve_accent` on the `Config` structure:
    ```rust
    pub fn resolve_accent(&self) -> String {
        if self.accent_color.is_empty() || self.accent_color.to_ascii_lowercase() == "none" {
            self.color.clone()
        } else {
            self.accent_color.clone()
        }
    }
    ```
  * Intercept and resolve this color centrally in the high-level face renderer dispatcher (`render_face` in `src/app.rs`) before passing the configuration reference to the 16 individual face renderers.
  * This guarantees that **any color overrides result in a 100% solid, pure, high-contrast clock face by default**, completely avoiding unexpected color blending for the user while fully preserving gradient functionality for power users who explicitly configure different colors!

---

## 15. Interactive Keyboard Color Cycling & Session Persistence (`app.rs`)
* **Interactive Color Cycling**: Allow users to toggle and customize clock themes live on the run.
  * Map the **`c`** key in the keypress event matching block to cycle through a beautifully curated palette of terminal-friendly, high-contrast presets:
    ```rust
    const PRESETS: &[&str] = &[
        "#38d9e8", // Cyan
        "#10b981", // Emerald Green
        "#f59e0b", // Amber
        "#ef4444", // Red
        "#a855f7", // Purple
        "#3b82f6", // Blue
        "#ffffff", // White
    ];
    ```
  * Update `cfg.color` dynamically, set `needs_clear = true` to force a clean screen redraw, and update the session-saver `persist_session` to check for color changes and automatically save the selected color permanently to disk on exit (`stored.color = after.color`). This creates a frictionless, fun, and highly responsive user configuration experience!

