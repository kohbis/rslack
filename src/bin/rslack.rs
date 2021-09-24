use std::io::{stdin, stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use rslack::api;
use rslack::config::Config;
use rslack::console;
use rslack::option::Opt;

const TOKEN_FILE: &str = ".token";

#[tokio::main]
async fn main() {
    let opts = Opt::get_opts();

    let mut channel = opts.channel;
    let mut message = opts.message;

    let config = match Config::new(TOKEN_FILE) {
        Ok(config) => config,
        Err(err) => return eprintln!("{}", err),
    };

    let channels = match api::get_channels(&config).await {
        Ok(channels) => channels,
        Err(err) => return eprintln!("{}", err),
    };
    let channel_names: Vec<&str> = channels
        .iter()
        .map(|channel| channel.name.as_str())
        .collect();

    #[rustfmt::skip]
    let max_len = channel_names.iter().max_by_key(|name| name.len()).unwrap().len() + 1;
    let col_count = console::term_size().0 as usize / (max_len + 2);
    let chunked_datas: Vec<Vec<&str>> = channel_names
        .chunks(col_count)
        .map(|chunk| chunk.to_vec())
        .collect();

    let stdout = stdout().into_raw_mode().unwrap();
    // Switch screen from Main to Alternate
    let mut stdout = AlternateScreen::from(stdout);

    if channel.trim().is_empty() || !channel_names.contains(&channel.as_str()) {
        let mut current: (usize, usize) = (0, 0);
        channel = chunked_datas[current.0][current.1].to_string();
        console::print_as_table(&mut stdout, &chunked_datas, max_len, &channel);

        let stdin = stdin();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('q') | Key::Ctrl('c') => return,
                Key::Char('\n') => break,
                Key::Left | Key::Char('h') => {
                    if 0 < current.1 {
                        current.1 -= 1;
                    }
                }
                Key::Right | Key::Char('l') => {
                    if current.1 < chunked_datas[current.0].len() - 1 {
                        current.1 += 1;
                    }
                }
                Key::Up | Key::Char('k') => {
                    if 0 < current.0 {
                        current.0 -= 1;
                    }
                }
                Key::Down | Key::Char('j') => {
                    if current.0 < chunked_datas.len() - 1
                        && current.1 <= chunked_datas[current.0 + 1].len() - 1
                    {
                        current.0 += 1;
                    }
                }
                _ => {}
            }
            channel = chunked_datas[current.0][current.1].to_string();
            console::print_as_table(&mut stdout, &chunked_datas, max_len, &channel);
        }
    }

    console::print_as_table(&mut stdout, &chunked_datas, max_len, &channel);

    if message.trim().is_empty() {
        let mut buffer: Vec<char> = Vec::new();

        #[rustfmt::skip]
        write!(stdout, "{}{}#{}", termion::cursor::Goto(1, 1), termion::clear::All, &channel).unwrap();
        #[rustfmt::skip]
        write!(stdout, "{}{}", termion::cursor::Goto(1, 2), "(post: ctrl-p / exit: ctrl-c)").unwrap();
        write!(stdout, "{}", termion::cursor::Goto(1, 4)).unwrap();
        stdout.flush().unwrap();

        let stdin = stdin();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Ctrl('c') => return,
                Key::Ctrl('p') => {
                    if message.trim().is_empty() {
                        buffer.clear();
                        #[rustfmt::skip]
                        write!(stdout, "{}{}", termion::cursor::Goto(1, 4), termion::clear::CurrentLine).unwrap();
                        stdout.flush().unwrap();
                        continue;
                    } else {
                        break;
                    }
                }
                Key::Char('\n') => {
                    buffer.push('\r');
                    buffer.push('\n');
                }
                Key::Char(c) => {
                    buffer.push(c);
                }
                Key::Backspace => {
                    if buffer.len() > 0 {
                        buffer.remove(buffer.len() - 1);
                        #[rustfmt::skip]
                        write!(stdout, "{}{}", termion::cursor::Left(1), termion::clear::AfterCursor).unwrap();
                    }
                }
                _ => {}
            }

            message = buffer.iter().collect();
            #[rustfmt::skip]
            write!(stdout, "{}{}", termion::cursor::Goto(1, 4), termion::clear::CurrentLine).unwrap();
            write!(stdout, "{}", &message).unwrap();
            stdout.flush().unwrap();
        }
    }

    // Switch screen from Alternate to Main
    drop(stdout);
    match api::post_message(&config, &channel, &message).await {
        Ok(_) => {
            println!("[Success] #{}\n {}", channel, message)
        }
        Err(err) => {
            eprintln!("[Failed] {}", err)
        }
    }
}
