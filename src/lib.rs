#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod bench;
mod cli;
mod compile_fail;
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
///     # #[cfg(not(miri))]
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
/// If values constructed in the benchmarked function implement [`Drop`], the
/// drop code can be deferred until after the sample measurement is taken. This
/// is done by simply returning the value. The following benchmarks will only
/// measure [`String`] construction and not measure [`Drop`]:
///
/// ```
/// # use divan::Bencher;
/// #[divan::bench]
/// fn make_string_1() -> String {
///     // Drop for `s` will not run in `make_string_1`:
///     let s: String = // ...
///     # String::new();
///     // ...
///     s
/// }
///
/// #[divan::bench]
/// fn make_string_2(bencher: Bencher) {
///     // Setup...
///
///     bencher.bench(|| -> String {
///         // Drop for `s` will not run in this closure:
///         let s: String = // ...
///         # String::new();
///         // ...
///         s
///     });
/// }
/// ```
///
/// Applying this attribute multiple times to the same item will cause a compile
/// error:
///
/// ```compile_fail
/// #[divan::bench]
/// #[divan::bench]
/// fn bench() {
///     // ...
/// }
/// ```
///
/// # Options
///
/// - `#[divan::bench(name = "...")]`
///
///   The benchmark uses the same name as the function. It can be overridden via
///   the `name` option:
///
///   ```
///   #[divan::bench(name = "my_add")]
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
/// - `#[divan::bench(sample_count = 1000)]`
///
///   The number of statistical sample recordings can be set to a predetermined
///   [`u32`] value via the `sample_count` option. This may be overridden at
///   runtime using either the `DIVAN_SAMPLE_COUNT` environment variable or
///   `--sample-count` CLI argument.
///
///   ```
///   #[divan::bench(sample_count = 1000)]
///   fn add() -> i32 {
///       // ...
///       # 0
///   }
///   ```
///
/// - `#[divan::bench(sample_size = 1000)]`
///
///   The number iterations within each statistics sample can be set to a
///   predetermined [`u32`] value via the `sample_size` option. This may be
///   overridden at runtime using either the `DIVAN_SAMPLE_SIZE` environment
///   variable or `--sample-size` CLI argument.
///
///   ```
///   #[divan::bench(sample_size = 1000)]
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
///   # #[cfg(not(miri))] divan::main();
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
///     # #[cfg(not(miri))]
///     divan::main();
/// }
/// ```
///
/// See [`#[divan::bench]`](macro@bench) for more examples.
pub fn main() {
    Divan::default().config_with_args().run();
}
