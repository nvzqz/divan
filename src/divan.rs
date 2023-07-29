use std::{fmt, time::Duration};

use clap::ColorChoice;
use regex::Regex;

use crate::{
    bench::{self, BenchOptions, Bencher},
    config::{Action, Filter, FormatStyle, ParsedSeconds, RunIgnored, SortingAttr},
    entry::{Entry, EntryTree},
    time::{FineDuration, Timer, TimerKind},
};

/// The benchmark runner.
#[derive(Default)]
pub struct Divan {
    action: Action,
    timer: TimerKind,
    reverse_sort: bool,
    sorting_attr: SortingAttr,
    color: ColorChoice,
    format_style: FormatStyle,
    filter: Option<Filter>,
    skip_filters: Vec<Filter>,
    run_ignored: RunIgnored,
    bench_options: BenchOptions,
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
        if let Some(filter) = &self.filter {
            if !filter.is_match(entry_path) {
                return false;
            }
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
            let mut tree = EntryTree::from_entries(crate::entry::ENTRIES);

            for group in crate::entry::ENTRY_GROUPS {
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

        if action.is_bench() {
            // Try pinning this thread's execution to the first CPU core to
            // help reduce variance from scheduling.
            if let Some(&[core_id, ..]) = core_affinity::get_core_ids().as_deref() {
                if core_affinity::set_for_current(core_id) {
                    eprintln!("Pinned thread to core {}", core_id.id);
                };
            }
        }

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

        let overhead = if action.is_bench() {
            bench::measure_overhead(timer)
        } else {
            FineDuration::default()
        };

        self.run_tree(
            &tree,
            action,
            timer,
            overhead,
            false,
            &BenchOptions::default(),
            match self.format_style {
                FormatStyle::Pretty => FormatContext::Pretty(TreeFormat {
                    parent_prefix: None,
                    max_name_span: EntryTree::max_name_span(&tree, 0),
                }),
                FormatStyle::Terse => FormatContext::Terse { parent_path: String::new() },
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn run_tree(
        &self,
        tree: &[EntryTree],
        action: Action,
        timer: Timer,
        overhead: FineDuration,
        parent_ignored: bool,
        parent_options: &BenchOptions,
        fmt_ctx: FormatContext,
    ) {
        for (i, child) in tree.iter().enumerate() {
            let is_last = i == tree.len() - 1;

            let list_format = || match &fmt_ctx {
                FormatContext::Pretty(tree_fmt) => {
                    let current_prefix = tree_fmt.current_prefix(is_last);
                    let name_pad_len =
                        tree_fmt.max_name_span.saturating_sub(current_prefix.chars().count());
                    let display_name = child.display_name();
                    format!("{current_prefix}{display_name:name_pad_len$}")
                }
                FormatContext::Terse { parent_path } => match child {
                    EntryTree::Leaf(Entry { file, line, .. }) => {
                        let display_name = child.display_name();
                        format!("{file} - {parent_path}::{display_name} (line {line}): bench")
                    }
                    EntryTree::Parent { .. } => unreachable!(),
                },
            };

            match child {
                EntryTree::Leaf(entry) => match (action, self.format_style) {
                    (Action::List, FormatStyle::Terse | FormatStyle::Pretty) => {
                        println!("{}", list_format());
                    }
                    _ => {
                        if self.format_style.is_pretty() {
                            print!("{} ", list_format());
                        }

                        self.run_entry(
                            entry,
                            action,
                            timer,
                            overhead,
                            parent_ignored,
                            parent_options,
                            &fmt_ctx,
                        );

                        if self.format_style.is_pretty() {
                            println!();
                        }
                    }
                },
                EntryTree::Parent { group, children, .. } => {
                    let group_options = group.and_then(|group| {
                        group
                            .bench_options
                            .map(|bench_options| bench_options().overwrite(parent_options))
                    });

                    if self.format_style.is_pretty() {
                        println!("{}", list_format());
                    }

                    self.run_tree(
                        children,
                        action,
                        timer,
                        overhead,
                        parent_ignored || group.map(|g| g.ignore).unwrap_or_default(),
                        group_options.as_ref().unwrap_or(parent_options),
                        match &fmt_ctx {
                            FormatContext::Pretty(tree_fmt) => FormatContext::Pretty(TreeFormat {
                                parent_prefix: Some(tree_fmt.child_prefix(is_last)),
                                max_name_span: tree_fmt.max_name_span,
                            }),
                            FormatContext::Terse { parent_path } => {
                                let display_name = child.display_name();
                                FormatContext::Terse {
                                    parent_path: if parent_path.is_empty() {
                                        display_name.to_owned()
                                    } else {
                                        format!("{parent_path}::{display_name}")
                                    },
                                }
                            }
                        },
                    );
                }
            };
        }
    }

    fn run_entry(
        &self,
        entry: &Entry,
        action: Action,
        timer: Timer,
        overhead: FineDuration,
        parent_ignored: bool,
        parent_options: &BenchOptions,
        fmt_ctx: &FormatContext,
    ) {
        use crate::bench::Context;

        if self.should_ignore(parent_ignored || entry.ignore) {
            match fmt_ctx {
                FormatContext::Pretty { .. } => print!("(ignored)"),
                FormatContext::Terse { parent_path } => {
                    println!("Ignoring '{parent_path}::{}'", entry.display_name);
                }
            }
            return;
        }

        if let FormatContext::Terse { parent_path } = fmt_ctx {
            println!("Running '{parent_path}::{}'", entry.display_name);
        }

        let mut options = entry
            .bench_options
            .map(|bench_options| bench_options())
            .unwrap_or_default()
            .overwrite(parent_options);

        options = self.bench_options.overwrite(&options);

        let mut context = Context::new(action.is_test(), timer, overhead, options);

        let mut did_run = false;
        (entry.bench)(Bencher { did_run: &mut did_run, context: &mut context });

        if !did_run {
            eprintln!("warning: No benchmark function registered for '{}'", entry.display_name);
            return;
        }

        if action.is_bench() {
            let stats = context.compute_stats();

            // TODO: Improve stats formatting.
            match self.format_style {
                FormatStyle::Pretty => {
                    print!("{stats:?}");
                }
                FormatStyle::Terse => {
                    println!("{stats:#?}");
                    println!();
                }
            }
        }
    }
}

enum FormatContext {
    Terse { parent_path: String },
    Pretty(TreeFormat),
}

struct TreeFormat {
    parent_prefix: Option<String>,
    max_name_span: usize,
}

impl TreeFormat {
    fn current_prefix(&self, is_last: bool) -> String {
        let next_part = if self.parent_prefix.is_none() {
            ""
        } else if !is_last {
            "├── "
        } else {
            "╰── "
        };
        let parent_prefix = self.parent_prefix.as_deref().unwrap_or_default();

        format!("{parent_prefix}{next_part}")
    }

    fn child_prefix(&self, is_last: bool) -> String {
        let next_part = if self.parent_prefix.is_none() {
            ""
        } else if !is_last {
            "│   "
        } else {
            "    "
        };
        let parent_prefix = self.parent_prefix.as_deref().unwrap_or_default();

        format!("{parent_prefix}{next_part}")
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

        let mut parse_filter = |filter: &str| {
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

        if let Some(filter) = matches.get_one::<String>("filter") {
            self.filter = Some(parse_filter(filter));
        }

        if let Some(skip_filters) = matches.get_many::<String>("skip") {
            self.skip_filters.extend(skip_filters.map(|skip_filter| parse_filter(skip_filter)));
        }

        if matches.get_flag("test") {
            self.action = Action::Test;
        } else if matches.get_flag("list") {
            self.action = Action::List;
        };

        if let Some(&color) = matches.get_one("color") {
            self.color = color;
        }

        if let Some(&format) = matches.get_one("format") {
            self.format_style = format;
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

        if let Some(mut skip_input_time) = matches.get_many::<bool>("skip-input-time") {
            // If the option is present without a value, then it's `true`.
            self.bench_options.skip_input_time =
                Some(matches!(skip_input_time.next(), Some(true) | None));
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

    /// Use pretty output formatting.
    pub fn format_pretty(mut self) -> Self {
        self.format_style = FormatStyle::Pretty;
        self
    }

    /// Use terse output formatting.
    pub fn format_terse(mut self) -> Self {
        self.format_style = FormatStyle::Terse;
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

    /// Skip time spent generating inputs when accounting for `min_time` or
    /// `max_time`.
    ///
    /// This option is equivalent to the `--skip-input-time` CLI argument.
    #[inline]
    pub fn skip_input_time(mut self, skip: bool) -> Self {
        self.bench_options.skip_input_time = Some(skip);
        self
    }
}
