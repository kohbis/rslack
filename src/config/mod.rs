use anyhow::{anyhow, bail, Result};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
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
        let path = match config_path {
            Some(path) => path,
            None => default_config_path()?,
        };

        // Environment variable takes precedence over the config file, unless it is empty.
        let token = match env::var(RSLACK_TOKEN).ok().filter(|t| !t.is_empty()) {
            Some(token) => token,
            None => read_token_from_file(&path)?.unwrap_or_default(),
        };

        let config = Self { token };
        config.validate()?;
        Ok(config)
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            bail!("{} not found.", RSLACK_TOKEN);
        }

        Ok(())
    }
}

/// Reads `RSLACK_TOKEN=...` from the config file; `None` when the file doesn't exist.
fn read_token_from_file(path: &Path) -> Result<Option<String>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => bail!("{}: {}", path.display(), err),
    };
    warn_if_insecure(path);

    let mut token = None;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|err| anyhow!("{}: {}", path.display(), err))?;
        if let Some((key, val)) = line.split_once('=') {
            if key.trim() == RSLACK_TOKEN {
                token = Some(val.trim().to_string());
            }
        }
    }

    Ok(token)
}

/// Returns `$XDG_CONFIG_HOME/rslack/config`, falling back to `~/.config/rslack/config`.
fn default_config_path() -> Result<PathBuf> {
    let base = match env::var_os(XDG_CONFIG_HOME).map(PathBuf::from) {
        // The XDG spec says relative paths must be ignored.
        Some(dir) if dir.is_absolute() => dir,
        _ => dirs::home_dir()
            .ok_or_else(|| anyhow!("Home directory not found."))?
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
    fn initialize_with_env_and_unreadable_file() {
        setup();
        env::set_var(RSLACK_TOKEN, "token-from-env-123");

        // The file isn't needed, so a broken one doesn't get in the way
        let config_path = PathBuf::from("tests/fixtures/config");
        let actual = Config::new(Some(config_path)).unwrap();
        assert_eq!(actual.token(), "token-from-env-123");
    }

    #[test]
    #[serial]
    fn initialize_with_empty_env_and_file() {
        setup();
        env::set_var(RSLACK_TOKEN, "");

        // An empty variable doesn't hide the token in the file
        let config_path = PathBuf::from("tests/fixtures/config/config.test.valid");
        let actual = Config::new(Some(config_path)).unwrap();
        let expected = Config {
            token: String::from("token-from-file-123"),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    #[serial]
    fn initialize_with_unreadable_file() {
        setup();

        // A directory can be opened but not read as lines
        let config_path = PathBuf::from("tests/fixtures/config");
        let err = Config::new(Some(config_path)).err().unwrap();
        assert!(err.to_string().starts_with("tests/fixtures/config: "));
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
        let config = Config {
            token: String::from("token"),
        };
        let actual = config.validate();
        assert!(actual.is_ok())
    }

    #[test]
    fn invalid_token() {
        let config = Config {
            token: String::new(),
        };
        let actual = config.validate();
        assert!(actual.is_err())
    }
}
