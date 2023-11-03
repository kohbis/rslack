use rpos::table::Table;
use std::io::{stdin, stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::Config;
use rslack::console;
use rslack::option::Opt;
use rslack::slack;

const SLACK_URL: &'static str = "https://slack.com";
const USAGE_MESSAGES: &'static str = "(post: ctrl-p / exit: ctrl-c)";

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
    let channel_names: Vec<&str> = slack::slack_channel_names(&channels);

    // Build data for display table
    let max_col_size = slack::max_channel_size(&channel_names) + 1;
    let col_count = console::term_size().0 as usize / (max_col_size + 2);
    let chunked_data = slack::chunk_slack_channel_names(col_count, &channel_names);

    let stdout = stdout().into_raw_mode().unwrap();
    // Switch screen from Main to Alternate
    let mut stdout = stdout.into_alternate_screen().unwrap();

    if channel.trim().is_empty() || !channel_names.contains(&channel.as_str()) {
        let mut cursor = Table::new(chunked_data.len(), chunked_data[0].len())
            .unwrap()
            .cursor;
        channel = chunked_data[cursor.current().0][cursor.current().1].to_string();
        console::print_as_table(&mut stdout, &chunked_data, max_col_size, &channel);

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
            console::print_as_table(&mut stdout, &chunked_data, max_col_size, &channel);
        }
    }

    console::print_as_table(&mut stdout, &chunked_data, max_col_size, &channel);

    if message.trim().is_empty() {
        let mut buffer: Vec<Vec<char>> = vec![vec![]];
        let mut cursor_line: usize = 0;

        write!(
            stdout,
            "{}{}#{}{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All,
            &channel,
            termion::cursor::Goto(1, 2),
            USAGE_MESSAGES,
            termion::cursor::Goto(1, 3)
        )
        .unwrap();
        stdout.flush().unwrap();

        let stdin = stdin();
        for c in stdin.keys() {
            match c.unwrap() {
                Key::Ctrl('c') => return,
                Key::Ctrl('p') => {
                    if message.trim().is_empty() {
                        buffer.clear();

                        write!(
                            stdout,
                            "{}{}",
                            termion::cursor::Goto(1, 4),
                            termion::clear::CurrentLine
                        )
                        .unwrap();
                        stdout.flush().unwrap();

                        continue;
                    } else {
                        break;
                    }
                }
                Key::Char('\n') => {
                    buffer[cursor_line].extend_from_slice(&['\r', '\n']);

                    // Add new line
                    buffer.push(vec![]);
                    cursor_line += 1;
                }
                Key::Char(c) => {
                    buffer[cursor_line].push(c);
                }
                Key::Backspace => {
                    if buffer[cursor_line].len() > 0 {
                        let remove_target = &buffer[cursor_line].len() - 1;
                        buffer[cursor_line].remove(remove_target);
                        write!(
                            stdout,
                            "{}{}",
                            termion::cursor::Left(1),
                            termion::clear::AfterCursor
                        )
                        .unwrap();
                    } else {
                        if buffer.len() > 1 {
                            // Remove current line
                            buffer.remove(cursor_line);
                            cursor_line -= 1;

                            // Remove ['\r', '\n']
                            let line_len = &buffer[cursor_line].len() - 2;
                            buffer[cursor_line].truncate(line_len);
                        }
                    }
                }
                _ => {}
            }

            message = buffer
                .iter()
                .map(|v| v.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("");
            write!(
                stdout,
                "{}{}{}",
                termion::cursor::Goto(1, 3),
                termion::clear::CurrentLine,
                &message
            )
            .unwrap();
            stdout.flush().unwrap();
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
