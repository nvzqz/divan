//! Measurement statistics.

use crate::time::FineDuration;

mod sample;

pub use sample::*;

/// Statistics from samples.
#[derive(Debug)]
pub struct Stats {
    /// Total number of samples taken.
    pub sample_count: u32,

    /// Total number of iterations (currently `sample_count * `sample_size`).
    pub total_count: u64,

    /// Mean time taken by all iterations.
    pub avg_duration: FineDuration,

    /// The minimum amount of time taken by an iteration.
    pub min_duration: FineDuration,

    /// The maximum amount of time taken by an iteration.
    pub max_duration: FineDuration,

    /// Midpoint time taken by an iteration.
    pub median_duration: FineDuration,
}
