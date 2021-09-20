use std::io::{stdin, BufRead};

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

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        if channel_names.contains(&channel.as_str()) {
            break;
        } else if !channel.trim().is_empty() {
            eprintln!("No channel named #{}", channel)
        }

        console::print_as_table(&channel_names, &channel);
        println!();

        console::prompt("channel # ").unwrap();
        channel = match lines.next().unwrap() {
            Ok(line) => {
                if channel_names.contains(&line.as_str()) {
                    console::print_as_table(&channel_names, &line);
                    println!();

                    line
                } else {
                    eprintln!("No channel named #{}", line);
                    continue;
                }
            }
            Err(err) => {
                eprintln!("{}", err);
                continue;
            }
        };

        break;
    }

    loop {
        if !message.trim().is_empty() {
            break;
        }

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

    match api::post_message(&config, &channel, &message).await {
        Ok(_) => {
            println!("[Success] #{} {}", channel, message)
        }
        Err(err) => {
            eprintln!("[Failed] {}", err)
        }
    }
}
