//! "TEN PAST FOUR" style word clock, rounded to the nearest 5 minutes.

use crate::config::Config;
use chrono::{DateTime, Local, Timelike};

const HOURS: [&str; 12] = [
    "TWELVE", "ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE", "TEN",
    "ELEVEN",
];

fn phrase(now: DateTime<Local>) -> String {
    let base_hour = (now.hour() % 12) as i64; // 0 = twelve
    let total = base_hour * 60 + now.minute() as i64;
    let rounded = ((total + 2) / 5 * 5).rem_euclid(12 * 60);
    let hour_idx = (rounded / 60) as usize % 12;
    let next_idx = (hour_idx + 1) % 12;

    match rounded % 60 {
        0 => format!("{} O'CLOCK", HOURS[hour_idx]),
        5 => format!("FIVE PAST {}", HOURS[hour_idx]),
        10 => format!("TEN PAST {}", HOURS[hour_idx]),
        15 => format!("QUARTER PAST {}", HOURS[hour_idx]),
        20 => format!("TWENTY PAST {}", HOURS[hour_idx]),
        25 => format!("TWENTY-FIVE PAST {}", HOURS[hour_idx]),
        30 => format!("HALF PAST {}", HOURS[hour_idx]),
        35 => format!("TWENTY-FIVE TO {}", HOURS[next_idx]),
        40 => format!("TWENTY TO {}", HOURS[next_idx]),
        45 => format!("QUARTER TO {}", HOURS[next_idx]),
        50 => format!("TEN TO {}", HOURS[next_idx]),
        55 => format!("FIVE TO {}", HOURS[next_idx]),
        _ => unreachable!("rounded to a multiple of 5"),
    }
}

/// Word-wraps `text` to at most `width` columns, breaking on spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        if !cur.is_empty() && cur.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

pub fn render(now: DateTime<Local>, cfg: &Config) -> Vec<String> {
    let mut lines = wrap(&phrase(now), 24);

    if cfg.show_seconds {
        lines.push(String::new());
        let fmt = if cfg.hour12 { "%I:%M:%S %p" } else { "%H:%M:%S" };
        lines.push(now.format(fmt).to_string());
    }
    if cfg.show_date {
        lines.push(String::new());
        lines.push(now.format("%A, %B %-d %Y").to_string());
    }
    lines
}
