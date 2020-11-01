use anyhow::{anyhow, Result};
use std::env;

const SLACK_TOKEN: &str = "SLACK_TOKEN";

#[derive(Debug)]
pub struct Config {
    pub token: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let mut config = Self {
            token: String::new(),
        };

        match env::var(SLACK_TOKEN) {
            Ok(val) => {
                config.token = val
            },
            Err(err) => {
                return Err(anyhow!("{}: {}", err, SLACK_TOKEN))
            },
        }

        Ok(config)
    }
}
