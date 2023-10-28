use std::{fmt, num::NonZeroUsize, time::Duration};

use clap::ColorChoice;
use regex::Regex;

use crate::{
    bench::{BenchOptions, Bencher},
    config::{Action, Filter, ParsedSeconds, RunIgnored, SortingAttr},
    counter::{BytesFormat, PrivBytesFormat},
    entry::{AnyBenchEntry, EntryTree},
    time::{FineDuration, Timer, TimerKind},
    tree_painter::{TreeColumn, TreePainter},
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
    filters: Vec<Filter>,
    skip_filters: Vec<Filter>,
    run_ignored: RunIgnored,
    bench_options: BenchOptions,
}

/// Immutable context shared between entry runs.
pub(crate) struct SharedContext {
    /// The specific action being performed.
    pub action: Action,

    /// The timer used to measure samples.
    pub timer: Timer,

    /// Per-iteration overhead.
    ///
    /// `min_time` and `max_time` do not consider this as benchmarking time.
    pub bench_overhead: FineDuration,
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
        if !self.filters.is_empty()
            && !self.filters.iter().any(|filter| filter.is_match(entry_path))
        {
            return false;
        }

        !self.skip_filters.iter().any(|filter| filter.is_match(entry_path))
    }

    pub(crate) fn should_ignore(&self, ignored: bool) -> bool {
        !self.run_ignored.should_run(ignored)
    }

    pub(crate) fn run_action(&self, action: Action) {
        let mut tree: Vec<EntryTree> = if cfg!(miri) {
            // Miri does not work with `linkme`.
            Vec::new()
        } else {
            let group_entries = &crate::entry::GROUP_ENTRIES;

            let generic_bench_entries = group_entries
                .iter()
                .flat_map(|group| group.generic_benches_iter().map(AnyBenchEntry::GenericBench));

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

        // Sorting is after filtering to compare fewer elements.
        EntryTree::sort_by_attr(&mut tree, self.sorting_attr, self.reverse_sort);

        let timer = match self.timer {
            TimerKind::Os => Timer::Os,

            TimerKind::Tsc => {
                match Timer::get_tsc() {
                    Ok(tsc) => tsc,
                    Err(error) => {
                        eprintln!("warning: CPU timestamp counter is unavailable ({error}), defaulting to OS");
                        Timer::Os
                    }
                }
            }
        };

        if action.is_bench() {
            eprintln!("Timer precision: {}", timer.precision());
        }

        let shared_context = SharedContext {
            action,
            timer,
            bench_overhead: if action.is_bench() {
                timer.measure_sample_loop_overhead()
            } else {
                FineDuration::default()
            },
        };

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

        let mut tree_painter = TreePainter::new(EntryTree::max_name_span(&tree, 0), column_widths);

        self.run_tree(action, &tree, &shared_context, None, &mut tree_painter);
    }

    fn run_tree(
        &self,
        action: Action,
        tree: &[EntryTree],
        shared_context: &SharedContext,
        parent_options: Option<&BenchOptions>,
        tree_painter: &mut TreePainter,
    ) {
        for (i, child) in tree.iter().enumerate() {
            let is_last = i == tree.len() - 1;

            let name = child.display_name();

            let child_options = child.bench_options();

            // Overwrite `parent_options` with `child_options` if applicable.
            let options: BenchOptions;
            let options: Option<&BenchOptions> = match (parent_options, child_options) {
                (None, None) => None,
                (Some(options), None) | (None, Some(options)) => Some(options),
                (Some(parent_options), Some(child_options)) => {
                    options = child_options.overwrite(parent_options);
                    Some(&options)
                }
            };

            match child {
                EntryTree::Leaf(child) => self.run_bench_entry(
                    action,
                    *child,
                    shared_context,
                    options,
                    tree_painter,
                    is_last,
                ),
                EntryTree::Parent { children, .. } => {
                    tree_painter.start_parent(name, is_last);

                    self.run_tree(action, children, shared_context, options, tree_painter);

                    tree_painter.finish_parent();
                }
            }
        }
    }

    fn run_bench_entry(
        &self,
        action: Action,
        bench_entry: AnyBenchEntry,
        shared_context: &SharedContext,
        entry_options: Option<&BenchOptions>,
        tree_painter: &mut TreePainter,
        is_last: bool,
    ) {
        use crate::bench::BenchContext;

        let display_name = bench_entry.display_name();

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
            tree_painter.ignore_leaf(display_name, is_last);
            return;
        }

        // Paint empty leaf when simply listing.
        if action.is_list() {
            tree_painter.start_leaf(display_name, is_last);
            tree_painter.finish_empty_leaf();
            return;
        }

        // TODO: Add threads options to `Divan`.
        let mut thread_counts: Vec<NonZeroUsize> = entry_options
            .and_then(|options| options.threads)
            .unwrap_or_default()
            .iter()
            .map(|&n| match NonZeroUsize::new(n) {
                Some(n) => n,
                None => crate::util::known_parallelism(),
            })
            .collect();

        thread_counts.sort_unstable();
        thread_counts.dedup();

        let thread_counts: &[NonZeroUsize] =
            if thread_counts.is_empty() { &[NonZeroUsize::MIN] } else { &thread_counts };

        // Whether we should emit child branches for thread counts.
        let has_thread_branches = thread_counts.len() > 1;

        if has_thread_branches {
            tree_painter.start_parent(display_name, is_last);
        } else {
            tree_painter.start_leaf(display_name, is_last);
        }

        for (i, &thread_count) in thread_counts.iter().enumerate() {
            let is_last = if has_thread_branches { i == thread_counts.len() - 1 } else { is_last };

            if has_thread_branches {
                tree_painter.start_leaf(&format!("t={thread_count}"), is_last);
            }

            let mut bench_context = BenchContext::new(shared_context, options, thread_count);
            bench_entry.bench(Bencher::new(&mut bench_context));

            let should_compute_stats = bench_context.did_run && shared_context.action.is_bench();

            if !bench_context.did_run {
                eprintln!("warning: No benchmark function registered for '{display_name}'");
            }

            if !should_compute_stats {
                tree_painter.finish_empty_leaf();
                continue;
            }

            let stats = bench_context.compute_stats();

            tree_painter.finish_leaf(is_last, &stats, self.bytes_format);
        }

        if has_thread_branches {
            tree_painter.finish_parent();
        }
    }
}

