use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use url::Url;

use super::config::Config;

#[derive(Deserialize, Debug)]
pub struct SlackResponce {
    pub ok: bool,
    pub channels: Vec<SlackChannel>,
}

#[derive(Deserialize, Debug)]
pub struct SlackChannel {
    pub name: String,
}

pub async fn get_channels(config: &Config) -> Result<Vec<SlackChannel>> {
    let params = vec![("token", &config.token)];
    let url = Url::parse_with_params("https://slack.com/api/conversations.list", params).unwrap();

    let client = Client::new().get(url);

    let res: SlackResponce = client.send().await?.json().await?;

    Ok(res.channels)
}
