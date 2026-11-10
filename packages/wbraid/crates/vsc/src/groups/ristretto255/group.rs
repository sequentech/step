// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! CryptographicGroup implementations for the Ristretto group

use crate::traits::groups::CryptographicGroup;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;

use crate::groups::ristretto255::element::RistrettoElement;
use crate::groups::ristretto255::scalar::RistrettoScalar;

use crate::utils::error::Error;
use crate::utils::hash;
use crate::utils::hash::Hasher;
use crate::utils::rng;

use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::{RistrettoPoint, Scalar as DalekScalar, constants as dalek_constants};
use sha3::Digest;
use zeroize::Zeroizing;

use rayon::prelude::*;

/// Ristretto implementation of [`CryptographicGroup`]
#[derive(Debug, Clone)]
pub struct Ristretto255Group;

impl CryptographicGroup for Ristretto255Group {
    type Element = RistrettoElement;
    type Scalar = RistrettoScalar;
    // This should be a default at the group trait, but Rust doesn't support that yet
    type Hasher = crate::context::CryptographicHasher;

    #[inline]
    fn generator() -> Self::Element {
        RistrettoElement::new(dalek_constants::RISTRETTO_BASEPOINT_POINT)
    }

    #[inline]
    fn g_exp(scalar: &Self::Scalar) -> Self::Element {
        RistrettoElement::new(RistrettoPoint::mul_base(&scalar.0))
    }

    /// # Errors
    ///
    /// Infallible
    fn hash_to_scalar(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Scalar, Error> {
        let mut hasher = Self::Hasher::hasher();
        hash::update_hasher(&mut hasher, input_slices, ds_tags);

        let ret = RistrettoScalar::from_hash::<Self::Hasher>(hasher);

        Ok(ret)
    }

    /// # Errors
    ///
    /// Infallible
    fn hash_to_element(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Element, Error> {
        let mut hasher = Self::Hasher::hasher();
        hash::update_hasher(&mut hasher, input_slices, ds_tags);

        let ret = RistrettoElement::from_hash::<Self::Hasher>(hasher);

        Ok(ret)
    }

    #[inline]
    fn random_element<R: rng::CRng>(rng: &mut R) -> Self::Element {
        Self::Element::random(rng)
    }

    #[inline]
    fn random_scalar<R: rng::CRng>(rng: &mut R) -> Self::Scalar {
        Self::Scalar::random(rng)
    }

    /// Encode bytes into the given number of Ristretto elements
    ///
    /// # Errors
    ///
    /// - `EncodingError` if a point was not found for the input, with negligible probability
    fn encode_bytes<const I: usize, const O: usize>(
        bytes: &[u8; I],
    ) -> Result<[RistrettoElement; O], Error> {
        // The input may be secret (a key share): the chunks are overwritten on return.
        let mut chunks = Zeroizing::new([[0u8; CHUNK_SIZE]; O]);
        Codec::<I, O>::split_into(bytes, &mut chunks);
        Self::encode_array(&chunks)
    }

    /// Decode bytes from the given number of Ristretto elements
    ///
    /// # Errors
    ///
    /// Infallible
    fn decode_bytes<const I: usize, const O: usize>(
        element: &[RistrettoElement; I],
    ) -> Result<[u8; O], Error> {
        let mut bytes = [0u8; O];
        Self::decode_bytes_into(element, &mut bytes);
        Ok(bytes)
    }

    /// Generate independent generators by hashing the label and index
    ///
    /// # Errors
    ///
    /// Infallible
    fn ind_generators(count: usize, label: &[u8]) -> Result<Vec<Self::Element>, Error> {
        let mut hasher = Self::Hasher::hasher();
        hasher.update(label);
        hasher.update(b"independent_generators_ristretto");

        #[cfg_attr(
            feature = "custom-warnings",
            crate::warning("The following code is not optimized. Parallelize with rayon")
        )]
        let ret: Vec<RistrettoElement> = (0..count)
            .into_par_iter()
            .map(|i| {
                let mut hasher = hasher.clone();
                // Cannot use platform dependent type in random oracle
                let i_u64 = i as u64;
                hasher.update(i_u64.to_be_bytes());
                let point = RistrettoPoint::from_hash(hasher);
                RistrettoElement(point)
            })
            .collect();

        Ok(ret)
    }

