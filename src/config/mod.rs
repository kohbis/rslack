use anyhow::{anyhow, Result};
use std::fs;

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

        match fs::read_to_string(TOKEN_FILE) {
            Ok(content) => {
                config.token = content.trim().to_string()
            },
            Err(e) => {
                return Err(anyhow!(e))
            },
        }

        Ok(config)
    }
}
