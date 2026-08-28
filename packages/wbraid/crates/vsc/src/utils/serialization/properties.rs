// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Property-based bijection tests for `Canonical` (`SERIALIZATION.md`, phase 3).
//!
//! Two properties together make `ser`/`deser` a bijection between values and
//! accepted byte strings:
//!
//! - **P1 (round trip)**: `deser(ser(x)) == x` for arbitrary values `x`.
//! - **P2 (strictness)**: `deser(b) == Ok(v)` implies `ser(v) == b` for
//!   arbitrary byte strings `b` — every *accepted* byte string is exactly the
//!   one its value serializes to.
//!
//! P2 is exercised over two input distributions: raw random bytes (mostly
//! rejected — the property constrains the survivors) and mutations of valid
//! encodings (truncations, extensions, byte edits — the distribution that
//! catches trailing-byte and length-prefix slack, i.e. findings S1–S6).
//!
//! The harness is deliberately **format-agnostic**: it pins the bijection,
//! never byte layouts, so it transfers unchanged to any future encoding
//! (`SERIALIZATION.md` §8, outcome 2) as its acceptance suite.
//!
//! The generated `Sink` type exercises every composition rule the derive and
//! the generic impls provide — fixed-width leaves, strings, options, vectors
//! (including the nested-variable `Vec<String>` and byte-vector cases),
//! arrays, struct nesting, `PhantomData`, and group elements/scalars — so the
//! properties cover the machinery all production artifacts are built from.

use std::marker::PhantomData;

use proptest::prelude::*;

use crate::context::{Context, P256Ctx, RistrettoCtx};
use crate::traits::groups::CryptographicGroup;
use crate::utils::serialization::{Deserializable, Serializable};
use canonical_derive::Canonical;

/// Variable-tier kitchen sink: every generic composition rule across a
/// nested struct tree (the tuple impls compose up to arity 8, and production
/// artifacts nest — so does this).
#[derive(Debug, Clone, PartialEq, Canonical)]
struct Sink<C: Context> {
    prims: Prims,
    vars: Vars,
    arr: [u32; 3],
    inner: Inner<C>,
    phantom: PhantomData<C>,
}

/// Primitive leaves.
#[derive(Debug, Clone, PartialEq, Canonical)]
struct Prims {
    a: u32,
    b: u64,
    c: u128,
    d: usize,
    e: bool,
    s: String,
}

/// Variable-size members, including the nested-variable cases.
#[derive(Debug, Clone, PartialEq, Canonical)]
struct Vars {
    o1: Option<u64>,
    o2: Option<Vec<u32>>,
    v: Vec<String>,
    bytes: Vec<u8>,
    nested_bytes: Vec<Vec<u8>>,
}

/// Nested struct with the cryptographic leaves.
#[derive(Debug, Clone, PartialEq, Canonical)]
struct Inner<C: Context> {
    g: C::Element,
    x: C::Scalar,
    cs: Vec<C::Element>,
}

/// All-fixed-width sink: a struct whose encoding is pure concatenation with
/// zero framing (mini-spec rules 1 and 3).
#[derive(Debug, Clone, PartialEq, Canonical)]
struct FSink<C: Context> {
    a: u32,
    b: u64,
    e: [C::Element; 2],
    x: C::Scalar,
}

/// Deterministic group element from proptest-supplied bytes.
fn element<C: Context>() -> impl Strategy<Value = C::Element> {
    any::<[u8; 32]>().prop_map(|b| {
        C::G::hash_to_element(&[&b], &[b"vser property test"])
            .expect("hash_to_element cannot fail on fixed-width input")
    })
}

/// Deterministic scalar from proptest-supplied bytes.
fn scalar<C: Context>() -> impl Strategy<Value = C::Scalar> {
    any::<[u8; 32]>().prop_map(|b| {
        C::G::hash_to_scalar(&[&b], &[b"vser property test"])
            .expect("hash_to_scalar cannot fail on fixed-width input")
    })
}

fn inner<C: Context>() -> impl Strategy<Value = Inner<C>> {
    (
        element::<C>(),
        scalar::<C>(),
        proptest::collection::vec(element::<C>(), 0..4),
    )
        .prop_map(|(g, x, cs)| Inner { g, x, cs })
}

fn prims() -> impl Strategy<Value = Prims> {
    (
        any::<u32>(),
        any::<u64>(),
        any::<u128>(),
        any::<u32>(), // usize via u32 to stay platform-independent
        any::<bool>(),
        ".{0,12}", // arbitrary short strings, including empty and multibyte
    )
        .prop_map(|(a, b, c, d, e, s)| Prims {
            a,
            b,
            c,
            d: d as usize,
            e,
            s,
        })
}

