//! Macros for [Divan](https://github.com/nvzqz/divan), a statistically-comfy
//! benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).
//!
//! See [`divan`](https://docs.rs/divan) crate for documentation.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};

#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut divan_crate = None::<syn::Path>;
    let mut bench_name_expr = None::<syn::Expr>;

    let mut bench_options = Vec::<(syn::Ident, syn::Expr)>::new();

    let attr_parser = syn::meta::parser(|meta| {
        let repeat_error = || Err(meta.error("repeated 'bench' property"));

        macro_rules! parse {
            ($storage:ident) => {
                if $storage.is_none() {
                    $storage = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    repeat_error()
                }
            };
        }

        if meta.path.is_ident("crate") {
            parse!(divan_crate)
        } else if meta.path.is_ident("name") {
            parse!(bench_name_expr)
        } else if let Some(ident) = meta.path.get_ident() {
            // Option reuse is handled by ensuring fields are not repeated.
            bench_options.push((ident.clone(), meta.value()?.parse()?));
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

    let fn_ident = &fn_item.sig.ident;
    let fn_name = fn_ident.to_string();
    let fn_name_pretty = fn_name.strip_prefix("r#").unwrap_or(&fn_name);

    let mut ignore = false;
    for attr in &fn_item.attrs {
        let path = attr.meta.path();

        if path.is_ident("ignore") {
            ignore = true;
            break;
        }
    }

    let bench_name_expr: &dyn ToTokens = match &bench_name_expr {
        Some(name) => name,
        None => &fn_name_pretty,
    };

    let fn_args = &fn_item.sig.inputs;

    // Directly set fields on `BenchOptions` held by `Context`. This simplifies
    // things by:
    // - Having a single source of truth
    // - Making unknown options a compile error
    //
    // We use `..` (struct update syntax) to ensure that no option is set twice,
    // even if raw identifiers are used. This also has the accidental benefit of
    // Rust Analyzer recognizing fields and emitting suggestions with docs and
    // type info.
    let bench_options = if bench_options.is_empty() {
        // Use the `Default` impl function to slightly reduce code generation in
        // the common case where there's no options set.
        quote! { #private_mod::Default::default }
    } else {
        let options_iter = bench_options.iter().map(|(option, value)| {
            quote! { #option: #private_mod::Some(#value), }
        });
        quote! {
            || {
                #[allow(clippy::needless_update)]
                #private_mod::BenchOptions {
                    #(#options_iter)*
                    ..#private_mod::Default::default()
                }
            }
        }
    };

    let bench_loop = if fn_args.is_empty() {
        // `fn(&mut divan::bench::Context) -> ()`.
        quote! {
            #private_mod::BenchLoop::Static(|__divan_context| {
                __divan_context.bench_loop(#fn_ident)
            })
        }
    } else {
        quote! {
            #private_mod::BenchLoop::Runtime(
                // `fn(divan::Bencher) -> ()`.
                #fn_ident
            )
        }
    };

    // Prefixed with "__" to prevent IDEs from recommending using this symbol.
    let fn_dup_ident =
        syn::Ident::new(&format!("__divan_{fn_name_pretty}_bench_is_duplicate"), fn_ident.span());

    let generated_items = quote! {
        // Causes a compile error if this attribute is used multiple times on
        // the same function.
        #[doc(hidden)]
        #[allow(warnings, clippy::all)]
        fn #fn_dup_ident() {}

        // This `const _` prevents collisions in the current scope by giving us
        // an anonymous scope to place our static in. As a result, this macro
        // can be used multiple times within the same scope.
        #[doc(hidden)]
        const _: () = {
            #[#linkme_crate::distributed_slice(#private_mod::ENTRIES)]
            #[linkme(crate = #linkme_crate)]
            static __DIVAN_BENCH_ENTRY: #private_mod::Entry = #private_mod::Entry {
                name: #bench_name_expr,
                module_path: #std_crate::module_path!(),
                full_path: #std_crate::concat!(#std_crate::module_path!(), "::", #bench_name_expr),

                // `Span` location info is nightly-only, so use macros.
                file: #std_crate::file!(),
                line: #std_crate::line!(),

                ignore: #ignore,

                bench_options: #bench_options,
                bench_loop: #bench_loop,
            };
        };
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(generated_items));
    result
}
