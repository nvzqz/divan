use clap::{builder::PossibleValue, value_parser, Arg, ArgAction, ColorChoice, Command, ValueEnum};

use crate::config::OutputFormat;

pub fn command() -> Command {
    fn option(name: &'static str) -> Arg {
        Arg::new(name).long(name)
    }

    fn flag(name: &'static str) -> Arg {
        option(name).action(ArgAction::SetTrue)
    }

    fn ignored_flag(name: &'static str) -> Arg {
        flag(name).hide(true)
    }

    Command::new("divan")
        .arg(
            Arg::new("filter")
                .value_name("FILTER")
                .help("Only run benchmarks whose names match this pattern"),
        )
        // libtest arguments:
        .arg(
            option("color")
                .value_name("WHEN")
                .help("Controls when to use colors")
                .value_parser(value_parser!(ColorChoice))
                .default_value("auto"),
        )
        .arg(
            option("format")
                .help("Configure formatting of output")
                .value_name("pretty|terse")
                .value_parser(value_parser!(OutputFormat))
                .default_value("pretty"),
        )
        .arg(
            option("skip")
                .value_name("FILTER")
                .help("Skip benchmarks whose names match this pattern")
                .action(ArgAction::Append),
        )
        .arg(flag("exact").help("Filter benchmarks by exact name rather than by pattern"))
        .arg(
            flag("test")
                .help("Run benchmarks once to ensure they run successfully")
                .conflicts_with("list"),
        )
        .arg(flag("list").help("Lists benchmarks").conflicts_with("test"))
        .arg(flag("ignored").help("Run only ignored benchmarks").conflicts_with("include-ignored"))
        .arg(
            flag("include-ignored")
                .help("Run ignored and not-ignored benchmarks")
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
