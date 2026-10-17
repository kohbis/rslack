use anyhow::Result;
use chrono::{Local, TimeZone};
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use super::table::{border, pad, term_size, truncate, wrap};
use crate::slack::{SlackFile, SlackMessage};

/// Widest the USER column gets; longer names are truncated
const MAX_USER_WIDTH: usize = 20;
/// Narrowest the MESSAGE column gets, even on a narrow terminal
const MIN_TEXT_WIDTH: usize = 20;

/// `#general` for a channel. A direct-message label already starts with `@`.
pub fn channel_heading(name: &str) -> String {
    if name.starts_with('@') {
        name.to_string()
    } else {
        format!("#{name}")
    }
}

/// Print messages to stdout (non-interactive, simple format).
/// `thread_ts` is set when these messages are one thread.
pub fn print_messages(channel_name: &str, thread_ts: Option<&str>, messages: &[SlackMessage]) {
    println!(
        "{}",
        history_heading(channel_name, thread_ts, messages.len())
    );
    println!("{}", "─".repeat(60));

    for message in messages.iter().rev() {
        println!("{}", message_header(message));
        for line in message_lines(message) {
            println!("  {}", line);
        }
        println!();
    }
}

/// Print messages as a JSON array on stdout, oldest first, with no other text
pub fn print_messages_json(messages: &[SlackMessage]) -> Result<()> {
    serde_json::to_writer_pretty(std::io::stdout(), &json_messages(messages))?;
    println!();
    Ok(())
}

#[derive(Serialize)]
struct JsonFile<'a> {
    name: Option<&'a str>,
    mimetype: Option<&'a str>,
    permalink: Option<&'a str>,
}

#[derive(Serialize)]
struct JsonMessage<'a> {
    ts: &'a str,
    thread_ts: Option<&'a str>,
    user: Option<&'a str>,
    user_name: Option<&'a str>,
    text: &'a str,
    reply_count: Option<u32>,
    files: Vec<JsonFile<'a>>,
}

fn json_file(file: &SlackFile) -> JsonFile<'_> {
    JsonFile {
        name: file.name.as_deref(),
        mimetype: file.mimetype.as_deref(),
        permalink: file.permalink.as_deref(),
    }
}

/// Oldest first, matching the text and table output
fn json_messages(messages: &[SlackMessage]) -> Vec<JsonMessage<'_>> {
    messages
        .iter()
        .rev()
        .map(|message| JsonMessage {
            ts: &message.ts,
            thread_ts: message.thread_ts.as_deref(),
            user: message.user.as_deref(),
            user_name: message.user_name.as_deref(),
            text: &message.text,
            reply_count: message.reply_count,
            files: message.files.iter().map(json_file).collect(),
        })
        .collect()
}

/// Print messages to stdout as a table (non-interactive)
pub fn print_message_table(channel_name: &str, thread_ts: Option<&str>, messages: &[SlackMessage]) {
    println!(
        "{}",
        history_heading(channel_name, thread_ts, messages.len())
    );
    for line in message_table(messages, term_size().0 as usize) {
        println!("{}", line);
    }
}

fn history_heading(channel_name: &str, thread_ts: Option<&str>, count: usize) -> String {
    let channel = channel_heading(channel_name);
    match thread_ts {
        Some(ts) => format!("{channel} - Thread {ts} ({count} messages)"),
        None => format!("{channel} - Latest {count} messages"),
    }
}

/// Header line: local time, author, and the `ts` passed to `--thread`
fn message_header(message: &SlackMessage) -> String {
    format!(
        "[{}] {}  {}",
        format_timestamp(&message.ts),
        message.author(),
        message.ts
    )
}

/// Render messages, oldest first, as table lines that fit in `width` columns
/// (the MESSAGE column wraps)
fn message_table(messages: &[SlackMessage], width: usize) -> Vec<String> {
    let rows: Vec<(String, &str, Vec<String>)> = messages
        .iter()
        .rev()
        .map(|m| (format_timestamp(&m.ts), m.author(), message_lines(m)))
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
    let mut lines: Vec<String> = message
        .text
        .lines()
        .map(str::to_string)
        .chain(files)
        .collect();
    if let Some(count) = message.reply_count.filter(|count| *count > 0) {
        let noun = if count == 1 { "reply" } else { "replies" };
        lines.push(format!("({count} {noun})"));
    }
    lines
}

fn format_timestamp(ts: &str) -> String {
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
    fn channel_heading_keeps_a_direct_message_label() {
        assert_eq!(channel_heading("general"), "#general");
        assert_eq!(channel_heading("@alice"), "@alice");
        assert_eq!(channel_heading("@alice (D1)"), "@alice (D1)");
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
        let date = format_timestamp("1609459600.000080");
        let date2 = format_timestamp("1609459700.000090");

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
    fn history_heading_for_a_thread() {
        assert_eq!(
            history_heading("general", None, 2),
            "#general - Latest 2 messages"
        );
        assert_eq!(
            history_heading("general", Some("1609459800.000100"), 2),
            "#general - Thread 1609459800.000100 (2 messages)"
        );
        assert_eq!(
            history_heading("@alice", Some("1609459800.000100"), 1),
            "@alice - Thread 1609459800.000100 (1 messages)"
        );
    }

    #[test]
    fn json_messages_are_oldest_first() {
        let mut latest: SlackMessage = serde_json::from_str(
            r#"{"user":"U2","ts":"1609459700.000090","text":"later","thread_ts":"1609459600.000080","reply_count":2,"files":[{"name":"a.png","mimetype":"image/png"}]}"#,
        )
        .unwrap();
        latest.user_name = Some("bob".to_string());
        let earlier: SlackMessage =
            serde_json::from_str(r#"{"user":"U1","ts":"1609459600.000080","text":"earlier"}"#)
                .unwrap();
        let value = serde_json::to_value(json_messages(&[latest, earlier])).unwrap();
        assert_eq!(
            value,
            serde_json::json!([
                {
                    "ts": "1609459600.000080",
                    "thread_ts": null,
                    "user": "U1",
                    "user_name": null,
                    "text": "earlier",
                    "reply_count": null,
                    "files": []
                },
                {
                    "ts": "1609459700.000090",
                    "thread_ts": "1609459600.000080",
                    "user": "U2",
                    "user_name": "bob",
                    "text": "later",
                    "reply_count": 2,
                    "files": [{"name": "a.png", "mimetype": "image/png", "permalink": null}]
                }
            ])
        );
    }

    #[test]
    fn message_header_includes_ts() {
        let message: SlackMessage =
            serde_json::from_str(r#"{"user":"U0123456789","ts":"1609459800.000100","text":"hi"}"#)
                .unwrap();
        let header = message_header(&message);
        assert!(header.ends_with("  1609459800.000100"));
        assert!(header.contains("U0123456789"));
    }

    #[test]
    fn message_lines_with_reply_count() {
        let one: SlackMessage =
            serde_json::from_str(r#"{"ts":"1609459800.000100","text":"hi","reply_count":1}"#)
                .unwrap();
        assert_eq!(message_lines(&one), ["hi", "(1 reply)"]);
        let many: SlackMessage =
            serde_json::from_str(r#"{"ts":"1609459800.000100","text":"hi","reply_count":3}"#)
                .unwrap();
        assert_eq!(message_lines(&many), ["hi", "(3 replies)"]);
    }

    #[test]
    fn format_timestamp_valid() {
        let ts = "1609459200.000000"; // 2021-01-01 00:00:00 UTC
        let expected = Local
            .timestamp_opt(1609459200, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M")
            .to_string();
        assert_eq!(format_timestamp(ts), expected);
    }
}
