extern crate rslack;

use std::io::{stdin, BufRead};

use rslack::api;
use rslack::config::Config;
use rslack::console;

#[tokio::main]
async fn main() {
    let config = match Config::new() {
        Ok(config) => config,
        Err(e) => {
            return eprintln!("{}", e);
        },
    };

    let channels = api::get_channels(&config).await.unwrap();
    let channel_names = channels.iter().map(|channel| channel.name.as_str()).collect::<Vec<&str>>();

    console::print_as_table(&channel_names);

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        console::prompt("channel > ").unwrap();
        let channel = match lines.next().unwrap() {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}", e);
                continue
            },
        };

        console::prompt("message > ").unwrap();
        let message = match lines.next().unwrap() {
            Ok(line) => line,
            Err(e) => {
                eprintln!("{}", e);
                continue
            },
        };

        match api::post_message(&config, &channel, &message).await {
            Ok(_) => {
                break println!("\n[Success] #{} {}\n", channel, message)
            },
            Err(e) => {
                break eprintln!("{}", e)
            },
        }
    }
}
