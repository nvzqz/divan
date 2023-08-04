use std::time::Duration;

/// Benchmarking options set directly by the user in `#[divan::bench]` and
/// `#[divan::bench_group]`.
///
/// Changes to fields must be reflected in the "Options" sections of the docs
/// for `#[divan::bench]` and `#[divan::bench_group]`.
#[derive(Clone, Default)]
pub struct BenchOptions {
    /// The number of sample recordings.
    pub sample_count: Option<u32>,

    /// The number of iterations inside a single sample.
    pub sample_size: Option<u32>,

    /// The time floor for benchmarking a function.
    pub min_time: Option<Duration>,

    /// The time ceiling for benchmarking a function.
    pub max_time: Option<Duration>,

    /// Skip time spent generating inputs when accounting for `min_time` or
    /// `max_time`.
    pub skip_input_time: Option<bool>,
}

impl BenchOptions {
    /// Overwrites `other` with values set in `self`.
    #[must_use]
    pub(crate) fn overwrite(&self, other: &Self) -> Self {
        Self {
            sample_count: self.sample_count.or(other.sample_count),
            sample_size: self.sample_size.or(other.sample_size),
            min_time: self.min_time.or(other.min_time),
            max_time: self.max_time.or(other.max_time),
            skip_input_time: self.skip_input_time.or(other.skip_input_time),
        }
    }
}
