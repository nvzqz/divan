use std::{
    cell::UnsafeCell,
    fmt,
    mem::{self, MaybeUninit},
};

use crate::{
    black_box,
    stats::{Sample, Stats},
    time::{AnyTimestamp, FineDuration, Timer, Timestamp},
    util::ConfigFnMut,
};

#[cfg(test)]
mod tests;

mod defer;
mod options;

use defer::{DeferSlot, DeferStore};
pub use options::BenchOptions;

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
pub struct Bencher<'a, C = BencherConfig> {
    pub(crate) did_run: &'a mut bool,
    pub(crate) context: &'a mut Context,
    pub(crate) config: C,
}

/// Public-in-private type for statically-typed `Bencher` configuration.
///
/// This enables configuring `Bencher` using the builder pattern with zero
/// runtime cost.
#[derive(Default)]
pub struct BencherConfig<GenI = (), BeforeS = (), AfterS = ()> {
    gen_input: GenI,
    before_sample: BeforeS,
    after_sample: AfterS,
}

impl<C> fmt::Debug for Bencher<'_, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bencher").finish_non_exhaustive()
    }
}

impl<'a> Bencher<'a> {
    #[inline]
    pub(crate) fn new(did_run: &'a mut bool, context: &'a mut Context) -> Self {
        Self { did_run, context, config: BencherConfig::default() }
    }
}

impl<'a, BeforeS, AfterS> Bencher<'a, BencherConfig<(), BeforeS, AfterS>>
where
    BeforeS: ConfigFnMut,
    AfterS: ConfigFnMut,
{
    /// Benchmarks a function.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher.bench(|| {
    ///         // Benchmarked code…
    ///     });
    /// }
    /// ```
    pub fn bench<O, B>(self, mut benched: B)
    where
        B: FnMut() -> O,
    {
        // Reusing `bench_values` for a zero-sized non-drop input type should
        // have no overhead.
        self.with_inputs(|| ()).bench_values(|_: ()| benched());
    }

    /// Generate inputs for the [benchmarked function](#input-bench).
    ///
    /// Time spent generating inputs does not affect benchmark timing.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .with_inputs(|| {
    ///             // Generate input:
    ///             String::from("…")
    ///         })
    ///         .bench_values(|s| {
    ///             // Use input by-value:
    ///             s + "123"
    ///         });
    /// }
    /// ```
    pub fn with_inputs<I, G>(self, gen_input: G) -> Bencher<'a, BencherConfig<G, BeforeS, AfterS>>
    where
        G: FnMut() -> I,
    {
        Bencher {
            did_run: self.did_run,
            context: self.context,
            config: BencherConfig {
                gen_input,
                before_sample: self.config.before_sample,
                after_sample: self.config.after_sample,
            },
        }
    }
}

impl<'a, GenI, BeforeS, AfterS> Bencher<'a, BencherConfig<GenI, BeforeS, AfterS>> {
    /// Calls the given function immediately before measuring a sample timing.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .before_sample(|| {
    ///             // Prepare for the next sample…
    ///         })
    ///         .bench(|| {
    ///             // Sampled code…
    ///         });
    /// }
    /// ```
    pub fn before_sample<F>(self, before_sample: F) -> Bencher<'a, BencherConfig<GenI, F, AfterS>>
    where
        F: FnMut(),
    {
        Bencher {
            did_run: self.did_run,
            context: self.context,
            config: BencherConfig {
                gen_input: self.config.gen_input,
                before_sample,
                after_sample: self.config.after_sample,
            },
        }
    }

    /// Calls the given function immediately after measuring a sample timing.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .before_sample(|| {
    ///             // Prepare for the next sample…
    ///         })
    ///         .after_sample(|| {
    ///             // Collect info since `before_sample`…
    ///         })
    ///         .bench(|| {
    ///             // Sampled code…
    ///         });
    /// }
    /// ```
    pub fn after_sample<F>(self, after_sample: F) -> Bencher<'a, BencherConfig<GenI, BeforeS, F>>
    where
        F: FnMut(),
    {
        Bencher {
            did_run: self.did_run,
            context: self.context,
            config: BencherConfig {
                gen_input: self.config.gen_input,
                before_sample: self.config.before_sample,
                after_sample,
            },
        }
    }
}

