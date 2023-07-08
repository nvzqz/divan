use std::{fmt, mem::MaybeUninit, time::Instant};

use crate::{
    black_box,
    stats::{Sample, Stats},
    time::SmallDuration,
};

/// Enables contextual benchmarking in [`#[divan::bench]`](attr.bench.html).
///
/// # Examples
///
/// ```
/// use divan::{Bencher, black_box};
///
/// #[divan::bench]
/// fn copy_from_slice(bencher: Bencher) {
///     // Input and output buffers get used in the closure.
///     let src = (0..100).collect::<Vec<i32>>();
///     let mut dst = vec![0; src.len()];
///
///     bencher.bench(|| {
///         black_box(&mut dst).copy_from_slice(black_box(&src));
///     });
/// }
/// ```
#[must_use = "a benchmark function must be registered"]
pub struct Bencher<'a> {
    pub(crate) did_run: &'a mut bool,
    pub(crate) context: &'a mut Context,
}

impl fmt::Debug for Bencher<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bencher").finish_non_exhaustive()
    }
}

impl Bencher<'_> {
    /// Benchmarks the given function.
    pub fn bench<R>(self, f: impl FnMut() -> R) {
        *self.did_run = true;
        self.context.bench_loop(f);
    }
}

/// `#[divan::bench]` loop context.
///
/// Functions called within the benchmark loop should be `#[inline(always)]` to
/// ensure instruction cache locality.
///
/// Instances of this type are publicly accessible to generated code, so care
/// should be taken when making fields and methods fully public.
pub struct Context {
    /// Recorded samples.
    samples: Vec<Sample>,

    /// The number of iterations between recording samples.
    sample_size: u32,
}

impl Context {
    /// Creates a context for actual benchmarking.
    pub(crate) fn bench() -> Self {
        Self {
            // TODO: Pick these numbers dynamically.
            samples: Vec::with_capacity(1_000),
            sample_size: 1_000,
        }
    }

    /// Creates a context for testing benchmarked functions.
    pub(crate) fn test() -> Self {
        Self { samples: Vec::with_capacity(1), sample_size: 1 }
    }

    /// Runs the loop for benchmarking `f`.
    pub fn bench_loop<R>(&mut self, mut f: impl FnMut() -> R) {
        // `drop_store` prevents any drop destructor for `R` from affecting
        // sample measurements. It defers `Drop` by storing instances within a
        // pre-allocated buffer during the sample loop. The allocation is reused
        // between samples to reduce time spent between samples.
        let mut drop_store = Vec::<R>::new();

        // TODO: Set sample count and size dynamically.
        for _ in 0..self.target_sample_count() {
            let sample_size = self.sample_size as usize;

            // If `R` needs to be dropped, we defer drop in the sample loop by
            // inserting it into `drop_store`. Otherwise, we just loop up to
            // `sample_size`.
            if std::mem::needs_drop::<R>() {
                // Drop values from the previous sample.
                drop_store.clear();

                // The sample loop below is over `sample_size` number of slots
                // of pre-allocated memory in `drop_store`.
                drop_store.reserve_exact(sample_size);
                let drop_slots = drop_store.spare_capacity_mut()[..sample_size].iter_mut();

                // Sample loop:
                let start = self.start_sample();
                for drop_slot in drop_slots {
                    *drop_slot = MaybeUninit::new(black_box(f()));
                }
                self.end_sample(start);

                // Increase length to mark stored values as initialized so that
                // they can be dropped.
                //
                // SAFETY: All values were initialized in the sample loop.
                unsafe { drop_store.set_len(sample_size) };
            } else {
                // Sample loop:
                let start = self.start_sample();
                for _ in 0..sample_size {
                    _ = black_box(f());
                }
                self.end_sample(start);
            }
        }
    }

    /// Returns the number of samples that should be taken.
    #[inline(always)]
    fn target_sample_count(&self) -> usize {
        self.samples.capacity()
    }

    /// Begins info measurement at the start of a loop.
    #[inline(always)]
    fn start_sample(&self) -> Instant {
        // Prevent other operations from affecting timing measurements.
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        Instant::now()
    }

    /// Records measurement info at the end of a loop.
    #[inline(always)]
    fn end_sample(&mut self, start: Instant) {
        let end = Instant::now();

        // Prevent other operations from affecting timing measurements.
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        self.samples.push(Sample { start, end });
    }

    pub(crate) fn compute_stats(&self) -> Option<Stats> {
        let sample_count = self.samples.len();
        let total_count = sample_count * self.sample_size as usize;

        let first = self.samples.first()?;
        let last = self.samples.last()?;

        let total_duration = last.end.duration_since(first.start);
        let avg_duration = SmallDuration::average(total_duration, total_count as u128);

        let mut all_durations: Vec<SmallDuration> = self
            .samples
            .iter()
            .map(|sample| {
                SmallDuration::average(
                    sample.end.duration_since(sample.start),
                    self.sample_size as u128,
                )
            })
            .collect();

        all_durations.sort_unstable();

        let min_duration = *all_durations.first().unwrap();
        let max_duration = *all_durations.last().unwrap();

        let median_duration = if sample_count % 2 == 0 {
            // Take average of two middle numbers.
            let a = all_durations[sample_count / 2];
            let b = all_durations[(sample_count / 2) - 1];

            SmallDuration { picos: (a.picos + b.picos) / 2 }
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
