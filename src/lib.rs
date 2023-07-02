#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

#[doc(inline)]
pub use divan_macros::*;

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod entry;

/// Runs all registered benchmarks.
///
/// # Examples
///
/// ```
/// #[divan::bench]
/// fn add() {
///     // ...
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
    for benchmark in entry::ENTRIES {
        println!("Running '{}'", benchmark.path);
        (benchmark.bench_loop)();
    }
}
