#![allow(clippy::too_many_arguments)]

use std::{borrow::Cow, cell::RefCell, fmt, num::NonZeroUsize, time::Duration};

use clap::ColorChoice;
use regex::Regex;

use crate::{
    benchmark::BenchOptions,
    config::{
        filter::{Filter, FilterSet},
        Action, ParsedSeconds, RunIgnored, SortingAttr,
    },
    counter::{
        BytesCount, BytesFormat, CharsCount, CyclesCount, IntoCounter,
        ItemsCount, MaxCountUInt, PrivBytesFormat,
    },
    entry::{AnyBenchEntry, BenchEntryRunner, EntryTree},
    time::{Timer, TimerKind},
    tree_painter::{TreeColumn, TreePainter},
    util::{self, thread::ThreadPool, IntoRegex},
    Bencher,
};

/// The benchmark runner.
#[derive(Default)]
pub struct Divan {
    action: Action,
    timer: TimerKind,
    reverse_sort: bool,
    sorting_attr: SortingAttr,
    color: ColorChoice,
    bytes_format: BytesFormat,
    filters: FilterSet,
    run_ignored: RunIgnored,
    bench_options: BenchOptions<'static>,
}

/// Context shared across all benchmarks.
pub(crate) struct SharedContext {
    /// The specific action being performed.
    pub action: Action,

    /// The timer used to measure samples.
    pub timer: Timer,

    /// Pre-spawned pool of threads for running benchmarks on.
    pub thread_pool: ThreadPool,
}

impl fmt::Debug for Divan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Divan").finish_non_exhaustive()
    }
}

impl Divan {
    /// Perform the configured action.
    ///
    /// By default, this will be [`Divan::run_benches`].
    pub fn main(&self) {
        self.run_action(self.action);
    }

    /// Benchmark registered functions.
    pub fn run_benches(&self) {
        self.run_action(Action::Bench);
    }

    /// Test registered functions as if the `--test` flag was used.
    ///
    /// Unlike [`Divan::run_benches`], this runs each benchmarked function only
    /// once.
    pub fn test_benches(&self) {
        self.run_action(Action::Test);
    }

    /// Print registered functions as if the `--list` flag was used.
    pub fn list_benches(&self) {
        self.run_action(Action::Test);
    }

    /// Returns `true` if an entry at the given path should be considered for
    /// running.
    ///
    /// This does not take into account `entry.ignored` because that is handled
    /// separately.
    fn filter(&self, entry_path: &str) -> bool {
        self.filters.is_match(entry_path)
    }

    pub(crate) fn should_ignore(&self, ignored: bool) -> bool {
        !self.run_ignored.should_run(ignored)
    }

    pub(crate) fn run_action(&self, action: Action) {
        let mut tree: Vec<EntryTree> = if cfg!(miri) {
            // Miri does not work with our linker tricks.
            Vec::new()
        } else {
            let group_entries = &crate::entry::GROUP_ENTRIES;

            let generic_bench_entries =
                group_entries.iter().flat_map(|group| {
                    group
                        .generic_benches_iter()
                        .map(AnyBenchEntry::GenericBench)
                });

            let bench_entries = crate::entry::BENCH_ENTRIES
                .iter()
                .map(AnyBenchEntry::Bench)
                .chain(generic_bench_entries);

            let mut tree = EntryTree::from_benches(bench_entries);

            for group in group_entries.iter() {
                EntryTree::insert_group(&mut tree, group);
            }

            tree
        };

        // Filter after inserting groups so that we can properly use groups'
        // display names.
        EntryTree::retain(&mut tree, |entry_path| self.filter(entry_path));

        // Quick exit without doing unnecessary work.
        if tree.is_empty() {
            return;
        }

        // When run under `cargo-nextest`, it provides `--list --format terse`.
        // We don't currently accept this action under any other circumstances.
        if action.is_list_terse() {
            self.run_tree_list(&tree, "");
            return;
        }

        // Sorting is after filtering to compare fewer elements.
        EntryTree::sort_by_attr(
            &mut tree,
            self.sorting_attr,
            self.reverse_sort,
        );

        let timer = match self.timer {
            TimerKind::Os => Timer::Os,

            TimerKind::Tsc => match Timer::get_tsc() {
                Ok(tsc) => tsc,
                Err(error) => {
                    eprintln!("warning: CPU timestamp counter is unavailable ({error}), defaulting to OS");
                    Timer::Os
                }
            },
        };

        if action.is_bench() {
            eprintln!("Timer precision: {}", timer.precision());
        }

        let shared_context =
            SharedContext { action, timer, thread_pool: ThreadPool::new() };

        let column_widths = if action.is_bench() {
            TreeColumn::ALL.map(|column| {
                if column.is_last() {
                    // The last column doesn't use padding.
                    0
                } else {
                    EntryTree::common_column_width(&tree, column)
                }
            })
        } else {
            [0; TreeColumn::COUNT]
        };

        let tree_painter = RefCell::new(TreePainter::new(
            EntryTree::max_name_span(&tree, 0),
            column_widths,
        ));

        self.run_tree(action, &tree, &shared_context, None, &tree_painter);
    }

