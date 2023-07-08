#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod bench;
mod cli;
mod config;
mod divan;
mod entry;
mod stats;
mod time;

#[doc(inline)]
pub use std::hint::black_box;

/// Registers a benchmarking function.
///
/// # Examples
///
/// The quickest way to get started is to benchmark the function as-is:
///
/// ```
/// use divan::black_box;
///
/// #[divan::bench]
/// fn add() -> i32 {
///     black_box(1) + black_box(42)
/// }
///
/// fn main() {
///     // Run `add` benchmark:
///     divan::main();
/// }
/// ```
///
/// If context is needed within the benchmarked function, take a [`Bencher`] and
/// use [`Bencher::bench`]:
///
/// ```
/// use divan::{Bencher, black_box};
///
/// #[divan::bench]
/// fn copy_from_slice(bencher: Bencher) {
///     let src = (0..100).collect::<Vec<i32>>();
///     let mut dst = vec![0; src.len()];
///
///     bencher.bench(move || {
///         black_box(&mut dst).copy_from_slice(black_box(&src));
///     });
/// }
/// ```
///
/// # Options
///
/// - `#[divan::bench(name = "...")]`
///
///   By default, the benchmark is named after the function's [canonical path](https://doc.rust-lang.org/reference/paths.html#canonical-paths)
///   (i.e. `module_path!() + "::" + fn_name`). This can be overridden via the
///   `name` option:
///
///   ```
///   #[divan::bench(name = "Add It")]
///   fn add() -> i32 {
///       // ...
///       # 0
///   }
///   ```
///
/// - `#[divan::bench(crate = path::to::divan)]`
///
///   The path to the specific `divan` crate instance used by this macro's
///   generated code can be specified via the `crate` option. This is applicable
///   when using `divan` via a macro from your own crate.
///
///   ```
///   extern crate divan as sofa;
///
///   #[::sofa::bench(crate = ::sofa)]
///   fn add() -> i32 {
///       // ...
///       # 0
///   }
///   ```
///
/// - [`#[ignore]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-ignore-attribute)
///
///   Like [`#[test]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-test-attribute),
///   `#[divan::bench]` functions can be ignored:
///
///   ```
///   #[divan::bench]
///   #[ignore = "not yet implemented"]
///   fn todo() {
///       unimplemented!();
///   }
///   # divan::main();
///   ```
pub use divan_macros::bench;

#[doc(inline)]
pub use crate::{bench::Bencher, divan::Divan};

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
