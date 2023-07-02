//! Macros for [Divan](https://github.com/nvzqz/divan), a statistically-comfy
//! benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;

/// Registers a benchmarking function.
///
/// # Examples
///
/// ```
/// use std::hint::black_box as bb;
///
/// #[divan::bench]
/// fn add() -> i32 {
///     bb(1) + bb(42)
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
///   fn add() -> i32 {
///       // ...
///       # 0
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

    // Items needed by generated code.
    //
    // Access to libstd is through a re-export because it's possible (although
    // unlikely) to do `extern crate x as std`, which would cause `::std` to
    // reference crate `x` instead.
    let divan_crate = divan_crate.unwrap_or_else(|| syn::parse_quote!(::divan));
    let private_mod = quote! { #divan_crate::__private };
    let linkme_crate = quote! { #private_mod::linkme };
    let std_crate = quote! { #private_mod::std };

    let fn_item = item.clone();
    let fn_item = syn::parse_macro_input!(fn_item as syn::ItemFn);
    let fn_name = &fn_item.sig.ident;

    // String expression of the benchmark's fully-qualified path.
    let bench_path_expr = {
        let fn_name = fn_name.to_string();
        let fn_name = fn_name.strip_prefix("r#").unwrap_or(&fn_name);

        quote! { #std_crate::concat!(#std_crate::module_path!(), "::", #fn_name) }
    };

    let entry_item = quote! {
        // This `const _` prevents collisions in the current scope by giving us
        // an anonymous scope to place our static in. As a result, this macro
        // can be used multiple times within the same scope.
        #[doc(hidden)]
        const _: () = {
            #[#linkme_crate::distributed_slice(#private_mod::ENTRIES)]
            #[linkme(crate = #linkme_crate)]
            static __DIVAN_BENCH_ENTRY: #private_mod::Entry = #private_mod::Entry {
                path: #bench_path_expr,

                // `Span` location info is nightly-only, so use macros.
                file: #std_crate::file!(),
                line: #std_crate::line!(),

                bench_loop: |__divan_context| {
                    for _ in 0..__divan_context.target_sample_count() {
                        for _ in 0..__divan_context.iter_per_sample {
                            // Discard any result.
                            _ = #std_crate::hint::black_box(#fn_name());
                        }
                        __divan_context.record_sample();
                    }
                },

                get_id: || #std_crate::any::Any::type_id(&#fn_name),
            };
        };
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(entry_item));
    result
}