    /// Emits the entries in `tree` for the purpose of `--list --format terse`.
    ///
    /// This only happens when running under `cargo-nextest` (`NEXTEST=1`).
    fn run_tree_list(&self, tree: &[EntryTree], parent_path: &str) {
        let mut full_path = String::with_capacity(parent_path.len());

        for child in tree {
            let ignore = child
                .bench_options()
                .and_then(|options| options.ignore)
                .unwrap_or_default();

            if self.should_ignore(ignore) {
                continue;
            }

            full_path.clear();

            if !parent_path.is_empty() {
                full_path.push_str(parent_path);
                full_path.push_str("::");
            }

            full_path.push_str(child.display_name());

            match child {
                EntryTree::Leaf { args: None, .. } => {
                    println!("{full_path}: benchmark")
                }
                EntryTree::Leaf { args: Some(args), .. } => {
                    for arg in args {
                        println!("{full_path}::{arg}: benchmark")
                    }
                }
                EntryTree::Parent { children, .. } => {
                    self.run_tree_list(children, &full_path)
                }
            }
        }
    }

    fn run_tree(
        &self,
        action: Action,
        tree: &[EntryTree],
        shared_context: &SharedContext,
        parent_options: Option<&BenchOptions>,
        tree_painter: &RefCell<TreePainter>,
    ) {
        for (i, child) in tree.iter().enumerate() {
            let is_last = i == tree.len() - 1;

            let name = child.display_name();

            let child_options = child.bench_options();

            // Overwrite `parent_options` with `child_options` if applicable.
            let options: BenchOptions;
            let options: Option<&BenchOptions> =
                match (parent_options, child_options) {
                    (None, None) => None,
                    (Some(options), None) | (None, Some(options)) => {
                        Some(options)
                    }
                    (Some(parent_options), Some(child_options)) => {
                        options = child_options.overwrite(parent_options);
                        Some(&options)
                    }
                };

            match child {
                EntryTree::Leaf { entry, args } => self.run_bench_entry(
                    action,
                    *entry,
                    args.as_deref(),
                    shared_context,
                    options,
                    tree_painter,
                    is_last,
                ),
                EntryTree::Parent { children, .. } => {
                    tree_painter.borrow_mut().start_parent(name, is_last);

                    self.run_tree(
                        action,
                        children,
                        shared_context,
                        options,
                        tree_painter,
                    );

                    tree_painter.borrow_mut().finish_parent();
                }
            }
        }
    }

