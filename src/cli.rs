use clap::{value_parser, Arg, ArgAction, ArgMatches, ColorChoice, Command};
use regex::Regex;

pub struct CliArgs {
    pub matches: ArgMatches,
    pub action: CliAction,
    pub color: ColorChoice,
}

fn command() -> Command {
    fn ignored_flag(name: &'static str) -> Arg {
        Arg::new(name).long(name).num_args(0).hide(true)
    }

    Command::new("divan")
        .arg(
            Arg::new("BENCHNAME")
                .help("If specified, only run benches matching this pattern in their names")
                .value_parser(value_parser!(Regex)),
        )
        // libtest arguments:
        .arg(
            Arg::new("color")
                .long("color")
                .value_name("WHEN")
                .help("Controls when to use colors")
                .value_parser(value_parser!(ColorChoice))
                .default_value("auto"),
        )
        .arg(
            Arg::new("test")
                .long("test")
                .help("Run benchmarks once to ensure they run successfully")
                .action(ArgAction::SetTrue)
                .conflicts_with("list"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .help("Lists benchmarks")
                .action(ArgAction::SetTrue)
                .conflicts_with("test"),
        )
        // ignored:
        .args([ignored_flag("bench"), ignored_flag("nocapture"), ignored_flag("show-output")])
}

impl CliArgs {
    pub fn parse() -> Self {
        let matches = command().get_matches();

        CliArgs {
            action: if matches.get_flag("test") {
                CliAction::Test
            } else if matches.get_flag("list") {
                CliAction::List
            } else {
                CliAction::Bench
            },
            color: matches.get_one("color").copied().unwrap(),
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
    /// Run benchmark loops.
    Bench,

    /// Run benchmarked functions once to ensure they run successfully.
    Test,

    /// List benchmarks.
    List,
}