fn vars() -> impl Strategy<Value = Vars> {
    (
        proptest::option::of(any::<u64>()),
        proptest::option::of(proptest::collection::vec(any::<u32>(), 0..4)),
        proptest::collection::vec(".{0,8}", 0..4),
        proptest::collection::vec(any::<u8>(), 0..16),
        proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..8), 0..3),
    )
        .prop_map(|(o1, o2, v, bytes, nested_bytes)| Vars {
            o1,
            o2,
            v,
            bytes,
            nested_bytes,
        })
}

fn sink<C: Context>() -> impl Strategy<Value = Sink<C>> {
    (prims(), vars(), any::<[u32; 3]>(), inner::<C>()).prop_map(|(prims, vars, arr, inner)| {
        Sink {
            prims,
            vars,
            arr,
            inner,
            phantom: PhantomData,
        }
    })
}

fn fsink<C: Context>() -> impl Strategy<Value = FSink<C>> {
    (
        any::<u32>(),
        any::<u64>(),
        [element::<C>(), element::<C>()],
        scalar::<C>(),
    )
        .prop_map(|(a, b, e, x)| FSink { a, b, e, x })
}

/// A mutation applied to a valid encoding: truncate, extend, or edit a byte.
/// Indices are taken modulo the buffer length at application time so every
/// generated mutation is applicable to every buffer.
#[derive(Debug, Clone)]
enum Mutation {
    Truncate(usize),
    Extend(Vec<u8>),
    Edit { index: usize, xor: u8 },
}

fn mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        (1usize..64).prop_map(Mutation::Truncate),
        proptest::collection::vec(any::<u8>(), 1..16).prop_map(Mutation::Extend),
        (any::<usize>(), 1u8..=255).prop_map(|(index, xor)| Mutation::Edit { index, xor }),
    ]
}

fn apply(mutation: &Mutation, mut bytes: Vec<u8>) -> Vec<u8> {
    match mutation {
        Mutation::Truncate(n) => {
            let keep = bytes.len().saturating_sub(*n);
            bytes.truncate(keep);
        }
        Mutation::Extend(tail) => bytes.extend_from_slice(tail),
        Mutation::Edit { index, xor } => {
            if let Some(byte) = index
                .checked_rem(bytes.len())
                .and_then(|i| bytes.get_mut(i))
            {
                *byte ^= xor;
            }
        }
    }
    bytes
}

/// Instantiate the property suite for one `Context`.
macro_rules! bijection_properties {
    ($mod_name:ident, $ctx:ty) => {
        mod $mod_name {
            use super::*;

            proptest! {
                /// P1: every value round-trips.
                #[test]
                #[cfg_attr(miri, ignore)]
                fn p1_roundtrip(x in sink::<$ctx>()) {
                    let bytes = x.ser();
                    prop_assert_eq!(Sink::<$ctx>::deser(&bytes).unwrap(), x);
                }

                /// P2 over mutations of valid encodings: anything accepted
                /// must re-serialize to exactly the accepted bytes. This is
                /// the distribution that catches trailing-byte and
                /// length-prefix slack.
                #[test]
                #[cfg_attr(miri, ignore)]
                fn p2_strict_mutated(x in sink::<$ctx>(), m in mutation()) {
                    let bytes = apply(&m, x.ser());
                    if let Ok(v) = Sink::<$ctx>::deser(&bytes) {
                        prop_assert_eq!(v.ser(), bytes);
                    }
                }

                /// P2 over raw random bytes: mostly rejected; the property
                /// constrains any survivor.
                #[test]
                #[cfg_attr(miri, ignore)]
                fn p2_strict_random(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
                    if let Ok(v) = Sink::<$ctx>::deser(&bytes) {
                        prop_assert_eq!(v.ser(), bytes);
                    }
                }

                /// P1/P2 for an all-fixed-width struct (zero-framing
                /// composition, mini-spec rules 1 and 3).
                #[test]
                #[cfg_attr(miri, ignore)]
                fn fixed_p1_roundtrip_p2_strict(x in fsink::<$ctx>(), m in mutation()) {
                    let bytes = x.ser();
                    prop_assert_eq!(FSink::<$ctx>::deser(&bytes).unwrap(), x.clone());

                    let mutated = apply(&m, x.ser());
                    if let Ok(v) = FSink::<$ctx>::deser(&mutated) {
                        prop_assert_eq!(v.ser(), mutated);
                    }
                }
            }
        }
    };
}

bijection_properties!(ristretto, RistrettoCtx);
bijection_properties!(p256, P256Ctx);