    fn run_bench_entry(
        &self,
        action: Action,
        bench_entry: AnyBenchEntry,
        bench_arg_names: Option<&[&&str]>,
        shared_context: &SharedContext,
        entry_options: Option<&BenchOptions>,
        tree_painter: &RefCell<TreePainter>,
        is_last_entry: bool,
    ) {
        use crate::benchmark::BenchContext;

        let entry_display_name = bench_entry.display_name();

        // User runtime options override all other options.
        let options: BenchOptions;
        let options: &BenchOptions = match entry_options {
            None => &self.bench_options,
            Some(entry_options) => {
                options = self.bench_options.overwrite(entry_options);
                &options
            }
        };

        if self.should_ignore(options.ignore.unwrap_or_default()) {
            tree_painter
                .borrow_mut()
                .ignore_leaf(entry_display_name, is_last_entry);
            return;
        }

        // Paint empty leaf when simply listing.
        if action.is_list() {
            let mut tree_painter = tree_painter.borrow_mut();
            tree_painter.start_leaf(entry_display_name, is_last_entry);
            tree_painter.finish_empty_leaf();
            return;
        }

        let mut thread_counts: Vec<NonZeroUsize> = options
            .threads
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|&n| match NonZeroUsize::new(n) {
                Some(n) => n,
                None => crate::util::known_parallelism(),
            })
            .collect();

        thread_counts.sort_unstable();
        thread_counts.dedup();

        let thread_counts: &[NonZeroUsize] = if thread_counts.is_empty() {
            &[NonZeroUsize::MIN]
        } else {
            &thread_counts
        };

        // Whether we should emit child branches for thread counts.
        let has_thread_branches = thread_counts.len() > 1;

        let run_bench =
            |bench_display_name: &str,
             is_last_bench: bool,
             with_bencher: &dyn Fn(Bencher)| {
                if has_thread_branches {
                    tree_painter
                        .borrow_mut()
                        .start_parent(bench_display_name, is_last_bench);
                } else {
                    tree_painter
                        .borrow_mut()
                        .start_leaf(bench_display_name, is_last_bench);
                }

                for (i, &thread_count) in thread_counts.iter().enumerate() {
                    let is_last_thread_count = if has_thread_branches {
                        i == thread_counts.len() - 1
                    } else {
                        is_last_bench
                    };

                    if has_thread_branches {
                        tree_painter.borrow_mut().start_leaf(
                            &format!("t={thread_count}"),
                            is_last_thread_count,
                        );
                    }

                    let mut bench_context = BenchContext::new(
                        shared_context,
                        options,
                        thread_count,
                    );
                    with_bencher(Bencher::new(&mut bench_context));

                    if !bench_context.did_run {
                        eprintln!(
                        "warning: No benchmark function registered for '{bench_display_name}'"
                    );
                    }

                    let should_compute_stats = bench_context.did_run
                        && shared_context.action.is_bench();

                    if should_compute_stats {
                        let stats = bench_context.compute_stats();
                        tree_painter.borrow_mut().finish_leaf(
                            is_last_thread_count,
                            &stats,
                            self.bytes_format,
                        );
                    } else {
                        tree_painter.borrow_mut().finish_empty_leaf();
                    }
                }

                if has_thread_branches {
                    tree_painter.borrow_mut().finish_parent();
                }
            };

        match bench_entry.bench_runner() {
            BenchEntryRunner::Plain(bench) => {
                run_bench(entry_display_name, is_last_entry, bench)
            }

            BenchEntryRunner::Args(bench_runner) => {
                tree_painter
                    .borrow_mut()
                    .start_parent(entry_display_name, is_last_entry);

                let bench_runner = bench_runner();
                let orig_arg_names = bench_runner.arg_names();
                let bench_arg_names = bench_arg_names.unwrap_or_default();

                for (i, &arg_name) in bench_arg_names.iter().enumerate() {
                    let is_last_arg = i == bench_arg_names.len() - 1;
                    let arg_index =
                        util::slice_ptr_index(orig_arg_names, arg_name);

                    run_bench(arg_name, is_last_arg, &|bencher| {
                        bench_runner.bench(bencher, arg_index);
                    });
                }

                tree_painter.borrow_mut().finish_parent();
            }
        }
    }
}

/// Configuration options.
impl Divan {
    /// Creates an instance with options set by parsing CLI arguments.
    pub fn from_args() -> Self {
        Self::default().config_with_args()
    }

