// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to serialize and deserialize data.
//! use strand::context::Ctx;
//! use strand::backend::ristretto::RistrettoCtx;
//! use strand::elgamal::{PrivateKey, PublicKey};
//! use strand::serialization::{StrandDeserialize, StrandSerialize};
//! use strand::elgamal::Ciphertext;
//!
//! let ctx = RistrettoCtx;
//! let mut rng = ctx.get_rng();
//! // generate an ElGamal keypair
//! let sk1 = PrivateKey::gen(&ctx);
//! let pk1 = sk1.get_pk();
//!
//! // generate a random plaintext
//! let plaintext = ctx.rnd_plaintext(&mut rng);
//! let encoded = ctx.encode(&plaintext).unwrap();
//!
//! // encrypt
//! let ciphertext = pk1.encrypt(&encoded);
//! // serialize
//! let serialized = ciphertext.strand_serialize().unwrap();
//! // deserialize
//! let deserialized = Ciphertext::<RistrettoCtx>::strand_deserialize(&serialized).unwrap();
//!
//! // decrypt
//! let decrypted = sk1.decrypt(&deserialized);
//! let plaintext_ = ctx.decode(&decrypted);
//! assert_eq!(plaintext, plaintext_);
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use std::io::{Error, ErrorKind};

use crate::util::{Par, StrandError};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Serialization frontend trait.
pub trait StrandSerialize {
    /// Serializes to bytes. Currently only supports the borsh backend.
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError>;
}

/// Deserialization frontend trait.
pub trait StrandDeserialize {
    /// Serializes from bytes. Currently only supports the borsh backend.
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError>
    where
        Self: Sized;
}

/// Any implementer of borshserialize implements strandserialize
impl<T: BorshSerialize> StrandSerialize for T {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        borsh::to_vec(self).map_err(|e| e.into())
    }
}

/// Any implementer of borshdeserialize implements stranddeserialize
impl<T: BorshDeserialize> StrandDeserialize for T {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError>
    where
        Self: Sized,
    {
        let value = T::try_from_slice(bytes);
        value.map_err(|e| e.into())
    }
}

#[derive(Clone, Debug)]
/// Parallel serialization vector
pub struct StrandVector<T: BorshSerialize + BorshDeserialize>(pub Vec<T>);

/// Parallel serialization for vectors
impl<T: BorshSerialize + BorshDeserialize + Send + Sync> BorshSerialize
    for StrandVector<T>
{
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| borsh::to_vec(t)).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

/// Parallel serialization for vectors
impl<T: BorshSerialize + BorshDeserialize + Send + Sync> BorshDeserialize
    for StrandVector<T>
{
    /*fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let results: std::io::Result<Vec<T>> =
            vectors.par().map(|v| T::try_from_slice(&v)).collect();

        Ok(StrandVector(results?))
    }*/
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let vectors = <Vec<Vec<u8>>>::deserialize_reader(reader)?;

        let results: std::io::Result<Vec<T>> =
            vectors.par().map(|v| T::try_from_slice(&v)).collect();

        Ok(StrandVector(results?))
    }
}

cfg_if::cfg_if! {
if #[cfg(not(feature = "wasm"))] {
use crate::shuffler_product::StrandRectangle;

/// Parallel serialization for rectangles
impl<T: Send + Sync + BorshSerialize> BorshSerialize for StrandRectangle<T> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = self.rows();

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| borsh::to_vec(t)).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

/// Parallel serialization for rectangles
impl<T: Send + Sync + BorshDeserialize> BorshDeserialize
    for StrandRectangle<T>
{
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let vectors = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let results: std::io::Result<Vec<Vec<T>>> = vectors
            .par()
            .map(|v| Vec::<T>::try_from_slice(&v))
            .collect();

        StrandRectangle::new(results?).map_err(|_| {
            Error::new(ErrorKind::Other, "Parsed bytes were not rectangular")
        })
    }

}

}}

