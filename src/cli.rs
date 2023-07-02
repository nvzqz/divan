use clap::{value_parser, Arg, ArgMatches, Command};
use regex::Regex;

pub struct CliArgs {
    pub matches: ArgMatches,
}

fn command() -> Command {
    Command::new("divan")
        .arg(
            Arg::new("BENCHNAME")
                .help("If specified, only run benches matching this pattern in their names")
                .value_parser(value_parser!(Regex)),
        )
        // libtest arguments:
        .arg(Arg::new("bench").long("bench").num_args(0).hide(true))
}

impl CliArgs {
    pub fn parse() -> Self {
        Self {
            matches: command().get_matches(),
        }
    }

    pub fn filter(&self) -> Option<&Regex> {
        self.matches.get_one("BENCHNAME")
    }
}
