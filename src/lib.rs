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
    let filter = cli_args.filter();

    let mut entries: Vec<&_> = entry::ENTRIES
        .iter()
        .filter(|entry| filter.map(|f| f.is_match(entry.path)).unwrap_or(true))
        .collect();

    // Run benchmarks in alphabetical order, breaking ties by line order.
    entries.sort_unstable_by_key(|e| (e.path, e.line));

    match cli_args.action {
        CliAction::Bench => {
            for entry in &entries {
                println!("Running '{}'", entry.path);

                let mut context = bench::Context::new();
                (entry.bench_loop)(&mut context);

                println!("{:#?}", context.compute_stats().unwrap());
                println!();
            }
        }
        CliAction::List => {
            for Entry {
                file, path, line, ..
            } in &entries
            {
                println!("{file} - {path} (line {line}): bench");
            }
        }
    }
}
