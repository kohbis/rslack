use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::Path;

const SLACK_TOKEN: &str = "SLACK_TOKEN";
const TOKEN_FILE: &str = ".token";

#[derive(Debug)]
pub struct Config {
    pub token: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let mut config = Self {
            token: String::new(),
        };

        config.read_from_env()?;

        if Path::new(TOKEN_FILE).exists() {
            config.read_from_file()?;
        }

        config.validate()?;

        Ok(config)
    }

    fn read_from_env(&mut self) -> Result<&Self> {
        match env::var(SLACK_TOKEN) {
            Ok(token) => self.token = token,
            Err(_) => {},
        }

        Ok(self)
    }

    fn read_from_file(&mut self) -> Result<&Self> {
        match fs::read_to_string(TOKEN_FILE) {
            Ok(content) => self.token = content.trim().to_string(),
            Err(_) => {},
        }

        Ok(self)
    }

    fn validate(&mut self) -> Result<()> {
        if self.token.is_empty() {
            return Err(anyhow!("{} not found.", SLACK_TOKEN))
        }

        Ok(())
    }
}
