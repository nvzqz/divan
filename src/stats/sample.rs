use crate::time::FineDuration;

/// Measurement datum.
pub struct Sample {
    /// The time this sample took to run.
    pub total_duration: FineDuration,
}

/// [`Sample`] collection.
#[derive(Default)]
pub struct SampleCollection {
    /// The number of iterations within each sample.
    pub sample_size: u32,

    /// Collected samples.
    pub all: Vec<Sample>,
}

impl SampleCollection {
    /// Computes the total number of iterations across all samples.
    ///
    /// We use `u64` in case sample count and sizes are huge.
    #[inline]
    pub fn iter_count(&self) -> u64 {
        self.sample_size as u64 * self.all.len() as u64
    }

    /// Computes the total time across all samples.
    #[inline]
    pub fn total_duration(&self) -> FineDuration {
        FineDuration { picos: self.all.iter().map(|s| s.total_duration.picos).sum() }
    }
}
