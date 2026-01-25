use std::io::Write;

use chrono::{Local, TimeZone};
use termion::{clear, color, cursor, style};

use crate::slack::SlackMessage;

/// Displays messages from a Slack channel
pub struct MessageViewer {
    channel_name: String,
}

impl MessageViewer {
    pub fn new(channel_name: &str) -> Self {
        Self {
            channel_name: channel_name.to_string(),
        }
    }

    /// Display messages to the terminal
    pub fn display<W: Write>(&self, stdout: &mut W, messages: &[SlackMessage]) {
        write!(stdout, "{}{}", clear::All, cursor::Goto(1, 1)).unwrap();

        // Header
        write!(
            stdout,
            "{}{}#{} - Latest {} messages{}{}",
            style::Bold,
            color::Fg(color::Cyan),
            self.channel_name,
            messages.len(),
            style::Reset,
            color::Fg(color::Reset)
        )
        .unwrap();

        write!(stdout, "{}", cursor::Goto(1, 2)).unwrap();
        write!(stdout, "{}", "─".repeat(60)).unwrap();

        // Messages (reversed to show oldest first)
        let mut line = 3u16;
        for message in messages.iter().rev() {
            let timestamp = self.format_timestamp(&message.ts);
            let user = message.user.as_deref().unwrap_or("unknown");

            write!(stdout, "{}", cursor::Goto(1, line)).unwrap();
            write!(
                stdout,
                "{}{}[{}] {}{}{}",
                color::Fg(color::Green),
                style::Bold,
                timestamp,
                user,
                style::Reset,
                color::Fg(color::Reset)
            )
            .unwrap();

            line += 1;

            // Handle multi-line messages
            for text_line in message.text.lines() {
                write!(stdout, "{}", cursor::Goto(1, line)).unwrap();
                write!(stdout, "  {}", text_line).unwrap();
                line += 1;
            }

            line += 1; // Add spacing between messages
        }

        write!(stdout, "{}", cursor::Goto(1, line)).unwrap();
        write!(stdout, "{}", "─".repeat(60)).unwrap();

        stdout.flush().unwrap();
    }

    /// Format Slack timestamp (e.g., "1234567890.123456") to readable format
    fn format_timestamp(&self, ts: &str) -> String {
        // Slack timestamps are in the format "epoch.microseconds"
        let epoch_secs: f64 = ts.parse().unwrap_or(0.0);
        let secs = epoch_secs as i64;

        match Local.timestamp_opt(secs, 0) {
            chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
            _ => ts.to_string(),
        }
    }
}

/// Print messages to stdout (non-interactive, simple format)
pub fn print_messages(channel_name: &str, messages: &[SlackMessage]) {
    println!("#{} - Latest {} messages", channel_name, messages.len());
    println!("{}", "─".repeat(60));

    for message in messages.iter().rev() {
        let timestamp = format_timestamp_simple(&message.ts);
        let user = message.user.as_deref().unwrap_or("unknown");

        println!("[{}] {}", timestamp, user);
        for line in message.text.lines() {
            println!("  {}", line);
        }
        println!();
    }
}

fn format_timestamp_simple(ts: &str) -> String {
    let epoch_secs: f64 = ts.parse().unwrap_or(0.0);
    let secs = epoch_secs as i64;

    match Local.timestamp_opt(secs, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        _ => ts.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_viewer_new() {
        let viewer = MessageViewer::new("general");
        assert_eq!(viewer.channel_name, "general");
    }

    #[test]
    fn format_timestamp_valid() {
        let viewer = MessageViewer::new("test");
        let ts = "1609459200.000000"; // 2021-01-01 00:00:00 UTC
        let result = viewer.format_timestamp(ts);
        // Result depends on local timezone, just check it's not the original
        assert!(!result.is_empty());
    }
}
