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
mod config;
mod divan;
mod entry;

#[doc(inline)]
pub use divan::Divan;

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
    Divan::default().config_with_args().run();
}
