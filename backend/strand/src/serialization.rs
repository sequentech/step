// SPDX-FileCopyrightText: 2023 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::elgamal::Ciphertext;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::context::Ctx;
use crate::zkp::ChaumPedersen;

use crate::util::{Par, StrandError};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

/// Serialization frontend trait.
pub trait StrandSerialize {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError>;
}

/// Deserialization frontend trait.
pub trait StrandDeserialize {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError>
    where
        Self: Sized;
}

impl<T: BorshSerialize> StrandSerialize for T {
    fn strand_serialize(&self) -> Result<Vec<u8>, StrandError> {
        self.try_to_vec().map_err(|e| e.into())
    }
}

impl<T: BorshDeserialize> StrandDeserialize for T {
    fn strand_deserialize(bytes: &[u8]) -> Result<Self, StrandError>
    where
        Self: Sized,
    {
        let value = T::try_from_slice(bytes);
        value.map_err(|e| e.into())
    }
}

// Optimized (par) serialization vectors

/// Parallelized serialization for plaintext vectors.
#[derive(Clone, Debug)]
pub struct StrandVectorP<C: Ctx>(pub Vec<C::P>);

impl<C: Ctx> BorshSerialize for StrandVectorP<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for StrandVectorP<C> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let results: std::io::Result<Vec<C::P>> =
            vectors.par().map(|v| C::P::try_from_slice(&v)).collect();

        Ok(StrandVectorP(results?))
    }
}

/// Parallelized serialization for group element vectors.
#[derive(Clone, Debug)]
pub struct StrandVectorE<C: Ctx>(pub Vec<C::E>);

impl<C: Ctx> BorshSerialize for StrandVectorE<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for StrandVectorE<C> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let results: std::io::Result<Vec<C::E>> =
            vectors.par().map(|v| C::E::try_from_slice(&v)).collect();

        Ok(StrandVectorE(results?))
    }
}

/// Parallelized serialization for "exponent" vectors.
#[derive(Clone, Debug)]
pub struct StrandVectorX<C: Ctx>(pub Vec<C::X>);

impl<C: Ctx> BorshSerialize for StrandVectorX<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for StrandVectorX<C> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let results: std::io::Result<Vec<C::X>> =
            vectors.par().map(|v| C::X::try_from_slice(&v)).collect();

        Ok(StrandVectorX(results?))
    }
}

/// Parallelized serialization for ciphertext vectors.
#[derive(Clone, Debug)]
pub struct StrandVectorC<C: Ctx>(pub Vec<Ciphertext<C>>);

impl<C: Ctx> BorshSerialize for StrandVectorC<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for StrandVectorC<C> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;
        let results: std::io::Result<Vec<Ciphertext<C>>> = vectors
            .par()
            .map(|v| Ciphertext::<C>::try_from_slice(&v))
            .collect();

        Ok(StrandVectorC(results?))
    }
}

/// Parallelized serialization for ChaumPedersen proof vectors.
#[derive(Debug)]
pub struct StrandVectorCP<C: Ctx>(pub Vec<ChaumPedersen<C>>);

impl<C: Ctx> BorshSerialize for StrandVectorCP<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let vector = &self.0;

        let vecs: Result<Vec<Vec<u8>>, std::io::Error> =
            vector.par().map(|t| t.try_to_vec()).collect();
        let inside = vecs?;

        inside.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for StrandVectorCP<C> {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let vectors = <Vec<Vec<u8>>>::deserialize(buf)?;

        let results: std::io::Result<Vec<ChaumPedersen<C>>> = vectors
            .par()
            .map(|v| ChaumPedersen::<C>::try_from_slice(&v))
            .collect();

        Ok(StrandVectorCP(results?))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::StrandDeserialize;
    use super::StrandSerialize;
    use crate::context::Ctx;
    use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
    use crate::util;
    use crate::zkp::{ChaumPedersen, Schnorr, Zkp};

    pub(crate) fn test_borsh_element<C: Ctx>(ctx: &C) {
        let e = ctx.rnd();

        let encoded_e = e.strand_serialize().unwrap();
        let decoded_e = C::E::strand_deserialize(&encoded_e).unwrap();
        assert!(e == decoded_e);
    }

    pub(crate) fn test_borsh_elements<C: Ctx>(ctx: &C) {
        let elements: Vec<C::E> =
            (0..10).into_iter().map(|_| ctx.rnd()).collect();

        let encoded_e = elements.strand_serialize().unwrap();
        let decoded_e = Vec::<C::E>::strand_deserialize(&encoded_e).unwrap();
        assert!(elements == decoded_e);
    }

    pub(crate) fn test_borsh_exponent<C: Ctx>(ctx: &C) {
        let x = ctx.rnd_exp();

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
        let zkp = Zkp::new(ctx);
        let g = ctx.generator();
        let secret = ctx.rnd_exp();
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
        let zkp = Zkp::new(ctx);
        let g1 = ctx.generator();
        let g2 = ctx.rnd();
        let secret = ctx.rnd_exp();
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
