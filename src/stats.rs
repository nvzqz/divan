//! Measurement statistics.

use std::time::{Duration, Instant};

use crate::time::FineDuration;

/// Statistics from samples.
#[derive(Debug)]
pub struct Stats {
    /// Total number of samples taken.
    pub sample_count: usize,

    /// Total number of iterations (`sample_count * iter_per_sample`).
    pub total_count: usize,

    /// The total amount of time spent benchmarking.
    pub total_duration: Duration,

    /// Mean time taken by all iterations.
    pub avg_duration: FineDuration,

    /// The minimum amount of time taken by an iteration.
    pub min_duration: FineDuration,

    /// The maximum amount of time taken by an iteration.
    pub max_duration: FineDuration,

    /// Midpoint time taken by an iteration.
    pub median_duration: FineDuration,
}

/// Measurement datum.
pub struct Sample {
    /// When the sample began.
    pub start: Instant,

    /// When the sample stopped.
    pub end: Instant,
}
