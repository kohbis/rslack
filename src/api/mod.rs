use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::str;
use url::Url;

use super::config::Config;

#[derive(Deserialize, Debug)]
pub struct SlackResponce {
    pub ok: bool,
    pub error: Option<String>,
    pub channels: Option<Vec<SlackChannel>>,
}

#[derive(Deserialize, Debug)]
pub struct SlackChannel {
    pub name: String,
}

lazy_static! {
    static ref FILETYPE_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("txt", "text");
        m.insert("rs", "rust");
        m.insert("Dockerfile", "dockerfile");
        m
    };
}

fn get_filetype_from_path(path: &str) -> &str {
    let path = Path::new(path);
    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => {
            if let Some(filetype) =  FILETYPE_MAP.get(ext) {
                return filetype;
            }
        },
        _ => {
            if let Some(filename) = path.file_name().and_then(OsStr::to_str) {
                if let Some(filetype) = FILETYPE_MAP.get(filename) {
                    return filetype;
                }
            }
        },
    }

    &"text"
}

pub async fn get_channels(config: &Config) -> Result<Vec<SlackChannel>> {
    let params = vec![("token", &config.token)];
    let url = Url::parse_with_params("https://slack.com/api/conversations.list", params).unwrap();

    let client = Client::new().get(url);

    let res: SlackResponce = client.send().await?.json().await?;

    if res.ok {
        Ok(res.channels.unwrap())
    } else {
        Err(anyhow!("{}", res.error.unwrap()))
    }
}

pub async fn post_message(config: &Config, channel: &str, text: &str) -> Result<SlackResponce> {
    let body = vec![
        ("channel", channel),
        ("text", text),
        ("token", &config.token),
    ];
    let url = Url::parse("https://slack.com/api/chat.postMessage").unwrap();

    let client = Client::new().post(url).form(&body);

    let res: SlackResponce = client.send().await?.json().await?;

    if res.ok {
        Ok(res)
    } else {
        Err(anyhow!("{}", res.error.unwrap()))
    }
}

pub async fn upload_file(config: &Config, channel: &str, path: &str) -> Result<SlackResponce> {
    let content = fs::read_to_string(path)?;
    let filetype = get_filetype_from_path(path);

    let body = vec![
        ("channels", channel),
        ("filename", path),
        ("filetype", filetype),
        ("content", &content),
        ("token", &config.token),
    ];
    let url = Url::parse("https://slack.com/api/files.upload").unwrap();

    let client = Client::new().post(url).form(&body);

    let res: SlackResponce = client.send().await?.json().await?;
    if res.ok {
        Ok(res)
    } else {
        Err(anyhow!("{}", res.error.unwrap()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_collect_filetype_from_path() {
        assert_eq!(&get_filetype_from_path("tests/check_ext.txt"), &"text");
        assert_eq!(&get_filetype_from_path("tests/check_ext.rs"), &"rust");
        assert_eq!(&get_filetype_from_path("tests/Dockerfile"), &"dockerfile");
    }
}
