//! # umm_derive
//!
//! Defines some proc macros to make exporting functions to rhai easier.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use proc_macro::TokenStream;
use quote::{
    format_ident,
    quote,
    ToTokens,
};
use syn::{
    parse_macro_input,
    punctuated::Punctuated,
    FnArg,
    Token,
};

#[proc_macro_attribute]
/// Generates a version of a fallible function (that uses anyhow Result) that
/// returns an EvalAltResult instead.
///
/// * `input`: a token stream for a function that returns an anyhow::Result
pub fn generate_rhai_variant(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);
    let og_fn = input.to_token_stream();
    let fn_name = input.sig.ident;
    let new_fn_name = format_ident!("{}_script", fn_name);

    let sig_args = input.sig.inputs;
    let mut is_impl_self_fn = false;
    let mut is_impl_fn = attr.to_string() == "Impl";
    let mut args = Punctuated::<_, Token![,]>::new();
    for arg in sig_args.clone().into_iter() {
        let arg = match arg {
            FnArg::Receiver(_) => {
                is_impl_self_fn = true;
                is_impl_fn = true;
                continue;
            }
            FnArg::Typed(a) => a.pat,
        };
        args.push(arg);
    }

    let output = {
        let output = input.sig.output.into_token_stream().to_string();

        let output = output.replace("-> ", "").replace(' ', "");

        if &output == "Result<()>" {
            quote!(-> Result<(), Box<EvalAltResult>>)
        } else if output.starts_with("Result<") {
            if output.replace("Result<", "").starts_with("Vec<") {
                let inner_type = format_ident!(
                    "{}",
                    output
                        .replace("Result<", "")
                        .replace("Vec<", "")
                        .replace('>', "")
                );

                quote! {-> Result<Vec<#inner_type>, Box<EvalAltResult>>}
            } else {
                let inner_type =
                    format_ident!("{}", output.replace("Result<", "").replace('>', ""));

                quote! {-> Result<#inner_type, Box<EvalAltResult>>}
            }
        } else {
            quote! {}
        }
    };

    // Build the output, possibly using quasi-quotation
    let expanded = if is_impl_self_fn {
        quote! {
        #og_fn

        /// Macro generated version that returns EvalAltResult.
        /// This allows the function to be used in scripts.
        pub fn #new_fn_name(#sig_args) #output {
            // TODO: create an args that has only names serpated by commas
            match self.#fn_name(#args) {
                Ok(res) => Ok(res),
                Err(e) => Err(e.to_string().into()),
            }
        }
        }
    } else if is_impl_fn {
        quote! {
        #og_fn

        /// Macro generated version that returns EvalAltResult.
        /// This allows the function to be used in scripts.
        pub fn #new_fn_name(#sig_args) #output {
            // TODO: create an args that has only names serpated by commas
            match Self::#fn_name(#args) {
                Ok(res) => Ok(res),
                Err(e) => Err(e.to_string().into()),
            }
        }
        }
    } else {
        quote! {
        #og_fn

        /// Macro generated version that returns EvalAltResult.
        /// This allows the function to be used in scripts.
        pub fn #new_fn_name(#sig_args) #output {
            // TODO: create an args that has only names serpated by commas
            match #fn_name(#args) {
                Ok(res) => Ok(res),
                Err(e) => Err(e.to_string().into()),
            }
        }
        }
    };

    // println!("{}", expanded);
    // Hand the output tokens back to the compiler
    expanded.into()
}
