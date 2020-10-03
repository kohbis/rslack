use anyhow::Result;
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

        config.token = fs::read_to_string(TOKEN_FILE).unwrap().trim().to_string();

        Ok(config)
    }
}
