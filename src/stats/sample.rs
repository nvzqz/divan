use std::collections::HashMap;

use crate::{
    alloc::ThreadAllocTallyMap,
    counter::KnownCounterKind,
    time::{FineDuration, Timer, Timestamp},
};

/// Timing measurement.
pub(crate) struct TimeSample {
    /// The time this sample took to run.
    ///
    /// This is gotten from [`RawSample`] with:
    /// `end.duration_since(start, timer).clamp_to(timer.precision())`.
    pub duration: FineDuration,
}

/// Unprocessed measurement.
///
/// This cannot be serialized because [`Timestamp`] is an implementation detail
/// for both the `Instant` and TSC timers.
pub(crate) struct RawSample {
    pub start: Timestamp,
    pub end: Timestamp,
    pub timer: Timer,
    pub alloc_tallies: ThreadAllocTallyMap,
    pub counter_totals: [u128; KnownCounterKind::COUNT],
}

/// Multi-thread measurement.
pub(crate) struct ThreadSample {
    /// The total wall clock time spent over the collected samples.
    ///
    /// This is the earliest [`RawSample::start`] subtracted from the latest
    /// [`RawSample::end`] across all threads for the multi-thread sample set.
    /// In other words, it is the time spent between the timing section
    /// barriers.
    // TODO: Report counter throughput.
    #[allow(dead_code)]
    pub total_wall_time: FineDuration,
}

impl RawSample {
    /// Simply computes `end - start` without clamping to precision.
    #[inline]
    pub fn duration(&self) -> FineDuration {
        self.end.duration_since(self.start, self.timer)
    }
}

/// Sample collection.
#[derive(Default)]
pub(crate) struct SampleCollection {
    /// The number of iterations within each sample.
    pub sample_size: u32,

    /// Collected timings.
    pub time_samples: Vec<TimeSample>,

    /// Allocation information associated with `time_samples` by index.
    pub alloc_tallies: HashMap<u32, ThreadAllocTallyMap>,

    /// Collected multi-thread data.
    ///
    /// To associate this with samples in `all`, stride over `all` with the
    /// thread count.
    pub threads: Vec<ThreadSample>,
}

impl SampleCollection {
    /// Discards all recorded data.
    #[inline]
    pub fn clear(&mut self) {
        self.time_samples.clear();
        self.alloc_tallies.clear();
        self.threads.clear();
    }

    /// Computes the total number of iterations across all samples.
    ///
    /// We use `u64` in case sample count and sizes are huge.
    #[inline]
    pub fn iter_count(&self) -> u64 {
        self.sample_size as u64 * self.time_samples.len() as u64
    }

    /// Computes the total time across all samples.
    #[inline]
    pub fn total_duration(&self) -> FineDuration {
        FineDuration { picos: self.time_samples.iter().map(|s| s.duration.picos).sum() }
    }

    /// Returns all samples sorted by duration.
    #[inline]
    pub fn sorted_samples(&self) -> Vec<&TimeSample> {
        let mut result: Vec<&TimeSample> = self.time_samples.iter().collect();
        result.sort_unstable_by_key(|s| s.duration);
        result
    }
}