/// Makes `Divan::skip_regex` input polymorphic.
pub trait SkipRegex {
    fn skip_regex(self, divan: &mut Divan);
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

        let matches = command.get_matches_mut();
        let is_exact = matches.get_flag("exact");

        let mut parse_filter = |filter: &String| {
            if is_exact {
                Filter::Exact(filter.to_owned())
            } else {
                match Regex::new(filter) {
                    Ok(r) => Filter::Regex(r),
                    Err(error) => {
                        let kind = clap::error::ErrorKind::ValueValidation;
                        command.error(kind, error).exit();
                    }
                }
            }
        };

        if let Some(filters) = matches.get_many::<String>("filter") {
            self.filters.extend(filters.map(&mut parse_filter));
        }

        if let Some(skip_filters) = matches.get_many::<String>("skip") {
            self.skip_filters.extend(skip_filters.map(&mut parse_filter));
        }

        self.action = if matches.get_flag("list") {
            Action::List
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

        if let Some(&PrivBytesFormat(bytes_format)) = matches.get_one("bytes-format") {
            self.bytes_format = bytes_format;
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

        if let Some(&ParsedSeconds(min_time)) = matches.get_one("min-time") {
            self.bench_options.min_time = Some(min_time);
        }

        if let Some(&ParsedSeconds(max_time)) = matches.get_one("max-time") {
            self.bench_options.max_time = Some(max_time);
        }

        if let Some(mut skip_ext_time) = matches.get_many::<bool>("skip-ext-time") {
            // If the option is present without a value, then it's `true`.
            self.bench_options.skip_ext_time =
                Some(matches!(skip_ext_time.next(), Some(true) | None));
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

    /// Determines how [`BytesCount`](crate::counter::BytesCount) is scaled in
    /// benchmark outputs.
    ///
    /// This option is equivalent to the `--bytes-format` CLI argument.
    #[inline]
    pub fn bytes_format(mut self, format: BytesFormat) -> Self {
        self.bytes_format = format;
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
    pub fn skip_regex(mut self, filter: impl SkipRegex) -> Self {
        impl SkipRegex for Regex {
            fn skip_regex(self, divan: &mut Divan) {
                divan.skip_filters.push(Filter::Regex(self));
            }
        }

        impl SkipRegex for &str {
            #[track_caller]
            fn skip_regex(self, divan: &mut Divan) {
                Regex::new(self).unwrap().skip_regex(divan);
            }
        }

        impl SkipRegex for String {
            #[track_caller]
            fn skip_regex(self, divan: &mut Divan) {
                self.as_str().skip_regex(divan)
            }
        }

        filter.skip_regex(&mut self);
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
        self.skip_filters.push(Filter::Exact(filter.into()));
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
        self.bench_options.min_time = Some(time);
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
