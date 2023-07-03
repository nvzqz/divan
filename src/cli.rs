use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use regex::Regex;

pub struct CliArgs {
    pub matches: ArgMatches,
    pub action: CliAction,
}

fn command() -> Command {
    Command::new("divan")
        .arg(
            Arg::new("BENCHNAME")
                .help("If specified, only run benches matching this pattern in their names")
                .value_parser(value_parser!(Regex)),
        )
        // libtest arguments:
        .arg(Arg::new("list").long("list").help("Lists benchmarks").action(ArgAction::SetTrue))
        // ignored:
        .arg(Arg::new("bench").long("bench").num_args(0).hide(true))
}

impl CliArgs {
    pub fn parse() -> Self {
        let matches = command().get_matches();

        CliArgs {
            action: if matches.get_flag("list") { CliAction::List } else { CliAction::Bench },
            matches,
        }
    }

    pub fn filter(&self) -> Option<&Regex> {
        self.matches.get_one("BENCHNAME")
    }
}

/// The primary action to perform in `main()`.
#[derive(Clone, Copy)]
pub enum CliAction {
    /// Run benchmarks.
    Bench,

    /// List benchmarks.
    List,
}
