use anyhow::{anyhow, Result};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const RSLACK_TOKEN: &str = "RSLACK_TOKEN";

#[derive(Debug, PartialEq)]
pub struct Config {
    pub token: String,
}

impl Config {
    pub fn new(filename: &str) -> Result<Self> {
        let mut config = Self {
            token: String::new(),
        };

        config.read_from_env()?;

        if Path::new(filename).exists() {
            config.read_from_file(filename)?;
        }

        config.validate()?;

        Ok(config)
    }

    #[allow(clippy::single_match)]
    fn read_from_env(&mut self) -> Result<&Self> {
        match env::var(RSLACK_TOKEN) {
            Ok(token) => self.token = token,
            Err(_) => {}
        }

        Ok(self)
    }

    #[allow(clippy::single_match)]
    fn read_from_file(&mut self, filename: &str) -> Result<&Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(err) => return Err(anyhow!(err)),
        };

        for line in BufReader::new(file).lines() {
            if let Ok(line) = line {
                let entries: Vec<_> = line.split('=').map(str::trim).collect();

                if entries.len() == 2 {
                    let (key, val) = (entries[0].trim(), entries[1].trim().to_string());

                    match key {
                        RSLACK_TOKEN => self.token = val,
                        _ => {}
                    }
                }
            }
        }

        Ok(self)
    }

    fn validate(&mut self) -> Result<()> {
        if self.token.is_empty() {
            return Err(anyhow!("{} not found.", RSLACK_TOKEN));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup() {
        env::remove_var(RSLACK_TOKEN);
    }

    #[test]
    #[serial]
    fn initialize_with_valid_file() {
        setup();
        let actual = Config::new("tests/token.test.valid").unwrap();
        let expected = Config {
            token: String::from("token-from-file-123"),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn initialize_with_invalid_file() {
        setup();
        Config::new("tests/token.test.invalid").unwrap();
    }

    #[test]
    #[serial]
    fn initialize_with_env() {
        setup();
        env::set_var(RSLACK_TOKEN, "token-from-env-123");
        let actual = Config::new("no_file").unwrap();
        let expected = Config {
            token: String::from("token-from-env-123"),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    #[should_panic]
    fn initialize_with_empty_env() {
        setup();
        env::set_var(RSLACK_TOKEN, "");
        Config::new("no_file").unwrap();
    }

    #[test]
    #[serial]
    fn initialize_with_env_and_file() {
        setup();
        env::set_var(RSLACK_TOKEN, "token-from-env-123");
        let expected = Config {
            token: String::from("token-from-file-123"),
        };
        let actual = Config::new("tests/token.test.valid").unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn valid_token() {
        let mut config = Config {
            token: String::from("token"),
        };
        let actual = config.validate();
        assert!(actual.is_ok())
    }

    #[test]
    fn invalid_token() {
        let mut config = Config {
            token: String::new(),
        };
        let actual = config.validate();
        assert!(actual.is_err())
    }
}
