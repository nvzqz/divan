use std::{
    fmt,
    time::{Duration, Instant},
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
///     // Input and output buffers get moved into the closure.
///     let src = (0..100).collect::<Vec<i32>>();
///     let mut dst = vec![0; src.len()];
///
///     bencher.bench(move || {
///         black_box(&mut dst).copy_from_slice(black_box(&src));
///     });
/// }
/// ```
#[must_use = "a benchmark function must be registered"]
pub struct Bencher<'a> {
    #[allow(clippy::type_complexity)]
    pub(crate) bench_loop: &'a mut Option<Box<dyn FnMut(&mut Context)>>,
}

impl fmt::Debug for Bencher<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bencher").finish_non_exhaustive()
    }
}

impl Bencher<'_> {
    /// Registers the given function to be benchmarked.
    pub fn bench<F, R>(self, mut f: F)
    where
        F: FnMut() -> R + 'static,
    {
        *self.bench_loop = Some(Box::new(move |cx| {
            // Prevents `Drop` from being measured automatically.
            let mut drop_store = DropStore::with_capacity(cx.iter_per_sample as usize);

            for _ in 0..cx.target_sample_count() {
                drop_store.prepare(cx.iter_per_sample as usize);

                let sample = cx.start_sample();
                for _ in 0..cx.iter_per_sample {
                    // NOTE: `push` is a no-op if the result of the benchmarked
                    // function does not need to be dropped.
                    drop_store.push(std::hint::black_box(f()));
                }
                cx.end_sample(sample);
            }
        }));
    }
}

/// `#[divan::bench]` loop context.
///
/// Functions called within the benchmark loop should be `#[inline(always)]` to
/// ensure instruction cache locality.
///
/// Instances of this type are publicly accessible to generated code, so care
/// should be taken when making fields fully public.
pub struct Context {
    /// Recorded samples.
    pub(crate) samples: Vec<Sample>,

    /// The number of iterations between recording samples.
    pub iter_per_sample: u32,
}

impl Context {
    /// Creates a context for actual benchmarking.
    pub(crate) fn bench() -> Self {
        Self {
            // TODO: Pick these numbers dynamically.
            samples: Vec::with_capacity(1_000),
            iter_per_sample: 1_000,
        }
    }

    /// Creates a context for testing benchmarked functions.
    pub(crate) fn test() -> Self {
        Self { samples: Vec::with_capacity(1), iter_per_sample: 1 }
    }

    /// Returns the number of samples that should be taken.
    #[inline(always)]
    pub fn target_sample_count(&self) -> usize {
        self.samples.capacity()
    }

    /// Begins info measurement at the start of a loop.
    #[inline(always)]
    pub fn start_sample(&self) -> Instant {
        // Prevent other operations from affecting timing measurements.
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        Instant::now()
    }

    /// Records measurement info at the end of a loop.
    #[inline(always)]
    pub fn end_sample(&mut self, start: Instant) {
        let end = Instant::now();

        // Prevent other operations from affecting timing measurements.
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        self.samples.push(Sample { start, end });
    }

    pub(crate) fn compute_stats(&self) -> Option<Stats> {
        let sample_count = self.samples.len();
        let total_count = sample_count * self.iter_per_sample as usize;

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
                    self.iter_per_sample as u128,
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

/// Measurement datum.
pub struct Sample {
    /// When the sample began.
    pub start: Instant,

    /// When the sample stopped.
    pub end: Instant,
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
        Self { picos: (duration.as_nanos() * 1_000) / n }
    }
}

/// Defers `Drop` of items produced while benchmarking.
pub struct DropStore<T> {
    items: Vec<T>,
}

#[allow(missing_docs)]
impl<T> DropStore<T> {
    const IS_NO_OP: bool = !std::mem::needs_drop::<T>();

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { items: if Self::IS_NO_OP { Vec::new() } else { Vec::with_capacity(capacity) } }
    }

    /// Prepares the store for storing a sample.
    #[inline(always)]
    pub fn prepare(&mut self, capacity: usize) {
        if !Self::IS_NO_OP {
            self.items.clear();
            self.items.reserve_exact(capacity);
        }
    }

    #[inline(always)]
    pub fn push(&mut self, item: T) {
        if !Self::IS_NO_OP {
            self.items.push(item);
        }
    }
}
