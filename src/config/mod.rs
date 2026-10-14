use anyhow::{anyhow, Result};
use dirs;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const RSLACK_TOKEN: &str = "RSLACK_TOKEN";
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
const CONFIG_DIR: &str = "rslack";
const CONFIG_FILE: &str = "config";

/// Default Slack API base URL
pub const SLACK_URL: &str = "https://slack.com";

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Config {
    token: String,
}

impl Config {
    pub fn new(config_path: Option<PathBuf>) -> Result<Self> {
        let mut config = Self {
            token: String::new(),
        };

        let path = match config_path {
            Some(path) => path,
            None => default_config_path()?,
        };
        if config.read_from_file(&path).is_ok() {
            warn_if_insecure(&path);
        }

        // Environment variable takes precedence over the config file.
        config.read_from_env()?;

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
    fn read_from_file(&mut self, path: &Path) -> Result<&Self> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => anyhow::bail!("{}: {}", path.display(), err),
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

    pub fn token(&self) -> &str {
        &self.token
    }

    fn validate(&mut self) -> Result<()> {
        if self.token.is_empty() {
            anyhow::bail!("{} not found.", RSLACK_TOKEN);
        }

        Ok(())
    }
}

/// Returns `$XDG_CONFIG_HOME/rslack/config`, falling back to `~/.config/rslack/config`.
fn default_config_path() -> Result<PathBuf> {
    let base = match env::var_os(XDG_CONFIG_HOME).map(PathBuf::from) {
        // The XDG spec says relative paths must be ignored.
        Some(dir) if dir.is_absolute() => dir,
        _ => dirs::home_dir()
            .ok_or(anyhow!("Home directory not found."))?
            .join(".config"),
    };

    Ok(base.join(CONFIG_DIR).join(CONFIG_FILE))
}

#[cfg(unix)]
fn warn_if_insecure(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.permissions().mode() & 0o077 != 0 {
            eprintln!(
                "Warning: {} is accessible by other users. Run `chmod 600 {}`.",
                path.display(),
                path.display()
            );
        }
    }
}

#[cfg(not(unix))]
fn warn_if_insecure(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn setup() {
        env::remove_var(RSLACK_TOKEN);
        env::remove_var(XDG_CONFIG_HOME);
    }

    #[test]
    #[serial]
    fn initialize_with_valid_file() {
        setup();

        let config_path = PathBuf::from("tests/fixtures/config/config.test.valid");
        let actual = Config::new(Some(config_path)).unwrap();
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

        let config_path = PathBuf::from("tests/config.test.invalid");
        Config::new(Some(config_path)).unwrap();
    }

    #[test]
    #[serial]
    fn initialize_with_env() {
        setup();
        env::set_var(RSLACK_TOKEN, "token-from-env-123");

        let config_path = PathBuf::from("no_file");
        let actual = Config::new(Some(config_path)).unwrap();
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

        let config_path = PathBuf::from("no_file");
        Config::new(Some(config_path)).unwrap();
    }

    #[test]
    #[serial]
    fn initialize_with_env_and_file() {
        setup();
        env::set_var(RSLACK_TOKEN, "token-from-env-123");

        let expected = Config {
            token: String::from("token-from-env-123"),
        };
        let config_path = PathBuf::from("tests/fixtures/config/config.test.valid");
        let actual = Config::new(Some(config_path)).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    fn default_config_path_with_xdg_config_home() {
        setup();
        env::set_var(XDG_CONFIG_HOME, "/xdg/config");

        let actual = default_config_path().unwrap();
        assert_eq!(actual, PathBuf::from("/xdg/config/rslack/config"));
    }

    #[test]
    #[serial]
    fn default_config_path_without_xdg_config_home() {
        setup();

        let actual = default_config_path().unwrap();
        let expected = dirs::home_dir().unwrap().join(".config/rslack/config");
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    fn default_config_path_ignores_relative_xdg_config_home() {
        setup();
        env::set_var(XDG_CONFIG_HOME, "relative/config");

        let actual = default_config_path().unwrap();
        let expected = dirs::home_dir().unwrap().join(".config/rslack/config");
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
