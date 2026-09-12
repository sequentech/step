// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::quote;
use syn::{parse_quote, GenericArgument, ItemFn, PathArguments, ReturnType, Type};

#[cfg(test)]
#[path = "../tests/support/expansion.rs"]
mod tests;

/// Convert a function's returned error using `Into::into`.
///
/// Accepts `Result<T, E>` and `Result<T>` aliases with a default error type,
/// including qualified paths. Other return types remain unchanged. The body
/// keeps its original error context for both `return` and `?`; synchronous and
/// asynchronous functions share this contract. Const functions cannot use the
/// non-const conversion and receive a compiler diagnostic.
#[proc_macro_attribute]
pub fn wrap_map_err(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_attribute(attr.into(), item.into()).into()
}

/// Convert parser errors into ordinary compiler diagnostics, preserving their
/// source spans instead of panicking inside the compiler's macro process.
fn expand_attribute(attr: Tokens, item: Tokens) -> Tokens {
    transform(attr, item).unwrap_or_else(syn::Error::into_compile_error)
}

fn transform(attr: Tokens, item: Tokens) -> syn::Result<Tokens> {
    let mut function: ItemFn = syn::parse2(item)?;
    let target_error: Type = syn::parse2(attr)?;
    let Some((original_type, success_type)) = result_types(&function.sig.output) else {
        return Ok(quote! { #function });
    };
    if let Some(const_token) = function.sig.constness {
        return Err(syn::Error::new_spanned(
            const_token,
            "wrap_map_err cannot convert errors in a const function",
        ));
    }

    let original_return = &function.sig.output;
    let original_body = &function.block;
    let body = if function.sig.asyncness.is_some() {
        // An async block gives early returns their own boundary while keeping
        // awaits lazy. Its result annotation also preserves '?' conversions
        // through the original error type, before the outer conversion.
        quote! { let result: #original_type = (async #original_body).await; }
    } else {
        quote! { let result = (|| #original_return #original_body)(); }
    };

    function.block = Box::new(parse_quote!({
        #body
        result.map_err(::core::convert::Into::into)
    }));
    function.sig.output = parse_quote!(-> ::core::result::Result<#success_type, #target_error>);
    Ok(quote! { #function })
}

/// Inspect syntax only. A proc macro cannot resolve arbitrary type aliases, so
/// only a path ending in Result with one or two type arguments is recognized.
fn result_types(output: &ReturnType) -> Option<(&Type, &Type)> {
    let ReturnType::Type(_, return_type) = output else {
        return None;
    };
    let Type::Path(path) = return_type.as_ref() else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let arguments: Vec<_> = arguments.args.iter().collect();
    match arguments.as_slice() {
        [GenericArgument::Type(success)]
        | [GenericArgument::Type(success), GenericArgument::Type(_)] => {
            Some((return_type.as_ref(), success))
        }
        _ => None,
    }
}
