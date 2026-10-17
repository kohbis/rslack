use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use reqwest::{Client, RequestBuilder};
use serde::Deserialize;
use unicode_width::UnicodeWidthStr;
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
    pub user: Option<SlackUser>,
    pub response_metadata: Option<ResponseMetadata>,
}

/// Pagination info of list methods such as conversations.list
#[derive(Deserialize)]
pub struct ResponseMetadata {
    pub next_cursor: Option<String>,
}

/// Channels fetched per conversations.list call (Slack recommends at most 200)
const CHANNELS_PER_PAGE: &str = "200";

/// Public channels, private channels, group DMs, and direct messages
const CONVERSATION_TYPES: &str = "public_channel,private_channel,mpim,im";

pub struct SlackChannels {
    pub channels: Vec<SlackChannel>,
    /// Conversation types left out because the token lacks the scope
    pub unavailable: Vec<&'static str>,
}

impl From<Vec<SlackChannel>> for SlackChannels {
    fn from(channels: Vec<SlackChannel>) -> Self {
        Self {
            channels,
            unavailable: vec![],
        }
    }
}

#[derive(Clone, Default, Deserialize)]
pub struct SlackChannel {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_im: bool,
    #[serde(default)]
    pub is_mpim: bool,
    /// The other user, set on a direct message
    #[serde(default)]
    pub user: Option<String>,
    /// `@name` filled in by `SlackClient::resolve_direct_message_names`
    #[serde(skip)]
    pub display_name: Option<String>,
}

impl SlackChannel {
    /// Name shown in the selector and accepted by `-c`
    pub fn label(&self) -> String {
        if let Some(name) = self.display_name.as_deref().filter(|name| !name.is_empty()) {
            return name.to_string();
        }
        if self.is_im {
            return format!("@{}", self.user.as_deref().unwrap_or(self.id.as_str()));
        }
        if self.is_mpim {
            return mpim_label(&self.name);
        }
        if self.name.is_empty() {
            self.id.clone()
        } else {
            self.name.clone()
        }
    }
}

/// `mpdm-alice--bob--carol-1` -> `@alice, @bob, @carol`
fn mpim_label(name: &str) -> String {
    let Some(rest) = name.strip_prefix("mpdm-") else {
        return if name.is_empty() {
            "group dm".to_string()
        } else {
            name.to_string()
        };
    };
    let names = match rest.rsplit_once('-') {
        Some((names, suffix))
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) =>
        {
            names
        }
        _ => rest,
    };
    let people: Vec<String> = names
        .split("--")
        .filter(|part| !part.is_empty())
        .map(|part| format!("@{part}"))
        .collect();
    if people.is_empty() {
        name.to_string()
    } else {
        people.join(", ")
    }
}

#[derive(Clone, Deserialize)]
pub struct SlackMessage {
    #[serde(default)]
    pub text: String,
    pub user: Option<String>,
    pub ts: String,
    /// Attached files (images, etc.)
    #[serde(default)]
    pub files: Vec<SlackFile>,
    /// Display name of `user`, filled in by `SlackClient::resolve_user_names`
    #[serde(skip)]
    pub user_name: Option<String>,
}

impl SlackMessage {
    /// Name to show as the author: the resolved name, falling back to the user ID
    pub fn author(&self) -> &str {
        self.user_name
            .as_deref()
            .or(self.user.as_deref())
            .unwrap_or("unknown")
    }
}

#[derive(Clone, Deserialize)]
pub struct SlackFile {
    pub name: Option<String>,
    pub mimetype: Option<String>,
    pub permalink: Option<String>,
}

impl SlackFile {
    pub fn is_image(&self) -> bool {
        self.mimetype
            .as_deref()
            .is_some_and(|mimetype| mimetype.starts_with("image/"))
    }
}

#[derive(Deserialize)]
pub struct SlackUser {
    pub name: String,
    pub real_name: Option<String>,
    pub profile: Option<SlackUserProfile>,
}

#[derive(Deserialize)]
pub struct SlackUserProfile {
    pub display_name: Option<String>,
}

