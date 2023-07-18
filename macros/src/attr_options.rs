use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, Expr, Ident};

/// Values from parsed options shared between `#[divan::bench]` and
/// `#[divan::bench_group]`.
///
/// The `crate` option is not included because it is only needed to get proper
/// access to `__private`.
pub(crate) struct AttrOptions {
    /// `divan::__private`.
    pub private_mod: proc_macro2::TokenStream,

    /// `divan::__private::std`.
    ///
    /// Access to libstd is through a re-export because it's possible (although
    /// unlikely) to do `extern crate x as std`, which would cause `::std` to
    /// reference crate `x` instead.
    pub std_crate: proc_macro2::TokenStream,

    /// `divan::__private::linkme`.
    pub linkme_crate: proc_macro2::TokenStream,

    /// Custom name for the benchmark or group.
    pub name_expr: Option<Expr>,

    /// Options used directly as `BenchOptions` fields.
    ///
    /// Option reuse is handled by the compiler ensuring `BenchOptions` fields
    /// are not repeated.
    pub bench_options: Vec<(Ident, Expr)>,
}

impl AttrOptions {
    pub fn parse(tokens: TokenStream, macro_name: &str) -> Result<Self, TokenStream> {
        let mut divan_crate = None::<syn::Path>;
        let mut name_expr = None::<Expr>;
        let mut bench_options = Vec::new();

        let attr_parser = syn::meta::parser(|meta| {
            let repeat_error = || Err(meta.error(format_args!("repeated '{macro_name}' option")));

            macro_rules! parse {
                ($storage:expr) => {
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
                parse!(name_expr)
            } else if let Some(ident) = meta.path.get_ident() {
                bench_options.push((ident.clone(), meta.value()?.parse()?));
                Ok(())
            } else {
                Err(meta.error(format_args!("unsupported '{macro_name}' option")))
            }
        });

        match attr_parser.parse(tokens) {
            Ok(()) => {}
            Err(error) => return Err(error.into_compile_error().into()),
        }

        let divan_crate = divan_crate.unwrap_or_else(|| syn::parse_quote!(::divan));
        let private_mod = quote! { #divan_crate::__private };

        Ok(Self {
            std_crate: quote! { #private_mod::std },
            linkme_crate: quote! { #private_mod::linkme },
            private_mod,
            name_expr,
            bench_options,
        })
    }

    /// Produces a function expression for creating `BenchOptions`.
    pub fn bench_options_fn(&self) -> proc_macro2::TokenStream {
        let private_mod = &self.private_mod;

        // Directly set fields on `BenchOptions`. This simplifies things by:
        // - Having a single source of truth
        // - Making unknown options a compile error
        //
        // We use `..` (struct update syntax) to ensure that no option is set
        // twice, even if raw identifiers are used. This also has the accidental
        // benefit of Rust Analyzer recognizing fields and emitting suggestions
        // with docs and type info.
        if self.bench_options.is_empty() {
            quote! { #private_mod::None }
        } else {
            let options_iter = self.bench_options.iter().map(|(option, value)| {
                quote! { #option: #private_mod::Some(#value), }
            });
            quote! {
                #private_mod::Some(|| {
                    #[allow(clippy::needless_update)]
                    #private_mod::BenchOptions {
                        #(#options_iter)*
                        ..#private_mod::Default::default()
                    }
                })
            }
        }
    }
}
