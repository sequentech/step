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
//! bijection live in `properties`.
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
use rayon::prelude::*;

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
    /// The encoded width, when every value of the type encodes to the same
    /// number of bytes: fixed-width leaves, arrays of them, and structs of
    /// them (the derive sums its fields). `None` — the default — for anything
    /// whose width varies (`Vec`, `String`, `Option`, and structs containing
    /// them).
    ///
    /// The one consumer is [`Vec<T>::read`]: with the width known, a list's
    /// element boundaries are computable before any element is decoded, so
    /// a long list is decoded in parallel. The hint changes nothing about
    /// the encoding or about what is accepted.
    const FIXED_WIDTH: Option<usize> = None;

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
            const FIXED_WIDTH: Option<usize> = Some(size_of::<$t>());

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
    const FIXED_WIDTH: Option<usize> = Some(size_of::<u64>());

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
    const FIXED_WIDTH: Option<usize> = Some(1);

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
    const FIXED_WIDTH: Option<usize> = match T::FIXED_WIDTH {
        Some(width) => width.checked_mul(N),
        None => None,
    };

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

/// Lists at least this long are encoded — and, when the element width is
/// known, decoded — on the rayon pool. Below it the per-task overhead is not
/// worth paying; the bytes produced and accepted are identical either way.
pub const PAR_MIN_ELEMENTS: usize = 1024;

impl<T: Serializable + Sync> Serializable for Vec<T> {
    fn write(&self, out: &mut Vec<u8>) {
        write_list(self, out);
    }
}
impl<T: Deserializable + Send> Deserializable for Vec<T> {
    // A list is never fixed-width: its count varies. (The default `None`.)

    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        let count = read_len(input)?;
        if let Some(width) = T::FIXED_WIDTH {
            if width > 0 && count >= PAR_MIN_ELEMENTS {
                return read_fixed_width_list_parallel(input, count, width);
            }
        }
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

/// The list encoding — a big-endian `u64` count, then the elements in order —
/// with the elements encoded on the rayon pool from [`PAR_MIN_ELEMENTS`] up:
/// each into its own buffer, then concatenated, which is byte-identical to
/// writing them one after another.
fn write_list<T: Serializable + Sync>(items: &[T], out: &mut Vec<u8>) {
    let count: u64 = items.len().try_into().expect("usize fits in u64");
    count.write(out);
    if items.len() < PAR_MIN_ELEMENTS {
        for item in items {
            item.write(out);
        }
        return;
    }
    let encoded: Vec<Vec<u8>> = items.par_iter().map(Serializable::ser).collect();
    let total: usize = encoded.iter().map(Vec::len).sum();
    out.reserve(total);
    for chunk in &encoded {
        out.extend_from_slice(chunk);
    }
}

/// Decode `count` elements of `width` bytes each from the front of `input`,
/// on the rayon pool. Accepts exactly what the sequential loop accepts: the
/// bytes are taken up front (so a short input fails as "Input too short"
/// before anything is decoded, and no allocation exceeds the input), every
/// element goes through the same `T::read` on exactly its `width` bytes, and
/// the first failing element in list order is the error reported.
fn read_fixed_width_list_parallel<T: Deserializable + Send>(
    input: &mut &[u8],
    count: usize,
    width: usize,
) -> Result<Vec<T>, Error> {
    let total = count
        .checked_mul(width)
        .ok_or_else(|| Error::DeserializationError("Input too short".to_string()))?;
    let bytes = take(input, total)?;
    let decoded: Vec<Result<T, Error>> = bytes
        .par_chunks_exact(width)
        .map(|chunk| {
            let mut cursor = chunk;
            let value = T::read(&mut cursor)?;
            if !cursor.is_empty() {
                return Err(Error::DeserializationError(
                    "Fixed-width element consumed fewer bytes than its width".to_string(),
                ));
            }
            Ok(value)
        })
        .collect();
    decoded.into_iter().collect()
}

/// The sum of fixed widths, or `None` if any is unknown — the width of a
/// struct from the widths of its fields (used by the `Canonical` derive).
#[must_use]
pub const fn fixed_width_sum(widths: &[Option<usize>]) -> Option<usize> {
    let mut total: usize = 0;
    let mut i = 0;
    while i < widths.len() {
        match widths[i] {
            Some(width) => match total.checked_add(width) {
                Some(sum) => total = sum,
                None => return None,
            },
            None => return None,
        }
        i += 1;
    }
    Some(total)
}

// ---------------------------------------------------------------------------
// Parallel serialization of slices
// ---------------------------------------------------------------------------

/// The list encoding of a slice — what `Vec<T>::ser` produces for the same
/// elements, byte for byte — for callers holding a slice: the transcript
/// derivations encode borrowed element and ciphertext lists with it.
#[must_use]
pub fn par_ser<T: Serializable + Sync>(items: &[T]) -> Vec<u8> {
    let mut out = Vec::new();
    write_list(items, &mut out);
    out
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
    const FIXED_WIDTH: Option<usize> = Some(0);

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