impl SlackUser {
    /// Name as shown in the Slack UI: display name, then real name, then username
    pub fn display_name(&self) -> &str {
        [
            self.profile
                .as_ref()
                .and_then(|p| p.display_name.as_deref()),
            self.real_name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .find(|name| !name.is_empty())
        .unwrap_or(&self.name)
    }
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

/// Error returned by the Slack API in `"error"` of an `"ok": false` response,
/// e.g. `missing_scope` or `user_not_found`
#[derive(Debug)]
pub struct SlackApiError(pub String);

impl std::fmt::Display for SlackApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SlackApiError {}

impl SlackApiError {
    /// Whether `err` is a Slack API error with the given code
    pub fn is(err: &anyhow::Error, code: &str) -> bool {
        err.downcast_ref::<Self>().is_some_and(|e| e.0 == code)
    }
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
     * Get unarchived conversations the token can see: public channels, private
     * channels, group DMs, and direct messages.
     * A token missing one of those scopes gets `missing_scope` for the combined
     * request, so fall back to asking for each type on its own.
     */
    pub async fn get_channels(&self) -> Result<SlackChannels> {
        match self.list_channels(CONVERSATION_TYPES).await {
            Ok(channels) => {
                if channels.is_empty() {
                    return Err(anyhow!("No channels found"));
                }
                Ok(SlackChannels {
                    channels,
                    unavailable: vec![],
                })
            }
            Err(err) if SlackApiError::is(&err, "missing_scope") => {
                self.list_channels_by_type().await
            }
            Err(err) => Err(err),
        }
    }

    async fn list_channels_by_type(&self) -> Result<SlackChannels> {
        const TYPES: &[&str] = &["public_channel", "private_channel", "mpim", "im"];
        let mut channels = vec![];
        let mut unavailable = vec![];
        for conversation_type in TYPES {
            match self.list_channels(conversation_type).await {
                Ok(mut found) => channels.append(&mut found),
                Err(err) if SlackApiError::is(&err, "missing_scope") => {
                    unavailable.push(*conversation_type);
                }
                Err(err) => return Err(err),
            }
        }
        if channels.is_empty() {
            if unavailable.len() == TYPES.len() {
                return Err(SlackApiError("missing_scope".into()).into());
            }
            return Err(anyhow!("No channels found"));
        }
        Ok(SlackChannels {
            channels,
            unavailable,
        })
    }

    async fn list_channels(&self, types: &str) -> Result<Vec<SlackChannel>> {
        let mut channels = vec![];
        let mut cursor: Option<String> = None;
        loop {
            let mut query = vec![
                ("exclude_archived", "true"),
                ("limit", CHANNELS_PER_PAGE),
                ("types", types),
            ];
            if let Some(cursor) = &cursor {
                query.push(("cursor", cursor));
            }
            let res = self.get("conversations.list", &query).await?;
            channels.extend(res.channels.unwrap_or_default());

            cursor = res
                .response_metadata
                .and_then(|metadata| metadata.next_cursor)
                .filter(|next_cursor| !next_cursor.is_empty());
            if cursor.is_none() {
                break;
            }
        }
        Ok(channels)
    }

    /*
     * Fill in `@name` for direct messages, looking up each user only once.
     */
    pub async fn resolve_direct_message_names(&self, channels: &mut [SlackChannel]) -> Result<()> {
        let mut names: HashMap<String, String> = HashMap::new();
        for channel in channels.iter().filter(|channel| channel.is_im) {
            let Some(user_id) = &channel.user else {
                continue;
            };
            if names.contains_key(user_id) {
                continue;
            }
            match self.get_user_name(user_id).await {
                Ok(name) => {
                    names.insert(user_id.clone(), name);
                }
                Err(err) if SlackApiError::is(&err, "user_not_found") => {}
                Err(err) => return Err(err),
            }
        }
        for channel in channels.iter_mut().filter(|channel| channel.is_im) {
            if let Some(user_id) = &channel.user {
                if let Some(name) = names.get(user_id) {
                    channel.display_name = Some(format!("@{name}"));
                }
            }
        }
        Ok(())
    }

    /*
     * Post slack message.
     */
    pub async fn post_message(&self, channel: &str, text: &str) -> Result<SlackResponse> {
        self.post("chat.postMessage", &[("channel", channel), ("text", text)])
            .await
    }

    /*
     * Get channel messages (conversations.history).
     */
    pub async fn get_messages(&self, channel_id: &str, limit: usize) -> Result<SlackMessages> {
        let limit = limit.to_string();
        let res = self
            .get(
                "conversations.history",
                &[("channel", channel_id), ("limit", &limit)],
            )
            .await?;
        match res.messages {
            Some(messages) => Ok(SlackMessages::from(messages)),
            None => Err(anyhow!("No messages found")),
        }
    }

    /*
     * Get a user's display name (users.info).
     */
    pub async fn get_user_name(&self, user_id: &str) -> Result<String> {
        let res = self.get("users.info", &[("user", user_id)]).await?;
        match res.user {
            Some(user) => Ok(user.display_name().to_string()),
            None => Err(anyhow!("No user found")),
        }
    }

    /*
     * Fill in `user_name` of each message and replace user mentions in its text
     * with names, looking up each user only once.
     */
    pub async fn resolve_user_names(&self, messages: &mut [SlackMessage]) -> Result<()> {
        let mut user_ids: Vec<String> = vec![];
        for message in messages.iter() {
            user_ids.extend(message.user.clone());
            replace_mentions(&message.text, |id| {
                user_ids.push(id.to_string());
                None
            });
        }

        let mut names: HashMap<String, String> = HashMap::new();
        for user_id in user_ids {
            if names.contains_key(&user_id) {
                continue;
            }
            match self.get_user_name(&user_id).await {
                Ok(name) => {
                    names.insert(user_id, name);
                }
                // e.g. users of another workspace via Slack Connect: keep showing the ID
                Err(err) if SlackApiError::is(&err, "user_not_found") => {}
                Err(err) => return Err(err),
            }
        }

        for message in messages.iter_mut() {
            message.user_name = message
                .user
                .as_ref()
                .and_then(|user_id| names.get(user_id).cloned());
            message.text = replace_mentions(&message.text, |id| names.get(id).cloned());
        }
        Ok(())
    }

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
        let res = self
            .post(
                "files.getUploadURLExternal",
                &[("filename", filename), ("length", &length)],
            )
            .await?;
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
        self.post("files.completeUploadExternal", &body).await
    }

    /// Call a Web API method with GET and the given query parameters
    async fn get(&self, method: &str, query: &[(&str, &str)]) -> Result<SlackResponse> {
        let mut url = self.method_url(method)?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        self.send(self.client.get(url)).await
    }

    /// Call a Web API method with POST and the given form parameters
    async fn post(&self, method: &str, form: &[(&str, &str)]) -> Result<SlackResponse> {
        let request = self.client.post(self.method_url(method)?).form(form);
        self.send(request).await
    }

    fn method_url(&self, method: &str) -> Result<Url> {
        Url::parse(&format!("{}/api/{}", self.base_url, method))
            .map_err(|e| anyhow!("Invalid URL: {}", e))
    }

    /// Send an authorized request, turning an `"ok": false` response into `SlackApiError`
    async fn send(&self, request: RequestBuilder) -> Result<SlackResponse> {
        let res: SlackResponse = request
            .bearer_auth(&self.bearer_token)
            .send()
            .await?
            .json()
            .await?;
        if res.ok {
            Ok(res)
        } else {
            let code = res.error.unwrap_or_else(|| "Unknown error".to_string());
            Err(SlackApiError(code).into())
        }
    }
}

/// Replace user mentions (`<@U123>` or `<@U123|label>`) with `@name`, where the name
/// comes from `resolve`, then the label, then the user ID.
fn replace_mentions(text: &str, mut resolve: impl FnMut(&str) -> Option<String>) -> String {
    let mut replaced = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<@") {
        let Some(len) = rest[start..].find('>') else {
            break;
        };
        let mention = &rest[start + 2..start + len];
        let (user_id, label) = mention.split_once('|').unwrap_or((mention, ""));
        let name = resolve(user_id)
            .or_else(|| (!label.is_empty()).then(|| label.to_string()))
            .unwrap_or_else(|| user_id.to_string());

        replaced.push_str(&rest[..start]);
        replaced.push('@');
        replaced.push_str(&name);
        rest = &rest[start + len + 1..];
    }
    replaced.push_str(rest);
    replaced
}

