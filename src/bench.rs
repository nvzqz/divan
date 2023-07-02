use std::{
    fmt,
    time::{Duration, Instant},
};

/// `#[divan::bench]` loop context.
///
/// Functions called within the benchmark loop should be `#[inline(always)]` to
/// ensure instruction cache locality.
///
/// Instances of this type are publicly accessible to generated code, so care
/// should be taken when making fields fully public.
pub struct Context {
    /// When benchmarking began.
    pub(crate) start: Instant,

    /// Recorded samples.
    pub(crate) samples: Vec<Sample>,

    /// The number of iterations between recording samples.
    pub iter_per_sample: u32,
}

impl Context {
    #[inline(always)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            // TODO: Pick these numbers dynamically.
            samples: Vec::with_capacity(1_000),
            iter_per_sample: 1_000,
        }
    }

    /// Returns the number of samples that should be taken.
    #[inline(always)]
    pub fn target_sample_count(&self) -> usize {
        self.samples.capacity()
    }

    /// Records measurement info at the end of a loop.
    #[inline(always)]
    pub fn record_sample(&mut self) {
        self.samples.push(Sample {
            instant: Instant::now(),
        });
    }

    pub fn compute_stats(&self) -> Option<Stats> {
        // Converts a sample duration to an iteration duration.
        let sample_to_iter_duration =
            |sample| SmallDuration::average(sample, self.iter_per_sample as u128);

        let sample_count = self.samples.len();
        let total_count = sample_count * self.iter_per_sample as usize;

        let (first, rest) = self.samples.split_first()?;

        let first_duration = sample_to_iter_duration(first.instant.duration_since(self.start));
        let mut all_durations = vec![first_duration];

        let mut prev_instant = first.instant;
        for sample in rest {
            all_durations.push(sample_to_iter_duration(
                sample.instant.duration_since(prev_instant),
            ));
            prev_instant = sample.instant;
        }

        let total_duration = prev_instant.duration_since(self.start);
        let avg_duration = SmallDuration::average(total_duration, total_count as u128);

        all_durations.sort_unstable();

        let min_duration = *all_durations.first().unwrap();
        let max_duration = *all_durations.last().unwrap();

        let median_duration = if sample_count % 2 == 0 {
            // Take average of two middle numbers.
            let a = all_durations[sample_count / 2];
            let b = all_durations[(sample_count / 2) - 1];

            SmallDuration {
                picos: (a.picos + b.picos) / 2,
            }
        } else {
            // Single middle number.
            all_durations[sample_count / 2]
        };

        Some(Stats {
            sample_count,
            total_count,
            total_duration,
            avg_duration,
            min_duration,
            max_duration,
            median_duration,
        })
    }
}

/// Measurement datum.
pub struct Sample {
    /// When the sample was recorded.
    pub instant: Instant,
}

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
    pub avg_duration: SmallDuration,

    /// The minimum amount of time taken by an iteration.
    pub min_duration: SmallDuration,

    /// The maximum amount of time taken by an iteration.
    pub max_duration: SmallDuration,

    /// Midpoint time taken by an iteration.
    pub median_duration: SmallDuration,
}

/// [Picosecond](https://en.wikipedia.org/wiki/Picosecond)-precise [`Duration`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SmallDuration {
    picos: u128,
}

impl fmt::Debug for SmallDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // `Duration` has no notion of picoseconds, so we manually format
        // picoseconds and nanoseconds ourselves.
        if self.picos < 1_000 {
            write!(f, "{}ps", self.picos)
        } else if self.picos < 1_000_000 {
            let nanos = self.picos as f64 / 1_000.0;
            write!(f, "{}ns", nanos)
        } else {
            Duration::from_nanos((self.picos / 1_000) as u64).fmt(f)
        }
    }
}

impl SmallDuration {
    /// Computes the average of a duration over a number of elements.
    fn average(duration: Duration, n: u128) -> Self {
        Self {
            picos: (duration.as_nanos() * 1_000) / n,
        }
    }
}
