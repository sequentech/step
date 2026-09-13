// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Parser-level tests cover inputs that cannot occur in a compiled consumer.
//! Successful expansions are also compiled and called in behavior.rs.

use super::*;

#[test]
fn result_signatures_preserve_nested_success_types_and_function_metadata() {
    for signature in [
        quote!(
            #[inline]
            #[doc = "Retain this contract"]
            pub fn load<'a, T: Clone>(value: &'a T) -> Result<Option<&'a T>>
            where
                T: PartialEq,
            {
                Ok(Some(value))
            }
        ),
        quote!(
            async fn load() -> std::result::Result<Vec<u8>, SourceError> {
                Ok(vec![])
            }
        ),
        quote!(
            fn load() -> Result<(), SourceError> {
                Ok(())
            }
        ),
    ] {
        let original: ItemFn = syn::parse2(signature.clone()).unwrap();
        let (_, expected_success) = result_types(&original.sig.output).unwrap();
        let result: ItemFn =
            syn::parse2(expand_attribute(quote!(errors::TaskError), signature)).unwrap();
        let expected_return: ReturnType =
            parse_quote!(-> ::core::result::Result<#expected_success, errors::TaskError>);
        let actual_return = &result.sig.output;
        assert_eq!(
            quote!(#expected_return).to_string(),
            quote!(#actual_return).to_string()
        );
        // The macro changes only the body and return type. Metadata belongs to
        // consumers such as Celery and must survive expansion unchanged.
        assert_eq!(result.sig.ident, original.sig.ident);
        assert_eq!(
            result.sig.asyncness.is_some(),
            original.sig.asyncness.is_some()
        );
        let mut restored = result;
        restored.sig.output = original.sig.output.clone();
        restored.block = original.block.clone();
        assert_eq!(quote!(#restored).to_string(), quote!(#original).to_string());
    }
}

#[test]
fn unrelated_return_types_are_left_entirely_unchanged() {
    for input in [
        quote!(
            fn plain() {}
        ),
        quote!(
            fn flag() -> bool {
                true
            }
        ),
        quote!(
            fn optional() -> Option<u32> {
                None
            }
        ),
        quote!(
            fn tuple() -> (u32, u32) {
                (1, 2)
            }
        ),
        quote!(
            fn reference() -> &'static str {
                "value"
            }
        ),
        quote!(
            fn pair() -> Pair<u32, u32> {
                todo!()
            }
        ),
        quote!(
            const fn unchanged() -> u32 {
                7
            }
        ),
    ] {
        assert_eq!(
            expand_attribute(quote!(TaskError), input.clone()).to_string(),
            input.to_string()
        );
    }
}

#[test]
fn malformed_attributes_and_non_function_items_emit_diagnostics_without_panics() {
    for (attribute, item) in [
        (
            quote!(),
            quote!(
                fn load() {}
            ),
        ),
        (
            quote!(Error, Extra),
            quote!(
                fn load() {}
            ),
        ),
        (
            quote!(Error),
            quote!(
                struct NotAFunction;
            ),
        ),
        (quote!(Error), quote!(fn)),
        (
            quote!(Error),
            quote!(
                const fn load() -> Result<()> {
                    Ok(())
                }
            ),
        ),
    ] {
        let diagnostic = expand_attribute(attribute, item).to_string();
        assert!(
            diagnostic.contains("compile_error"),
            "expected compiler diagnostic: {diagnostic}"
        );
    }
}

#[test]
fn unsupported_result_syntax_is_not_guessed() {
    for output in [
        parse_quote!(-> Result),
        parse_quote!(-> Result<'static>),
        parse_quote!(-> Result<u32, Error, Extra>),
        parse_quote!(-> Result<u32, 7>),
    ] {
        assert!(result_types(&output).is_none());
    }
}
