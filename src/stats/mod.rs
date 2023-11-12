//! Measurement statistics.

use crate::{
    counter::{KnownCounterKind, MaxCountUInt},
    time::FineDuration,
};

mod sample;

pub(crate) use sample::*;

/// Statistics from samples.
#[derive(serde::Serialize)]
pub(crate) struct Stats {
    /// Total number of samples taken.
    pub sample_count: u32,

    /// Total number of iterations (currently `sample_count * `sample_size`).
    pub iter_count: u64,

    pub time: StatsSet<FineDuration>,

    pub counts: [Option<StatsSet<MaxCountUInt>>; KnownCounterKind::COUNT],
}

impl Stats {
    pub fn get_counts(&self, counter_kind: KnownCounterKind) -> Option<&StatsSet<MaxCountUInt>> {
        self.counts[counter_kind as usize].as_ref()
    }
}

#[derive(Clone, Debug, serde::Serialize)]
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
