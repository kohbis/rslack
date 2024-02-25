use std::io::{stdin, stdout, Write};

use rpos::table::Table as RposTable;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::Config;
use rslack::console::{Editor, Table};
use rslack::option::Opt;
use rslack::slack;

const SLACK_URL: &str = "https://slack.com";

#[tokio::main]
async fn main() {
    let opts = Opt::get_opts();
    let mut channel = opts.channel;
    let mut message = opts.message;

    let config = match Config::new(None) {
        Ok(config) => config,
        Err(err) => return eprintln!("{}", err),
    };

    let slack_client = slack::SlackClient::new(&config, SLACK_URL);

    // Get slack channels
    let channels = match slack_client.get_channels().await {
        Ok(channels) => channels,
        Err(err) => return eprintln!("{}", err),
    };

    // Build data for display table
    let channel_names = channels.channel_names();
    let max_col_size = channels.max_channel_size() + 1;
    let table = Table::new("CHANNELS".to_string(), channel_names.clone(), max_col_size);

    let stdout = stdout().into_raw_mode().unwrap();
    // Switch screen from Main to Alternate
    let mut stdout = stdout.into_alternate_screen().unwrap();

    if channel.trim().is_empty() || !&channel_names.contains(&channel) {
        let chunked_data = table.chunked_data();
        let mut cursor = RposTable::new(chunked_data.len(), chunked_data[0].len())
            .unwrap()
            .cursor;
        channel = chunked_data[cursor.current().0][cursor.current().1].to_string();
        table.draw(&mut stdout, &channel);

        let stdin = stdin();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('q') | Key::Ctrl('c') => return,
                Key::Char('\n') => break,
                Key::Left | Key::Char('h') => {
                    if 0 < cursor.current().1 {
                        cursor.left();
                    } else if 0 < cursor.current().0 {
                        cursor.up();
                        cursor
                            .set_column(chunked_data[cursor.current().0].len() - 1)
                            .unwrap();
                    } else if cursor.current() == (0, 0) {
                        cursor.set_line(chunked_data.len() - 1).unwrap();
                        cursor
                            .set_column(chunked_data[cursor.current().0].len() - 1)
                            .unwrap();
                    }
                }
                Key::Right | Key::Char('l') => {
                    if cursor.current().1 < chunked_data[cursor.current().0].len() - 1 {
                        cursor.right();
                    } else if cursor.current().0 < chunked_data.len() - 1 {
                        cursor.down();
                        cursor.set_column(0).unwrap();
                    } else if cursor.current().0 == chunked_data.len() - 1 {
                        cursor.set(0, 0).unwrap();
                    }
                }
                Key::Up | Key::Char('k') => {
                    if 0 < cursor.current().0 {
                        cursor.up();
                    }
                }
                Key::Down | Key::Char('j') => {
                    if cursor.current().0 < chunked_data.len() - 1
                        && cursor.current().1 <= chunked_data[cursor.current().0 + 1].len() - 1
                    {
                        cursor.down();
                    }
                }
                _ => {}
            }

            channel = chunked_data[cursor.current().0][cursor.current().1].to_string();
            table.draw(&mut stdout, &channel);
        }
    }
    table.draw(&mut stdout, &channel);

    if message.trim().is_empty() {
        let mut editor = Editor::new();
        editor.draw_header(&mut stdout, &channel);

        let stdin = stdin();
        for c in stdin.keys() {
            match c.unwrap() {
                Key::Ctrl('c') => return,
                Key::Ctrl('p') => {
                    if message.trim().is_empty() {
                        editor.clear(&mut stdout);
                        continue;
                    } else {
                        break;
                    }
                }
                Key::Char('\n') => {
                    // Add new line
                    editor.buffer.push(String::new());
                    editor.cursor_line += 1;
                }
                Key::Char(c) => {
                    editor.buffer[editor.cursor_line].push(c);
                }
                Key::Up => {
                    if editor.cursor_line > 0 {
                        editor.cursor_line -= 1;
                    }
                }
                Key::Down => {
                    if editor.cursor_line < editor.buffer.len() - 1 {
                        editor.cursor_line += 1;
                    }
                }
                Key::Backspace => {
                    if editor.buffer[editor.cursor_line].len() > 0 {
                        editor.buffer[editor.cursor_line].pop();
                        write!(
                            stdout,
                            "{}{}",
                            termion::cursor::Left(1),
                            termion::clear::AfterCursor
                        )
                        .unwrap();
                    } else {
                        if editor.buffer.len() > 1 {
                            // Remove current line
                            editor.buffer.remove(editor.cursor_line);
                            editor.cursor_line -= 1;
                        }
                    }
                }
                _ => {}
            }

            editor.draw_message(&mut stdout);
            message = editor.message();
        }
    }

    // Switch screen from Alternate to Main
    drop(stdout);

    // Post slack message
    match slack_client.post_message(&channel, &message).await {
        Ok(_) => {
            println!("[Success] #{}\n {}", channel, message)
        }
        Err(err) => {
            eprintln!("[Failed] {}", err)
        }
    }
}
