//! Measurement statistics.

use std::fmt;

use crate::{
    counter::{AnyCounter, BytesFormat},
    time::FineDuration,
};

mod sample;

pub(crate) use sample::*;

/// Statistics from samples.
pub(crate) struct Stats {
    /// Total number of samples taken.
    pub sample_count: u32,

    /// Total number of iterations (currently `sample_count * `sample_size`).
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

impl Stats {
    pub fn debug(&self, bytes_format: BytesFormat) -> impl fmt::Debug + '_ {
        DebugStats { stats: self, bytes_format }
    }
}

struct DebugStats<'a> {
    stats: &'a Stats,
    bytes_format: BytesFormat,
}

impl fmt::Debug for DebugStats<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stats = self.stats;
        let time = &stats.time;
        let bytes_format = self.bytes_format;

        f.debug_struct("Stats")
            .field("time", &stats.time)
            .field(
                "thrpt",
                &stats.counter.as_ref().map(|counter| StatsSet {
                    fastest: counter.fastest.display_throughput(time.fastest, bytes_format),
                    slowest: counter.slowest.display_throughput(time.slowest, bytes_format),
                    median: counter.median.display_throughput(time.median, bytes_format),
                    mean: counter.mean.display_throughput(time.mean, bytes_format),
                }),
            )
            .field("sample_count", &stats.sample_count)
            .field("total_count", &stats.total_count)
            .finish()
    }
}
