// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_quote, ItemFn, ReturnType, Type};

/// The `wrap_map_err` attribute macro transforms a function that returns a
/// `Result<T, E>` into one that returns a `Result<T, ProvidedError>` by mapping
/// the error with `Into::into`.
///
/// If the function does not return a `Result`, the macro leaves it unchanged.
#[proc_macro_attribute]
pub fn wrap_map_err(attr: TokenStream, item: TokenStream) -> TokenStream {
    wrap_map_err_impl(attr.into(), item.into()).into()
}

/// Internal helper that uses proc_macro2 for compatibility in unit tests
fn wrap_map_err_impl(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut input: ItemFn = syn::parse2(item).expect("Failed to parse function");
    let error_type: Type = syn::parse2(attr).expect("Expected an error type as an argument");

    if let ReturnType::Type(_, ref mut ret_type) = input.sig.output {
        // Extract the Ok type from the original Result.
        let ok_type = if let Type::Path(type_path) = ret_type.as_ref() {
            if let Some(segment) = type_path.path.segments.last() {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 2 {
                        if let syn::GenericArgument::Type(ty) = &args.args[0] {
                            quote! { #ty }
                        } else {
                            quote! { () }
                        }
                    } else {
                        quote! { () }
                    }
                } else {
                    quote! { () }
                }
            } else {
                quote! { () }
            }
        } else {
            quote! { () }
        };

        // Save the original return type.
        let orig_ret_type = ret_type.clone();
        // Replace the return type with Result<OkType, ProvidedError>.
        *ret_type = syn::parse2(quote! { Result<#ok_type, #error_type> })
            .expect("Failed to parse new return type");

        // Wrap the original function body with error mapping.
        let block = &input.block;
        input.block = syn::parse2(quote!({
            let result: #orig_ret_type = #block;
            result.map_err(::std::convert::Into::into)
        }))
        .expect("Failed to parse new function body");

        quote! { #input }
    } else {
        // Function doesn't return a Result – leave unchanged.
        quote! { #input }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{parse_quote, ItemFn};

    /// Test that wrap_map_err correctly transforms a function that returns Result<T, E>
    /// into one that returns Result<T, ProvidedError> and wraps its body with map_err.
    #[test]
    fn test_wrap_map_err_transformation() {
        // Build a test function.
        let input_fn: ItemFn = parse_quote! {
            fn test_func() -> Result<u32, OtherError> {
                Ok(42)
            }
        };
        // Create attribute and item token streams.
        let attr: TokenStream = quote! { MyError };
        let item: TokenStream = quote! { #input_fn };
        let output = wrap_map_err_impl(attr, item);
        let output_str = output.to_string();
        // Check that the output now has the return type "Result < u32 , MyError >".
        assert!(output_str.contains("Result < u32 , MyError >"));
        // Check that the function body is wrapped with map_err(Into::into).
        assert!(output_str.contains(".map_err ( :: std :: convert :: Into :: into )"));
    }

    /// Test that a function that does not return a Result remains unchanged.
    #[test]
    fn test_no_result_function_unchanged() {
        let input_fn: ItemFn = parse_quote! {
            fn test_func() -> u32 {
                42
            }
        };
        let attr: TokenStream = quote! { MyError };
        let item: TokenStream = quote! { #input_fn };
        let output = wrap_map_err_impl(attr, item);
        let output_str = output.to_string();
        // The function signature should remain as returning u32 and there should be no map_err.
        assert!(output_str.contains("fn test_func ( ) -> u32"));
        assert!(!output_str.contains("map_err"));
    }
}
