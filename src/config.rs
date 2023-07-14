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

/// How time should be measured.
#[derive(Clone, Copy, Default)]
pub enum Timer {
    /// Use the operating system's timer.
    #[default]
    Os,

    /// Use the timestamp counter.
    Tsc,
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

    pub fn is_terse(&self) -> bool {
        matches!(self, Self::Terse)
    }
}
