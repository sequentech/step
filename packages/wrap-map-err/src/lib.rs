// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn, ReturnType, Type};

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
