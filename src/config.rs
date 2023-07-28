use std::{error::Error, str::FromStr, time::Duration};

pub use clap::ColorChoice;
use regex::Regex;

/// `Duration` wrapper for parsing seconds from the CLI.
#[derive(Clone, Copy)]
pub(crate) struct ParsedSeconds(pub Duration);

impl FromStr for ParsedSeconds {
    type Err = Box<dyn Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Duration::try_from_secs_f64(f64::from_str(s)?)?))
    }
}

/// The primary action to perform.
#[derive(Clone, Copy, Default)]
pub(crate) enum Action {
    /// Run benchmark loops.
    #[default]
    Bench,

    /// Run benchmarked functions once to ensure they run successfully.
    Test,

    /// List benchmarks.
    List,
}

#[allow(dead_code)]
impl Action {
    pub fn is_bench(&self) -> bool {
        matches!(self, Self::Bench)
    }

    pub fn is_test(&self) -> bool {
        matches!(self, Self::Test)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List)
    }
}

/// Filters which benchmark to run based on name.
pub enum Filter {
    Regex(Regex),
    Exact(String),
}

impl Filter {
    /// Returns `true` if a string matches this filter.
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            Self::Regex(r) => r.is_match(s),
            Self::Exact(e) => e == s,
        }
    }
}

/// How to treat benchmarks based on whether they're marked as `#[ignore]`.
#[derive(Copy, Clone, Default)]
pub enum RunIgnored {
    /// Skip ignored.
    #[default]
    No,

    /// `--include-ignored`.
    Yes,

    /// `--ignored`.
    Only,
}

impl RunIgnored {
    pub fn run_ignored(self) -> bool {
        matches!(self, Self::Yes | Self::Only)
    }

    pub fn run_non_ignored(self) -> bool {
        matches!(self, Self::Yes | Self::No)
    }

    pub fn should_run(self, ignored: bool) -> bool {
        if ignored {
            self.run_ignored()
        } else {
            self.run_non_ignored()
        }
    }
}

/// The style with which to format output.
#[derive(Clone, Copy, Default)]
pub enum FormatStyle {
    /// Benchmarks are formatted as a tree.
    #[default]
    Pretty,

    /// Each benchmark is printed on its own line.
    Terse,
}

impl FormatStyle {
    pub fn is_pretty(&self) -> bool {
        matches!(self, Self::Pretty)
    }
}

/// The attribute to sort benchmarks by.
#[derive(Clone, Copy, Default)]
pub enum SortingAttr {
    /// Sort by kind, then by name and location.
    #[default]
    Kind,

    /// Sort by name, then by location and kind.
    Name,

    /// Sort by location, then by kind and name.
    Location,
}

impl SortingAttr {
    /// Returns an array containing `self` along with other attributes that
    /// should break ties if attributes are equal.
    pub fn with_tie_breakers(self) -> [Self; 3] {
        use SortingAttr::*;

        match self {
            Kind => [self, Name, Location],
            Name => [self, Location, Kind],
            Location => [self, Kind, Name],
        }
    }
}
