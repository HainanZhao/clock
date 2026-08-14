//! Binary-coded-decimal clock: each decimal digit of HH:MM(:SS) shown as a
//! column of 4 dots (bit weights 8,4,2,1 top to bottom).

use crate::config::Config;
use chrono::{DateTime, Local, Timelike};

const ON: char = '●';
const OFF: char = '○';

fn digit_column(d: u32) -> [char; 4] {
    [
        if d & 0b1000 != 0 { ON } else { OFF },
        if d & 0b0100 != 0 { ON } else { OFF },
        if d & 0b0010 != 0 { ON } else { OFF },
        if d & 0b0001 != 0 { ON } else { OFF },
    ]
}

pub fn render(now: DateTime<Local>, cfg: &Config) -> Vec<String> {
    let (hour, _) = if cfg.hour12 {
        let h = now.hour12().1;
        (if h == 0 { 12 } else { h }, ())
    } else {
        (now.hour(), ())
    };

    let mut digits = vec![hour / 10, hour % 10, now.minute() / 10, now.minute() % 10];
    let mut header = vec!["H", "H", "M", "M"];
    if cfg.show_seconds {
        digits.push(now.second() / 10);
        digits.push(now.second() % 10);
        header.push("S");
        header.push("S");
    }

    let columns: Vec<[char; 4]> = digits.into_iter().map(digit_column).collect();

    let mut lines = Vec::with_capacity(6);
    lines.push(header.join("   "));
    lines.push(String::new());
    for row in 0..4 {
        let cells: Vec<String> = columns.iter().map(|c| c[row].to_string()).collect();
        lines.push(cells.join("   "));
    }

    if cfg.show_date {
        lines.push(String::new());
        lines.push(now.format("%A, %B %-d %Y").to_string());
    }

    lines
}
