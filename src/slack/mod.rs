use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::config::Config;

#[derive(Deserialize)]
pub struct SlackResponse {
    ok: bool,
    pub error: Option<String>,
    pub channels: Option<Vec<SlackChannel>>,
}

pub struct SlackChannels {
    pub channels: Vec<SlackChannel>,
}

impl From<Vec<SlackChannel>> for SlackChannels {
    fn from(channels: Vec<SlackChannel>) -> Self {
        Self { channels }
    }
}

#[derive(Deserialize)]
pub struct SlackChannel {
    pub name: String,
}

pub struct SlackClient {
    pub client: Client,
    pub base_url: String,
    pub bearer_token: String,
}

impl SlackClient {
    pub fn new(config: &Config, base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_owned(),
            bearer_token: config.token.clone(),
        }
    }

    /*
     * Get slack channels.
     */
    pub async fn get_channels(&self) -> Result<SlackChannels> {
        let url = Url::parse(&format!("{}{}", self.base_url, "/api/conversations.list")).unwrap();

        let res: SlackResponse = self
            .client
            .get(url)
            .bearer_auth(&self.bearer_token)
            .send()
            .await?
            .json()
            .await?;

        if res.ok {
            match res.channels {
                Some(channels) => Ok(SlackChannels::from(channels)),
                None => {
                    return Err(anyhow!("No channels found"));
                }
            }
        } else {
            Err(anyhow!("{}", res.error.unwrap()))
        }
    }

    /*
     * Post slack message.
     */
    pub async fn post_message(&self, channel: &str, text: &str) -> Result<SlackResponse> {
        let body = vec![("channel", channel), ("text", text)];
        let url = Url::parse(&format!("{}{}", self.base_url, "/api/chat.postMessage")).unwrap();

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

impl SlackChannels {
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn channel_names(&self) -> Vec<&str> {
        self.channels
            .iter()
            .map(|channel| channel.name.as_str())
            .collect()
    }

    pub fn max_channel_size(&self) -> usize {
        match self.channel_names().iter().max_by_key(|name| name.len()) {
            Some(name) => name.len(),
            _ => 80,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn it_create_slack_client() {
        let config = Config {
            token: "token".to_string(),
        };
        let slack_client = SlackClient::new(&config, "https://example.com");
        assert_eq!(slack_client.bearer_token, config.token);
    }

    #[tokio::test]
    #[serial]
    async fn it_get_channels() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/api/conversations.list")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/conversations_list/ok.json")
            .create_async()
            .await;

        let config = Config {
            token: "token".to_string(),
        };
        let slack_client = SlackClient::new(&config, &server.url());
        let channels = slack_client.get_channels().await.unwrap();
        assert_eq!(channels.len(), 2);
    }

    #[test]
    fn it_get_slack_channnel_names() {
        let channels = vec!["apple", "grape", "orange"];
        let slack_channels = SlackChannels {
            channels: vec![
                SlackChannel {
                    name: "apple".to_string(),
                },
                SlackChannel {
                    name: "grape".to_string(),
                },
                SlackChannel {
                    name: "orange".to_string(),
                },
            ],
        };
        assert_eq!(channels, slack_channels.channel_names());
    }

    #[test]
    fn it_get_max_channel_size() {
        let slack_channels = SlackChannels {
            channels: vec![
                SlackChannel {
                    name: "apple".to_string(),
                },
                SlackChannel {
                    name: "grape".to_string(),
                },
                SlackChannel {
                    name: "orange".to_string(),
                },
            ],
        };
        assert_eq!(6, slack_channels.max_channel_size());
    }
}