    /// Encrypt a scalar with ElGamal encryption using the given public key
    ///
    /// # Errors
    ///
    /// - `EncodingError` if the scalar cannot be encoded
    /// - `SerializationError` if the ciphertext cannot be serialized
    fn encrypt_scalar(scalar: &Self::Scalar, public_key: &Self::Element) -> Result<Vec<u8>, Error> {
        use crate::context::RistrettoCtx;
        use crate::cryptosystem::elgamal::{Ciphertext, PublicKey};
        use crate::utils::serialization::Serializable;

        // Encode scalar into 2 elements, overwritten when dropped
        let elements = Zeroizing::new(Self::encode_scalar(scalar)?);

        // Create public key and encrypt
        let pk = PublicKey::new(*public_key);
        let ciphertext: Ciphertext<RistrettoCtx, 2> = pk.encrypt(&elements);

        // Serialize to bytes
        Ok(ciphertext.ser())
    }

    /// Decrypt a scalar from ElGamal-encrypted serialized ciphertext
    ///
    /// # Errors
    ///
    /// - `DeserializationError` if the ciphertext cannot be deserialized
    /// - `ScalarDecodeError` if the decrypted elements cannot be decoded
    fn decrypt_scalar(ciphertext: &[u8], secret_key: &Self::Scalar) -> Result<Self::Scalar, Error> {
        use crate::context::RistrettoCtx;
        use crate::cryptosystem::elgamal::{self, Ciphertext};
        use crate::utils::serialization::Deserializable;

        // Deserialize ciphertext
        let ct: Ciphertext<RistrettoCtx, 2> = Ciphertext::deser(ciphertext).map_err(|e| {
            Error::DeserializationError(format!("Failed to deserialize ciphertext: {e:?}"))
        })?;

        // Decrypt to get elements, overwritten when dropped. The secret key is
        // used by reference, not copied.
        let elements = Zeroizing::new(elgamal::decrypt::<RistrettoCtx, 2>(
            ct.u(),
            ct.v(),
            secret_key,
        ));

        // Decode elements back to scalar
        Self::decode_scalar(&elements)
    }
}

impl Ristretto255Group {
    /// Encode an array of 30-byte chunks into an array of `Element`s.
    ///
    /// The elements carry the input, so they are built in a buffer that is
    /// overwritten on return, also when an encoding fails.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if any point was not found for the input, with negligible probability
    pub fn encode_array<const W: usize>(p: &[[u8; 30]; W]) -> Result<[RistrettoElement; W], Error> {
        let mut elements = Zeroizing::new([RistrettoElement::default(); W]);
        for (element, chunk) in elements.iter_mut().zip(p) {
            *element = Self::encode_30_bytes(chunk)?;
        }
        Ok(*elements)
    }

    /// Decode an array of 'Plaintext' types from an array of 'Message' types.
    ///
    /// The elements are read by reference, not copied.
    ///
    /// # Errors
    ///
    /// Infallible
    pub fn decode_array<const W: usize>(p: &[RistrettoElement; W]) -> Result<[[u8; 30]; W], Error> {
        let mut chunks = [[0u8; 30]; W];
        for (chunk, message) in chunks.iter_mut().zip(p) {
            Self::decode_30_bytes_into(message, chunk);
        }
        Ok(chunks)
    }

    /// Decode bytes from the given number of Ristretto elements into `out`.
    ///
    /// The chunks are held in a buffer that is overwritten on return, and the
    /// bytes go into a buffer the caller owns, so that a secret (a key share)
    /// is not left in a value this function returns.
    pub(crate) fn decode_bytes_into<const I: usize, const O: usize>(
        element: &[RistrettoElement; I],
        out: &mut [u8; O],
    ) {
        let mut chunks = Zeroizing::new([[0u8; CHUNK_SIZE]; I]);
        for (chunk, message) in chunks.iter_mut().zip(element) {
            Self::decode_30_bytes_into(message, chunk);
        }
        Codec::<O, I>::join_into(&chunks, out);
    }

