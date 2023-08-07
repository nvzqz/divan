//! Macros for [Divan](https://github.com/nvzqz/divan), a statistically-comfy
//! benchmarking library brought to you by [Nikolai Vazquez](https://hachyderm.io/@nikolai).
//!
//! See [`divan`](https://docs.rs/divan) crate for documentation.

use proc_macro::TokenStream;
use quote::{quote, ToTokens};

mod attr_options;

use attr_options::*;
use syn::Expr;

#[derive(Clone, Copy)]
enum Macro<'a> {
    Bench { fn_sig: &'a syn::Signature },
    BenchGroup,
}

impl Macro<'_> {
    fn name(&self) -> &'static str {
        match self {
            Self::Bench { .. } => "bench",
            Self::BenchGroup => "bench_group",
        }
    }
}

#[proc_macro_attribute]
pub fn bench(options: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = item.clone();
    let fn_item = syn::parse_macro_input!(fn_item as syn::ItemFn);
    let fn_sig = &fn_item.sig;

    let options = match AttrOptions::parse(options, Macro::Bench { fn_sig }) {
        Ok(options) => options,
        Err(compile_error) => return compile_error,
    };

    // Items needed by generated code.
    let AttrOptions { private_mod, linkme_crate, .. } = &options;

    let fn_ident = &fn_sig.ident;
    let fn_name = fn_ident.to_string();
    let fn_name_pretty = fn_name.strip_prefix("r#").unwrap_or(&fn_name);

    let ignore = fn_item.attrs.iter().any(|attr| attr.meta.path().is_ident("ignore"));

    // If the function is `extern "ABI"`, it is wrapped in a Rust-ABI function.
    let is_extern_abi = fn_sig.abi.is_some();

    let fn_args = &fn_sig.inputs;

    // Prefixed with "__" to prevent IDEs from recommending using this symbol.
    //
    // The static is local to intentionally cause a compile error if this
    // attribute is used multiple times on the same function.
    let static_ident = syn::Ident::new(
        &format!("__DIVAN_BENCH_{}", fn_name_pretty.to_uppercase()),
        fn_ident.span(),
    );

    let meta = entry_meta_expr(&fn_name, &options, ignore);

    let make_bench_fn = |generic: Option<&dyn ToTokens>, generic_const: bool| {
        let fn_expr = match generic {
            Some(generic) => {
                if generic_const {
                    quote! { #fn_ident::<{ #generic }> }
                } else {
                    quote! { #fn_ident::<#generic> }
                }
            }
            None => fn_ident.to_token_stream(),
        };

        match (is_extern_abi, fn_args.is_empty()) {
            (false, false) => fn_expr,
            (false, true) => quote! { |divan| divan.bench(#fn_expr) },
            (true, false) => quote! { |divan| #fn_expr(divan) },
            (true, true) => quote! { |divan| divan.bench(|| #fn_expr()) },
        }
    };

    let generated_items = match (&options.generic_types, &options.generic_consts) {
        // No generic types; generate a simple benchmark entry.
        (None, None) => {
            let bench_fn = make_bench_fn(None, false);
            quote! {
                #[#linkme_crate::distributed_slice(#private_mod::BENCH_ENTRIES)]
                #[linkme(crate = #linkme_crate)]
                #[doc(hidden)]
                static #static_ident: #private_mod::BenchEntry = #private_mod::BenchEntry {
                    meta: #meta,
                    bench: #bench_fn,
                };
            }
        }

        // `types = []` specified; generate nothing.
        (Some(generic_types), _) if generic_types.is_empty() => Default::default(),

        // Generate a benchmark group entry with generic benchmark entries.
        (Some(GenericTypes::List(generic_types)), _) => {
            let generic_benches = generic_types.iter().map(|ty| {
                let bench = make_bench_fn(Some(ty), false);
                quote! {
                    #private_mod::GenericBenchEntry::Type {
                        group: &#static_ident,
                        bench: #bench,
                        get_type_name: #private_mod::any::type_name::<#ty>,
                        get_type_id: #private_mod::any::TypeId::of::<#ty>,
                    }
                }
            });

            quote! {
                #[#linkme_crate::distributed_slice(#private_mod::GROUP_ENTRIES)]
                #[linkme(crate = #linkme_crate)]
                #[doc(hidden)]
                static #static_ident: #private_mod::GroupEntry = #private_mod::GroupEntry {
                    meta: #meta,
                    generic_benches: #private_mod::Some(
                        &[#(#generic_benches),*]
                    ),
                };
            }
        }

        // `consts = []` specified; generate nothing.
        (None, Some((_, Expr::Array(generic_consts)))) if generic_consts.elems.is_empty() => {
            Default::default()
        }

        // Generate a benchmark group entry with generic benchmark entries over
        // an array literal of constants.
        (None, Some((_, Expr::Array(generic_consts)))) => {
            let count = generic_consts.elems.len();

            let generic_benches = generic_consts.elems.iter().map(|expr| {
                let bench = make_bench_fn(Some(expr), true);
                quote! {
                    #private_mod::GenericBenchEntry::Const {
                        group: &#static_ident,
                        bench: #bench,
                        value: #private_mod::EntryConst::new(&#expr),
                    }
                }
            });

            quote! {
                #[#linkme_crate::distributed_slice(#private_mod::GROUP_ENTRIES)]
                #[linkme(crate = #linkme_crate)]
                #[doc(hidden)]
                static #static_ident: #private_mod::GroupEntry = #private_mod::GroupEntry {
                    meta: #meta,
                    generic_benches: #private_mod::Some({
                        // `static` is necessary because `EntryConst` uses
                        // interior mutability to cache the `ToString` result.
                        static __DIVAN_GENERIC_BENCHES: [#private_mod::GenericBenchEntry; #count] = [#(#generic_benches),*];
                        &__DIVAN_GENERIC_BENCHES
                    }),
                };
            }
        }

        // Generate a benchmark group entry with generic benchmark entries over
        // an expression of constants.
        //
        // This is limited to a maximum of 20 because we need some constant to
        // instantiate each function instance.
        (None, Some((const_param, generic_consts))) => {
            // The maximum number of elements for non-array expressions.
            const MAX_EXTERN_COUNT: usize = 20;

            let const_type = &const_param.ty;

            let generic_benches = (0..MAX_EXTERN_COUNT).map(|i| {
                let expr = quote! {
                    // Fallback to the first constant if out of bounds.
                    __DIVAN_CONSTS[if #i < __DIVAN_CONST_COUNT { #i } else { 0 }]
                };

                let bench = make_bench_fn(Some(&expr), true);
                quote! {
                    {
                        #private_mod::GenericBenchEntry::Const {
                            group: &#static_ident,
                            bench: #bench,
                            value: #private_mod::EntryConst::new(&#expr),
                        }
                    }
                }
            });

            quote! {
                #[#linkme_crate::distributed_slice(#private_mod::GROUP_ENTRIES)]
                #[linkme(crate = #linkme_crate)]
                #[doc(hidden)]
                static #static_ident: #private_mod::GroupEntry = #private_mod::GroupEntry {
                    meta: #meta,
                    generic_benches: #private_mod::Some({
                        const __DIVAN_CONST_COUNT: usize = __DIVAN_CONSTS.len();
                        const __DIVAN_CONSTS: &[#const_type] = &#generic_consts;

                        // `static` is necessary because `EntryConst` uses
                        // interior mutability to cache the `ToString` result.
                        static __DIVAN_GENERIC_BENCHES: [#private_mod::GenericBenchEntry; __DIVAN_CONST_COUNT]
                            = match #private_mod::shrink_array([#(#generic_benches),*]) {
                                #private_mod::Some(array) => array,
                                _ => panic!("external 'consts' cannot contain more than 20 values"),
                            };

                        &__DIVAN_GENERIC_BENCHES
                    }),
                };
            }
        }
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(generated_items));
    result
}

