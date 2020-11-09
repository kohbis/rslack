use anyhow::{anyhow, Result};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
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
        let file = match File::open(TOKEN_FILE) {
            Ok(file) => file,
            Err(err) => {
                return Err(anyhow!(err))
            },
        };

        for line in BufReader::new(file).lines() {
            if let Ok(line) = line {
                let entries: Vec<_> = line.split("=").map(str::trim).collect();

                if entries.len() == 2 {
                    let (key, val) = (entries[0].trim(), entries[1].trim().to_string());

                    match key {
                        SLACK_TOKEN => self.token = val,
                        _ => {},
                    }
                }
            }
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
