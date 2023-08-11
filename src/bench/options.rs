use std::time::Duration;

use crate::{counter::AnyCounter, time::FineDuration};

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

    /// Counts the number of values processed each iteration of a benchmarked
    /// function.
    pub counter: Option<AnyCounter>,

    /// The time floor for benchmarking a function.
    pub min_time: Option<Duration>,

    /// The time ceiling for benchmarking a function.
    pub max_time: Option<Duration>,

    /// When accounting for `min_time` or `max_time`, skip time external to
    /// benchmarked functions, such as time spent generating inputs and running
    /// [`Drop`].
    pub skip_ext_time: Option<bool>,
}

impl BenchOptions {
    /// Overwrites `other` with values set in `self`.
    #[must_use]
    pub(crate) fn overwrite(&self, other: &Self) -> Self {
        Self {
            // `Copy` values:
            sample_count: self.sample_count.or(other.sample_count),
            sample_size: self.sample_size.or(other.sample_size),
            min_time: self.min_time.or(other.min_time),
            max_time: self.max_time.or(other.max_time),
            skip_ext_time: self.skip_ext_time.or(other.skip_ext_time),

            // `Clone` values:
            counter: self.counter.as_ref().or(other.counter.as_ref()).cloned(),
        }
    }

    /// Returns `true` if non-zero samples are specified.
    #[inline]
    pub(crate) fn has_samples(&self) -> bool {
        self.sample_count != Some(0) && self.sample_size != Some(0)
    }

    #[inline]
    pub(crate) fn min_time(&self) -> FineDuration {
        self.min_time.map(FineDuration::from).unwrap_or_default()
    }

    #[inline]
    pub(crate) fn max_time(&self) -> FineDuration {
        self.max_time.map(FineDuration::from).unwrap_or(FineDuration::MAX)
    }
}
