pub use clap::ColorChoice;
use regex::Regex;

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

/// How benchmarks should be sorted.
#[derive(Clone, Copy, Default)]
pub enum BenchSorting {
    /// Sort by kind, name, then location.
    #[default]
    Kind,

    /// Sort name, location, then kind.
    Name,

    /// Sort by location, kind, then name.
    Location,
}
