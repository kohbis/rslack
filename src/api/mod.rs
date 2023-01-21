use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::config::Config;

#[derive(Deserialize, Debug)]
pub struct SlackResponse {
    ok: bool,
    pub error: Option<String>,
    pub channels: Option<Vec<SlackChannel>>,
}

#[derive(Deserialize, Debug)]
pub struct SlackChannel {
    pub name: String,
}

/*
 * Get slack channels.
 */
pub async fn get_channels(config: &Config) -> Result<Vec<SlackChannel>> {
    let url = Url::parse("https://slack.com/api/conversations.list").unwrap();

    let client = Client::new().get(url).bearer_auth(&config.token);

    let res: SlackResponse = client.send().await?.json().await?;

    if res.ok {
        Ok(res.channels.unwrap())
    } else {
        Err(anyhow!("{}", res.error.unwrap()))
    }
}

/*
 * Post slack message.
 */
pub async fn post_message(config: &Config, channel: &str, text: &str) -> Result<SlackResponse> {
    let body = vec![("channel", channel), ("text", text)];
    let url = Url::parse("https://slack.com/api/chat.postMessage").unwrap();

    let client = Client::new()
        .post(url)
        .bearer_auth(&config.token)
        .form(&body);

    let res: SlackResponse = client.send().await?.json().await?;

    if res.ok {
        Ok(res)
    } else {
        Err(anyhow!("{}", res.error.unwrap()))
    }
}
