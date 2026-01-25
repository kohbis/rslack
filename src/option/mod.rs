use clap::Parser;

/// Default number of messages to fetch
pub const DEFAULT_MESSAGE_LIMIT: usize = 10;

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
}
