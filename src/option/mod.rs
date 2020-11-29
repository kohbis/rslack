use structopt::StructOpt;

#[derive(Debug, PartialEq, StructOpt)]
pub struct Opt {
    #[structopt(short, long, default_value = "")]
    pub channel: String,

    #[structopt(short, long, default_value = "")]
    pub message: String,
}

impl Opt {
    pub fn get_opts() -> Self {
        Opt::from_args()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_with_default() {
        assert_eq!(
            Opt::get_opts(),
            Opt {
                channel: "".to_string(),
                message: "".to_string()
            }
        )
    }
}
