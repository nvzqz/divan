use std::fmt;

use clap::ColorChoice;
use regex::Regex;

use crate::{
    bench::Bencher,
    config::{Action, Filter, OutputFormat, RunIgnored},
    entry::{BenchLoop, Entry, EntryTree},
};

/// The benchmark runner.
#[derive(Default)]
pub struct Divan {
    action: Action,
    color: ColorChoice,
    format: OutputFormat,
    filter: Option<Filter>,
    skip_filters: Vec<Filter>,
    run_ignored: RunIgnored,
    sample_size: Option<u32>,
    sample_count: Option<u32>,
}

impl fmt::Debug for Divan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Divan").finish_non_exhaustive()
    }
}

impl Divan {
    /// Benchmark registered functions.
    pub fn bench(&self) {
        self.run_action(Action::Bench)
    }

    /// Test registered functions.
    pub fn test(&self) {
        self.run_action(Action::Test)
    }

    pub(crate) fn run(&self) {
        self.run_action(self.action)
    }

    /// Returns the entries that should be considered for running.
    fn get_entries(&self) -> Vec<&'static Entry> {
        let mut entries: Vec<&_> =
            crate::entry::ENTRIES.iter().filter(|entry| self.filter(entry)).collect();

        // Run benchmarks in order.
        entries.sort_unstable_by_key(|e| e.sorting_key());

        entries
    }

    /// Returns `true` if `entry` should be considered for running.
    ///
    /// This does not take into account `entry.ignored` because that is handled
    /// separately.
    fn filter(&self, entry: &Entry) -> bool {
        if let Some(filter) = &self.filter {
            if !filter.is_match(entry.full_path) {
                return false;
            }
        }

        !self.skip_filters.iter().any(|filter| filter.is_match(entry.full_path))
    }

    pub(crate) fn should_ignore(&self, entry: &Entry) -> bool {
        !self.run_ignored.should_run(entry.ignore)
    }

    pub(crate) fn run_action(&self, action: Action) {
        let entries = self.get_entries();

        // Quick exit without setting CPU affinity.
        if entries.is_empty() {
            return;
        }

        if action.is_bench() {
            // Try pinning this thread's execution to the first CPU core to
            // help reduce variance from scheduling.
            if let Some(&[core_id, ..]) = core_affinity::get_core_ids().as_deref() {
                if core_affinity::set_for_current(core_id) {
                    eprintln!("Pinned thread to core {}", core_id.id);
                };
            }
        }

        match self.format {
            OutputFormat::Pretty => {
                let mut tree = EntryTree::from_entries(entries.iter().copied());
                EntryTree::sort_by_kind(&mut tree);

                self.run_tree(
                    action,
                    &tree,
                    TreeFormat {
                        parent_prefix: None,
                        max_name_span: EntryTree::max_name_span(&tree, 0),
                    },
                );
                return;
            }
            OutputFormat::Terse => {}
        }

        if action.is_list() {
            for Entry { file, name, line, .. } in entries {
                println!("{file} - {name} (line {line}): bench");
            }
            return;
        }

        for entry in entries {
            self.run_entry(action, entry);
        }
    }

    fn run_tree(&self, action: Action, children: &[EntryTree], fmt: TreeFormat) {
        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;

            let [current_prefix, child_prefix] = if fmt.parent_prefix.is_none() {
                ["", ""]
            } else if !is_last {
                ["├── ", "│   "]
            } else {
                ["╰── ", "    "]
            };

            let parent_prefix = fmt.parent_prefix.unwrap_or_default();
            let current_prefix = format!("{parent_prefix}{current_prefix}");

            let name_pad_len = fmt.max_name_span.saturating_sub(current_prefix.chars().count());

            match child {
                EntryTree::Leaf(entry) => {
                    let name = entry.name;
                    if action.is_list() {
                        println!("{current_prefix}{name:name_pad_len$}");
                    } else {
                        print!("{current_prefix}{name:name_pad_len$} ");
                        self.run_entry(action, entry);
                        println!();
                    }
                }
                EntryTree::Parent { name, children } => {
                    println!("{current_prefix}{name:name_pad_len$}");
                    self.run_tree(
                        action,
                        children,
                        TreeFormat {
                            parent_prefix: Some(&format!("{parent_prefix}{child_prefix}")),
                            ..fmt
                        },
                    );
                }
            };
        }
    }

    fn run_entry(&self, action: Action, entry: &Entry) {
        use crate::bench::Context;

        if self.should_ignore(entry) {
            match self.format {
                OutputFormat::Pretty => print!("(ignored)"),
                OutputFormat::Terse => println!("Ignoring '{}'", entry.full_path),
            }
            return;
        }

        if self.format.is_terse() {
            println!("Running '{}'", entry.full_path);
        }

        let mut context = Context::new(action.is_test(), (entry.bench_options)());

        if let Some(sample_count) = self.sample_count {
            context.options.sample_count = Some(sample_count);
        }

        if let Some(sample_size) = self.sample_size {
            context.options.sample_size = Some(sample_size);
        }

        match &entry.bench_loop {
            // Run the statically-constructed function.
            BenchLoop::Static(bench_loop) => bench_loop(&mut context),

            // Run the function with context via `Bencher`.
            BenchLoop::Runtime(bench) => {
                let mut did_run = false;
                bench(Bencher { did_run: &mut did_run, context: &mut context });

                if !did_run {
                    eprintln!("warning: No benchmark function registered for '{}'", entry.name);
                    return;
                }
            }
        }

        if action.is_bench() {
            let stats = context.compute_stats();

            // TODO: Improve stats formatting.
            match self.format {
                OutputFormat::Pretty => {
                    print!("{stats:?}");
                }
                OutputFormat::Terse => {
                    println!("{stats:#?}");
                    println!();
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct TreeFormat<'a> {
    parent_prefix: Option<&'a str>,
    max_name_span: usize,
}

/// Makes `Divan::skip_regex` input polymorphic.
pub trait SkipRegex {
    fn skip_regex(self, divan: &mut Divan);
}

/// Configuration options.
impl Divan {
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
            self.format = format;
        }

        if matches.get_flag("ignored") {
            self.run_ignored = RunIgnored::Only;
        } else if matches.get_flag("include-ignored") {
            self.run_ignored = RunIgnored::Yes;
        }

        if let Some(&sample_count) = matches.get_one("sample-count") {
            self.sample_count = Some(sample_count);
        }

        if let Some(&sample_size) = matches.get_one("sample-size") {
            self.sample_size = Some(sample_size);
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
        self.format = OutputFormat::Pretty;
        self
    }

    /// Use terse output formatting.
    pub fn format_terse(mut self) -> Self {
        self.format = OutputFormat::Terse;
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
        self.sample_count = Some(count);
        self
    }

    /// Sets the number of iterations inside a single sample.
    ///
    /// This option is equivalent to the `--sample-size` CLI argument.
    #[inline]
    pub fn sample_size(mut self, count: u32) -> Self {
        self.sample_size = Some(count);
        self
    }
}
