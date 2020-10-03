extern crate rslack;

use rslack::api;
use rslack::config::Config;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    let channels = api::get_channels(&config).await.unwrap();

    println!("{:?}", channels.iter().map(|channel| channel.name.to_string()).collect::<Vec<String>>())
}
