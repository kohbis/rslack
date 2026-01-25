use clap::Parser;

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
            },
            Opt::parse_from::<[&str; 0], &str>([]),
        )
    }
}
