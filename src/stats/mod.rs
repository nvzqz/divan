//! Measurement statistics.

use std::fmt;

use crate::{counter::AnyCounter, time::FineDuration};

mod sample;

pub use sample::*;

/// Statistics from samples.
pub struct Stats {
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

impl fmt::Debug for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Set<T> {
            fastest: T,
            slowest: T,
            median: T,
            mean: T,
        }

        f.debug_struct("Stats")
            .field(
                "time",
                &Set {
                    fastest: self.fastest_time,
                    slowest: self.slowest_time,
                    median: self.median_time,
                    mean: self.mean_time,
                },
            )
            .field(
                "thrpt",
                &self.counter.as_ref().map(|counter| Set {
                    fastest: counter.display_throughput(self.fastest_time),
                    slowest: counter.display_throughput(self.slowest_time),
                    median: counter.display_throughput(self.median_time),
                    mean: counter.display_throughput(self.mean_time),
                }),
            )
            .field("sample_count", &self.sample_count)
            .field("total_count", &self.total_count)
            .finish()
    }
}