#[proc_macro_attribute]
pub fn bench_group(options: TokenStream, item: TokenStream) -> TokenStream {
    let options = match AttrOptions::parse(options, Macro::BenchGroup) {
        Ok(options) => options,
        Err(compile_error) => return compile_error,
    };

    // Items needed by generated code.
    let AttrOptions { private_mod, linkme_crate, .. } = &options;

    // TODO: Make module parsing cheaper by parsing only the necessary parts.
    let mod_item = item.clone();
    let mod_item = syn::parse_macro_input!(mod_item as syn::ItemMod);

    let mod_ident = &mod_item.ident;
    let mod_name = mod_ident.to_string();
    let mod_name_pretty = mod_name.strip_prefix("r#").unwrap_or(&mod_name);

    // TODO: Fix `unused_attributes` warning when using `#[ignore]` on a module.
    let ignore = mod_item.attrs.iter().any(|attr| attr.meta.path().is_ident("ignore"));

    // Prefixed with "__" to prevent IDEs from recommending using this symbol.
    let static_ident = syn::Ident::new(
        &format!("__DIVAN_GROUP_{}", mod_name_pretty.to_uppercase()),
        mod_ident.span(),
    );

    let meta = entry_meta_expr(&mod_name, &options, ignore);

    let generated_items = quote! {
        // By having the static be local, we cause a compile error if this
        // attribute is used multiple times on the same function.
        #[#linkme_crate::distributed_slice(#private_mod::GROUP_ENTRIES)]
        #[linkme(crate = #linkme_crate)]
        #[doc(hidden)]
        static #static_ident: #private_mod::GroupEntry = #private_mod::GroupEntry {
            meta: #meta,
            generic_benches: #private_mod::None,
        };
    };

    // Append our generated code to the existing token stream.
    let mut result = item;
    result.extend(TokenStream::from(generated_items));
    result
}

/// Constructs an `EntryMeta` expression.
fn entry_meta_expr(
    raw_name: &str,
    options: &AttrOptions,
    ignore: bool,
) -> proc_macro2::TokenStream {
    let AttrOptions { private_mod, std_crate, .. } = &options;

    let raw_name_pretty = raw_name.strip_prefix("r#").unwrap_or(raw_name);

    let display_name: &dyn ToTokens = match &options.name_expr {
        Some(name) => name,
        None => &raw_name_pretty,
    };

    let bench_options_fn = options.bench_options_fn();

    quote! {
        #private_mod::EntryMeta {
            raw_name: #raw_name,
            display_name: #display_name,
            module_path: #std_crate::module_path!(),

            // `Span` location info is nightly-only, so use macros.
            location: #private_mod::EntryLocation {
                file: #std_crate::file!(),
                line: #std_crate::line!(),
                col: #std_crate::column!(),
            },

            ignore: #ignore,

            bench_options: #bench_options_fn,
        }
    }
}