impl SlackChannels {
    pub fn channel_names(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|channel| channel.label())
            .collect()
    }

    /// Display width of the longest channel name
    pub fn max_channel_size(&self) -> usize {
        self.channels
            .iter()
            .map(|channel| channel.label().width())
            .max()
            .unwrap_or(0)
    }

    /// Match a channel name, a direct-message label (`@alice`), a channel id, or the other user's id.
    /// Two conversations that both match is ambiguous, so this returns none.
    pub fn find_by_name(&self, name: &str) -> Option<&SlackChannel> {
        let name = name.trim();
        let mut found = None;
        for channel in &self.channels {
            let matches = channel.label() == name
                || channel.name == name
                || channel.id == name
                || channel.user.as_deref() == Some(name);
            if !matches {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(channel);
        }
        found
    }

    /// When two conversations share a label, append the channel id so a choice cannot hit the wrong one
    pub fn disambiguate_labels(&mut self) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for channel in &self.channels {
            *counts.entry(channel.label()).or_default() += 1;
        }
        for channel in &mut self.channels {
            let label = channel.label();
            if counts.get(&label).copied().unwrap_or(0) > 1 {
                let id = channel.id.clone();
                channel.display_name = Some(format!("{label} ({id})"));
            }
        }
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
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("exclude_archived".into(), "true".into()),
                mockito::Matcher::UrlEncoded("limit".into(), "200".into()),
                mockito::Matcher::UrlEncoded(
                    "types".into(),
                    "public_channel,private_channel,mpim,im".into(),
                ),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/conversations_list/ok.json")
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let channels = slack_client.get_channels().await.unwrap();
        assert_eq!(channels.channels.len(), 2);
    }

    #[tokio::test]
    #[serial]
    async fn it_get_channels_with_pagination() {
        let mut server = mockito::Server::new_async().await;
        let first = server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::Regex(
                "^exclude_archived=true&limit=200&types=public_channel%2Cprivate_channel%2Cmpim%2Cim$"
                    .into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"ok":true,"channels":[{"id":"C1","name":"apple"}],
                    "response_metadata":{"next_cursor":"page2"}}"#,
            )
            .expect(1)
            .create_async()
            .await;
        let second = server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::UrlEncoded(
                "cursor".into(),
                "page2".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"ok":true,"channels":[{"id":"C2","name":"grape"}],
                    "response_metadata":{"next_cursor":""}}"#,
            )
            .expect(1)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let channels = slack_client.get_channels().await.unwrap();
        assert_eq!(channels.channel_names(), ["apple", "grape"]);
        first.assert_async().await;
        second.assert_async().await;
    }

    #[tokio::test]
    #[serial]
    async fn it_get_channels_with_no_channels() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true,"channels":[]}"#)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let err = slack_client.get_channels().await.err().unwrap();
        assert_eq!(err.to_string(), "No channels found");
    }

    #[test]
    fn it_get_slack_channnel_names() {
        let channels = vec!["apple", "grape", "orange"];
        let slack_channels = SlackChannels {
            unavailable: vec![],
            channels: vec![
                channel("ID001", "apple"),
                channel("ID002", "grape"),
                channel("ID003", "orange"),
            ],
        };
        assert_eq!(channels, slack_channels.channel_names());
    }

    #[test]
    fn it_get_max_channel_size() {
        let slack_channels = SlackChannels {
            unavailable: vec![],
            channels: vec![
                channel("ID001", "apple"),
                channel("ID002", "grape"),
                channel("ID003", "orange"),
            ],
        };
        assert_eq!(6, slack_channels.max_channel_size());

        let wide = SlackChannels::from(vec![channel("ID004", "雑談")]);
        assert_eq!(4, wide.max_channel_size());
    }

    #[test]
    fn it_find_by_name() {
        let slack_channels = SlackChannels {
            unavailable: vec![],
            channels: vec![channel("ID001", "apple"), channel("ID002", "grape")],
        };
        let found = slack_channels.find_by_name("apple");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "ID001");

        let not_found = slack_channels.find_by_name("banana");
        assert!(not_found.is_none());
    }

    #[test]
    fn it_channel_label() {
        assert_eq!(channel("C1", "secret").label(), "secret");
        let mut dm = channel("D1", "");
        dm.is_im = true;
        dm.user = Some("U1".to_string());
        assert_eq!(dm.label(), "@U1");
        dm.display_name = Some("@alice".to_string());
        assert_eq!(dm.label(), "@alice");
        let mut group = channel("G1", "mpdm-alice--bob-smith--carol-1");
        group.is_mpim = true;
        assert_eq!(group.label(), "@alice, @bob-smith, @carol");
        assert_eq!(mpim_label("not-a-group"), "not-a-group");
    }

    #[test]
    fn it_find_direct_message() {
        let mut dm = channel("D1", "");
        dm.is_im = true;
        dm.user = Some("U1".to_string());
        dm.display_name = Some("@alice".to_string());
        let channels = SlackChannels {
            unavailable: vec![],
            channels: vec![channel("C1", "general"), dm],
        };
        assert_eq!(channels.find_by_name("@alice").unwrap().id, "D1");
        assert_eq!(channels.find_by_name("D1").unwrap().id, "D1");
        assert_eq!(channels.find_by_name("U1").unwrap().id, "D1");
        assert_eq!(channels.find_by_name("general").unwrap().id, "C1");
    }

    #[test]
    fn it_disambiguate_duplicate_labels() {
        let mut first = channel("D1", "");
        first.is_im = true;
        first.display_name = Some("@alice".to_string());
        let mut second = channel("D2", "");
        second.is_im = true;
        second.display_name = Some("@alice".to_string());
        let mut channels = SlackChannels {
            unavailable: vec![],
            channels: vec![first, second],
        };
        channels.disambiguate_labels();
        assert_eq!(
            channels.channel_names(),
            ["@alice (D1)".to_string(), "@alice (D2)".to_string()]
        );
        assert!(channels.find_by_name("@alice").is_none());
        assert_eq!(channels.find_by_name("D2").unwrap().id, "D2");
    }

    #[tokio::test]
    #[serial]
    async fn it_get_channels_falls_back_when_a_scope_is_missing() {
        let mut server = mockito::Server::new_async().await;
        let combined = server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::UrlEncoded(
                "types".into(),
                "public_channel,private_channel,mpim,im".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":false,"error":"missing_scope"}"#)
            .expect(1)
            .create_async()
            .await;
        let public = server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::UrlEncoded(
                "types".into(),
                "public_channel".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true,"channels":[{"id":"C1","name":"general"}]}"#)
            .expect(1)
            .create_async()
            .await;
        let missing = server
            .mock("GET", "/api/conversations.list")
            .match_query(mockito::Matcher::Regex(
                "types=(private_channel|mpim|im)$".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":false,"error":"missing_scope"}"#)
            .expect(3)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let channels = slack_client.get_channels().await.unwrap();
        assert_eq!(channels.channel_names(), ["general"]);
        assert_eq!(channels.unavailable, ["private_channel", "mpim", "im"]);
        combined.assert_async().await;
        public.assert_async().await;
        missing.assert_async().await;
    }

    #[tokio::test]
    #[serial]
    async fn it_resolve_direct_message_names() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/api/users.info")
            .match_query(mockito::Matcher::UrlEncoded("user".into(), "U1".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/users_info/ok.json")
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let mut dm = channel("D1", "");
        dm.is_im = true;
        dm.user = Some("U1".to_string());
        let mut channels = vec![dm];
        slack_client
            .resolve_direct_message_names(&mut channels)
            .await
            .unwrap();
        assert_eq!(channels[0].label(), "@alice.s");
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

    #[test]
    fn it_get_user_display_name() {
        let user = |display_name: &str, real_name: Option<&str>| SlackUser {
            name: "alice".to_string(),
            real_name: real_name.map(str::to_string),
            profile: Some(SlackUserProfile {
                display_name: Some(display_name.to_string()),
            }),
        };
        assert_eq!(
            user("alice.s", Some("Alice Smith")).display_name(),
            "alice.s"
        );
        assert_eq!(user("", Some("Alice Smith")).display_name(), "Alice Smith");
        assert_eq!(user("", Some("")).display_name(), "alice");
        assert_eq!(user("", None).display_name(), "alice");
    }

    #[test]
    fn it_get_message_author() {
        let mut message = message_from(None);
        assert_eq!(message.author(), "unknown");
        message.user = Some("U0123456789".to_string());
        assert_eq!(message.author(), "U0123456789");
        message.user_name = Some("alice.s".to_string());
        assert_eq!(message.author(), "alice.s");
    }

    #[test]
    fn it_replace_mentions() {
        let resolve = |id: &str| (id == "U0123456789").then(|| "alice.s".to_string());
        assert_eq!(
            replace_mentions(
                "hi <@U0123456789>, cc <@U0987654321|bob> <@U0000000000>",
                resolve
            ),
            "hi @alice.s, cc @bob @U0000000000"
        );
        assert_eq!(
            replace_mentions("no mention <@ here", resolve),
            "no mention <@ here"
        );
    }

    #[tokio::test]
    #[serial]
    async fn it_resolve_user_names() {
        let mut server = mockito::Server::new_async().await;
        let alice = server
            .mock("GET", "/api/users.info")
            .match_query(mockito::Matcher::UrlEncoded(
                "user".into(),
                "U0123456789".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_file("tests/fixtures/slack/users_info/ok.json")
            .expect(1)
            .create_async()
            .await;
        let bob = server
            .mock("GET", "/api/users.info")
            .match_query(mockito::Matcher::UrlEncoded(
                "user".into(),
                "U0987654321".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":true,"user":{"id":"U0987654321","name":"bob"}}"#)
            .expect(1)
            .create_async()
            .await;
        let external = server
            .mock("GET", "/api/users.info")
            .match_query(mockito::Matcher::UrlEncoded(
                "user".into(),
                "U0000000000".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":false,"error":"user_not_found"}"#)
            .expect(1)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let mut messages = vec![
            message_from(Some("U0123456789")),
            message_from(Some("U0987654321")),
            message_from(Some("U0000000000")),
            message_from(None),
        ];
        messages[1].text = "thanks <@U0123456789>".to_string();
        slack_client
            .resolve_user_names(&mut messages)
            .await
            .unwrap();

        let authors: Vec<&str> = messages.iter().map(|m| m.author()).collect();
        assert_eq!(authors, ["alice.s", "bob", "U0000000000", "unknown"]);
        assert_eq!(messages[1].text, "thanks @alice.s");
        // Each user is looked up only once
        alice.assert_async().await;
        bob.assert_async().await;
        external.assert_async().await;
    }

    #[tokio::test]
    #[serial]
    async fn it_resolve_user_names_with_error() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/api/users.info")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ok":false,"error":"missing_scope"}"#)
            .create_async()
            .await;

        std::env::set_var("RSLACK_TOKEN", "test-token");
        let config = Config::new(None).unwrap();
        let slack_client = SlackClient::new(&config, &server.url());
        let mut messages = vec![message_from(Some("U0123456789"))];
        let err = slack_client
            .resolve_user_names(&mut messages)
            .await
            .unwrap_err();
        assert!(SlackApiError::is(&err, "missing_scope"));
        assert!(!SlackApiError::is(&err, "user_not_found"));
        assert_eq!(messages[0].author(), "U0123456789");
    }

    fn channel(id: &str, name: &str) -> SlackChannel {
        SlackChannel {
            id: id.to_string(),
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn message_from(user: Option<&str>) -> SlackMessage {
        SlackMessage {
            text: "hi".to_string(),
            user: user.map(str::to_string),
            ts: "1609459800.000100".to_string(),
            files: vec![],
            user_name: None,
        }
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
