// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Derive macro for the canonical serialization traits.

use proc_macro::TokenStream;
use quote::quote;

use syn::{DeriveInput, parse_macro_input};

/// Derives the canonical serialization traits (`SERIALIZATION.md` §9) for a
/// struct: [`Serializable`], [`Deserializable`], and `std::hash::Hash` via
/// the serialized bytes.
///
/// [`Serializable`]: ../cryptography/utils/serialization/trait.Serializable.html
/// [`Deserializable`]: ../cryptography/utils/serialization/trait.Deserializable.html
///
/// Emits mini-spec rule 3 directly: `write` appends each field's encoding in
/// declaration order; `read` consumes each field's encoding in the same order.
/// There is no intermediate representation and no arity limit — the generated
/// code is exactly what would be written by hand:
///
/// ```ignore
/// fn write(&self, out: &mut Vec<u8>) { self.a.write(out); self.b.write(out); }
/// fn read(input: &mut &[u8]) -> Result<Self, Error> {
///     Ok(Self { a: A::read(input)?, b: B::read(input)? })
/// }
/// ```
///
/// Enums are not supported: they implement the traits by hand (mini-spec rule
/// 7 — a `u8` discriminant in declaration order, then the variant payload).
#[proc_macro_derive(Canonical)]
pub fn canonical_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_canonical(&ast)
}

fn impl_canonical(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let generics = ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Per-field write statements and the constructor expression.
    let write_stmts: proc_macro2::TokenStream;
    let read_ctor: proc_macro2::TokenStream;

    match &ast.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(fields) => {
                let names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let tys: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();
                write_stmts = quote! { #( Serializable::write(&self.#names, out); )* };
                read_ctor =
                    quote! { Self { #( #names: <#tys as Deserializable>::read(input)?, )* } };
            }
            syn::Fields::Unnamed(fields) => {
                let indices = (0..fields.unnamed.len()).map(syn::Index::from);
                let tys: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
                write_stmts = quote! { #( Serializable::write(&self.#indices, out); )* };
                read_ctor = quote! { Self( #( <#tys as Deserializable>::read(input)?, )* ) };
            }
            syn::Fields::Unit => {
                write_stmts = quote! {};
                read_ctor = quote! { Self };
            }
        },
        _ => {
            return quote! { compile_error!("Canonical can only be derived for structs; enums implement the traits by hand (mini-spec rule 7)."); }
                .into();
        }
    }

    let generated = quote! {
        impl #impl_generics ::cryptography::utils::serialization::Serializable for #name #ty_generics #where_clause {
            fn write(&self, out: &mut Vec<u8>) {
                use ::cryptography::utils::serialization::Serializable;
                #write_stmts
            }
        }

        impl #impl_generics ::cryptography::utils::serialization::Deserializable for #name #ty_generics #where_clause {
            fn read(input: &mut &[u8]) -> Result<Self, ::cryptography::utils::error::Error> {
                use ::cryptography::utils::serialization::{Serializable, Deserializable};
                Ok(#read_ctor)
            }
        }

        impl #impl_generics std::hash::Hash for #name #ty_generics #where_clause {
            fn hash<H>(&self, h: &mut H) where H: std::hash::Hasher {
                use ::cryptography::utils::serialization::Serializable;
                h.write(&self.ser());
            }
        }
    };
    generated.into()
}