/// <span id="input-bench"></span> Benchmark over [generated inputs](Self::with_inputs).
impl<I, GenI, BeforeS, AfterS> Bencher<'_, BencherConfig<GenI, BeforeS, AfterS>>
where
    GenI: FnMut() -> I,
    BeforeS: ConfigFnMut,
    AfterS: ConfigFnMut,
{
    /// Benchmarks a function over per-iteration [generated inputs](Self::with_inputs),
    /// provided by-value.
    ///
    /// Per-iteration means the benchmarked function is called exactly once for
    /// each generated input.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .with_inputs(|| {
    ///             // Generate input:
    ///             String::from("…")
    ///         })
    ///         .bench_values(|s| {
    ///             // Use input by-value:
    ///             s + "123"
    ///         });
    /// }
    /// ```
    pub fn bench_values<O, B>(self, mut benched: B)
    where
        B: FnMut(I) -> O,
    {
        *self.did_run = true;

        self.context.bench_loop(
            self.config,
            |input| {
                // SAFETY: Input is guaranteed to be initialized and not
                // currently referenced by anything else.
                let input = unsafe { input.get().read().assume_init() };

                benched(input)
            },
            // Input ownership is transferred to `benched`.
            |_input| {},
        );
    }

    /// Benchmarks a function over per-iteration [generated inputs](Self::with_inputs),
    /// provided by-reference.
    ///
    /// Per-iteration means the benchmarked function is called exactly once for
    /// each generated input.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .with_inputs(|| {
    ///             // Generate input:
    ///             String::from("…")
    ///         })
    ///         .bench_refs(|s| {
    ///             // Use input by-reference:
    ///             *s += "123";
    ///         });
    /// }
    /// ```
    pub fn bench_refs<O, B>(self, mut benched: B)
    where
        B: FnMut(&mut I) -> O,
    {
        // TODO: Allow `O` to reference `&mut I` as long as `I` outlives `O`.
        *self.did_run = true;

        self.context.bench_loop(
            self.config,
            |input| {
                // SAFETY: Input is guaranteed to be initialized and not
                // currently referenced by anything else.
                let input = unsafe { (*input.get()).assume_init_mut() };

                benched(input)
            },
            // Input ownership was not transferred to `benched`.
            |input| {
                // SAFETY: This function is called after `benched` outputs are
                // dropped, so we have exclusive access.
                unsafe { (*input.get()).assume_init_drop() }
            },
        );
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
    ///
    /// `min_time` and `max_time` do not consider `overhead` as benchmarking
    /// time.
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
    ///
    /// # Safety
    ///
    /// When `benched` is called:
    /// - `I` is guaranteed to be initialized.
    /// - No external `&I` or `&mut I` exists.
    ///
    /// When `drop_input` is called:
    /// - All instances of `O` returned from `benched` have been dropped.
    /// - The same guarantees for `I` apply as in `benched`, unless `benched`
    ///   escaped references to `I`.
    pub fn bench_loop<I, O>(
        &mut self,
        mut config: BencherConfig<impl FnMut() -> I, impl ConfigFnMut, impl ConfigFnMut>,
        mut benched: impl FnMut(&UnsafeCell<MaybeUninit<I>>) -> O,
        drop_input: impl Fn(&UnsafeCell<MaybeUninit<I>>),
    ) {
        // The time spent benchmarking, in picoseconds.
        //
        // Unless `skip_ext_time` is set, this includes time external to
        // `benched`, such as time spent generating inputs and running drop.
        let mut elapsed_picos: u128 = 0;

        // The minimum time for benchmarking, in picoseconds.
        let min_picos = self
            .options
            .min_time
            .map(|min_time| FineDuration::from(min_time).picos)
            .unwrap_or_default();

        // The remaining time left for benchmarking, in picoseconds.
        let max_picos = self
            .options
            .max_time
            .map(|max_time| FineDuration::from(max_time).picos)
            .unwrap_or(u128::MAX);

        // Don't bother running if 0 max time is specified.
        if max_picos == 0 {
            return;
        }

        let timer_kind = self.timer.kind();

        // Defer:
        // - Usage of `gen_input` values.
        // - Drop destructor for `O`, preventing it from affecting sample
        //   measurements. Outputs are stored into a pre-allocated buffer during
        //   the sample loop. The allocation is reused between samples to reduce
        //   time spent between samples.
        let mut defer_store: DeferStore<I, O> = DeferStore::default();

        // TODO: Set sample count and size dynamically if not set by the user.
        let mut rem_samples =
            if self.is_test { 1 } else { self.options.sample_count.unwrap_or(1_000) };

        let sample_size = if self.is_test { 1 } else { self.options.sample_size.unwrap_or(1_000) };

        if rem_samples == 0 || sample_size == 0 {
            return;
        }

        // The per-sample benchmarking overhead.
        let sample_overhead =
            FineDuration { picos: self.overhead.picos.saturating_mul(sample_size as u128) };

        if !self.is_test {
            self.samples.reserve_exact(rem_samples as usize);
        }

        let skip_ext_time = self.options.skip_ext_time.unwrap_or_default();
        let initial_start = if skip_ext_time { None } else { Some(Timestamp::start(timer_kind)) };

        while {
            // Conditions for when sampling is over:
            if elapsed_picos >= max_picos {
                // Depleted the benchmarking time budget. This is a strict
                // condition regardless of sample count and minimum time.
                false
            } else if rem_samples > 0 {
                // More samples expected.
                true
            } else {
                // Continue if we haven't reached the time floor.
                elapsed_picos < min_picos
            }
        } {
            // The following logic chooses how to efficiently sample the
            // benchmark function once and assigns `sample_start`/`sample_end`
            // before/after the sample loop.
            //
            // NOTE: Testing and benchmarking should behave exactly the same
            // when getting the sample time span. We don't want to introduce
            // extra work that may worsen measurement quality for real
            // benchmarking.
            let sample_start: AnyTimestamp;
            let sample_end: AnyTimestamp;

            if (mem::size_of::<I>() == 0 && mem::size_of::<O>() == 0)
                || (mem::size_of::<I>() == 0 && !mem::needs_drop::<O>())
            {
                // Use a range instead of `defer_store` to make the benchmarking
                // loop cheaper.

                // Run `gen_input` the expected number of times in case it
                // updates external state used by `benched`. We `mem::forget`
                // here because inputs are consumed/dropped later.
                for _ in 0..sample_size {
                    mem::forget((config.gen_input)());
                }

                config.before_sample.call_mut();
                sample_start = AnyTimestamp::start(timer_kind);

                // Sample loop:
                for _ in 0..sample_size {
                    // SAFETY: Input is a ZST, so we can construct one out of
                    // thin air.
                    let input = unsafe { UnsafeCell::new(MaybeUninit::<I>::zeroed()) };

                    mem::forget(black_box(benched(&input)));
                }

                sample_end = AnyTimestamp::end(timer_kind);
                config.after_sample.call_mut();

                // Drop outputs and inputs.
                for _ in 0..sample_size {
                    // Output only needs drop if ZST.
                    if mem::size_of::<O>() == 0 {
                        // SAFETY: Output is a ZST, so we can construct one out
                        // of thin air.
                        unsafe { _ = mem::zeroed::<O>() }
                    }

                    if mem::needs_drop::<I>() {
                        // SAFETY: Input is a ZST, so we can construct one out
                        // of thin air and not worry about aliasing.
                        unsafe { drop_input(&UnsafeCell::new(MaybeUninit::<I>::zeroed())) }
                    }
                }
            } else {
                defer_store.prepare(sample_size as usize);

                match defer_store.slots() {
                    // Output needs to be dropped. We defer drop in the sample
                    // loop by inserting it into `defer_store`.
                    Ok(defer_slots_slice) => {
                        // Initialize and store inputs.
                        for DeferSlot { input, .. } in defer_slots_slice {
                            // SAFETY: We have exclusive access to `input`.
                            let input = unsafe { &mut *input.get() };

                            *input = MaybeUninit::new((config.gen_input)());
                        }

                        // Create iterator before the sample timing section to
                        // reduce benchmarking overhead.
                        let defer_slots_iter = black_box(defer_slots_slice.iter());

                        config.before_sample.call_mut();
                        sample_start = AnyTimestamp::start(timer_kind);

                        // Sample loop:
                        for defer_slot in defer_slots_iter {
                            // SAFETY: All inputs in `defer_store` were
                            // initialized and we have exclusive access to the
                            // output slot.
                            unsafe {
                                let output = benched(&defer_slot.input);
                                *defer_slot.output.get() = MaybeUninit::new(output);
                            }

                            // PERF: `black_box` the slot address because:
                            // - It prevents `input` mutation from being
                            //   optimized out.
                            // - `black_box` writes its input to the stack.
                            //   Using the slot address instead of the output
                            //   by-value reduces overhead when `O` is a larger
                            //   type like `String` since then it will write a
                            //   single word instead of three words.
                            _ = black_box(defer_slot);
                        }

                        sample_end = AnyTimestamp::end(timer_kind);
                        config.after_sample.call_mut();

                        // Drop outputs and inputs.
                        for DeferSlot { input, output } in defer_slots_slice {
                            // SAFETY: All outputs were initialized in the
                            // sample loop and we have exclusive access.
                            unsafe { (*output.get()).assume_init_drop() }

                            if mem::needs_drop::<I>() {
                                // SAFETY: The output was dropped and thus we
                                // have exclusive access to inputs.
                                unsafe { drop_input(input) }
                            }
                        }
                    }

                    // Output does not need to be dropped.
                    Err(defer_inputs_slice) => {
                        // Initialize and store inputs.
                        for input in defer_inputs_slice {
                            // SAFETY: We have exclusive access to `input`.
                            let input = unsafe { &mut *input.get() };

                            *input = MaybeUninit::new((config.gen_input)());
                        }

                        // Create iterator before the sample timing section to
                        // reduce benchmarking overhead.
                        let defer_inputs_iter = black_box(defer_inputs_slice.iter());

                        config.before_sample.call_mut();
                        sample_start = AnyTimestamp::start(timer_kind);

                        // Sample loop:
                        for input in defer_inputs_iter {
                            // SAFETY: All inputs in `defer_store` were
                            // initialized.
                            _ = black_box(unsafe { benched(input) });
                        }

                        sample_end = AnyTimestamp::end(timer_kind);
                        config.after_sample.call_mut();

                        // Drop inputs.
                        if mem::needs_drop::<I>() {
                            for input in defer_inputs_slice {
                                // SAFETY: We have exclusive access to inputs.
                                unsafe { drop_input(input) }
                            }
                        }
                    }
                }
            }

            // If testing, exit the benchmarking loop immediately after timing a
            // single run.
            if self.is_test {
                break;
            }

            // SAFETY: These values are guaranteed to be the correct variant
            // because they were created from the same `timer_kind`.
            let [sample_start, sample_end] = unsafe {
                [sample_start.into_timestamp(timer_kind), sample_end.into_timestamp(timer_kind)]
            };

            let raw_duration = sample_end.duration_since(sample_start, self.timer);

            // Account for the per-sample benchmarking overhead.
            let adjusted_duration =
                FineDuration { picos: raw_duration.picos.saturating_sub(sample_overhead.picos) };

            self.samples.push(Sample {
                start: sample_start,
                end: sample_end,
                size: sample_size,
                total_duration: adjusted_duration,
            });

            rem_samples = rem_samples.saturating_sub(1);

            if let Some(initial_start) = initial_start {
                elapsed_picos = sample_end.duration_since(initial_start, self.timer).picos;
            } else {
                // Progress by at least 1ns to prevent extremely fast
                // functions from taking forever when `min_time` is set.
                let progress_picos = raw_duration.picos.max(1_000);
                elapsed_picos = elapsed_picos.saturating_add(progress_picos);
            }
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

    let sample_count: usize = 100;
    let sample_size: usize = 10_000;

    // The minimum non-zero sample.
    let mut min_sample = FineDuration::default();

    for _ in 0..sample_count {
        let start = AnyTimestamp::start(timer_kind);

        for i in 0..sample_size {
            _ = black_box(i);
        }

        let end = AnyTimestamp::end(timer_kind);

        // SAFETY: These values are guaranteed to be the correct variant because
        // they were created from the same `timer_kind`.
        let [start, end] =
            unsafe { [start.into_timestamp(timer_kind), end.into_timestamp(timer_kind)] };

        let mut sample = end.duration_since(start, timer);
        sample.picos /= sample_size as u128;

        if min_sample.picos == 0 {
            min_sample = sample;
        } else if sample.picos > 0 {
            min_sample = min_sample.min(sample);
        }
    }

    min_sample
}
