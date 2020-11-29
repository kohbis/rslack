extern crate rslack;

use structopt::StructOpt;
use std::io::{stdin, BufRead};

use rslack::api;
use rslack::config::Config;
use rslack::console;

const TOKEN_FILE: &str = ".token";

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long, default_value = "")]
    channel: String,

    #[structopt(short, long, default_value = "")]
    message: String,
}

#[tokio::main]
async fn main() {
    let opts = Opt::from_args();

    #[allow(unused_assignments)]
    let mut channel = opts.channel;
    #[allow(unused_assignments)]
    let mut message = opts.message;

    let config = match Config::new(TOKEN_FILE) {
        Ok(config) => config,
        Err(err) => {
            return eprintln!("{}", err)
        },
    };

    let channels = match api::get_channels(&config).await {
        Ok(channels) => channels,
        Err(err) => {
            return eprintln!("{}", err)
        },
    };
    let channel_names = channels.iter().map(|channel| channel.name.as_str()).collect::<Vec<&str>>();

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        if channel_names.contains(&channel.as_str()) {
            break
        } else {
            if !channel.trim().is_empty() {
                eprintln!("No channel named #{}", channel)
            }
        }

        console::print_as_table(&channel_names);
        println!();

        console::prompt("channel > ").unwrap();
        channel = match lines.next().unwrap() {
            Ok(line) => {
                if channel_names.contains(&line.as_str()) {
                    line
                } else {
                    eprintln!("No channel named #{}", line);
                    continue
                }
            },
            Err(err) => {
                eprintln!("{}", err);
                continue
            },
        };

        break
    }

    if message.is_empty() {
        loop {
            console::prompt("message > ").unwrap();
            message = match lines.next().unwrap() {
                Ok(line) => {
                    line
                },
                Err(err) => {
                    eprintln!("{}", err);
                    continue
                },
            };

            break
        }
    }

    match api::post_message(&config, &channel, &message).await {
        Ok(_) => {
            println!("\n[Success] #{} {}\n", channel, message)
        },
        Err(err) => {
            eprintln!("\n[Failed] {}\n", err)
        },
    }
}
