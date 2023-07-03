#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

#[doc(inline)]
pub use divan_macros::*;

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod bench;
mod cli;
mod entry;

/// Runs all registered benchmarks.
///
/// # Examples
///
/// ```
/// #[divan::bench]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
///
/// fn main() {
///     // Run `add` benchmark:
///     divan::main();
/// }
/// ```
///
/// See [`#[divan::bench]`](macro@bench) for more examples.
pub fn main() {
    use cli::CliAction;
    use entry::Entry;

    let cli_args = cli::CliArgs::parse();
    let filter = cli_args.filter.as_ref();

    let mut entries: Vec<&_> = entry::ENTRIES
        .iter()
        .filter(|entry| filter.map(|f| f.is_match(entry.name)).unwrap_or(true))
        .collect();

    // Run benchmarks in alphabetical order, breaking ties by line order.
    entries.sort_unstable_by_key(|e| (e.name, e.line));

    let ignore_entry = |entry: &Entry| !cli_args.ignored_mode.should_run(entry.ignore);

    match cli_args.action {
        CliAction::Bench => {
            for entry in &entries {
                if ignore_entry(entry) {
                    println!("Ignoring '{}'", entry.name);
                    continue;
                }

                println!("Running '{}'", entry.name);

                let mut context = bench::Context::new();
                (entry.bench_loop)(&mut context);

                println!("{:#?}", context.compute_stats().unwrap());
                println!();
            }
        }
        CliAction::Test => {
            for entry in &entries {
                if ignore_entry(entry) {
                    println!("Ignoring '{}'", entry.name);
                    continue;
                }

                println!("Running '{}'", entry.name);
                (entry.test)();
            }
        }
        CliAction::List => {
            for Entry { file, name, line, .. } in &entries {
                println!("{file} - {name} (line {line}): bench");
            }
        }
    }
}
