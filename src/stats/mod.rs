//! Measurement statistics.

use crate::{counter::AnyCounter, time::FineDuration};

mod sample;

pub(crate) use sample::*;

/// Statistics from samples.
pub(crate) struct Stats {
    /// Total number of samples taken.
    #[allow(dead_code)]
    pub sample_count: u32,

    /// Total number of iterations (currently `sample_count * `sample_size`).
    #[allow(dead_code)]
    pub total_count: u64,

    pub time: StatsSet<FineDuration>,
    pub counter: Option<StatsSet<AnyCounter>>,
}

#[derive(Debug)]
pub(crate) struct StatsSet<T> {
    /// Associated with minimum amount of time taken by an iteration.
    pub fastest: T,

    /// Associated with maximum amount of time taken by an iteration.
    pub slowest: T,

    /// Associated with midpoint time taken by an iteration.
    pub median: T,

    /// Associated with average time taken by all iterations.
    pub mean: T,
}
