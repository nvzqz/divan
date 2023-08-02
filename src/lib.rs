//! [bench_attr]: macro@bench
//! [bench_attr_examples]: macro@bench#examples
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![allow(unused_unsafe, clippy::needless_doctest_main)]

// Used by generated code. Not public API and thus not subject to SemVer.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

mod bench;
mod cli;
mod compile_fail;
mod config;
mod defer;
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
/// # Drop
///
/// When a benchmarked function returns a value, it will not be [dropped][Drop]
/// until after the current sample loop is finished. This allows for more
/// precise timing measurements.
///
/// Note that there is an inherent memory cost to defer drop, including
/// allocations inside not-yet-dropped values. Also, if the benchmark
/// [panics](macro@std::panic), the values will never be dropped.
///
/// The following example benchmarks will only measure [`String`] construction
/// time, but not deallocation time:
///
/// ```
/// use divan::{Bencher, black_box};
///
/// #[divan::bench]
/// fn freestanding() -> String {
///     black_box("hello").to_uppercase()
/// }
///
/// #[divan::bench]
/// fn contextual(bencher: Bencher) {
///     // Setup:
///     let s: String = // ...
///     # String::new();
///
///     bencher.bench(|| -> String {
///         black_box(&s).to_lowercase()
///     });
/// }
/// ```
///
/// If the returned value *does not* need to be dropped, there is no memory
/// cost. Because of this, the following example benchmarks are equivalent:
///
/// ```
/// #[divan::bench]
/// fn with_return() -> i32 {
///     let n: i32 = // ...
///     # 0;
///     n
/// }
///
/// #[divan::bench]
/// fn without_return() {
///     let n: i32 = // ...
///     # 0;
///     divan::black_box(n);
/// }
/// ```
///
/// # Options
///
/// - [`name`]
/// - [`crate`]
/// - [`sample_count`]
/// - [`sample_size`]
/// - [`min_time`]
/// - [`max_time`]
/// - [`skip_input_time`]
/// - [`ignore`]
///
/// ## `name`
/// [`name`]: #name
///
/// By default, the benchmark uses the function's name. It can be overridden via
/// the [`name`] option:
///
/// ```
/// #[divan::bench(name = "my_add")]
/// fn add() -> i32 {
///     // Will appear as "crate_name::my_add".
///     # 0
/// }
/// ```
///
/// ## `crate`
/// [`crate`]: #crate
///
/// The path to the specific `divan` crate instance used by this macro's
/// generated code can be specified via the [`crate`] option. This is applicable
/// when using `divan` via a macro from your own crate.
///
/// ```
/// extern crate divan as sofa;
///
/// #[::sofa::bench(crate = ::sofa)]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// ## `sample_count`
/// [`sample_count`]: #sample_count
///
/// The number of statistical sample recordings can be set to a predetermined
/// [`u32`] value via the [`sample_count`] option. This may be overridden at
/// runtime using either the `DIVAN_SAMPLE_COUNT` environment variable or
/// `--sample-count` CLI argument.
///
/// ```
/// #[divan::bench(sample_count = 1000)]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// ## `sample_size`
/// [`sample_size`]: #sample_size
///
/// The number iterations within each statistics sample can be set to a
/// predetermined [`u32`] value via the [`sample_size`] option. This may be
/// overridden at runtime using either the `DIVAN_SAMPLE_SIZE` environment
/// variable or `--sample-size` CLI argument.
///
/// ```
/// #[divan::bench(sample_size = 1000)]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// ## `min_time`
/// [`min_time`]: #min_time
///
/// The minimum time spent benchmarking each function can be set to a
/// predetermined [`Duration`] via the [`min_time`] option. This may be
/// overridden at runtime using either the `DIVAN_MIN_TIME` environment variable
/// or `--min-time` CLI argument.
///
/// Unless [`skip_input_time`] is set, this includes input generation time.
///
/// ```
/// use std::time::Duration;
///
/// #[divan::bench(min_time = Duration::from_secs(3))]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// For convenience, [`min_time`] can also be set with seconds as [`u64`] or
/// [`f64`]. Invalid values will cause a panic at runtime.
///
/// ```
/// #[divan::bench(min_time = 2)]
/// fn int_secs() -> i32 {
///     // ...
///     # 0
/// }
///
/// #[divan::bench(min_time = 1.5)]
/// fn float_secs() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// ## `max_time`
/// [`max_time`]: #max_time
///
/// The maximum time spent benchmarking each function can be set to a
/// predetermined [`Duration`] via the [`max_time`] option. This may be
/// overridden at runtime using either the `DIVAN_MAX_TIME` environment variable
/// or `--max-time` CLI argument.
///
/// Unless [`skip_input_time`] is set, this includes input generation time.
///
/// If `min_time > max_time`, then [`max_time`] has priority and [`min_time`]
/// will not be reached.
///
/// ```
/// use std::time::Duration;
///
/// #[divan::bench(max_time = Duration::from_secs(5))]
/// fn add() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// For convenience, like [`min_time`], [`max_time`] can also be set with
/// seconds as [`u64`] or [`f64`]. Invalid values will cause a panic at runtime.
///
/// ```
/// #[divan::bench(max_time = 8)]
/// fn int_secs() -> i32 {
///     // ...
///     # 0
/// }
///
/// #[divan::bench(max_time = 9.5)]
/// fn float_secs() -> i32 {
///     // ...
///     # 0
/// }
/// ```
///
/// ## `skip_input_time`
/// [`skip_input_time`]: #skip_input_time
///
/// When [`min_time`] or [`max_time`] is set, time spent generating inputs is
/// included by default. Enabling the [`skip_input_time`] option will make only
/// the time spent actually running the benchmarked function be considered. This
/// may be overridden at runtime using either the `DIVAN_SKIP_INPUT_TIME`
/// environment variable or `--skip-input-time` CLI argument.
///
/// In the following example, [`max_time`] will only consider the time spent
/// running `measured_function`:
///
/// ```
/// # fn generate_input() {}
/// # fn measured_function(_: ()) {}
/// #[divan::bench(max_time = 5, skip_input_time)]
/// fn bench(bencher: divan::Bencher) {
///     bencher
///         .with_inputs(|| generate_input())
///         .bench_values(|input| measured_function(input));
/// }
/// ```
///
/// This option can be set to an explicit [`bool`] value to override parent
/// values:
///
/// ```
/// #[divan::bench(max_time = 5, skip_input_time = false)]
/// fn bench(bencher: divan::Bencher) {
///     // ...
/// }
/// ```
///
/// ## `ignore`
/// [`ignore`]: #ignore
///
/// Like [`#[test]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-test-attribute),
/// `#[divan::bench]` functions can use [`#[ignore]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-ignore-attribute):
///
/// ```
/// #[divan::bench]
/// #[ignore = "not yet implemented"]
/// fn todo() {
///     unimplemented!();
/// }
/// # divan::main();
/// ```
///
/// [`Duration`]: std::time::Duration
pub use divan_macros::bench;