    /// Sets options by parsing CLI arguments.
    ///
    /// This may override any previously-set options.
    #[must_use]
    pub fn config_with_args(mut self) -> Self {
        let mut command = crate::cli::command();

        let mut matches = command.get_matches_mut();
        let is_exact = matches.get_flag("exact");

        // Insert filters.
        {
            let mut parse_filter = |filter: String| -> Filter {
                if is_exact {
                    Filter::Exact(filter)
                } else {
                    Filter::Regex(Regex::new(&filter).unwrap_or_else(|error| {
                        let kind = clap::error::ErrorKind::ValueValidation;
                        command.error(kind, error).exit();
                    }))
                }
            };

            let inclusive_filters = matches.remove_many::<String>("filter");
            let exclusive_filters = matches.remove_many::<String>("skip");

            // Reduce allocation size and reallocation count.
            self.filters.reserve_exact({
                let inclusive_count = inclusive_filters
                    .as_ref()
                    .map(|f| f.len())
                    .unwrap_or_default();

                let exclusive_count = exclusive_filters
                    .as_ref()
                    .map(|f| f.len())
                    .unwrap_or_default();

                inclusive_count + exclusive_count
            });

            if let Some(inclusive_filters) = inclusive_filters {
                for filter in inclusive_filters {
                    self.filters.include(parse_filter(filter));
                }
            }

            if let Some(exclusive_filters) = exclusive_filters {
                for filter in exclusive_filters {
                    self.filters.exclude(parse_filter(filter));
                }
            }
        }

        self.action = if matches.get_flag("list") {
            // We support `--list --format terse` only under `cargo-nextest`.
            let is_terse = matches
                .try_get_one::<String>("format")
                .ok()
                .flatten()
                .map(|format| format == "terse")
                .unwrap_or_default();

            if is_terse {
                Action::ListTerse
            } else {
                Action::List
            }
        } else if matches.get_flag("test") || !matches.get_flag("bench") {
            // Either of:
            // `cargo bench -- --test`
            // `cargo test --benches`
            Action::Test
        } else {
            Action::Bench
        };

        if let Some(&color) = matches.get_one("color") {
            self.color = color;
        }

        if matches.get_flag("ignored") {
            self.run_ignored = RunIgnored::Only;
        } else if matches.get_flag("include-ignored") {
            self.run_ignored = RunIgnored::Yes;
        }

        if let Some(&timer) = matches.get_one("timer") {
            self.timer = timer;
        }

        if let Some(&sorting_attr) = matches.get_one("sortr") {
            self.reverse_sort = true;
            self.sorting_attr = sorting_attr;
        } else if let Some(&sorting_attr) = matches.get_one("sort") {
            self.reverse_sort = false;
            self.sorting_attr = sorting_attr;
        }

        if let Some(&sample_count) = matches.get_one("sample-count") {
            self.bench_options.sample_count = Some(sample_count);
        }

        if let Some(&sample_size) = matches.get_one("sample-size") {
            self.bench_options.sample_size = Some(sample_size);
        }

        if let Some(thread_counts) = matches.get_many::<usize>("threads") {
            let mut threads: Vec<usize> = thread_counts.copied().collect();
            threads.sort_unstable();
            threads.dedup();
            self.bench_options.threads = Some(Cow::Owned(threads));
        }

        if let Some(&ParsedSeconds(min_time)) = matches.get_one("min-time") {
            self.bench_options.min_time = Some(min_time);
        }

        if let Some(&ParsedSeconds(max_time)) = matches.get_one("max-time") {
            self.bench_options.max_time = Some(max_time);
        }

        if let Some(mut skip_ext_time) =
            matches.get_many::<bool>("skip-ext-time")
        {
            // If the option is present without a value, then it's `true`.
            self.bench_options.skip_ext_time =
                Some(matches!(skip_ext_time.next(), Some(true) | None));
        }

        if let Some(&count) = matches.get_one::<MaxCountUInt>("items-count") {
            self.counter_mut(ItemsCount::new(count));
        }

        if let Some(&count) = matches.get_one::<MaxCountUInt>("bytes-count") {
            self.counter_mut(BytesCount::new(count));
        }

        if let Some(&PrivBytesFormat(bytes_format)) =
            matches.get_one("bytes-format")
        {
            self.bytes_format = bytes_format;
        }

        if let Some(&count) = matches.get_one::<MaxCountUInt>("chars-count") {
            self.counter_mut(CharsCount::new(count));
        }

        if let Some(&count) = matches.get_one::<MaxCountUInt>("cycles-count") {
            self.counter_mut(CyclesCount::new(count));
        }

        self
    }

