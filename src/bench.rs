use std::{
    fmt,
    mem::{self, MaybeUninit},
};

use crate::{
    black_box,
    defer::{DeferEntry, DeferStore},
    stats::{Sample, Stats},
    time::{fence, AnyTimestamp, FineDuration, Timer},
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
    /// Benchmarks a function.
    pub fn bench<O, B>(self, mut benched: B)
    where
        B: FnMut() -> O,
    {
        // Reusing `bench_with_values` for a zero-sized non-drop input type
        // should have no overhead.
        self.bench_with_values(|| (), |()| benched());
    }

    /// Benchmarks a function over per-iteration generated inputs, provided
    /// by-value.
    ///
    /// Per-iteration means the benchmarked function is called exactly once for
    /// each generated input.
    ///
    /// Time spent generating inputs does not affect benchmark timing.
    pub fn bench_with_values<I, O, G, B>(self, gen_input: G, benched: B)
    where
        G: FnMut() -> I,
        B: FnMut(I) -> O,
    {
        *self.did_run = true;
        self.context.bench_loop(gen_input, benched);
    }

    /// Benchmarks a function over per-iteration generated inputs, provided
    /// by-reference.
    ///
    /// Per-iteration means the benchmarked function is called exactly once for
    /// each generated input.
    ///
    /// Time spent generating inputs does not affect benchmark timing.
    pub fn bench_with_refs<I, O, G, B>(self, gen_input: G, mut benched: B)
    where
        G: FnMut() -> I,
        B: FnMut(&mut I) -> O,
    {
        // TODO: Make this more efficient by referencing the inputs buffer and
        // not moving inputs out of it. This should also allow `O` to safely
        // reference `&mut I` as long as `I` outlives `O`.
        self.bench_with_values(gen_input, |mut input| {
            let output = benched(&mut input);
            (input, output)
        });
    }
}

/// Benchmarking options set directly by the user in `#[divan::bench]` and
/// `#[divan::bench_group]`.
///
/// Changes to fields must be reflected in the "Options" sections of the docs
/// for `#[divan::bench]` and `#[divan::bench_group]`.
#[derive(Clone, Default)]
pub struct BenchOptions {
    /// The number of sample recordings.
    pub sample_count: Option<u32>,

    /// The number of iterations inside a single sample.
    pub sample_size: Option<u32>,
}

impl BenchOptions {
    /// Overwrites `other` with values set in `self`.
    #[must_use]
    pub(crate) fn overwrite(self, other: &Self) -> Self {
        Self {
            sample_count: self.sample_count.or(other.sample_count),
            sample_size: self.sample_size.or(other.sample_size),
        }
    }
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

    timer: Timer,

    /// Per-iteration overhead.
    overhead: FineDuration,

    /// User-configured options.
    pub options: BenchOptions,

    /// Recorded samples.
    samples: Vec<Sample>,
}

impl Context {
    /// Creates a new benchmarking context.
    pub fn new(is_test: bool, timer: Timer, overhead: FineDuration, options: BenchOptions) -> Self {
        Self { is_test, timer, overhead, options, samples: Vec::new() }
    }