/// Registers a benchmarking group.
///
/// # Examples
///
/// This is used for setting [options] shared across
/// [`#[divan::bench]`](macro@bench) functions in the same module:
///
/// ```
/// #[divan::bench_group(
///     sample_count = 100,
///     sample_size = 500,
/// )]
/// mod math {
///     use divan::black_box;
///
///     #[divan::bench]
///     fn add() -> i32 {
///         black_box(1) + black_box(42)
///     }
///
///     #[divan::bench]
///     fn div() -> i32 {
///         black_box(1) / black_box(42)
///     }
/// }
///
/// fn main() {
///     // Run `math::add` and `math::div` benchmarks:
///     divan::main();
/// }
/// ```
///
/// Benchmarking [options] set on parent groups cascade into child groups and
/// their benchmarks:
///
/// ```
/// #[divan::bench_group(
///     sample_count = 100,
///     sample_size = 500,
/// )]
/// mod parent {
///     #[divan::bench_group(sample_size = 1)]
///     mod child1 {
///         #[divan::bench]
///         fn bench() {
///             // Will be sampled 100 times with 1 iteration per sample.
///         }
///     }
///
///     #[divan::bench_group(sample_count = 42)]
///     mod child2 {
///         #[divan::bench]
///         fn bench() {
///             // Will be sampled 42 times with 500 iterations per sample.
///         }
///     }
///
///     mod child3 {
///         #[divan::bench(sample_count = 1)]
///         fn bench() {
///             // Will be sampled 1 time with 500 iterations per sample.
///         }
///     }
/// }
/// ```
///
/// Applying this attribute multiple times to the same item will cause a compile
/// error:
///
/// ```compile_fail
/// #[divan::bench_group]
/// #[divan::bench_group]
/// mod math {
///     // ...
/// }
/// ```
///
/// # Options
/// [options]: #options
///
/// - [`name`]
/// - [`crate`]
/// - [`sample_count`]
/// - [`sample_size`]
/// - [`min_time`]
/// - [`max_time`]
/// - [`skip_input_time`]
/// - [`ignore`]
///
/// ## `name`
/// [`name`]: #name
///
/// By default, the benchmark group uses the module's name. It can be overridden
/// via the `name` option:
///
/// ```
/// #[divan::bench_group(name = "my_math")]
/// mod math {
///     #[divan::bench(name = "my_add")]
///     fn add() -> i32 {
///         // Will appear as "crate_name::my_math::my_add".
///         # 0
///     }
/// }
/// ```
///
/// ## `crate`
/// [`crate`]: #crate
///
/// The path to the specific `divan` crate instance used by this macro's
/// generated code can be specified via the [`crate`] option. This is applicable
/// when using `divan` via a macro from your own crate.
///
/// ```
/// extern crate divan as sofa;
///
/// #[::sofa::bench_group(crate = ::sofa)]
/// mod math {
///     #[::sofa::bench(crate = ::sofa)]
///     fn add() -> i32 {
///         // ...
///         # 0
///     }
/// }
/// ```
///
/// ## `sample_count`
/// [`sample_count`]: #sample_count
///
/// The number of statistical sample recordings can be set to a predetermined
/// [`u32`] value via the [`sample_count`] option. This may be overridden at
/// runtime using either the `DIVAN_SAMPLE_COUNT` environment variable or
/// `--sample-count` CLI argument.
///
/// ```
/// #[divan::bench_group(sample_count = 1000)]
/// mod math {
///     #[divan::bench]
///     fn add() -> i32 {
///         // ...
///         # 0
///     }
/// }
/// ```
///
/// ## `sample_size`
/// [`sample_size`]: #sample_size
///
/// The number iterations within each statistical sample can be set to a
/// predetermined [`u32`] value via the [`sample_size`] option. This may be
/// overridden at runtime using either the `DIVAN_SAMPLE_SIZE` environment
/// variable or `--sample-size` CLI argument.
///
/// ```
/// #[divan::bench_group(sample_size = 1000)]
/// mod math {
///     #[divan::bench]
///     fn add() -> i32 {
///         // ...
///         # 0
///     }
/// }
/// ```
///
/// ## `min_time`
/// [`min_time`]: #min_time
///
/// The minimum time spent benchmarking each function can be set to a
/// predetermined [`Duration`] via the [`min_time`] option. This may be
/// overridden at runtime using either the `DIVAN_MIN_TIME` environment variable
/// or `--min-time` CLI argument.
///
/// Unless [`skip_input_time`] is set, this includes input generation time.
///
/// ```
/// use std::time::Duration;
///
/// #[divan::bench_group(min_time = Duration::from_secs(3))]
/// mod math {
///     #[divan::bench]
///     fn add() -> i32 {
///         // ...
///         # 0
///     }
/// }
/// ```
///
/// For convenience, [`min_time`] can also be set with seconds as [`u64`] or
/// [`f64`]. Invalid values will cause a panic at runtime.
///
/// ```
/// #[divan::bench_group(min_time = 2)]
/// mod int_secs {
///     // ...
/// }
///
/// #[divan::bench_group(min_time = 1.5)]
/// mod float_secs {
///     // ...
/// }
/// ```
///
/// ## `max_time`
/// [`max_time`]: #max_time
///
/// The maximum time spent benchmarking each function can be set to a
/// predetermined [`Duration`] via the [`max_time`] option. This may be
/// overridden at runtime using either the `DIVAN_MAX_TIME` environment variable
/// or `--max-time` CLI argument.
///
/// Unless [`skip_input_time`] is set, this includes input generation time.
///
/// If `min_time > max_time`, then [`max_time`] has priority and [`min_time`]
/// will not be reached.
///
/// ```
/// use std::time::Duration;
///
/// #[divan::bench_group(max_time = Duration::from_secs(5))]
/// mod math {
///     #[divan::bench]
///     fn add() -> i32 {
///         // ...
///         # 0
///     }
/// }
/// ```
///
/// For convenience, like [`min_time`], [`max_time`] can also be set with
/// seconds as [`u64`] or [`f64`]. Invalid values will cause a panic at runtime.
///
/// ```
/// #[divan::bench_group(max_time = 8)]
/// mod int_secs {
///     // ...
/// }
///
/// #[divan::bench_group(max_time = 9.5)]
/// mod float_secs {
///     // ...
/// }
/// ```
///
/// ## `skip_input_time`
/// [`skip_input_time`]: #skip_input_time
///
/// When [`min_time`] or [`max_time`] is set, time spent generating inputs is
/// included by default. Enabling the [`skip_input_time`] option will make only
/// the time spent actually running the benchmarked function be considered. This
/// may be overridden at runtime using either the `DIVAN_SKIP_INPUT_TIME`
/// environment variable or `--skip-input-time` CLI argument.
///
/// In the following example, [`max_time`] will only consider the time spent
/// running `measured_function`:
///
/// ```
/// #[divan::bench_group(skip_input_time)]
/// mod group {
///     # fn generate_input() {}
///     # fn measured_function(_: ()) {}
///     #[divan::bench(max_time = 5)]
///     fn bench(bencher: divan::Bencher) {
///         bencher
///             .with_inputs(|| generate_input())
///             .bench_values(|input| measured_function(input));
///     }
/// }
/// ```
///
/// This option can be set to an explicit [`bool`] value to override parent
/// values:
///
/// ```
/// #[divan::bench_group(skip_input_time = false)]
/// mod group {
///     // ...
/// }
/// ```
///
/// ## `ignore`
/// [`ignore`]: #ignore
///
/// Like [`#[test]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-test-attribute)
/// and [`#[divan::bench]`](macro@bench), `#[divan::bench_group]` functions can
/// use [`#[ignore]`](https://doc.rust-lang.org/reference/attributes/testing.html#the-ignore-attribute):
///
/// ```
/// #[divan::bench_group]
/// #[ignore]
/// mod math {
///     #[divan::bench]
///     fn todo() {
///         unimplemented!();
///     }
/// }
/// # divan::main();
/// ```
///
/// [`Duration`]: std::time::Duration
pub use divan_macros::bench_group;

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
    Divan::from_args().main();
}
