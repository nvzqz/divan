use std::{cmp::Ordering, num::NonZeroU64, sync::OnceLock};

use crate::time::{AnyTimestamp, FineDuration, TscTimestamp, TscUnavailable};

/// Measures time.
#[derive(Clone, Copy, Default)]
pub enum Timer {
    /// Operating system timer.
    #[default]
    Os,

    /// CPU timestamp counter.
    Tsc {
        /// [`TscTimestamp::frequency`].
        frequency: NonZeroU64,
    },
}

impl Timer {
    const COUNT: usize = 2;

    /// Returns all available timers.
    #[cfg(test)]
    pub fn available() -> Vec<Self> {
        let mut timers = vec![Self::Os];

        if let Ok(tsc) = Self::get_tsc() {
            timers.push(tsc);
        }

        timers
    }

    /// Attempts to get the CPU timestamp counter.
    #[inline]
    pub fn get_tsc() -> Result<Self, TscUnavailable> {
        Ok(Self::Tsc { frequency: TscTimestamp::frequency()? })
    }

    #[inline]
    pub fn kind(self) -> TimerKind {
        match self {
            Self::Os => TimerKind::Os,
            Self::Tsc { .. } => TimerKind::Tsc,
        }
    }

    /// Returns the smallest non-zero duration that this timer can measure.
    ///
    /// The result is cached.
    pub fn precision(self) -> FineDuration {
        static CACHED: [OnceLock<FineDuration>; Timer::COUNT] = [OnceLock::new(), OnceLock::new()];

        let cached = &CACHED[self.kind() as usize];

        *cached.get_or_init(|| self.measure_precision())
    }

    fn measure_precision(self) -> FineDuration {
        let timer_kind = self.kind();

        // Start with the worst possible minimum.
        let mut min_sample = FineDuration::MAX;
        let mut seen_count = 0;

        // If timing in immediate succession fails to produce a non-zero sample,
        // an artificial delay is added by looping. `usize` is intentionally
        // used to make looping cheap.
        let mut delay_len: usize = 0;

        loop {
            for _ in 0..100 {
                // Use `AnyTimestamp` to minimize overhead.
                let sample_start: AnyTimestamp;
                let sample_end: AnyTimestamp;

                if delay_len == 0 {
                    // Immediate succession.
                    sample_start = AnyTimestamp::start(timer_kind);
                    sample_end = AnyTimestamp::end(timer_kind);
                } else {
                    // Add delay.
                    sample_start = AnyTimestamp::start(timer_kind);
                    for n in 0..delay_len {
                        crate::black_box(n);
                    }
                    sample_end = AnyTimestamp::end(timer_kind);
                }

                // SAFETY: These values are guaranteed to be the correct variant
                // because they were created from the same `timer_kind`.
                let [sample_start, sample_end] = unsafe {
                    [sample_start.into_timestamp(timer_kind), sample_end.into_timestamp(timer_kind)]
                };

                let sample = sample_end.duration_since(sample_start, self);

                // Discard sample if irrelevant.
                if sample.picos == 0 {
                    continue;
                }

                match sample.cmp(&min_sample) {
                    Ordering::Greater => {
                        // If we already delayed a lot, and not hit the seen
                        // count threshold, then use current minimum.
                        if delay_len > 100 {
                            return min_sample;
                        }
                    }
                    Ordering::Equal => {
                        seen_count += 1;

                        // If we've seen this min 100 times, we have high
                        // confidence this is the smallest duration.
                        if seen_count >= 100 {
                            return min_sample;
                        }
                    }
                    Ordering::Less => {
                        min_sample = sample;
                        seen_count = 0;
                    }
                }
            }

            delay_len = delay_len.saturating_add(1);
        }
    }
}

/// [`Timer`] kind.
#[derive(Clone, Copy, Default)]
pub enum TimerKind {
    /// Operating system timer.
    #[default]
    Os,

    /// CPU timestamp counter.
    Tsc,
}
