use std::fmt;

use clap::ColorChoice;
use regex::Regex;

use crate::{
    config::{Action, Filter, OutputFormat, RunIgnored},
    entry::Entry,
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

        // Run benchmarks in alphabetical order, breaking ties by location order.
        entries.sort_unstable_by_key(|e| (e.name, e.file, e.line));

        // Run each benchmark once even if registered multiple times.
        entries.dedup_by_key(|e| (e.get_id)());

        entries
    }

    /// Returns `true` if `entry` should be considered for running.
    ///
    /// This does not take into account `entry.ignored` because that is handled
    /// separately.
    fn filter(&self, entry: &Entry) -> bool {
        let name = entry.name;

        if let Some(filter) = &self.filter {
            if !filter.is_match(name) {
                return false;
            }
        }

        !self.skip_filters.iter().any(|filter| filter.is_match(name))
    }

    pub(crate) fn should_ignore(&self, entry: &Entry) -> bool {
        !self.run_ignored.should_run(entry.ignore)
    }

    pub(crate) fn run_action(&self, action: Action) {
        let entries = self.get_entries();

        match action {
            Action::Bench => {
                for entry in entries {
                    if self.should_ignore(entry) {
                        println!("Ignoring '{}'", entry.name);
                        continue;
                    }

                    println!("Running '{}'", entry.name);

                    let mut context = crate::bench::Context::new();
                    (entry.bench_loop)(&mut context);

                    println!("{:#?}", context.compute_stats().unwrap());
                    println!();
                }
            }
            Action::Test => {
                for entry in entries {
                    if self.should_ignore(entry) {
                        println!("Ignoring '{}'", entry.name);
                        continue;
                    }

                    println!("Running '{}'", entry.name);
                    (entry.test)();
                }
            }
            Action::List => {
                for Entry { file, name, line, .. } in entries {
                    println!("{file} - {name} (line {line}): bench");
                }
            }
        }
    }
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
}
