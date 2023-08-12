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

    /// Average time taken by all iterations.
    pub mean_time: FineDuration,

    /// The minimum amount of time taken by an iteration.
    pub fastest_time: FineDuration,

    /// The maximum amount of time taken by an iteration.
    pub slowest_time: FineDuration,

    /// Midpoint time taken by an iteration.
    pub median_time: FineDuration,

    /// Throughput counter.
    pub counter: Option<AnyCounter>,
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
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Set<T> {
            fastest: T,
            slowest: T,
            median: T,
            mean: T,
        }

        let stats = self.stats;

        f.debug_struct("Stats")
            .field(
                "time",
                &Set {
                    fastest: stats.fastest_time,
                    slowest: stats.slowest_time,
                    median: stats.median_time,
                    mean: stats.mean_time,
                },
            )
            .field(
                "thrpt",
                &stats.counter.as_ref().map(|counter| {
                    let display_throughput = |t| counter.display_throughput(t, self.bytes_format);

                    Set {
                        fastest: display_throughput(stats.fastest_time),
                        slowest: display_throughput(stats.slowest_time),
                        median: display_throughput(stats.median_time),
                        mean: display_throughput(stats.mean_time),
                    }
                }),
            )
            .field("sample_count", &stats.sample_count)
            .field("total_count", &stats.total_count)
            .finish()
    }
}
