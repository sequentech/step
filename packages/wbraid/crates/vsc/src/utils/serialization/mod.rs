// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Canonical serialization: the wire format of every posted artifact, signed
//! statement, and hash identity.
//!
//! `ser`/`deser` form a **bijection** between values and accepted byte
//! strings: `ser` is injective (every value has exactly one encoding) and
//! `deser` accepts exactly the image of `ser`. The full format definition —
//! eight rules — lives in `SERIALIZATION.md` §9; property tests pinning the
//! bijection live in [`properties`].
//!
//! The design is cursor-based: every type's `read` consumes exactly the bytes
//! its `write` produced, from the front of a shared slice, so composition is
//! plain concatenation and needs no framing. Explicit lengths appear only
//! where the types genuinely lack the information: collection counts
//! ([`Vec`]), opaque byte lengths ([`String`]), and tag bytes ([`Option`],
//! hand-written enums). Everything else — integers, group elements, scalars,
//! digests, structs, tuples, arrays — is written raw and read by width.
//!
//! # Deriving
//!
//! `#[derive(Canonical)]` (from the `canonical_derive` crate, re-exported at
//! this crate's root) implements [`Serializable`], [`Deserializable`] and
//! `std::hash::Hash` for a struct by emitting field-by-field `write`/`read`
//! calls in declaration order. Enums are implemented by hand: a `u8`
//! discriminant in declaration order, then the variant payload, unknown
//! discriminants rejected.
//!
//! # Strictness
//!
//! There is a single strictness check in the whole format: [`deser`] errors
//! unless the input is exhausted. Nothing else needs validating because the
//! encoding states nothing twice.
//!
//! [`deser`]: Deserializable::deser
//!
//! * NOTE: It is the responsibility of the implementor to ensure consistency
//!   across builds. Changes to implementations can break challenge and data
//!   transfer functionality entirely. **In particular, serialization
//!   inconsistencies can cause otherwise valid proofs to fail.**

use crate::utils::error::Error;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod properties;

/// Types that serialize by appending their canonical encoding to a buffer.
pub trait Serializable: Sized {
    /// Append this value's encoding to `out`.
    fn write(&self, out: &mut Vec<u8>);

    /// Serialize this value into a fresh byte vector.
    fn ser(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write(&mut out);
        out
    }
}

/// Types that deserialize by consuming their canonical encoding from the
/// front of a slice.
pub trait Deserializable: Sized {
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
///
/// The workhorse of fixed-width `read` implementations.
///
/// # Errors
///
/// - If fewer than `n` bytes remain.
pub(crate) fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if input.len() < n {
        return Err(Error::DeserializationError("Input too short".to_string()));
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
        impl Serializable for $t {
            fn write(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
        }
        impl Deserializable for $t {
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
impl Serializable for usize {
    fn write(&self, out: &mut Vec<u8>) {
        let value: u64 = (*self).try_into().expect("usize fits in u64");
        value.write(out);
    }
}
impl Deserializable for usize {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        read_len(input)
    }
}

// ---------------------------------------------------------------------------
// Rule 2: bool
// ---------------------------------------------------------------------------

impl Serializable for bool {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
}
impl Deserializable for bool {
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

impl<T: Serializable, const N: usize> Serializable for [T; N] {
    fn write(&self, out: &mut Vec<u8>) {
        for item in self {
            item.write(out);
        }
    }
}
impl<T: Deserializable, const N: usize> Deserializable for [T; N] {
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

impl<T: Serializable> Serializable for Vec<T> {
    fn write(&self, out: &mut Vec<u8>) {
        let count: u64 = self.len().try_into().expect("usize fits in u64");
        count.write(out);
        for item in self {
            item.write(out);
        }
    }
}
impl<T: Deserializable> Deserializable for Vec<T> {
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

impl Serializable for String {
    fn write(&self, out: &mut Vec<u8>) {
        let len: u64 = self.len().try_into().expect("usize fits in u64");
        len.write(out);
        out.extend_from_slice(self.as_bytes());
    }
}
impl Deserializable for String {
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

impl<T: Serializable> Serializable for Option<T> {
    fn write(&self, out: &mut Vec<u8>) {
        self.is_some().write(out);
        if let Some(value) = self {
            value.write(out);
        }
    }
}
impl<T: Deserializable> Deserializable for Option<T> {
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

impl<T> Serializable for std::marker::PhantomData<T> {
    fn write(&self, _out: &mut Vec<u8>) {}
}
impl<T> Deserializable for std::marker::PhantomData<T> {
    fn read(_input: &mut &[u8]) -> Result<Self, Error> {
        Ok(std::marker::PhantomData)
    }
}

// ---------------------------------------------------------------------------
// Serialization through references (needed by generic callers)
// ---------------------------------------------------------------------------

impl<T: Serializable> Serializable for &T {
    fn write(&self, out: &mut Vec<u8>) {
        T::write(self, out);
    }
}

// There is deliberately no `BTreeMap` implementation: the one that existed in
// the previous format had no production callers, and its deserializer accepted
// unsorted and duplicate-keyed encodings (silently canonicalizing them), so
// distinct byte strings decoded to the same map. If a map is ever needed on
// the wire, its deserializer must reject out-of-order and duplicate keys.
//
// There is also deliberately no `LargeVector`: `Vec<T>` with fixed-size `T`
// has exactly its encoding (a count, then raw elements). Its reason to exist —
// parallel serialization of large collections — survives as a possible
// implementation strategy behind this same encoding, since fixed-size element
// boundaries are computable.
