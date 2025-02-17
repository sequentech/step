// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, ReturnType, Type};

/// The `wrap_map_err` attribute macro transforms a function that returns a `Result<T, E>`
/// into one that returns a `Result<T, ProvidedError>` by mapping the error with `Into::into`.
///
/// This is useful when you want to unify error types across your code by converting the error
/// returned by a function to a specific type.
///
/// # Arguments
///
/// * The macro takes a single argument: the error type to which all errors should be converted.
///
/// # Example
///
/// ```rust
/// # use wrap_map_err::wrap_map_err;
/// #[wrap_map_err(MyError)]
/// fn foo() -> Result<u32, OtherError> {
///     // function body
///     Ok(42)
/// }
/// // The function is transformed so that its signature becomes:
/// // fn foo() -> Result<u32, MyError>
/// // and its body is wrapped with `.map_err(Into::into)`.
/// ```
///
/// If the function does not return a `Result`, the macro leaves the function unchanged.
#[proc_macro_attribute]
pub fn wrap_map_err(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the function to which the attribute is applied.
    let item2 = item.clone();
    let mut input = parse_macro_input!(item2 as ItemFn);

    // Parse the error type provided as an argument to the attribute.
    let error_type: Type = syn::parse(attr).expect("Expected an error type as an argument");

    // Check if the function returns a Result type.
    if let ReturnType::Type(_, ref mut ret_type) = input.sig.output {
        // Extract the Ok type from the original Result
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

        // Construct the new Result type with explicit Ok and Err types
        let orig_ret_type = ret_type.clone();
        *ret_type = parse_quote!(Result<#ok_type, #error_type>);

        // Wrap the function body with error mapping.
        let block = &input.block;
        input.block = parse_quote!({
            let result: #orig_ret_type = #block;
             result.map_err(::std::convert::Into::into)
        });

        // Generate the expanded function.
        let output = quote! {
            #input
        };
        output.into()
    } else {
        // If the function does not return a Result type, return the original function.
        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro::TokenStream;
    use quote::quote;
    use syn::{parse_quote, ItemFn};

    /// Test that wrap_map_err correctly transforms a function that returns Result<T, E>
    /// into one that returns Result<T, ProvidedError> and wraps its body with map_err.
    #[test]
    fn test_wrap_map_err_transformation() {
        // Build a test function:
        let input_fn: ItemFn = parse_quote! {
            fn test_func() -> Result<u32, OtherError> {
                Ok(42)
            }
        };
        // Use our macro with an error type argument.
        let attr: TokenStream = quote! { MyError }.into();
        let item: TokenStream = quote! { #input_fn }.into();
        let output = wrap_map_err(attr, item);
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
        let attr: TokenStream = quote! { MyError }.into();
        let item: TokenStream = quote! { #input_fn }.into();
        let output = wrap_map_err(attr, item);
        let output_str = output.to_string();
        // The function signature should remain as returning u32 and there should be no map_err.
        assert!(output_str.contains("fn test_func () -> u32"));
        assert!(!output_str.contains("map_err"));
    }
}