    /// Sets whether output should be colored.
    ///
    /// This option is equivalent to the `--color` CLI argument, where [`None`]
    /// here means "auto".
    #[must_use]
    pub fn color(mut self, yes: impl Into<Option<bool>>) -> Self {
        self.color = match yes.into() {
            None => ColorChoice::Auto,
            Some(true) => ColorChoice::Always,
            Some(false) => ColorChoice::Never,
        };
        self
    }

    /// Also run benchmarks marked [`#[ignore]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-ignore-attribute).
    ///
    /// This option is equivalent to the `--include-ignored` CLI argument.
    #[must_use]
    pub fn run_ignored(mut self) -> Self {
        self.run_ignored = RunIgnored::Yes;
        self
    }

    /// Only run benchmarks marked [`#[ignore]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-ignore-attribute).
    ///
    /// This option is equivalent to the `--ignored` CLI argument.
    #[must_use]
    pub fn run_only_ignored(mut self) -> Self {
        self.run_ignored = RunIgnored::Only;
        self
    }

    /// Skips benchmarks that match `filter` as a regular expression pattern.
    ///
    /// This option is equivalent to the `--skip filter` CLI argument, without
    /// `--exact`.
    ///
    /// # Examples
    ///
    /// This method is commonly used with a [`&str`](prim@str) or [`String`]:
    ///
    /// ```
    /// # use divan::Divan;
    /// let filter = "(add|sub)";
    /// let divan = Divan::default().skip_regex(filter);
    /// ```
    ///
    /// A pre-built [`Regex`] can also be provided:
    ///
    /// ```
    /// # use divan::Divan;
    /// let filter = regex::Regex::new("(add|sub)").unwrap();
    /// let divan = Divan::default().skip_regex(filter);
    /// ```
    ///
    /// Calling this repeatedly will add multiple skip filters:
    ///
    /// ```
    /// # use divan::Divan;
    /// let divan = Divan::default()
    ///     .skip_regex("(add|sub)")
    ///     .skip_regex("collections.*default");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `filter` is a string and [`Regex::new`] fails.
    #[must_use]
    #[track_caller]
    pub fn skip_regex(mut self, filter: impl IntoRegex) -> Self {
        self.filters.exclude(Filter::Regex(filter.into_regex()));
        self
    }

    /// Skips benchmarks that exactly match `filter`.
    ///
    /// This option is equivalent to the `--skip filter --exact` CLI arguments.
    ///
    /// # Examples
    ///
    /// This method is commonly used with a [`&str`](prim@str) or [`String`]:
    ///
    /// ```
    /// # use divan::Divan;
    /// let filter = "arithmetic::add";
    /// let divan = Divan::default().skip_exact(filter);
    /// ```
    ///
    /// Calling this repeatedly will add multiple skip filters:
    ///
    /// ```
    /// # use divan::Divan;
    /// let divan = Divan::default()
    ///     .skip_exact("arithmetic::add")
    ///     .skip_exact("collections::vec::default");
    /// ```
    #[must_use]
    pub fn skip_exact(mut self, filter: impl Into<String>) -> Self {
        self.filters.exclude(Filter::Exact(filter.into()));
        self
    }

    /// Sets the number of sampling iterations.
    ///
    /// This option is equivalent to the `--sample-count` CLI argument.
    ///
    /// If a benchmark enables [`threads`](macro@crate::bench#threads), sample
    /// count becomes a multiple of the number of threads. This is because each
    /// thread operates over the same sample size to ensure there are always N
    /// competing threads doing the same amount of work.
    #[inline]
    pub fn sample_count(mut self, count: u32) -> Self {
        self.bench_options.sample_count = Some(count);
        self
    }

