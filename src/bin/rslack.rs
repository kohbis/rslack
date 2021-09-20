use std::io::{stdin, stdout, BufRead};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use rslack::api;
use rslack::config::Config;
use rslack::console;
use rslack::option::Opt;

const TOKEN_FILE: &str = ".token";

#[tokio::main]
async fn main() {
    let opts = Opt::get_opts();

    #[allow(unused_assignments)]
    let mut channel = opts.channel;
    #[allow(unused_assignments)]
    let mut message = opts.message;

    let config = match Config::new(TOKEN_FILE) {
        Ok(config) => config,
        Err(err) => return eprintln!("{}", err),
    };

    let channels = match api::get_channels(&config).await {
        Ok(channels) => channels,
        Err(err) => return eprintln!("{}", err),
    };
    let channel_names = channels
        .iter()
        .map(|channel| channel.name.as_str())
        .collect::<Vec<&str>>();

    #[rustfmt::skip]
    let max_len = channel_names.iter().max_by_key(|name| name.len()).unwrap().len() + 1;
    let col_count = console::term_size().0 as usize / (max_len + 2);
    let chunked_datas: Vec<Vec<&str>> = channel_names
        .chunks(col_count)
        .map(|chunk| chunk.to_vec())
        .collect();

    if channel.trim().is_empty() || !channel_names.contains(&channel.as_str()) {
        let mut current: (usize, usize) = (0, 0);
        channel = chunked_datas[current.0][current.1].to_string();
        console::print_as_table(&chunked_datas, max_len, &channel);

        let stdin = stdin();
        let mut _stdout = stdout().into_raw_mode().unwrap();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('q') | Key::Ctrl('c') => return,
                Key::Char('\n') => break,
                Key::Left | Key::Char('h') => {
                    if 0 < current.1 {
                        current.1 -= 1;
                    }
                }
                Key::Right | Key::Char('j') => {
                    if current.1 < chunked_datas[current.0].len() - 1 {
                        current.1 += 1;
                    }
                }
                Key::Up | Key::Char('k') => {
                    if 0 < current.0 {
                        current.0 -= 1;
                    }
                }
                Key::Down | Key::Char('l') => {
                    if current.0 < chunked_datas.len() - 1
                        && current.1 <= chunked_datas[current.0 + 1].len() - 1
                    {
                        current.0 += 1;
                    }
                }
                _ => {}
            }
            channel = chunked_datas[current.0][current.1].to_string();
            console::print_as_table(&chunked_datas, max_len, &channel);
        }
    }

    console::print_as_table(&chunked_datas, max_len, &channel);

    if message.trim().is_empty() {
        let stdin = stdin();
        let mut lines = stdin.lock().lines();

        loop {
            console::prompt("message > ").unwrap();
            message = match lines.next().unwrap() {
                Ok(line) => {
                    if line.trim().is_empty() {
                        eprintln!("Message is empty");
                        continue;
                    }

                    line
                }
                Err(err) => {
                    eprintln!("{}", err);
                    continue;
                }
            };

            break;
        }
    }

    match api::post_message(&config, &channel, &message).await {
        Ok(_) => {
            println!("[Success] #{} {}", channel, message)
        }
        Err(err) => {
            eprintln!("[Failed] {}", err)
        }
    }
}
