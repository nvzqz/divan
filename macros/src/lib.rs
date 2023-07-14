//! Macros for [Divan](https://github.com/nvzqz/divan), a statistically-comfy
//! benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).
//!
//! See [`divan`](https://docs.rs/divan) crate for documentation.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};

mod attr_options;

use attr_options::*;

#[proc_macro_attribute]
pub fn bench(options: TokenStream, item: TokenStream) -> TokenStream {
    let options = match AttrOptions::parse(options, "bench") {
        Ok(options) => options,
        Err(compile_error) => return compile_error,
    };

    // Items needed by generated code.
    let AttrOptions { private_mod, linkme_crate, std_crate, .. } = &options;

    let fn_item = item.clone();
    let fn_item = syn::parse_macro_input!(fn_item as syn::ItemFn);

    let fn_ident = &fn_item.sig.ident;
    let fn_name = fn_ident.to_string();
    let fn_name_pretty = fn_name.strip_prefix("r#").unwrap_or(&fn_name);

    let name_expr: &dyn ToTokens = match &options.name_expr {
        Some(name) => name,
        None => &fn_name_pretty,
    };

    let ignore = fn_item.attrs.iter().any(|attr| attr.meta.path().is_ident("ignore"));

    let fn_args = &fn_item.sig.inputs;
    let bench_fn = if fn_args.is_empty() {
        quote! { |divan| divan.bench(#fn_ident) }
    } else {
        quote! { #fn_ident }
    };

    // Prefixed with "__" to prevent IDEs from recommending using this symbol.
    let fn_dup_ident =
        syn::Ident::new(&format!("__divan_{fn_name_pretty}_bench_is_duplicate"), fn_ident.span());

    let bench_options_fn = options.bench_options_fn();

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
                display_name: #name_expr,
                raw_name: #fn_name,

                module_path: #std_crate::module_path!(),
                full_path: #std_crate::concat!(#std_crate::module_path!(), "::", #name_expr),

                // `Span` location info is nightly-only, so use macros.
                file: #std_crate::file!(),
                line: #std_crate::line!(),

                ignore: #ignore,

                bench_options: #bench_options_fn,
                bench: #bench_fn,
            };
        };
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(generated_items));
    result
}

#[proc_macro_attribute]
pub fn bench_group(options: TokenStream, item: TokenStream) -> TokenStream {
    let options = match AttrOptions::parse(options, "bench_group") {
        Ok(options) => options,
        Err(compile_error) => return compile_error,
    };

    // Items needed by generated code.
    let AttrOptions { private_mod, linkme_crate, std_crate, .. } = &options;

    // TODO: Make module parsing cheaper by parsing only the necessary parts.
    let mod_item = item.clone();
    let mod_item = syn::parse_macro_input!(mod_item as syn::ItemMod);

    let mod_ident = &mod_item.ident;
    let mod_name = mod_ident.to_string();
    let mod_name_pretty = mod_name.strip_prefix("r#").unwrap_or(&mod_name);

    let name_expr: &dyn ToTokens = match &options.name_expr {
        Some(name) => name,
        None => &mod_name_pretty,
    };

    // TODO: Fix `unused_attributes` warning when using `#[ignore]` on a module.
    let ignore = mod_item.attrs.iter().any(|attr| attr.meta.path().is_ident("ignore"));

    // Prefixed with "__" to prevent IDEs from recommending using this symbol.
    let fn_dup_ident =
        syn::Ident::new(&format!("__divan_{mod_name_pretty}_group_is_duplicate"), mod_ident.span());

    let bench_options_fn = options.bench_options_fn();

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
            #[#linkme_crate::distributed_slice(#private_mod::ENTRY_GROUPS)]
            #[linkme(crate = #linkme_crate)]
            static __DIVAN_GROUP_ENTRY: #private_mod::EntryGroup = #private_mod::EntryGroup {
                display_name: #name_expr,
                raw_name: #mod_name,

                module_path: #std_crate::module_path!(),
                full_path: #std_crate::concat!(#std_crate::module_path!(), "::", #mod_name),

                // `Span` location info is nightly-only, so use macros.
                file: #std_crate::file!(),
                line: #std_crate::line!(),

                ignore: #ignore,

                bench_options: #bench_options_fn,
            };
        };
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(generated_items));
    result
}
