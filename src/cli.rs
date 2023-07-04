use clap::{
    builder::PossibleValue, value_parser, Arg, ArgAction, ArgMatches, ColorChoice, Command,
    ValueEnum,
};
use regex::Regex;

use crate::entry::Entry;

pub struct CliArgs {
    pub matches: ArgMatches,
    pub filter: Option<CliFilter>,
    pub skip: Vec<CliFilter>,
    pub action: CliAction,
    pub color: ColorChoice,
    pub format: CliOutputFormat,
    pub ignored_mode: CliIgnoredMode,
}

fn command() -> Command {
    fn ignored_flag(name: &'static str) -> Arg {
        Arg::new(name).long(name).num_args(0).hide(true)
    }

    Command::new("divan")
        .arg(
            Arg::new("filter")
                .value_name("FILTER")
                .help("Only run benchmarks whose names match this pattern"),
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
            Arg::new("format")
                .long("format")
                .value_name("pretty|terse")
                .help("Configure formatting of output")
                .value_parser(value_parser!(CliOutputFormat))
                .default_value("pretty"),
        )
        .arg(
            Arg::new("skip")
                .long("skip")
                .value_name("FILTER")
                .help("Skip benchmarks whose names match this pattern")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("exact")
                .long("exact")
                .help("Filter benchmarks by exact name rather than by pattern")
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
        let is_exact = matches.get_flag("exact");

        let mut parse_filter = |filter: &str| {
            if is_exact {
                CliFilter::Exact(filter.to_owned())
            } else {
                match Regex::new(filter) {
                    Ok(r) => CliFilter::Regex(r),
                    Err(error) => {
                        let kind = clap::error::ErrorKind::ValueValidation;
                        command.error(kind, error).exit();
                    }
                }
            }
        };

        CliArgs {
            filter: matches.get_one::<String>("filter").map(|filter| parse_filter(filter)),
            skip: matches
                .get_many::<String>("skip")
                .map(|filters| filters.map(|filter| parse_filter(filter)).collect())
                .unwrap_or_default(),
            action: if matches.get_flag("test") {
                CliAction::Test
            } else if matches.get_flag("list") {
                CliAction::List
            } else {
                CliAction::Bench
            },
            color: matches.get_one("color").copied().unwrap(),
            format: matches.get_one("format").copied().unwrap(),
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

    /// Returns `true` if `entry` should be considered for running.
    ///
    /// This does not take into account `entry.ignored` because that is handled
    /// separately.
    pub fn filter(&self, entry: &Entry) -> bool {
        let name = entry.name;

        if let Some(filter) = &self.filter {
            if !filter.is_match(name) {
                return false;
            }
        }

        !self.skip.iter().any(|filter| filter.is_match(name))
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
pub enum CliOutputFormat {
    Pretty,
    Terse,
}

impl ValueEnum for CliOutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Pretty, Self::Terse]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let name = match self {
            Self::Pretty => "pretty",
            Self::Terse => "terse",
        };
        Some(PossibleValue::new(name))
    }
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
