extern crate rslack;

use std::io::{stdin, BufRead};

use rslack::api;
use rslack::config::Config;
use rslack::console;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    let channels = api::get_channels(&config).await.unwrap();
    let channel_names = channels.iter().map(|channel| channel.name.as_str()).collect::<Vec<&str>>();

    console::print_as_table(&channel_names);

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        console::prompt("channel > ").unwrap();
        let channel = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => {
                eprintln!("{}", e);
                continue
            },
            None => break,
        };

        console::prompt("message > ").unwrap();
        let message = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => {
                eprintln!("{}", e);
                continue
            },
            None => break,
        };

        match api::post_message(&config, &channel, &message).await {
            Ok(_) => {
                println!("#{} {}", channel, message);
                break
            },
            Err(e) => {
                eprintln!("{}", e);
                break
            }
        }
    }
}
