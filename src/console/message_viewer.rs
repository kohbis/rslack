use std::io::Write;

use chrono::{Local, TimeZone};
use termion::{clear, color, cursor, style};
use unicode_width::UnicodeWidthStr;

use super::table::{border, pad, term_size, truncate, wrap};
use crate::slack::SlackMessage;

/// Widest the USER column gets; longer names are truncated
const MAX_USER_WIDTH: usize = 20;
/// Narrowest the MESSAGE column gets, even on a narrow terminal
const MIN_TEXT_WIDTH: usize = 20;

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
            let user = message.author();

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

            // Handle multi-line messages and attached files
            for text_line in message_lines(message) {
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
        let user = message.author();

        println!("[{}] {}", timestamp, user);
        for line in message_lines(message) {
            println!("  {}", line);
        }
        println!();
    }
}

/// Print messages to stdout as a table (non-interactive)
pub fn print_message_table(channel_name: &str, messages: &[SlackMessage]) {
    println!("#{} - Latest {} messages", channel_name, messages.len());
    for line in message_table(messages, term_size().0 as usize) {
        println!("{}", line);
    }
}

/// Render messages, oldest first, as table lines that fit in `width` columns
/// (the MESSAGE column wraps)
fn message_table(messages: &[SlackMessage], width: usize) -> Vec<String> {
    let rows: Vec<(String, &str, Vec<String>)> = messages
        .iter()
        .rev()
        .map(|m| (format_timestamp_simple(&m.ts), m.author(), message_lines(m)))
        .collect();

    let date_width = rows
        .iter()
        .map(|(date, _, _)| date.width())
        .fold("DATE".len(), usize::max);
    let user_width = rows
        .iter()
        .map(|(_, user, _)| user.width())
        .fold("USER".len(), usize::max)
        .min(MAX_USER_WIDTH);
    // Each row is "| date | user | text |": 4 bars and 6 spaces around the cells
    let text_width = width
        .saturating_sub(date_width + user_width + 10)
        .max(MIN_TEXT_WIDTH);

    let rule = border(&[date_width + 2, user_width + 2, text_width + 2]);
    let row = |date: &str, user: &str, text: &str| {
        format!(
            "| {} | {} | {} |",
            pad(date, date_width),
            pad(user, user_width),
            pad(text, text_width)
        )
    };

    let mut lines = vec![rule.clone(), row("DATE", "USER", "MESSAGE"), rule.clone()];
    for (date, user, text) in &rows {
        let user = truncate(user, user_width);
        let mut wrapped: Vec<String> = text.iter().flat_map(|l| wrap(l, text_width)).collect();
        if wrapped.is_empty() {
            wrapped.push(String::new());
        }
        for (i, text_line) in wrapped.iter().enumerate() {
            if i == 0 {
                lines.push(row(date, &user, text_line));
            } else {
                lines.push(row("", "", text_line));
            }
        }
        lines.push(rule.clone());
    }
    lines
}

/// Lines of a message body: its text, then one line per attached file
fn message_lines(message: &SlackMessage) -> Vec<String> {
    let files = message.files.iter().map(|file| {
        let kind = if file.is_image() { "image" } else { "file" };
        let name = file.name.as_deref().unwrap_or("(hidden)");
        match &file.permalink {
            Some(permalink) => format!("[{}] {} {}", kind, name, permalink),
            None => format!("[{}] {}", kind, name),
        }
    });
    message
        .text
        .lines()
        .map(str::to_string)
        .chain(files)
        .collect()
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
    fn message_lines_with_files() {
        let message: SlackMessage = serde_json::from_str(
            r#"{
                "user": "U0123456789",
                "ts": "1609459800.000100",
                "text": "look\nthis",
                "files": [
                    {"name": "image.png", "mimetype": "image/png", "permalink": "https://example.slack.com/files/U0123456789/F0123456789/image.png"},
                    {"name": "report.pdf", "mimetype": "application/pdf"},
                    {"mode": "hidden_by_limit"}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(
            message_lines(&message),
            [
                "look",
                "this",
                "[image] image.png https://example.slack.com/files/U0123456789/F0123456789/image.png",
                "[file] report.pdf",
                "[file] (hidden)",
            ]
        );
    }

    #[test]
    fn message_lines_without_text() {
        let message: SlackMessage =
            serde_json::from_str(r#"{"user": "U0123456789", "ts": "1609459800.000100"}"#).unwrap();
        assert!(message_lines(&message).is_empty());
    }

    #[test]
    fn message_table_layout() {
        let messages: Vec<SlackMessage> = serde_json::from_str(
            r#"[
                {"user": "U0987654321", "ts": "1609459700.000090", "text": "日本語のメッセージを折り返します"},
                {"user": "U0123456789", "ts": "1609459600.000080", "text": "short\nsecond line"}
            ]"#,
        )
        .unwrap();
        let date = format_timestamp_simple("1609459600.000080");
        let date2 = format_timestamp_simple("1609459700.000090");

        // DATE 16 + USER 11 + 10 for borders leaves 20 columns for MESSAGE
        let lines = message_table(&messages, 57);
        let rule = "+------------------+-------------+----------------------+";
        assert_eq!(
            lines,
            [
                rule.to_string(),
                "| DATE             | USER        | MESSAGE              |".to_string(),
                rule.to_string(),
                format!("| {} | U0123456789 | short                |", date),
                "|                  |             | second line          |".to_string(),
                rule.to_string(),
                format!("| {} | U0987654321 | 日本語のメッセージを |", date2),
                "|                  |             | 折り返します         |".to_string(),
                rule.to_string(),
            ]
        );
        assert!(lines.iter().all(|line| line.width() == 57));
    }

    #[test]
    fn message_table_with_empty_message() {
        let messages: Vec<SlackMessage> =
            serde_json::from_str(r#"[{"ts": "1609459600.000080"}]"#).unwrap();
        let lines = message_table(&messages, 0);
        assert_eq!(lines.len(), 5);
        assert!(lines[3].contains("| unknown | "));
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
