use std::{
    cell::UnsafeCell,
    fmt,
    mem::{self, MaybeUninit},
};

use crate::{
    black_box,
    counter::{self, AnyCounter, IntoCounter},
    divan::SharedContext,
    stats::{Sample, SampleCollection, Stats},
    time::{FineDuration, Timestamp, UntaggedTimestamp},
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
pub struct Bencher<'a, 'b, C = BencherConfig> {
    pub(crate) context: &'a mut BenchContext<'b>,
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

impl<C> fmt::Debug for Bencher<'_, '_, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bencher").finish_non_exhaustive()
    }
}

impl<'a, 'b> Bencher<'a, 'b> {
    #[inline]
    pub(crate) fn new(context: &'a mut BenchContext<'b>) -> Self {
        Self { context, config: BencherConfig::default() }
    }
}

impl<'a, 'b, BeforeS, AfterS> Bencher<'a, 'b, BencherConfig<(), BeforeS, AfterS>>
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
    ///         // Benchmarked code...
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
    ///             String::from("...")
    ///         })
    ///         .bench_values(|s| {
    ///             // Use input by-value:
    ///             s + "123"
    ///         });
    /// }
    /// ```
    pub fn with_inputs<I, G>(
        self,
        gen_input: G,
    ) -> Bencher<'a, 'b, BencherConfig<G, BeforeS, AfterS>>
    where
        G: FnMut() -> I,
    {
        Bencher {
            context: self.context,
            config: BencherConfig {
                gen_input,
                before_sample: self.config.before_sample,
                after_sample: self.config.after_sample,
            },
        }
    }
}

impl<'a, 'b, GenI, BeforeS, AfterS> Bencher<'a, 'b, BencherConfig<GenI, BeforeS, AfterS>> {
    /// Assign a [`Counter`](crate::counter::Counter) for all iterations of the
    /// benchmarked function.
    ///
    /// If context is not needed, the counter can instead be set via
    /// [`#[divan::bench(counter = ...)]`](macro@crate::bench#counter).
    ///
    /// # Examples
    ///
    /// ```
    /// use divan::{Bencher, counter::Bytes};
    ///
    /// #[divan::bench]
    /// fn char_count(bencher: Bencher) {
    ///     let s: String = // ...
    ///     # String::new();
    ///
    ///     bencher
    ///         .counter(Bytes(s.len()))
    ///         .bench(|| {
    ///             divan::black_box(&s).chars().count()
    ///         });
    /// }
    /// ```
    #[doc(alias = "throughput")]
    pub fn counter<C>(self, counter: C) -> Self
    where
        C: IntoCounter,
    {
        self.context.counter = Some(counter::Sealed::into_any_counter(counter.into_counter()));
        self
    }