#[cfg(test)]
pub(crate) mod tests {
    use super::StrandDeserialize;
    use super::StrandSerialize;
    use crate::context::Ctx;
    use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
    use crate::util;
    use crate::zkp::{ChaumPedersen, Schnorr, Zkp};

    pub(crate) fn test_borsh_element<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let e = ctx.rnd(&mut rng);

        let encoded_e = e.strand_serialize().unwrap();
        let decoded_e = C::E::strand_deserialize(&encoded_e).unwrap();
        assert!(e == decoded_e);
    }

    pub(crate) fn test_borsh_elements<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let elements: Vec<C::E> =
            (0..10).into_iter().map(|_| ctx.rnd(&mut rng)).collect();

        let encoded_e = elements.strand_serialize().unwrap();
        let decoded_e = Vec::<C::E>::strand_deserialize(&encoded_e).unwrap();
        assert!(elements == decoded_e);
    }

    pub(crate) fn test_borsh_exponent<C: Ctx>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let x = ctx.rnd_exp(&mut rng);

        let encoded_x = x.strand_serialize().unwrap();
        let decoded_x = C::X::strand_deserialize(&encoded_x).unwrap();
        assert!(x == decoded_x);
    }

    pub(crate) fn test_ciphertext_borsh_generic<C: Ctx>(ctx: &C) {
        let c = util::random_ciphertexts(1, ctx).remove(0);
        let bytes = c.strand_serialize().unwrap();
        let back = Ciphertext::<C>::strand_deserialize(&bytes).unwrap();

        assert!(c.mhr == back.mhr && c.gr == back.gr);
    }

    pub(crate) fn test_key_borsh_generic<C: Ctx + Eq>(ctx: &C) {
        let sk = PrivateKey::gen(ctx);
        let pk = PublicKey::from_element(&sk.pk_element, ctx);

        let bytes = sk.strand_serialize().unwrap();
        let back = PrivateKey::<C>::strand_deserialize(&bytes).unwrap();

        assert!(sk == back);

        let bytes = pk.strand_serialize().unwrap();
        let back = PublicKey::<C>::strand_deserialize(&bytes).unwrap();

        assert!(pk == back);
    }

    pub(crate) fn test_schnorr_borsh_generic<C: Ctx + Eq>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let zkp = Zkp::new(ctx);
        let g = ctx.generator();
        let secret = ctx.rnd_exp(&mut rng);
        let public = ctx.gmod_pow(&secret);
        let schnorr = zkp
            .schnorr_prove(&secret, &public, Some(&g), &vec![])
            .unwrap();
        let verified = zkp.schnorr_verify(&public, Some(&g), &schnorr, &vec![]);
        assert!(verified);

        let bytes = schnorr.strand_serialize().unwrap();
        let back = Schnorr::<C>::strand_deserialize(&bytes).unwrap();
        assert!(schnorr == back);

        let verified = zkp.schnorr_verify(&public, Some(&g), &back, &vec![]);
        assert!(verified);
    }

    pub(crate) fn test_cp_borsh_generic<C: Ctx + Eq>(ctx: &C) {
        let mut rng = ctx.get_rng();
        let zkp = Zkp::new(ctx);
        let g1 = ctx.generator();
        let g2 = ctx.rnd(&mut rng);
        let secret = ctx.rnd_exp(&mut rng);
        let public1 = ctx.emod_pow(g1, &secret);
        let public2 = ctx.emod_pow(&g2, &secret);
        let proof = zkp
            .cp_prove(&secret, &public1, &public2, None, &g2, &vec![])
            .unwrap();
        let verified =
            zkp.cp_verify(&public1, &public2, None, &g2, &proof, &vec![]);
        assert!(verified);

        let bytes = proof.strand_serialize().unwrap();
        let back = ChaumPedersen::<C>::strand_deserialize(&bytes).unwrap();
        assert!(proof == back);

        let verified =
            zkp.cp_verify(&public1, &public2, None, &g2, &back, &vec![]);
        assert!(verified);
    }
}
