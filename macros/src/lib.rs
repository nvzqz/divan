//! \[WIP] Macros for [Divan](https://github.com/nvzqz/divan), a
//! statistically-comfy benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).

#![warn(missing_docs)]

use proc_macro::TokenStream;

/// Registers a benchmarking function.
///
/// # Examples
///
/// ```
/// use std::hint::black_box as bb;
///
/// #[divan::bench]
/// fn add() {
///     bb(bb(1) + bb(42));
/// }
///
/// fn main() {
///     // Run `add` benchmark:
///     divan::main();
/// }
/// ```
#[proc_macro_attribute]
pub fn bench(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
