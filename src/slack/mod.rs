use std::path::Path;

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
    pub messages: Option<Vec<SlackMessage>>,
    pub upload_url: Option<String>,
    pub file_id: Option<String>,
}

pub struct SlackChannels {
    pub channels: Vec<SlackChannel>,
}

impl From<Vec<SlackChannel>> for SlackChannels {
    fn from(channels: Vec<SlackChannel>) -> Self {
        Self { channels }
    }
}

#[derive(Clone, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Deserialize)]
pub struct SlackMessage {
    pub text: String,
    pub user: Option<String>,
    pub ts: String,
}

pub struct SlackMessages {
    pub messages: Vec<SlackMessage>,
}

impl From<Vec<SlackMessage>> for SlackMessages {
    fn from(messages: Vec<SlackMessage>) -> Self {
        Self { messages }
    }
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
            bearer_token: config.token().to_string(),
        }
    }

    /*
     * Get slack channels.
     */
    pub async fn get_channels(&self) -> Result<SlackChannels> {
        let url = Url::parse(&format!("{}{}", self.base_url, "/api/conversations.list"))
            .map_err(|e| anyhow!("Invalid URL: {}", e))?;

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
            Err(anyhow!(
                "{}",
                res.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    /*
     * Post slack message.
     */
    pub async fn post_message(&self, channel: &str, text: &str) -> Result<SlackResponse> {
        let body = vec![("channel", channel), ("text", text)];
        let url = Url::parse(&format!("{}{}", self.base_url, "/api/chat.postMessage"))
            .map_err(|e| anyhow!("Invalid URL: {}", e))?;

        let client = self
            .client
            .post(url)
            .bearer_auth(&self.bearer_token)
            .form(&body);

        let res: SlackResponse = client.send().await?.json().await?;

        if res.ok {
            Ok(res)
        } else {
            Err(anyhow!(
                "{}",
                res.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }

    /*
     * Get channel messages (conversations.history).
     */
    pub async fn get_messages(&self, channel_id: &str, limit: usize) -> Result<SlackMessages> {
        let url = Url::parse(&format!(
            "{}{}?channel={}&limit={}",
            self.base_url, "/api/conversations.history", channel_id, limit
        ))
        .map_err(|e| anyhow!("Invalid URL: {}", e))?;

        let res: SlackResponse = self
            .client
            .get(url)
            .bearer_auth(&self.bearer_token)
            .send()
            .await?
            .json()
            .await?;

        if res.ok {
            match res.messages {
                Some(messages) => Ok(SlackMessages::from(messages)),
                None => Err(anyhow!("No messages found")),
            }
        } else {
            Err(anyhow!(
                "{}",
                res.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }
}

impl SlackClient {
    /*
     * Upload a file to a channel (files.getUploadURLExternal + files.completeUploadExternal).
     */
    pub async fn upload_file(
        &self,
        channel_id: &str,
        path: &Path,
        comment: Option<&str>,
    ) -> Result<SlackResponse> {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid file path: {}", path.display()))?;
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| anyhow!("{}: {}", path.display(), e))?;

        // 1. Get an upload URL
        let length = bytes.len().to_string();
        let body = vec![("filename", filename), ("length", length.as_str())];
        let url = Url::parse(&format!(
            "{}{}",
            self.base_url, "/api/files.getUploadURLExternal"
        ))
        .map_err(|e| anyhow!("Invalid URL: {}", e))?;

        let res: SlackResponse = self
            .client
            .post(url)
            .bearer_auth(&self.bearer_token)
            .form(&body)
            .send()
            .await?
            .json()
            .await?;
        let res = Self::check(res)?;
        let (upload_url, file_id) = match (res.upload_url, res.file_id) {
            (Some(upload_url), Some(file_id)) => (upload_url, file_id),
            _ => return Err(anyhow!("No upload URL returned")),
        };

        // 2. Send the file content to the upload URL
        self.client
            .post(&upload_url)
            .body(bytes)
            .send()
            .await?
            .error_for_status()?;

        // 3. Complete the upload and share it to the channel
        let files = serde_json::json!([{ "id": file_id, "title": filename }]).to_string();
        let mut body = vec![("files", files.as_str()), ("channel_id", channel_id)];
        if let Some(comment) = comment.filter(|c| !c.trim().is_empty()) {
            body.push(("initial_comment", comment));
        }
        let url = Url::parse(&format!(
            "{}{}",
            self.base_url, "/api/files.completeUploadExternal"
        ))
        .map_err(|e| anyhow!("Invalid URL: {}", e))?;

        let res: SlackResponse = self
            .client
            .post(url)
            .bearer_auth(&self.bearer_token)
            .form(&body)
            .send()
            .await?
            .json()
            .await?;

        Self::check(res)
    }

    fn check(res: SlackResponse) -> Result<SlackResponse> {
        if res.ok {
            Ok(res)
        } else {
            Err(anyhow!(
                "{}",
                res.error.unwrap_or_else(|| "Unknown error".to_string())
            ))
        }
    }
}

impl SlackChannels {
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn channel_names(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|channel| channel.name.clone())
            .collect()
    }

    pub fn max_channel_size(&self) -> usize {
        match self.channel_names().iter().max_by_key(|name| name.len()) {
            Some(name) => name.len(),
            _ => 80,
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SlackChannel> {
        self.channels.iter().find(|c| c.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn it_create_slack_client() {
        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, "https://example.com");
        assert_eq!(slack_client.bearer_token, config.token());
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

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
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
                    id: "ID001".to_string(),
                    name: "apple".to_string(),
                },
                SlackChannel {
                    id: "ID002".to_string(),
                    name: "grape".to_string(),
                },
                SlackChannel {
                    id: "ID003".to_string(),
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
                    id: "ID001".to_string(),
                    name: "apple".to_string(),
                },
                SlackChannel {
                    id: "ID002".to_string(),
                    name: "grape".to_string(),
                },
                SlackChannel {
                    id: "ID003".to_string(),
                    name: "orange".to_string(),
                },
            ],
        };
        assert_eq!(6, slack_channels.max_channel_size());
    }

    #[test]
    fn it_find_by_name() {
        let slack_channels = SlackChannels {
            channels: vec![
                SlackChannel {
                    id: "ID001".to_string(),
                    name: "apple".to_string(),
                },
                SlackChannel {
                    id: "ID002".to_string(),
                    name: "grape".to_string(),
                },
            ],
        };
        let found = slack_channels.find_by_name("apple");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "ID001");

        let not_found = slack_channels.find_by_name("banana");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn it_get_messages() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"/api/conversations\.history.*".to_string()),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/conversations_history/ok.json")
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let messages = slack_client.get_messages("C0123456789", 10).await.unwrap();
        assert_eq!(messages.messages.len(), 3);
        assert_eq!(
            messages.messages[0].text,
            "Hello, this is the latest message!"
        );
    }

    #[tokio::test]
    #[serial]
    async fn it_upload_file() {
        let mut server = mockito::Server::new_async().await;
        let get_url = server
            .mock("POST", "/api/files.getUploadURLExternal")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("filename".into(), "image.png".into()),
                mockito::Matcher::UrlEncoded("length".into(), "8".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"ok":true,"upload_url":"{}/upload/F0123456789","file_id":"F0123456789"}}"#,
                server.url()
            ))
            .create_async()
            .await;
        let upload = server
            .mock("POST", "/upload/F0123456789")
            .match_body(b"\x89PNG\r\n\x1a\n".to_vec())
            .with_status(200)
            .create_async()
            .await;
        let complete = server
            .mock("POST", "/api/files.completeUploadExternal")
            .match_body(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("channel_id".into(), "C0123456789".into()),
                mockito::Matcher::UrlEncoded("initial_comment".into(), "look".into()),
                mockito::Matcher::UrlEncoded(
                    "files".into(),
                    r#"[{"id":"F0123456789","title":"image.png"}]"#.into(),
                ),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/files_complete_upload_external/ok.json")
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        slack_client
            .upload_file(
                "C0123456789",
                Path::new("tests/fixtures/image/image.png"),
                Some("look"),
            )
            .await
            .unwrap();

        get_url.assert_async().await;
        upload.assert_async().await;
        complete.assert_async().await;
    }

    #[tokio::test]
    #[serial]
    async fn it_upload_file_with_error() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("POST", "/api/files.getUploadURLExternal")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":false,"error":"missing_scope"}"#)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let result = slack_client
            .upload_file(
                "C0123456789",
                Path::new("tests/fixtures/image/image.png"),
                None,
            )
            .await;
        assert_eq!(result.err().unwrap().to_string(), "missing_scope");
    }
}
