//! Google Calendar Integration: fetches calendar events and checks
//! if we are within the 1-minute window before any upcoming event.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Local};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    pub summary: String,
    pub start_time: String, // RFC3339 format, e.g. "2026-08-14T16:30:00+08:00"
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CalendarConfig {
    pub api_key: String,
    pub calendar_id: String,
    pub mock_mode: bool,
    pub mock_events: Vec<CalendarEvent>,
}

impl CalendarConfig {
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("clock");
            p.push("calendar.json");
            p
        })
    }

    pub fn load_or_create() -> Self {
        let Some(path) = Self::default_path() else {
            return Self::mock_default();
        };

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<CalendarConfig>(&content) {
                return cfg;
            }
        }

        let default_cfg = Self::mock_default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(serialized) = serde_json::to_string_pretty(&default_cfg) {
            let _ = fs::write(&path, serialized);
        }
        default_cfg
    }

    fn mock_default() -> Self {
        CalendarConfig {
            api_key: String::new(),
            calendar_id: "primary".to_string(),
            mock_mode: true,
            mock_events: Vec::new(),
        }
    }
}

/// Parses an RFC3339 string into Local DateTime.
pub fn parse_time(time_str: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(time_str)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
}

/// Checks if we are 1 minute (1 to 60 seconds) before any event.
pub fn should_flash(now: DateTime<Local>, events: &[CalendarEvent]) -> bool {
    for event in events {
        if let Some(start) = parse_time(&event.start_time) {
            let diff = start.signed_duration_since(now).num_seconds();
            // Trigger 1 minute (1 to 60 seconds) before the event starts.
            if diff > 0 && diff <= 60 {
                return true;
            }
        }
    }
    false
}

/// Gets the list of upcoming events. If mock_mode is enabled, it returns the stored
/// mock events, plus a dynamically generated mock event starting in 65 seconds
/// so the user can easily see and test the red flash effect immediately!
pub fn get_events(cfg: &CalendarConfig, now: DateTime<Local>) -> Vec<CalendarEvent> {
    if cfg.mock_mode {
        let mut events = cfg.mock_events.clone();
        
        // Dynamically insert a mock event starting in 65 seconds
        let demo_start = now + chrono::Duration::seconds(65);
        events.push(CalendarEvent {
            summary: "Demo Team Standup".to_string(),
            start_time: demo_start.to_rfc3339(),
        });
        events
    } else {
        // Query Google Calendar API
        if cfg.api_key.is_empty() {
            return Vec::new();
        }
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events?key={}&timeMin={}&singleEvents=true&orderBy=startTime&maxResults=5",
            cfg.calendar_id,
            cfg.api_key,
            now.to_rfc3339()
        );
        match ureq::get(&url).call() {
            Ok(resp) => {
                #[derive(Deserialize)]
                struct GCalResponse {
                    items: Option<Vec<GCalEvent>>,
                }
                #[derive(Deserialize)]
                struct GCalEvent {
                    summary: Option<String>,
                    start: Option<GCalTime>,
                }
                #[derive(Deserialize)]
                struct GCalTime {
                    #[serde(rename = "dateTime")]
                    date_time: Option<String>,
                }

                if let Ok(res) = resp.into_json::<GCalResponse>() {
                    let mut events = Vec::new();
                    if let Some(items) = res.items {
                        for item in items {
                            if let (Some(summary), Some(start)) = (item.summary, item.start) {
                                if let Some(dt_str) = start.date_time {
                                    events.push(CalendarEvent {
                                        summary,
                                        start_time: dt_str,
                                    });
                                }
                            }
                        }
                    }
                    events
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }
}

pub fn init_integration() -> anyhow::Result<()> {
    let mut cal_cfg = CalendarConfig::load_or_create();
    
    // Only prompt for setup if they are in the initial unconfigured state (api_key is empty)
    if cal_cfg.api_key.is_empty() {
        println!();
        println!("=======================================================");
        println!("             Google Calendar Integration Setup         ");
        println!("=======================================================");
        println!("You have enabled the Google Calendar integration!");
        println!("By default, this is running in MOCK (demo) mode with a");
        println!("dummy event starting exactly 1 minute in the future.");
        println!();
        println!("Would you like to set up a real Google Calendar integration now? [y/N]: ");
        
        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let trimmed = input.trim().to_ascii_lowercase();
        
        if trimmed == "y" || trimmed == "yes" {
            println!();
            println!("Please enter your Google Calendar API Key: ");
            let mut api_key = String::new();
            let _ = std::io::stdin().read_line(&mut api_key);
            cal_cfg.api_key = api_key.trim().to_string();
            
            println!("Please enter your Google Calendar ID (default: primary): ");
            let mut cal_id = String::new();
            let _ = std::io::stdin().read_line(&mut cal_id);
            let cal_id_trimmed = cal_id.trim();
            cal_cfg.calendar_id = if cal_id_trimmed.is_empty() {
                "primary".to_string()
            } else {
                cal_id_trimmed.to_string()
            };
            
            cal_cfg.mock_mode = false;
            
            // Save the newly configured values to calendar.json
            if let Some(path) = CalendarConfig::default_path() {
                if let Ok(serialized) = serde_json::to_string_pretty(&cal_cfg) {
                    let _ = fs::write(&path, serialized);
                }
            }
            
            println!();
            println!("Google Calendar successfully linked! Launching clock...");
            std::thread::sleep(std::time::Duration::from_secs(1));
        } else {
            println!("Proceeding in MOCK mode. Launching clock...");
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_should_flash_triggers() {
        let now = Local::now();

        // 1. Event in 45 seconds -> SHOULD FLASH
        let event_soon = CalendarEvent {
            summary: "Soon".to_string(),
            start_time: (now + Duration::seconds(45)).to_rfc3339(),
        };
        assert!(should_flash(now, &[event_soon]));

        // 2. Event in 60 seconds -> SHOULD FLASH (edge of 1 minute)
        let event_exact = CalendarEvent {
            summary: "Exact".to_string(),
            start_time: (now + Duration::seconds(60)).to_rfc3339(),
        };
        assert!(should_flash(now, &[event_exact]));

        // 3. Event in 75 seconds -> SHOULD NOT FLASH
        let event_far = CalendarEvent {
            summary: "Far".to_string(),
            start_time: (now + Duration::seconds(75)).to_rfc3339(),
        };
        assert!(!should_flash(now, &[event_far]));

        // 4. Past event (started 10 seconds ago) -> SHOULD NOT FLASH
        let event_past = CalendarEvent {
            summary: "Past".to_string(),
            start_time: (now - Duration::seconds(10)).to_rfc3339(),
        };
        assert!(!should_flash(now, &[event_past]));
    }
}
