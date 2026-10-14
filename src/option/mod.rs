use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Default number of messages to fetch
pub const DEFAULT_MESSAGE_LIMIT: usize = 10;

/// How to display messages read from a channel
#[derive(Clone, Copy, Debug, Default, PartialEq, ValueEnum)]
pub enum MessageFormat {
    /// Each message as a header line followed by its text
    #[default]
    Text,
    /// A table with DATE, USER and MESSAGE columns
    Table,
}

#[derive(Parser)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[command(author, version, about, long_about = None)]
pub struct Opt {
    /// Slack channel name or ID
    #[arg(short, long)]
    pub channel: Option<String>,

    /// Message to post
    #[arg(short, long)]
    pub message: Option<String>,

    /// Read messages from channel instead of posting
    #[arg(short, long)]
    pub read: bool,

    /// Number of messages to fetch (default: 10)
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Image file to upload (message is posted as its comment)
    #[arg(short, long, conflicts_with = "read")]
    pub image: Option<PathBuf>,

    /// Format of messages read with --read
    #[arg(short, long, value_enum, default_value_t, requires = "read")]
    pub format: MessageFormat,
}

impl Opt {
    pub fn get_opts() -> Self {
        Opt::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_with_default() {
        assert_eq!(
            Opt {
                channel: None,
                message: None,
                read: false,
                limit: None,
                image: None,
                format: MessageFormat::Text,
            },
            Opt::parse_from::<[&str; 0], &str>([]),
        )
    }

    #[test]
    fn argument_with_read_flag() {
        let opts = Opt::parse_from(["rslack", "-r", "-c", "general"]);
        assert!(opts.read);
        assert_eq!(opts.channel, Some("general".to_string()));
    }

    #[test]
    fn argument_with_limit() {
        let opts = Opt::parse_from(["rslack", "-r", "-l", "20"]);
        assert!(opts.read);
        assert_eq!(opts.limit, Some(20));
    }

    #[test]
    fn argument_with_image() {
        let opts = Opt::parse_from(["rslack", "-c", "general", "-i", "image.png", "-m", "hi"]);
        assert_eq!(opts.image, Some(PathBuf::from("image.png")));
        assert_eq!(opts.message, Some("hi".to_string()));
    }

    #[test]
    fn argument_with_format() {
        let opts = Opt::parse_from(["rslack", "-r", "--format", "table"]);
        assert_eq!(opts.format, MessageFormat::Table);
        let opts = Opt::parse_from(["rslack", "-r", "-f", "text"]);
        assert_eq!(opts.format, MessageFormat::Text);
    }

    #[test]
    fn argument_with_format_requires_read() {
        assert!(Opt::try_parse_from(["rslack", "--format", "table"]).is_err());
        assert!(Opt::try_parse_from(["rslack", "-r", "--format", "csv"]).is_err());
    }

    #[test]
    fn argument_with_image_and_read_conflicts() {
        assert!(Opt::try_parse_from(["rslack", "-r", "-i", "image.png"]).is_err());
    }
}
