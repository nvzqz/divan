#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

#[doc(inline)]
pub use divan_macros::*;

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod bench;
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
    // Run benchmarks in alphabetical order, breaking ties by line order.
    let mut entries: Vec<&_> = entry::ENTRIES.iter().collect();
    entries.sort_unstable_by_key(|e| (e.path, e.line));

    for entry in &entries {
        println!("Running '{}'", entry.path);

        let mut context = bench::Context::new();
        (entry.bench_loop)(&mut context);

        println!("{:#?}", context.compute_stats().unwrap());
        println!();
    }
}
