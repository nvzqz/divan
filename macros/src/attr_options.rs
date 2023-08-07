use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    spanned::Spanned,
    Expr, Ident, Token, Type,
};

use crate::Macro;

/// Values from parsed options shared between `#[divan::bench]` and
/// `#[divan::bench_group]`.
///
/// The `crate` option is not included because it is only needed to get proper
/// access to `__private`.
pub(crate) struct AttrOptions<'a> {
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

    /// Generic types over which to instantiate benchmark functions.
    ///
    /// Currently only supported by `#[divan::bench]`.
    pub generic_types: Option<GenericTypes>,

    /// Generic `const` array over which to instantiate benchmark functions.
    ///
    /// Currently only supported by `#[divan::bench]` and mutually exclusive
    /// with `generic_types`.
    pub generic_consts: Option<(&'a syn::ConstParam, Expr)>,

    /// Options used directly as `BenchOptions` fields.
    ///
    /// Option reuse is handled by the compiler ensuring `BenchOptions` fields
    /// are not repeated.
    pub bench_options: Vec<(Ident, Expr)>,
}

impl<'a> AttrOptions<'a> {
    pub fn parse(tokens: TokenStream, target_macro: Macro<'a>) -> Result<Self, TokenStream> {
        let macro_name = target_macro.name();

        let mut divan_crate = None::<syn::Path>;
        let mut name_expr = None::<Expr>;
        let mut bench_options = Vec::new();

        let mut generic_types = None::<GenericTypes>;

        let mut generic_const_param = None::<&'a syn::ConstParam>;
        let mut generic_consts = None::<Expr>;

        let attr_parser = syn::meta::parser(|meta| {
            let Some(ident) = meta.path.get_ident() else {
                return Err(meta.error(format_args!("unsupported '{macro_name}' option")));
            };

            let ident_name = ident.to_string();
            let ident_name = ident_name.strip_prefix("r#").unwrap_or(&ident_name);

            let repeat_error =
                || Err(meta.error(format_args!("repeated '{macro_name}' option '{ident_name}'")));

            let unsupported_error = || {
                Err(meta.error(format_args!("unsupported '{macro_name}' option '{ident_name}'")))
            };

            let feature_mutex_error = |other_option: &str| {
                Err(meta.error(format_args!(
                    "'{macro_name}' option '{ident_name}' is not supported with '{other_option}'"
                )))
            };

            macro_rules! parse {
                ($storage:expr) => {
                    if $storage.is_none() {
                        $storage = Some(meta.value()?.parse()?);
                    } else {
                        return repeat_error();
                    }
                };
            }

            match ident_name {
                "crate" => parse!(divan_crate),
                "name" => parse!(name_expr),
                "types" => {
                    match target_macro {
                        Macro::Bench { fn_sig } => {
                            if fn_sig.generics.type_params().next().is_none() {
                                return Err(meta.error(format_args!("generic type required for '{macro_name}' option '{ident_name}'")));
                            }
                        }
                        _ => return unsupported_error(),
                    }

                    if generic_consts.is_some() {
                        return feature_mutex_error("consts");
                    }

                    parse!(generic_types);
                }
                "consts" => {
                    match target_macro {
                        Macro::Bench { fn_sig } => {
                            if let Some(param) = fn_sig.generics.const_params().next() {
                                generic_const_param = Some(param);
                            } else {
                                return Err(meta.error(format_args!("generic constant required for '{macro_name}' option '{ident_name}'")));
                            }
                        }
                        _ => return unsupported_error(),
                    }

                    if generic_types.is_some() {
                        return feature_mutex_error("types");
                    }

                    parse!(generic_consts);
                }
                _ => {
                    let value: Expr = match meta.value() {
                        Ok(value) => value.parse()?,

                        // If the option is missing `=`, use a `true` literal.
                        Err(_) => Expr::Lit(syn::ExprLit {
                            lit: syn::LitBool::new(true, meta.path.span()).into(),
                            attrs: Vec::new(),
                        }),
                    };

                    bench_options.push((ident.clone(), value));
                }
            }

            Ok(())
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
            generic_types,
            generic_consts: generic_const_param.and_then(|param| Some((param, generic_consts?))),
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
                let option_name = option.to_string();
                let option_name = option_name.strip_prefix("r#").unwrap_or(&option_name);

                let wrapped_value: proc_macro2::TokenStream;
                let value: &dyn ToTokens = match option_name {
                    // If the option is a `Duration`, use `IntoDuration` to be
                    // polymorphic over `Duration` or `u64`/`f64` seconds.
                    "min_time" | "max_time" => {
                        wrapped_value =
                            quote! { #private_mod::IntoDuration::into_duration(#value) };
                        &wrapped_value
                    }

                    _ => value,
                };

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

/// Generic types over which to instantiate benchmark functions.
pub enum GenericTypes {
    /// List of types, e.g. `[i32, String, ()]`.
    List(Vec<proc_macro2::TokenStream>),
}

impl Parse for GenericTypes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        Ok(Self::List(
            content
                .parse_terminated(Type::parse, Token![,])?
                .into_iter()
                .map(|ty| ty.into_token_stream())
                .collect(),
        ))
    }
}

impl GenericTypes {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::List(list) => list.is_empty(),
        }
    }
}
