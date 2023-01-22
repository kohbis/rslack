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

pub struct SlackClient {
    pub client: Client,
    pub bearer_token: String,
}

impl SlackClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            bearer_token: config.token.clone(),
        }
    }

    /*
     * Get slack channels.
     */
    pub async fn get_channels(&self) -> Result<Vec<SlackChannel>> {
        let url = Url::parse("https://slack.com/api/conversations.list").unwrap();

        let res: SlackResponse = self
            .client
            .get(url)
            .bearer_auth(&self.bearer_token)
            .send()
            .await?
            .json()
            .await?;

        if res.ok {
            Ok(res.channels.unwrap())
        } else {
            Err(anyhow!("{}", res.error.unwrap()))
        }
    }

    /*
     * Post slack message.
     */
    pub async fn post_message(&self, channel: &str, text: &str) -> Result<SlackResponse> {
        let body = vec![("channel", channel), ("text", text)];
        let url = Url::parse("https://slack.com/api/chat.postMessage").unwrap();

        let client = self
            .client
            .post(url)
            .bearer_auth(&self.bearer_token)
            .form(&body);

        let res: SlackResponse = client.send().await?.json().await?;

        if res.ok {
            Ok(res)
        } else {
            Err(anyhow!("{}", res.error.unwrap()))
        }
    }
}

pub fn slack_channel_names(channels: &Vec<SlackChannel>) -> Vec<&str> {
    channels
        .iter()
        .map(|channel| channel.name.as_str())
        .collect()
}

pub fn chunk_slack_channel_names<'a>(
    chunk_size: usize,
    channel_names: &Vec<&'a str>,
) -> Vec<Vec<&'a str>> {
    channel_names
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub fn max_channel_size(channel_names: &Vec<&str>) -> usize {
    match channel_names.iter().max_by_key(|name| name.len()) {
        Some(name) => name.len(),
        _ => 80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_build_slack_channel_names() {
        let channels = vec!["apple", "grape", "orange"];
        let slack_channels: Vec<SlackChannel> = channels
            .iter()
            .map(|channel| SlackChannel {
                name: channel.to_string(),
            })
            .collect();
        assert_eq!(channels, slack_channel_names(&slack_channels));
    }

    #[test]
    fn it_chunks_slack_channel_names() {
        let channel_names = vec!["apple", "grape", "orange", "peach"];
        let expected = vec![vec!["apple", "grape", "orange"], vec!["peach"]];
        assert_eq!(expected, chunk_slack_channel_names(3, &channel_names));
    }

    #[test]
    fn it_get_max_channel_size() {
        let channels = vec!["apple", "grape", "orange"];
        assert_eq!(6, max_channel_size(&channels),)
    }
}
