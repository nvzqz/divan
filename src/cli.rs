use clap::{value_parser, Arg, ArgAction, ArgMatches, ColorChoice, Command};
use regex::Regex;

pub struct CliArgs {
    pub matches: ArgMatches,
    pub filter: Option<CliFilter>,
    pub action: CliAction,
    pub color: ColorChoice,
    pub ignored_mode: CliIgnoredMode,
}

fn command() -> Command {
    fn ignored_flag(name: &'static str) -> Arg {
        Arg::new(name).long(name).num_args(0).hide(true)
    }

    Command::new("divan")
        .arg(
            Arg::new("filter")
                .value_name("BENCHNAME")
                .help("If specified, only run benches matching this pattern in their names"),
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
            Arg::new("exact")
                .long("exact")
                .help("Exactly match BENCHNAME rather than by pattern")
                .action(ArgAction::SetTrue),
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
        .arg(
            Arg::new("ignored")
                .long("ignored")
                .help("Run only ignored benchmarks")
                .action(ArgAction::SetTrue)
                .conflicts_with("include-ignored"),
        )
        .arg(
            Arg::new("include-ignored")
                .long("include-ignored")
                .help("Run ignored and not-ignored benchmarks")
                .action(ArgAction::SetTrue)
                .conflicts_with("ignored"),
        )
        // ignored:
        .args([ignored_flag("bench"), ignored_flag("nocapture"), ignored_flag("show-output")])
}

impl CliArgs {
    pub fn parse() -> Self {
        let mut command = command();
        let matches = command.get_matches_mut();

        CliArgs {
            filter: matches.get_one::<String>("filter").map(|filter| {
                if matches.get_flag("exact") {
                    CliFilter::Exact(filter.clone())
                } else {
                    match Regex::new(filter) {
                        Ok(r) => CliFilter::Regex(r),
                        Err(error) => {
                            let kind = clap::error::ErrorKind::ValueValidation;
                            command.error(kind, error).exit();
                        }
                    }
                }
            }),
            action: if matches.get_flag("test") {
                CliAction::Test
            } else if matches.get_flag("list") {
                CliAction::List
            } else {
                CliAction::Bench
            },
            color: matches.get_one("color").copied().unwrap(),
            ignored_mode: if matches.get_flag("ignored") {
                CliIgnoredMode::Only
            } else if matches.get_flag("include-ignored") {
                CliIgnoredMode::Include
            } else {
                CliIgnoredMode::Skip
            },
            matches,
        }
    }
}

/// Filters which benchmark to run based on name.
pub enum CliFilter {
    Regex(Regex),
    Exact(String),
}

impl CliFilter {
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            Self::Regex(r) => r.is_match(s),
            Self::Exact(e) => e == s,
        }
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

#[derive(Clone, Copy)]
pub enum CliIgnoredMode {
    Skip,
    Only,
    Include,
}

impl CliIgnoredMode {
    pub fn run_ignored(self) -> bool {
        matches!(self, Self::Only | Self::Include)
    }

    pub fn run_non_ignored(self) -> bool {
        matches!(self, Self::Skip | Self::Include)
    }

    pub fn should_run(self, ignored: bool) -> bool {
        if ignored {
            self.run_ignored()
        } else {
            self.run_non_ignored()
        }
    }
}
