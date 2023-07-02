//! Macros for [Divan](https://github.com/nvzqz/divan), a statistically-comfy
//! benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).

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
///
/// # Options
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
///   fn add() {
///       // ...
///   }
///   ```
#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut divan_crate = None::<syn::Path>;

    let attr_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("crate") {
            divan_crate = Some(meta.value()?.parse()?);
            Ok(())
        } else {
            Err(meta.error("unsupported 'bench' property"))
        }
    });

    syn::parse_macro_input!(attr with attr_parser);

    // All access to `divan` must go through this path.
    let _divan_crate = divan_crate.unwrap_or_else(|| syn::parse_quote!(::divan));

    item
}
