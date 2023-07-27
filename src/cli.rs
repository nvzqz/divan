use clap::{builder::PossibleValue, value_parser, Arg, ArgAction, ColorChoice, Command, ValueEnum};

use crate::{
    config::{FormatStyle, SortingAttr},
    time::TimerKind,
};

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
        // Our custom arguments, which are not supported by libtest:
        .arg(
            option("sample-count")
                .env("DIVAN_SAMPLE_COUNT")
                .value_name("N")
                .help("Set the number of sampling iterations")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            option("sample-size")
                .env("DIVAN_SAMPLE_SIZE")
                .value_name("N")
                .help("Set the number of iterations inside a single sample")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            option("timer")
                .env("DIVAN_TIMER")
                .value_name("os|tsc")
                .help("Set the timer used for measuring samples")
                .value_parser(value_parser!(TimerKind)),
        )
        .arg(
            option("sort-by")
                .env("DIVAN_SORT_BY")
                .value_name("ATTRIBUTE")
                .help("Sort benchmarks by a certain ordering")
                .value_parser(value_parser!(SortingAttr))
                .default_value("kind"),
        )
        // libtest-supported arguments:
        .arg(
            Arg::new("filter")
                .value_name("FILTER")
                .help("Only run benchmarks whose names match this pattern"),
        )
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
                .value_parser(value_parser!(FormatStyle))
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

impl ValueEnum for TimerKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Os, Self::Tsc]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let name = match self {
            Self::Os => "os",
            Self::Tsc => "tsc",
        };
        Some(PossibleValue::new(name))
    }
}

impl ValueEnum for FormatStyle {
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

impl ValueEnum for SortingAttr {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Kind, Self::Name, Self::Location]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let name = match self {
            Self::Kind => "kind",
            Self::Name => "name",
            Self::Location => "location",
        };
        Some(PossibleValue::new(name))
    }
}
