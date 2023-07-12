use std::{fmt, mem::MaybeUninit};

use crate::{
    black_box,
    config::Timer,
    stats::{Sample, Stats},
    time::{fence, AnyTimestamp, FineDuration, Tsc},
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

/// Options set directly by the user in `#[divan::bench]`.
///
/// Changes to fields must be reflected in the "Options" section in the
/// `#[divan::bench]` documentation.
#[derive(Default)]
pub struct BenchOptions {
    /// The number of sample recordings.
    pub sample_count: Option<u32>,

    /// The number of iterations inside a single sample.
    pub sample_size: Option<u32>,
}

/// `#[divan::bench]` loop context.
///
/// Functions called within the benchmark loop should be `#[inline(always)]` to
/// ensure instruction cache locality.
pub(crate) struct Context {
    /// Whether the benchmark is being run as `--test`.
    ///
    /// When `true`, the benchmark is run exactly once. To achieve this, sample
    /// count and size are each set to 1.
    is_test: bool,

    tsc_frequency: u64,

    /// User-configured options.
    pub options: BenchOptions,

    /// Recorded samples.
    samples: Vec<Sample>,
}

impl Context {
    /// Creates a new benchmarking context.
    pub fn new(is_test: bool, timer: Timer, options: BenchOptions) -> Self {
        let tsc_frequency = match timer {
            // TODO: Report `None` and `Some(0)` TSC frequency.
            Timer::Tsc => Tsc::frequency().unwrap_or_default(),
            Timer::Os => 0,
        };
        Self { is_test, tsc_frequency, options, samples: Vec::new() }
    }

    /// Runs the loop for benchmarking `f`.
    pub fn bench_loop<R>(&mut self, mut f: impl FnMut() -> R) {
        let tsc_frequency = self.tsc_frequency;
        let use_tsc = tsc_frequency != 0;

        // `drop_store` prevents any drop destructor for `R` from affecting
        // sample measurements. It defers `Drop` by storing instances within a
        // pre-allocated buffer during the sample loop. The allocation is reused
        // between samples to reduce time spent between samples.
        let mut drop_store = Vec::<R>::new();

        // TODO: Set sample count and size dynamically if not set by the user.
        let sample_count =
            if self.is_test { 1 } else { self.options.sample_count.unwrap_or(1_000) };

        let sample_size = if self.is_test { 1 } else { self.options.sample_size.unwrap_or(1_000) };

        if sample_count == 0 || sample_size == 0 {
            return;
        }

        self.samples.reserve_exact(sample_count as usize);

        // NOTE: Aside from handling sample count and size, testing and
        // benchmarking should behave exactly the same since we don't want to
        // introduce extra work in benchmarks just to handle tests. Doing so may
        // worsen measurement quality for real benchmarking.
        for _ in 0..sample_count {
            // If `R` needs to be dropped, we defer drop in the sample loop by
            // inserting it into `drop_store`. Otherwise, we just loop up to
            // `sample_size`.
            let [start, end] = if std::mem::needs_drop::<R>() {
                // Drop values from the previous sample.
                drop_store.clear();

                // The sample loop below is over `sample_size` number of slots
                // of pre-allocated memory in `drop_store`.
                drop_store.reserve_exact(sample_size as usize);
                let drop_slots = drop_store.spare_capacity_mut()[..sample_size as usize].iter_mut();

                fence::full_fence();
                let start = AnyTimestamp::now(use_tsc);
                fence::compiler_fence();

                // Sample loop:
                for drop_slot in drop_slots {
                    *drop_slot = MaybeUninit::new(f());

                    // PERF: We `black_box` the result's slot address instead of
                    // the result by-value because `black_box` currently writes
                    // its input to the stack. Using the slot address reduces
                    // overhead when `R` is a larger type like `String` since
                    // then it will write a single word instead of three words.
                    _ = black_box(drop_slot);
                }

                fence::compiler_fence();
                let end = AnyTimestamp::now(use_tsc);
                fence::full_fence();

                // Increase length to mark stored values as initialized so that
                // they can be dropped.
                //
                // SAFETY: All values were initialized in the sample loop.
                unsafe { drop_store.set_len(sample_size as usize) };

                [start, end]
            } else {
                fence::full_fence();
                let start = AnyTimestamp::now(use_tsc);
                fence::compiler_fence();

                // Sample loop:
                for _ in 0..sample_size {
                    _ = black_box(f());
                }

                fence::compiler_fence();
                let end = AnyTimestamp::now(use_tsc);
                fence::full_fence();

                [start, end]
            };

            // SAFETY: These values are guaranteed to be the correct variant
            // because they were created from the same `use_tsc`.
            let [start, end] =
                unsafe { [start.into_timestamp(use_tsc), end.into_timestamp(use_tsc)] };

            self.samples.push(Sample {
                start,
                end,
                size: sample_size,
                total_duration: end.duration_since(start, tsc_frequency),
            });
        }
    }

    /// Computes the total iteration count and duration.
    ///
    /// We use `u64` for total count in case sample count and sizes are huge.
    fn compute_totals(&self) -> (u64, FineDuration) {
        self.samples.iter().fold(Default::default(), |(mut count, mut duration), sample| {
            count += sample.size as u64;
            duration.picos += sample.total_duration.picos;
            (count, duration)
        })
    }

    pub fn compute_stats(&self) -> Stats {
        let sample_count = self.samples.len();
        let (total_count, total_duration) = self.compute_totals();

        // Samples ordered by each average duration.
        let mut ordered_samples: Vec<&Sample> = self.samples.iter().collect();
        ordered_samples.sort_unstable_by_key(|s| s.avg_duration());

        let avg_duration = FineDuration {
            picos: total_duration.picos.checked_div(total_count as u128).unwrap_or_default(),
        };

        let min_duration = ordered_samples.first().map(|s| s.avg_duration()).unwrap_or_default();
        let max_duration = ordered_samples.last().map(|s| s.avg_duration()).unwrap_or_default();

        let median_duration = if sample_count == 0 {
            FineDuration::default()
        } else if sample_count % 2 == 0 {
            // Take average of two middle numbers.
            let s1 = ordered_samples[sample_count / 2];
            let s2 = ordered_samples[(sample_count / 2) - 1];
            s1.avg_duration_between(s2)
        } else {
            // Single middle number.
            ordered_samples[sample_count / 2].avg_duration()
        };

        Stats {
            sample_count: sample_count as u32,
            total_count,
            total_duration,
            avg_duration,
            min_duration,
            max_duration,
            median_duration,
        }
    }
}