    /// Calls the given function immediately before measuring a sample timing.
    ///
    /// # Examples
    ///
    /// ```
    /// #[divan::bench]
    /// fn bench(bencher: divan::Bencher) {
    ///     bencher
    ///         .before_sample(|| {
    ///             // Prepare for the next sample...
    ///         })
    ///         .bench(|| {
    ///             // Sampled code...
    ///         });
    /// }
    /// ```
    pub fn before_sample<F>(
        self,
        before_sample: F,
    ) -> Bencher<'a, 'b, BencherConfig<GenI, F, AfterS>>
    where
        F: FnMut(),
    {
        Bencher {
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
    ///             // Prepare for the next sample...
    ///         })
    ///         .after_sample(|| {
    ///             // Collect info since `before_sample`...
    ///         })
    ///         .bench(|| {
    ///             // Sampled code...
    ///         });
    /// }
    /// ```
    pub fn after_sample<F>(
        self,
        after_sample: F,
    ) -> Bencher<'a, 'b, BencherConfig<GenI, BeforeS, F>>
    where
        F: FnMut(),
    {
        Bencher {
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
impl<I, GenI, BeforeS, AfterS> Bencher<'_, '_, BencherConfig<GenI, BeforeS, AfterS>>
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
    ///             String::from("...")
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
    ///             String::from("...")
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

/// State machine for how the benchmark is being run.
#[derive(Clone, Copy)]
pub(crate) enum BenchMode {
    /// The benchmark is being run as `--test`.
    ///
    /// Don't collect samples and run exactly once.
    Test,

    /// Scale `sample_size` to determine the right size for collecting.
    Tune { sample_size: u32 },

    /// Simply collect samples.
    Collect { sample_size: u32 },
}

impl BenchMode {
    #[inline]
    pub fn is_test(self) -> bool {
        matches!(self, Self::Test)
    }

    #[inline]
    pub fn is_tune(self) -> bool {
        matches!(self, Self::Tune { .. })
    }

    #[inline]
    pub fn is_collect(self) -> bool {
        matches!(self, Self::Collect { .. })
    }

    #[inline]
    pub fn sample_size(self) -> u32 {
        match self {
            Self::Test => 1,
            Self::Tune { sample_size, .. } | Self::Collect { sample_size, .. } => sample_size,
        }
    }
}

/// `#[divan::bench]` loop context.
///
/// Functions called within the benchmark loop should be `#[inline(always)]` to
/// ensure instruction cache locality.
pub(crate) struct BenchContext<'a> {
    shared_context: &'a SharedContext,

    /// User-configured options.
    options: &'a BenchOptions,

    /// Whether the benchmark loop was started.
    pub did_run: bool,

    /// Counter set by `Bencher::counter`.
    ///
    /// If `None`, fallback to `BenchOptions.counter`.
    counter: Option<AnyCounter>,

    /// Recorded samples.
    samples: SampleCollection,
}

impl<'a> BenchContext<'a> {
    /// Creates a new benchmarking context.
    pub fn new(shared_context: &'a SharedContext, options: &'a BenchOptions) -> Self {
        Self {
            shared_context,
            options,
            did_run: false,
            counter: None,
            samples: SampleCollection::default(),
        }
    }

    /// Returns the per-iteration counter.
    #[inline]
    fn counter(&self) -> Option<&AnyCounter> {
        self.counter.as_ref().or(self.options.counter.as_ref())
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
        const DEFAULT_SAMPLE_COUNT: u32 = 100;

        self.did_run = true;

        let mut current_mode = self.initial_mode();
        let is_test = current_mode.is_test();

        // The time spent benchmarking, in picoseconds.
        //
        // Unless `skip_ext_time` is set, this includes time external to
        // `benched`, such as time spent generating inputs and running drop.
        let mut elapsed_picos: u128 = 0;

        // The minimum time for benchmarking, in picoseconds.
        let min_picos = self.options.min_time().picos;

        // The remaining time left for benchmarking, in picoseconds.
        let max_picos = self.options.max_time().picos;

        // Don't bother running if user specifies 0 max time or 0 samples.
        if max_picos == 0 || !self.options.has_samples() {
            return;
        }

        let timer = self.shared_context.timer;
        let timer_kind = timer.kind();

        // Defer:
        // - Usage of `gen_input` values.
        // - Drop destructor for `O`, preventing it from affecting sample
        //   measurements. Outputs are stored into a pre-allocated buffer during
        //   the sample loop. The allocation is reused between samples to reduce
        //   time spent between samples.
        let mut defer_store: DeferStore<I, O> = DeferStore::default();

        let mut rem_samples = if current_mode.is_collect() {
            Some(self.options.sample_count.unwrap_or(DEFAULT_SAMPLE_COUNT))
        } else {
            None
        };

        // Only measure precision if we need to tune sample size.
        let timer_precision =
            if current_mode.is_tune() { timer.precision() } else { FineDuration::default() };

        if !is_test {
            self.samples.all.reserve(self.options.sample_count.unwrap_or(1) as usize);
        }

        let skip_ext_time = self.options.skip_ext_time.unwrap_or_default();
        let initial_start = if skip_ext_time { None } else { Some(Timestamp::start(timer_kind)) };

        while {
            // Conditions for when sampling is over:
            if elapsed_picos >= max_picos {
                // Depleted the benchmarking time budget. This is a strict
                // condition regardless of sample count and minimum time.
                false
            } else if rem_samples.unwrap_or(1) > 0 {
                // More samples expected.
                true
            } else {
                // Continue if we haven't reached the time floor.
                elapsed_picos < min_picos
            }
        } {
            let sample_size = current_mode.sample_size();
            self.samples.sample_size = sample_size;

            // The following logic chooses how to efficiently sample the
            // benchmark function once and assigns `sample_start`/`sample_end`
            // before/after the sample loop.
            //
            // NOTE: Testing and benchmarking should behave exactly the same
            // when getting the sample time span. We don't want to introduce
            // extra work that may worsen measurement quality for real
            // benchmarking.
            let sample_start: UntaggedTimestamp;
            let sample_end: UntaggedTimestamp;

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
                sample_start = UntaggedTimestamp::start(timer_kind);

                // Sample loop:
                for _ in 0..sample_size {
                    // SAFETY: Input is a ZST, so we can construct one out of
                    // thin air.
                    let input = unsafe { UnsafeCell::new(MaybeUninit::<I>::zeroed()) };

                    mem::forget(black_box(benched(&input)));
                }

                sample_end = UntaggedTimestamp::end(timer_kind);
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
                        sample_start = UntaggedTimestamp::start(timer_kind);

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

                        sample_end = UntaggedTimestamp::end(timer_kind);
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
                        sample_start = UntaggedTimestamp::start(timer_kind);

                        // Sample loop:
                        for input in defer_inputs_iter {
                            // SAFETY: All inputs in `defer_store` were
                            // initialized.
                            _ = black_box(unsafe { benched(input) });
                        }

                        sample_end = UntaggedTimestamp::end(timer_kind);
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
            if is_test {
                break;
            }

            // SAFETY: These values are guaranteed to be the correct variant
            // because they were created from the same `timer_kind`.
            let [sample_start, sample_end] = unsafe {
                [sample_start.into_timestamp(timer_kind), sample_end.into_timestamp(timer_kind)]
            };

            let raw_duration = sample_end.duration_since(sample_start, timer);

            // TODO: Make tuning be less influenced by early runs. Currently if
            // early runs are very quick but later runs are slow, benchmarking
            // will take a very long time.
            //
            // TODO: Make `sample_size` consider time generating inputs and
            // dropping inputs/outputs. Currently benchmarks like
            // `Bencher::bench_refs(String::clear)` take a very long time.
            if current_mode.is_tune() {
                // Clear previous smaller samples.
                self.samples.all.clear();

                // If within 100x timer precision, continue tuning.
                let precision_multiple = raw_duration.picos / timer_precision.picos;
                if precision_multiple <= 100 {
                    current_mode = BenchMode::Tune { sample_size: sample_size * 2 };
                } else {
                    current_mode = BenchMode::Collect { sample_size };
                    rem_samples = Some(self.options.sample_count.unwrap_or(DEFAULT_SAMPLE_COUNT));
                }
            }

            // The per-sample benchmarking overhead.
            let sample_overhead = FineDuration {
                picos: self.shared_context.bench_overhead.picos.saturating_mul(sample_size as u128),
            };

            // Account for the per-sample benchmarking overhead.
            let adjusted_duration =
                FineDuration { picos: raw_duration.picos.saturating_sub(sample_overhead.picos) };

            self.samples.all.push(Sample { duration: adjusted_duration });

            if let Some(rem_samples) = &mut rem_samples {
                *rem_samples = rem_samples.saturating_sub(1);
            }

            if let Some(initial_start) = initial_start {
                elapsed_picos = sample_end.duration_since(initial_start, timer).picos;
            } else {
                // Progress by at least 1ns to prevent extremely fast
                // functions from taking forever when `min_time` is set.
                let progress_picos = raw_duration.picos.max(1_000);
                elapsed_picos = elapsed_picos.saturating_add(progress_picos);
            }
        }
    }

    #[inline]
    fn initial_mode(&self) -> BenchMode {
        if self.shared_context.action.is_test() {
            BenchMode::Test
        } else if let Some(sample_size) = self.options.sample_size {
            BenchMode::Collect { sample_size }
        } else {
            BenchMode::Tune { sample_size: 1 }
        }
    }

    pub fn compute_stats(&self) -> Stats {
        let samples = &self.samples.all;
        let sample_count = samples.len();
        let sample_size = self.samples.sample_size;

        let total_count = self.samples.iter_count();

        let total_duration = self.samples.total_duration();
        let mean_duration = FineDuration {
            picos: total_duration.picos.checked_div(total_count as u128).unwrap_or_default(),
        };

        // Samples sorted by duration.
        let sorted_samples = self.samples.sorted_samples();

        let min_duration =
            sorted_samples.first().map(|s| s.duration / sample_size).unwrap_or_default();
        let max_duration =
            sorted_samples.last().map(|s| s.duration / sample_size).unwrap_or_default();

        let median_duration = if sample_count == 0 {
            FineDuration::default()
        } else if sample_count % 2 == 0 {
            // Take average of two middle numbers.
            let s1 = sorted_samples[sample_count / 2];
            let s2 = sorted_samples[(sample_count / 2) - 1];
            (s1.duration + s2.duration) / (sample_size * 2)
        } else {
            // Single middle number.
            sorted_samples[sample_count / 2].duration / sample_size
        };

        Stats {
            sample_count: sample_count as u32,
            total_count,
            mean_time: mean_duration,
            min_time: min_duration,
            max_time: max_duration,
            median_time: median_duration,
            counter: self.counter().cloned(),
        }
    }
}
