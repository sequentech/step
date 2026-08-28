// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! The v2 encoding (`SERIALIZATION.md` §9): cursor-based serialization,
//! canonical and strict by construction.
//!
//! **Spike status**: this module lives alongside the v1 tiers and is not yet
//! wired to production types. The checkpoint types and property tests are in
//! the test module below; the flip — migrating leaves, derives and callers,
//! and deleting the v1 tiers — happens only after the checkpoint review.
//!
//! The format, in full (normative form: `SERIALIZATION.md` §9):
//!
//! 1. Fixed-width leaves (integers big-endian, elements, scalars, digests):
//!    their v1 encodings, no framing.
//! 2. `bool`: one byte, `0` or `1`.
//! 3. Structs, tuples, arrays: concatenation in declaration order.
//! 4. `Vec<T>`: `u64` count, then elements concatenated; an element consuming
//!    zero bytes is an error.
//! 5. `String`: `u64` byte length, then UTF-8 bytes.
//! 6. `Option<T>`: one tag byte, then `Some`'s payload.
//! 7. Enums (hand-written): one `u8` discriminant, then the variant payload.
//! 8. `PhantomData`: zero bytes.
//!
//! `read` consumes exactly the bytes `write` produced; `deser` adds the single
//! top-level strictness check (input exhausted). There is no other validation
//! anywhere, because there is nothing redundant to validate.

use crate::utils::error::Error;

/// Types that serialize by appending their encoding to a buffer.
pub trait VSerializable: Sized {
    /// Append this value's encoding to `out`.
    fn write(&self, out: &mut Vec<u8>);

    /// Serialize this value into a fresh byte vector.
    fn ser(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write(&mut out);
        out
    }
}

/// Types that deserialize by consuming their encoding from the front of a
/// slice.
pub trait VDeserializable: Sized {
    /// Consume exactly this value's encoding from the front of `input`,
    /// advancing it.
    ///
    /// # Errors
    ///
    /// - If the input does not begin with a valid encoding of this type.
    fn read(input: &mut &[u8]) -> Result<Self, Error>;

    /// Deserialize a value from exactly `buffer` — the whole of it.
    ///
    /// # Errors
    ///
    /// - If `buffer` does not begin with a valid encoding of this type.
    /// - If any bytes remain after it (strictness: `deser` accepts exactly
    ///   the image of `ser`).
    fn deser(buffer: &[u8]) -> Result<Self, Error> {
        let mut input = buffer;
        let value = Self::read(&mut input)?;
        if !input.is_empty() {
            return Err(Error::DeserializationError(
                "Trailing bytes after value".to_string(),
            ));
        }
        Ok(value)
    }
}

/// Consume exactly `n` bytes from the front of `input`.
fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if input.len() < n {
        return Err(Error::DeserializationError(
            "Input too short".to_string(),
        ));
    }
    let (head, tail) = input.split_at(n);
    *input = tail;
    Ok(head)
}

/// Read a `u64` length/count and convert it to `usize`.
fn read_len(input: &mut &[u8]) -> Result<usize, Error> {
    let len = u64::read(input)?;
    len.try_into()
        .map_err(|_| Error::DeserializationError("Length exceeds usize".to_string()))
}

// ---------------------------------------------------------------------------
// Rule 1: fixed-width integer leaves (big-endian)
// ---------------------------------------------------------------------------

/// Implement rule 1 for a big-endian fixed-width integer.
macro_rules! impl_int {
    ($($t:ty),+) => {$(
        impl VSerializable for $t {
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
        }
        impl VDeserializable for $t {
            fn read(input: &mut &[u8]) -> Result<Self, Error> {
                let bytes = take(input, size_of::<$t>())?;
                Ok(<$t>::from_be_bytes(
                    bytes.try_into().expect("take returns exactly size_of bytes"),
                ))
            }
        }
    )+};
}
impl_int!(u8, u16, u32, u64, u128);

/// `usize` travels as `u64` for platform independence.
impl VSerializable for usize {
    fn write(&self, out: &mut Vec<u8>) {
        let value: u64 = (*self).try_into().expect("usize fits in u64");
        value.write(out);
    }
}
impl VDeserializable for usize {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        read_len(input)
    }
}