    /// Sets the number of iterations inside a single sample.
    ///
    /// This option is equivalent to the `--sample-size` CLI argument.
    #[inline]
    pub fn sample_size(mut self, count: u32) -> Self {
        self.bench_options.sample_size = Some(count);
        self
    }

    /// Run across multiple threads.
    ///
    /// This enables you to measure contention on [atomics and
    /// locks](std::sync). A value of 0 indicates [available
    /// parallelism](std::thread::available_parallelism).
    ///
    /// This option is equivalent to the `--threads` CLI argument or
    /// `DIVAN_THREADS` environment variable.
    #[inline]
    pub fn threads<T>(mut self, threads: T) -> Self
    where
        T: IntoIterator<Item = usize>,
    {
        self.bench_options.threads = {
            let mut threads: Vec<usize> = threads.into_iter().collect();
            threads.sort_unstable();
            threads.dedup();
            Some(Cow::Owned(threads))
        };
        self
    }

    /// Sets the time floor for benchmarking a function.
    ///
    /// This option is equivalent to the `--min-time` CLI argument.
    #[inline]
    pub fn min_time(mut self, time: Duration) -> Self {
        self.bench_options.min_time = Some(time);
        self
    }

    /// Sets the time ceiling for benchmarking a function.
    ///
    /// This option is equivalent to the `--max-time` CLI argument.
    #[inline]
    pub fn max_time(mut self, time: Duration) -> Self {
        self.bench_options.max_time = Some(time);
        self
    }

    /// When accounting for `min_time` or `max_time`, skip time external to
    /// benchmarked functions.
    ///
    /// This option is equivalent to the `--skip-ext-time` CLI argument.
    #[inline]
    pub fn skip_ext_time(mut self, skip: bool) -> Self {
        self.bench_options.skip_ext_time = Some(skip);
        self
    }
}

/// Use [`Counter`s](crate::counter::Counter) to get throughput across all
/// benchmarks.
impl Divan {
    #[inline]
    fn counter_mut<C: IntoCounter>(&mut self, counter: C) -> &mut Self {
        self.bench_options.counters.insert(counter);
        self
    }

    /// Counts the number of values processed.
    #[inline]
    pub fn counter<C: IntoCounter>(mut self, counter: C) -> Self {
        self.counter_mut(counter);
        self
    }

    /// Sets the number of items processed.
    ///
    /// This option is equivalent to the `--items-count` CLI argument or
    /// `DIVAN_ITEMS_COUNT` environment variable.
    #[inline]
    pub fn items_count<C: Into<ItemsCount>>(self, count: C) -> Self {
        self.counter(count.into())
    }

    /// Sets the number of bytes processed.
    ///
    /// This option is equivalent to the `--bytes-count` CLI argument or
    /// `DIVAN_BYTES_COUNT` environment variable.
    #[inline]
    pub fn bytes_count<C: Into<BytesCount>>(self, count: C) -> Self {
        self.counter(count.into())
    }

    /// Determines how [`BytesCount`] is scaled in benchmark outputs.
    ///
    /// This option is equivalent to the `--bytes-format` CLI argument or
    /// `DIVAN_BYTES_FORMAT` environment variable.
    #[inline]
    pub fn bytes_format(mut self, format: BytesFormat) -> Self {
        self.bytes_format = format;
        self
    }

    /// Sets the number of bytes processed.
    ///
    /// This option is equivalent to the `--chars-count` CLI argument or
    /// `DIVAN_CHARS_COUNT` environment variable.
    #[inline]
    pub fn chars_count<C: Into<CharsCount>>(self, count: C) -> Self {
        self.counter(count.into())
    }

    /// Sets the number of cycles processed, displayed as Hertz.
    ///
    /// This option is equivalent to the `--cycles-count` CLI argument or
    /// `DIVAN_CYCLES_COUNT` environment variable.
    #[inline]
    pub fn cycles_count<C: Into<CyclesCount>>(self, count: C) -> Self {
        self.counter(count.into())
    }
}