    // see https://github.com/dalek-cryptography/curve25519-dalek/issues/322
    // see https://github.com/hdevalence/ristretto255-data-encoding/blob/master/src/main.rs
    /// Encode 30 bytes into an `Element`.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if a point was not found for the input, with negligible probability
    pub fn encode_30_bytes(input: &[u8; 30]) -> Result<RistrettoElement, Error> {
        // The input may be secret (part of a key share): the candidate encoding
        // is built in place, in a buffer that is overwritten on return.
        let mut buffer = Zeroizing::new(CompressedRistretto([0u8; 32]));
        let candidate: &mut CompressedRistretto = &mut buffer;
        candidate.0[1..=input.len()].copy_from_slice(input);
        for j in 0..64u8 {
            candidate.0[31] = j;
            for i in 0..128u8 {
                // cannot overflow, 127 * 2 < u8::MAX
                #[allow(clippy::arithmetic_side_effects)]
                let byte = 2 * i;
                candidate.0[0] = byte;
                if let Some(point) = candidate.decompress() {
                    return Ok(RistrettoElement(point));
                }
            }
        }
        Err(Error::EncodingError(
            "Failed to encode into ristretto point".to_string(),
        ))
    }

    /// Decodes a 30 bytes from an 'Element'.
    ///
    /// # Errors
    ///
    /// Infallible
    ///
    /// # Panics
    ///
    /// - Panics if the slice length is not 30, which should not happen
    pub fn decode_30_bytes(message: &RistrettoElement) -> Result<[u8; 30], Error> {
        let mut ret = [0u8; 30];
        Self::decode_30_bytes_into(message, &mut ret);

        Ok(ret)
    }

    /// Decode 30 bytes from an `Element` into `out`.
    ///
    /// The compressed point is held in a buffer that is overwritten on return,
    /// and the bytes go into a buffer the caller owns, so that a secret (part
    /// of a key share) is not left in a value this function returns.
    pub(crate) fn decode_30_bytes_into(message: &RistrettoElement, out: &mut [u8; 30]) {
        let compressed = Zeroizing::new(message.0.compress());
        // the 30 bytes of data are placed in the range 1-30
        out.copy_from_slice(&compressed.as_bytes()[1..31]);
    }

    /// Encode a 'Scalar' into two 'Element's.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if a point was not found for the input, with negligible probability
    pub fn encode_scalar(scalar: &RistrettoScalar) -> Result<[RistrettoElement; 2], Error> {
        // The scalar may be secret (a key share): it is encoded from a reference
        // to its bytes, not from a copy.
        Self::encode_bytes(scalar.0.as_bytes())
    }

    /// Decode a 'Scalar' from two 'Element's.
    ///
    /// # Errors
    ///
    /// - `ScalarDecodeError` if the bytes could not be parsed into a Ristretto scalar
    pub fn decode_scalar(element: &[RistrettoElement; 2]) -> Result<RistrettoScalar, Error> {
        // The scalar may be secret (a key share): its bytes are decoded into a
        // buffer that is overwritten on return.
        let mut bytes = Zeroizing::new([0u8; 32]);
        Self::decode_bytes_into::<2, 32>(element, &mut bytes);
        let opt: Option<RistrettoScalar> = DalekScalar::from_canonical_bytes(*bytes)
            .map(RistrettoScalar)
            .into();

        opt.ok_or(Error::ScalarDecodeError(
            "Failed to parse Ristretto scalar bytes".to_string(),
        ))
    }
}

/// Chunk size for encoding, Ristretto points can hold 30 bytes of data
const CHUNK_SIZE: usize = 30;
/// Byte-array chunking into per-element units, shared with the other backends.
type Codec<const BYTES: usize, const ELEMENTS: usize> =
    crate::groups::codec::Codec<CHUNK_SIZE, BYTES, ELEMENTS>;
