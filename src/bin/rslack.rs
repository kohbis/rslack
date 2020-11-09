extern crate rslack;

use std::io::{stdin, BufRead};

use rslack::api;
use rslack::config::Config;
use rslack::console;

const TOKEN_FILE: &str = ".token";

#[tokio::main]
async fn main() {
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

    console::print_as_table(&channel_names);
    println!();

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        console::prompt("channel > ").unwrap();
        let channel = match lines.next().unwrap() {
            Ok(line) => line,
            Err(err) => {
                eprintln!("{}", err);
                continue
            },
        };

        console::prompt("message > ").unwrap();
        let message = match lines.next().unwrap() {
            Ok(line) => line,
            Err(err) => {
                eprintln!("{}", err);
                continue
            },
        };

        match api::post_message(&config, &channel, &message).await {
            Ok(_) => {
                break println!("\n[Success] #{} {}\n", channel, message)
            },
            Err(err) => {
                break eprintln!("\n{}\n", err)
            },
        }
    }
}
