use clap::{builder::PossibleValue, value_parser, Arg, ArgAction, ColorChoice, Command, ValueEnum};

use crate::config::OutputFormat;

pub fn command() -> Command {
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
                .value_parser(value_parser!(OutputFormat))
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

impl ValueEnum for OutputFormat {
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