    /// Runs the loop for benchmarking `benched`.
    pub fn bench_loop<I, O>(
        &mut self,
        mut gen_input: impl FnMut() -> I,
        mut benched: impl FnMut(I) -> O,
    ) {
        let timer_kind = self.timer.kind();
        let tsc_frequency = self.timer.tsc_frequency().unwrap_or_default();

        // Defer:
        // - Usage of `gen_input` values.
        // - Drop destructor for `O`, preventing it from affecting sample
        //   measurements. Outputs are stored into a pre-allocated buffer during
        //   the sample loop. The allocation is reused between samples to reduce
        //   time spent between samples.
        let mut defer_store: DeferStore<I, O> = DeferStore::default();

        // TODO: Set sample count and size dynamically if not set by the user.
        let sample_count =
            if self.is_test { 1 } else { self.options.sample_count.unwrap_or(1_000) };

        let sample_size = if self.is_test { 1 } else { self.options.sample_size.unwrap_or(1_000) };

        if sample_count == 0 || sample_size == 0 {
            return;
        }

        // The per-sample benchmarking overhead.
        let sample_overhead =
            FineDuration { picos: self.overhead.picos.saturating_mul(sample_size as u128) };

        self.samples.reserve_exact(sample_count as usize);

        // NOTE: Aside from handling sample count and size, testing and
        // benchmarking should behave exactly the same since we don't want to
        // introduce extra work in benchmarks just to handle tests. Doing so may
        // worsen measurement quality for real benchmarking.
        for _ in 0..sample_count {
            let start: AnyTimestamp;
            let end: AnyTimestamp;

            if mem::size_of::<I>() == 0 && !mem::needs_drop::<O>() {
                // If inputs are ZSTs and outputs do not need to be dropped, we
                // can skip using `defer_store` altogether. This makes the
                // benchmarking loop cheaper for e.g. `Bencher::bench`, which
                // uses `()` inputs.

                // Run `gen_input` the expected number of times in case it
                // updates external state used by `benched`. We `mem::forget`
                // here because inputs are consumed/dropped later by `benched`.
                for _ in 0..sample_size {
                    mem::forget(gen_input());
                }

                fence::full_fence();
                start = AnyTimestamp::now(timer_kind);
                fence::compiler_fence();

                // Sample loop:
                for _ in 0..sample_size {
                    // SAFETY: Input is a ZST, so we can construct one out of
                    // thin air.
                    let input: I = unsafe { mem::zeroed() };

                    _ = black_box(benched(input));
                }

                fence::compiler_fence();
                end = AnyTimestamp::now(timer_kind);
                fence::full_fence();
            } else {
                let defer_entries = defer_store.prepare(sample_size as usize);

                // Initialize and store inputs.
                for entry in &mut *defer_entries {
                    entry.input = MaybeUninit::new(gen_input());
                }

                // Create iterator before the sample timing section to reduce
                // benchmarking overhead.
                let defer_entries = black_box(defer_entries.iter_mut());

                if mem::needs_drop::<O>() {
                    // If output needs to be dropped, we defer drop in the
                    // sample loop by inserting it into `defer_entries`.

                    fence::full_fence();
                    start = AnyTimestamp::now(timer_kind);
                    fence::compiler_fence();

                    // Sample loop:
                    for DeferEntry { input, output } in defer_entries {
                        // SAFETY: All inputs in `defer_store` were initialized.
                        let input = unsafe { input.assume_init_read() };

                        *output = MaybeUninit::new(benched(input));

                        // PERF: We `black_box` the output's slot address
                        // instead of the result by-value because `black_box`
                        // currently writes its input to the stack. Using the
                        // slot address reduces overhead when `O` is a larger
                        // type like `String` since then it will write a single
                        // word instead of three words.
                        _ = black_box(output);
                    }

                    fence::compiler_fence();
                    end = AnyTimestamp::now(timer_kind);
                    fence::full_fence();

                    // SAFETY: All outputs were initialized in the sample loop.
                    unsafe { defer_store.drop_outputs() };
                } else {
                    // Outputs do not need to have deferred drop, but inputs
                    // still need to be retrieved from `defer_entries`.

                    fence::full_fence();
                    start = AnyTimestamp::now(timer_kind);
                    fence::compiler_fence();

                    // Sample loop:
                    for DeferEntry { input, .. } in defer_entries {
                        // SAFETY: All inputs in `defer_store` were initialized.
                        let input = unsafe { input.assume_init_read() };

                        _ = black_box(benched(input));
                    }

                    fence::compiler_fence();
                    end = AnyTimestamp::now(timer_kind);
                    fence::full_fence();
                }
            }

            // SAFETY: These values are guaranteed to be the correct variant
            // because they were created from the same `timer_kind`.
            let [start, end] =
                unsafe { [start.into_timestamp(timer_kind), end.into_timestamp(timer_kind)] };

            let raw_duration = end.duration_since(start, tsc_frequency);

            // Account for the per-sample benchmarking overhead.
            let adjusted_duration =
                FineDuration { picos: raw_duration.picos.saturating_sub(sample_overhead.picos) };

            self.samples.push(Sample {
                start,
                end,
                size: sample_size,
                total_duration: adjusted_duration,
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

/// Attempts to calculate the benchmarking loop overhead.
pub fn measure_overhead(timer: Timer) -> FineDuration {
    let timer_kind = timer.kind();
    let tsc_frequency = timer.tsc_frequency().unwrap_or_default();

    let sample_count: usize = 100;
    let sample_size: usize = 10_000;

    // The minimum non-zero sample.
    let mut min_sample = FineDuration::default();

    for _ in 0..sample_count {
        fence::full_fence();
        let start = AnyTimestamp::now(timer_kind);
        fence::compiler_fence();

        for i in 0..sample_size {
            _ = black_box(i);
        }

        fence::compiler_fence();
        let end = AnyTimestamp::now(timer_kind);
        fence::full_fence();

        // SAFETY: These values are guaranteed to be the correct variant because
        // they were created from the same `timer_kind`.
        let [start, end] =
            unsafe { [start.into_timestamp(timer_kind), end.into_timestamp(timer_kind)] };

        let mut sample = end.duration_since(start, tsc_frequency);
        sample.picos /= sample_size as u128;

        if min_sample.picos == 0 {
            min_sample = sample;
        } else if sample.picos > 0 {
            min_sample = min_sample.min(sample);
        }
    }

    min_sample
}

/// Tests every benchmarking loop combination in `Bencher`. When run under Miri,
/// this catches memory leaks and UB in `unsafe` code.
#[cfg(test)]
mod tests {
    use super::*;

    // We use a small number of runs because Miri is very slow.
    const SAMPLE_COUNT: u32 = 5;
    const SAMPLE_SIZE: u32 = 5;

    #[track_caller]
    fn test_bencher(mut test: impl FnMut(Bencher<'_>)) {
        let timers = match Timer::get_tsc() {
            Some(tsc) => vec![Timer::Os, tsc],
            None => vec![Timer::Os],
        };

        for is_test in [true, false] {
            for &timer in &timers {
                let mut did_run = false;
                test(Bencher {
                    did_run: &mut did_run,
                    context: &mut Context::new(
                        is_test,
                        timer,
                        FineDuration::default(),
                        BenchOptions {
                            sample_count: Some(SAMPLE_COUNT),
                            sample_size: Some(SAMPLE_SIZE),
                        },
                    ),
                });
                assert!(did_run);
            }
        }
    }

    fn make_string() -> String {
        ('a'..='z').collect()
    }

    mod string_input {
        use super::*;

        #[test]
        fn string_output() {
            test_bencher(|b| b.bench_with_values(make_string, |s| s.to_ascii_uppercase()));
        }

        #[test]
        fn no_output() {
            test_bencher(|b| b.bench_with_refs(make_string, |s| s.make_ascii_uppercase()));
        }
    }

    mod no_input {
        use super::*;

        #[test]
        fn string_output() {
            test_bencher(|b| b.bench(make_string));
        }

        #[test]
        fn no_output() {
            test_bencher(|b| b.bench(|| _ = black_box(make_string())));
        }
    }
}
