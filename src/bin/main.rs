extern crate rslack;

use std::io::{stdin, BufRead};

use rslack::api;
use rslack::config::Config;
use rslack::terminal;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    let channels = api::get_channels(&config).await.unwrap();

    println!("{:?}", channels.iter().map(|channel| channel.name.to_string()).collect::<Vec<String>>());

    let stdin = stdin();
    let mut lines = stdin.lock().lines();

    loop {
        terminal::prompt("channel > ").unwrap();
        let channel = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => {
                eprintln!("{}", e);
                continue
            },
            None => break,
        };

        terminal::prompt("message > ").unwrap();
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