// ---------------------------------------------------------------------------
// Rule 2: bool
// ---------------------------------------------------------------------------

impl VSerializable for bool {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
}
impl VDeserializable for bool {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        match u8::read(input)? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(Error::DeserializationError(format!(
                "Non-canonical bool encoding: {other:#04x}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Rule 3: arrays (structs and tuples are the derive's job — same rule)
// ---------------------------------------------------------------------------

impl<T: VSerializable, const N: usize> VSerializable for [T; N] {
    fn write(&self, out: &mut Vec<u8>) {
        for item in self {
            item.write(out);
        }
    }
}
impl<T: VDeserializable, const N: usize> VDeserializable for [T; N] {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        let mut items = Vec::with_capacity(N);
        for _ in 0..N {
            items.push(T::read(input)?);
        }
        items.try_into().map_err(|_| {
            Error::DeserializationError("Failed converting Vec<T> to [T; N]".to_string())
        })
    }
}

// ---------------------------------------------------------------------------
// Rule 4: Vec
// ---------------------------------------------------------------------------

impl<T: VSerializable> VSerializable for Vec<T> {
    fn write(&self, out: &mut Vec<u8>) {
        let count: u64 = self.len().try_into().expect("usize fits in u64");
        count.write(out);
        for item in self {
            item.write(out);
        }
    }
}
impl<T: VDeserializable> VDeserializable for Vec<T> {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        let count = read_len(input)?;
        // No allocation is sized by the attacker-controlled count: the vector
        // grows per parsed element, and each element must consume input, so
        // the loop is bounded by the input length.
        let mut items = Vec::new();
        for _ in 0..count {
            let before = input.len();
            items.push(T::read(input)?);
            if input.len() == before {
                return Err(Error::DeserializationError(
                    "Collection element consumed no bytes".to_string(),
                ));
            }
        }
        Ok(items)
    }
}

// ---------------------------------------------------------------------------
// Rule 5: String
// ---------------------------------------------------------------------------

impl VSerializable for String {
    fn write(&self, out: &mut Vec<u8>) {
        let len: u64 = self.len().try_into().expect("usize fits in u64");
        len.write(out);
        out.extend_from_slice(self.as_bytes());
    }
}
impl VDeserializable for String {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        let len = read_len(input)?;
        let bytes = take(input, len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| Error::DeserializationError("Invalid UTF-8 in String".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Rule 6: Option
// ---------------------------------------------------------------------------

impl<T: VSerializable> VSerializable for Option<T> {
    fn write(&self, out: &mut Vec<u8>) {
        self.is_some().write(out);
        if let Some(value) = self {
            value.write(out);
        }
    }
}
impl<T: VDeserializable> VDeserializable for Option<T> {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        if bool::read(input)? {
            Ok(Some(T::read(input)?))
        } else {
            Ok(None)
        }
    }
}

// ---------------------------------------------------------------------------
// Rule 8: PhantomData
// ---------------------------------------------------------------------------

impl<T> VSerializable for std::marker::PhantomData<T> {
    fn write(&self, _out: &mut Vec<u8>) {}
}
impl<T> VDeserializable for std::marker::PhantomData<T> {
    fn read(_input: &mut &[u8]) -> Result<Self, Error> {
        Ok(std::marker::PhantomData)
    }
}

// ---------------------------------------------------------------------------
// Serialization through references (needed by generic callers)
// ---------------------------------------------------------------------------

impl<T: VSerializable> VSerializable for &T {
    fn write(&self, out: &mut Vec<u8>) {
        T::write(self, out);
    }
}

// ---------------------------------------------------------------------------
// Rule 1 leaves: group elements, scalars (v1 encodings, byte-for-byte)
// ---------------------------------------------------------------------------
//
// Spike note: these impls live here so the spike touches no production file;
// at flip time each moves next to its type.

use crate::groups::p256::element::P256Element;
use crate::groups::p256::scalar::P256Scalar;
use crate::groups::ristretto255::element::RistrettoElement;
use crate::groups::ristretto255::scalar::RistrettoScalar;
use crate::utils::serialization::variable::VDeserializable as V1Deserializable;
use crate::utils::serialization::variable::VSerializable as V1Serializable;

/// Implement the v2 traits for a fixed-width leaf by delegating to its v1
/// encoding (which is unchanged in v2), with `width` bytes consumed on read.
macro_rules! impl_leaf_via_v1 {
    ($t:ty, $width:expr) => {
        impl VSerializable for $t {
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&V1Serializable::ser(self));
            }
        }
        impl VDeserializable for $t {
            fn read(input: &mut &[u8]) -> Result<Self, Error> {
                let bytes = take(input, $width)?;
                V1Deserializable::deser(bytes)
            }
        }
    };
}
impl_leaf_via_v1!(RistrettoElement, 32);
impl_leaf_via_v1!(RistrettoScalar, 32);
impl_leaf_via_v1!(P256Element, 33);
impl_leaf_via_v1!(P256Scalar, 32);

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::marker::PhantomData;
    use vser_derive::VSer2;

    use crate::context::{Context, RistrettoCtx};
    use crate::traits::groups::CryptographicGroup;

    use proptest::prelude::*;

    // -- The three checkpoint types (SERIALIZATION.md §7 guardrail 2) --------

    /// Representative fixed struct (only fixed-width members).
    #[derive(Debug, Clone, PartialEq, VSer2)]
    struct FixedDemo {
        a: u32,
        b: u64,
        e: [RistrettoElement; 2],
        x: RistrettoScalar,
    }

    /// Representative `Vec`-bearing artifact (the `Shares`/`Ballots` shape).
    #[derive(Debug, Clone, PartialEq, VSer2)]
    struct ArtifactDemo {
        items: Vec<FixedDemo>,
        encrypted: Vec<Vec<u8>>,
        name: String,
        proof: Option<FixedDemo>,
        phantom: PhantomData<RistrettoCtx>,
    }

    /// Representative enum, mirroring `braid::messages::predicate::Predicate`'s
    /// hand-written tag dispatch (rule 7): `u8` discriminant in declaration
    /// order, then the variant payload; unknown discriminants rejected.
    #[derive(Debug, Clone, PartialEq)]
    enum PredicateDemo {
        Fixed(FixedDemo),
        Artifact(ArtifactDemo),
        Empty,
    }

    impl VSerializable for PredicateDemo {
        fn write(&self, out: &mut Vec<u8>) {
            match self {
                PredicateDemo::Fixed(p) => {
                    0u8.write(out);
                    p.write(out);
                }
                PredicateDemo::Artifact(p) => {
                    1u8.write(out);
                    p.write(out);
                }
                PredicateDemo::Empty => 2u8.write(out),
            }
        }
    }
    impl VDeserializable for PredicateDemo {
        fn read(input: &mut &[u8]) -> Result<Self, Error> {
            match u8::read(input)? {
                0 => Ok(PredicateDemo::Fixed(FixedDemo::read(input)?)),
                1 => Ok(PredicateDemo::Artifact(ArtifactDemo::read(input)?)),
                2 => Ok(PredicateDemo::Empty),
                other => Err(Error::DeserializationError(format!(
                    "unknown PredicateDemo discriminant {other}"
                ))),
            }
        }
    }

    // -- Generators ----------------------------------------------------------

    fn element() -> impl Strategy<Value = RistrettoElement> {
        any::<[u8; 32]>().prop_map(|b| {
            <RistrettoCtx as Context>::G::hash_to_element(&[&b], &[b"v2 property test"])
                .expect("hash_to_element cannot fail on fixed-width input")
        })
    }

    fn scalar() -> impl Strategy<Value = RistrettoScalar> {
        any::<[u8; 32]>().prop_map(|b| {
            <RistrettoCtx as Context>::G::hash_to_scalar(&[&b], &[b"v2 property test"])
                .expect("hash_to_scalar cannot fail on fixed-width input")
        })
    }

    fn fixed_demo() -> impl Strategy<Value = FixedDemo> {
        (any::<u32>(), any::<u64>(), [element(), element()], scalar())
            .prop_map(|(a, b, e, x)| FixedDemo { a, b, e, x })
    }

    fn artifact_demo() -> impl Strategy<Value = ArtifactDemo> {
        (
            proptest::collection::vec(fixed_demo(), 0..3),
            proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..8), 0..3),
            ".{0,12}",
            proptest::option::of(fixed_demo()),
        )
            .prop_map(|(items, encrypted, name, proof)| ArtifactDemo {
                items,
                encrypted,
                name,
                proof,
                phantom: PhantomData,
            })
    }

    fn predicate_demo() -> impl Strategy<Value = PredicateDemo> {
        prop_oneof![
            fixed_demo().prop_map(PredicateDemo::Fixed),
            artifact_demo().prop_map(PredicateDemo::Artifact),
            Just(PredicateDemo::Empty),
        ]
    }

    // -- Mutations (as in v1's properties.rs) --------------------------------

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

    // -- The bijection properties over the checkpoint types ------------------

    macro_rules! bijection {
        ($p1:ident, $p2m:ident, $p2r:ident, $ty:ty, $strategy:expr) => {
            proptest! {
                #[test]
                #[cfg_attr(miri, ignore)]
                fn $p1(x in $strategy) {
                    let bytes = x.ser();
                    prop_assert_eq!(<$ty>::deser(&bytes).unwrap(), x);
                }

                #[test]
                #[cfg_attr(miri, ignore)]
                fn $p2m(x in $strategy, m in mutation()) {
                    let bytes = apply(&m, x.ser());
                    if let Ok(v) = <$ty>::deser(&bytes) {
                        prop_assert_eq!(v.ser(), bytes);
                    }
                }

                #[test]
                #[cfg_attr(miri, ignore)]
                fn $p2r(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
                    if let Ok(v) = <$ty>::deser(&bytes) {
                        prop_assert_eq!(v.ser(), bytes);
                    }
                }
            }
        };
    }

    bijection!(fixed_p1, fixed_p2_mutated, fixed_p2_random, FixedDemo, fixed_demo());
    bijection!(
        artifact_p1,
        artifact_p2_mutated,
        artifact_p2_random,
        ArtifactDemo,
        artifact_demo()
    );
    bijection!(
        predicate_p1,
        predicate_p2_mutated,
        predicate_p2_random,
        PredicateDemo,
        predicate_demo()
    );

    // -- Concrete pins --------------------------------------------------------

    /// The S8 kill, demonstrated: 100 payload bytes cost 108 in v2 (count
    /// only) versus 908 in v1 (a length prefix per byte).
    #[test]
    fn vec_u8_overhead_is_count_only() {
        let bytes: Vec<u8> = vec![0xAB; 100];
        assert_eq!(108, VSerializable::ser(&bytes).len());
        assert_eq!(908, V1Serializable::ser(&bytes).len());
    }

    /// Fixed-width members compose with zero framing: the struct's encoding
    /// is exactly the sum of its leaves.
    #[test]
    fn fixed_struct_has_zero_framing() {
        let mut input: &[u8] = &[0u8; 0];
        assert!(FixedDemo::read(&mut input).is_err());

        let x = FixedDemo {
            a: 1,
            b: 2,
            e: [
                <RistrettoCtx as Context>::generator(),
                <RistrettoCtx as Context>::generator(),
            ],
            x: <RistrettoCtx as Context>::random_scalar(),
        };
        assert_eq!(4 + 8 + 32 + 32 + 32, x.ser().len());
    }

    /// A collection element that consumes no bytes must be rejected: the
    /// count must be bound by content (mini-spec rule 4).
    #[test]
    fn zero_consuming_vec_elements_are_rejected() {
        // Vec<PhantomData<u8>> with a claimed count of 3 and no content.
        let bytes = VSerializable::ser(&3u64);
        assert!(<Vec<PhantomData<u8>> as VDeserializable>::deser(&bytes).is_err());
    }
}
