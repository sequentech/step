// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! CryptographicGroup implementations for the P-256 group

use crate::groups::p256::element::P256Element;
use crate::groups::p256::scalar::P256Scalar;
use crate::traits::groups::CryptographicGroup;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;

use p256::NistP256;
use p256::ProjectivePoint;
use p256::hash2curve::{ExpandMsgXmd, GroupDigest, hash_to_scalar};
use p256::elliptic_curve::array::sizes::U32;

use crate::utils::error::Error;
use crate::utils::rng;

/// P-256 implementation of [`CryptographicGroup`]
pub struct P256Group;

#[allow(clippy::arithmetic_side_effects)]
impl CryptographicGroup for P256Group {
    type Element = P256Element;
    type Scalar = P256Scalar;
    // This should be a default at the group trait, but Rust doesn't support that yet
    type Hasher = crate::context::CryptographicHasher;

    fn generator() -> Self::Element {
        P256Element::new(ProjectivePoint::GENERATOR)
    }

    fn g_exp(scalar: &Self::Scalar) -> Self::Element {
        P256Element::new(ProjectivePoint::GENERATOR * scalar.0)
    }

    /// # Errors
    ///
    /// - `HashToScalarError` if `NistP256::hash_to_scalar` returns error
    #[crate::warning("Panics on empty input")]
    fn hash_to_scalar(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Scalar, Error> {
        let ret = hash_to_scalar::<NistP256, ExpandMsgXmd<Self::Hasher>, U32>(input_slices, ds_tags)
            .map_err(Error::HashToScalarError);

        Ok(P256Scalar(ret?))
    }

    /// # Errors
    ///
    /// - `HashToElementError` if `NistP256::hash_from_bytes` returns error
    #[crate::warning("Panics on empty input")]
    fn hash_to_element(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Element, Error> {
        let ret = NistP256::hash_from_bytes(input_slices, ds_tags);
        let ret: Result<ProjectivePoint, Error> =
            ret.map_err(|e| Error::HashToElementError(e.to_string()));

        Ok(P256Element(ret?))
    }

    fn random_element<R: rng::CRng>(rng: &mut R) -> Self::Element {
        Self::Element::random(rng)
    }

    fn random_scalar<R: rng::CRng>(rng: &mut R) -> Self::Scalar {
        Self::Scalar::random(rng)
    }

    /// Encode bytes into the given number of P-256 elements.
    ///
    /// The input is split into `CHUNK_SIZE`-byte (30-byte) chunks, one per element; `O`
    /// must equal `I.div_ceil(CHUNK_SIZE)`, which is checked at compile time.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if no point was found for a chunk, with negligible
    ///   probability (see [`P256Group::encode_30_bytes`])
    fn encode_bytes<const I: usize, const O: usize>(
        bytes: &[u8; I],
    ) -> Result<[Self::Element; O], Error> {
        let chunks = Codec::<I, O>::split(bytes);
        let elements: Result<Vec<P256Element>, Error> =
            chunks.iter().map(Self::encode_30_bytes).collect();
        Ok(elements?.try_into().expect("elements.len() == O"))
    }

    /// Decode bytes from the given number of P-256 elements. Inverse of
    /// [`encode_bytes`](Self::encode_bytes).
    ///
    /// # Errors
    ///
    /// - `EncodingError` if an element is the identity, which encodes no data
    fn decode_bytes<const I: usize, const O: usize>(
        element: &[Self::Element; I],
    ) -> Result<[u8; O], Error> {
        let chunks: Result<Vec<[u8; CHUNK_SIZE]>, Error> =
            element.iter().map(Self::decode_30_bytes).collect();
        let chunks: [[u8; CHUNK_SIZE]; I] =
            chunks?.try_into().expect("chunks.len() == I");
        Ok(Codec::<O, I>::join(&chunks))
    }

    /// # Errors
    ///
    /// - `HashToElementError` if `NistP256::hash_from_bytes` returns error
    fn ind_generators(count: usize, label: &[u8]) -> Result<Vec<Self::Element>, Error> {
        let ds_tags: &[&[u8]] = &[b"context", b"independent_generators_p256_counter"];
        let mut ret = vec![];

        #[crate::warning("The following code is not optimized. Parallelize with rayon")]
        for i in 0..count {
            // Cannot use platform dependent type in random oracle
            let i_u64 = i as u64;
            let inputs = &[label, &i_u64.to_be_bytes()];
            let point = NistP256::hash_from_bytes(inputs, ds_tags)
                .map_err(|e| Error::HashToElementError(e.to_string()));
            
            ret.push(P256Element(point?));
        }

        Ok(ret)
    }

    /// Encrypt a scalar with ElGamal encryption using the given public key
    ///
    /// # Errors
    ///
    /// - `EncodingError` if the scalar cannot be encoded into elements
    fn encrypt_scalar(scalar: &Self::Scalar, public_key: &Self::Element) -> Result<Vec<u8>, Error> {
        use crate::context::P256Ctx;
        use crate::cryptosystem::elgamal::{Ciphertext, PublicKey};
        use crate::utils::serialization::Serializable;

        let elements = Self::encode_scalar(scalar)?;
        let pk = PublicKey::new(*public_key);
        let ciphertext: Ciphertext<P256Ctx, 2> = pk.encrypt(&elements);

        Ok(ciphertext.ser())
    }

    /// Decrypt a scalar from ElGamal-encrypted serialized ciphertext
    ///
    /// # Errors
    ///
    /// - `DeserializationError` if the ciphertext cannot be deserialized
    /// - `ScalarDecodeError` if the decrypted elements cannot be decoded
    fn decrypt_scalar(ciphertext: &[u8], secret_key: &Self::Scalar) -> Result<Self::Scalar, Error> {
        use crate::context::P256Ctx;
        use crate::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
        use crate::utils::serialization::Deserializable;

        let ciphertext = Ciphertext::<P256Ctx, 2>::deser(ciphertext)?;
        let pk = PublicKey::new(Self::generator().exp(secret_key));
        let keypair = KeyPair {
            skey: *secret_key,
            pkey: pk,
        };
        let elements = keypair.decrypt(&ciphertext);

        Self::decode_scalar(&elements)
    }
}

/// Bytes of payload carried by one P-256 element.
///
/// An x-coordinate is 32 bytes. One leading byte is held at zero so the value is
/// always below the field prime `p` (whose top byte is `0xff`), and one trailing
/// byte is a search counter, leaving 30 bytes of payload — the same capacity as
/// the Ristretto encoding, so both backends chunk identically.
const CHUNK_SIZE: usize = 30;

/// Byte-array chunking into per-element units, shared with the other backends.
type Codec<const BYTES: usize, const ELEMENTS: usize> =
    crate::groups::codec::Codec<CHUNK_SIZE, BYTES, ELEMENTS>;

impl P256Group {
    /// Embed 30 bytes into a curve point.
    ///
    /// Data is placed in an x-coordinate and a counter byte is varied until the
    /// candidate is on the curve. Roughly half of all field elements are valid
    /// x-coordinates, so a byte of counter gives a failure probability around
    /// `2^-256`; the error case is therefore unreachable in practice but is
    /// reported rather than panicked on.
    ///
    /// Layout of the 32-byte x-coordinate:
    ///
    /// ```text
    /// [0]      0x00      guard, keeps x < p
    /// [1..31]  payload   the 30 data bytes
    /// [31]     counter   varied until a point is found
    /// ```
    ///
    /// The y-coordinate carries no data; the even root is always chosen so the
    /// encoding is deterministic.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if no counter value yields a point on the curve
    pub fn encode_30_bytes(input: &[u8; CHUNK_SIZE]) -> Result<P256Element, Error> {
        use p256::elliptic_curve::point::DecompressPoint;
        use p256::elliptic_curve::subtle::Choice;

        let mut x = [0u8; 32];
        x[1..=CHUNK_SIZE].copy_from_slice(input);

        for counter in 0..=u8::MAX {
            x[31] = counter;
            let candidate =
                p256::AffinePoint::decompress(&x.into(), Choice::from(0));
            if bool::from(candidate.is_some()) {
                let affine = candidate.expect("candidate.is_some() == true");
                return Ok(P256Element(ProjectivePoint::from(affine)));
            }
        }

        Err(Error::EncodingError(
            "no point found for input, this should not happen".to_string(),
        ))
    }

    /// Recover the 30 payload bytes embedded by [`Self::encode_30_bytes`].
    ///
    /// # Errors
    ///
    /// - `EncodingError` if the element is the identity, which carries no data
    pub fn decode_30_bytes(element: &P256Element) -> Result<[u8; CHUNK_SIZE], Error> {
        let (x, _y) = element.to_affine_xy().ok_or_else(|| {
            Error::EncodingError("cannot decode the identity element".to_string())
        })?;
        let mut out = [0u8; CHUNK_SIZE];
        out.copy_from_slice(&x[1..=CHUNK_SIZE]);
        Ok(out)
    }

    /// Encode a scalar into two elements.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if a point was not found, with negligible probability
    pub fn encode_scalar(scalar: &P256Scalar) -> Result<[P256Element; 2], Error> {
        use crate::utils::serialization::Serializable;
        let mut bytes = Vec::with_capacity(32);
        scalar.write(&mut bytes);
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| Error::EncodingError("scalar is not 32 bytes".to_string()))?;

        Self::encode_bytes(&bytes)
    }

    /// Decode a scalar from two elements.
    ///
    /// # Errors
    ///
    /// - `ScalarDecodeError` if the bytes are not a canonical P-256 scalar
    pub fn decode_scalar(element: &[P256Element; 2]) -> Result<P256Scalar, Error> {
        use crate::utils::serialization::Deserializable;
        let bytes = Self::decode_bytes::<2, 32>(element)?;
        P256Scalar::deser(&bytes)
    }
}
