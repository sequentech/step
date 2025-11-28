// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to obtain a context to use the ristretto backend.
//! use strand::context::{Ctx, Element};
//! use strand::backend::ristretto::RistrettoCtx;
//! let ctx = RistrettoCtx;
//! let mut rng = ctx.get_rng();
//! // g^ab = g^ba
//! let g = ctx.generator();
//! let a = ctx.rnd_exp(&mut rng);
//! let b = ctx.rnd_exp(&mut rng);
//! let g_ab = ctx.emod_pow(&ctx.emod_pow(g, &a), &b);
//! let g_ba = ctx.emod_pow(&ctx.emod_pow(g, &b), &a);
//! assert_eq!(g_ab, g_ba);
//! ```
use std::io::Error;
use std::io::ErrorKind;

use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_TABLE;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::Identity;
use rand::RngCore;

use crate::context::{Ctx, Element, Exponent, Plaintext};
use crate::elgamal::Ciphertext;
use crate::elgamal::{PrivateKey, PublicKey};
use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util;
use crate::util::StrandError;

#[derive(Eq, PartialEq, Clone, Debug, BorshSerialize, BorshDeserialize)]
/// [Ristretto](https://ristretto.group/what_is_ristretto.html) implementation of a strand modular arithmetic context.
pub struct RistrettoCtx;

#[derive(PartialEq, Eq, Clone)]
/// A ristretto [RistrettoPoint](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/ristretto/struct.RistrettoPoint.html) newtype.
pub struct RistrettoPointS(pub(crate) RistrettoPoint);
#[derive(PartialEq, Eq, Debug, Clone)]
/// A ristretto [Scalar](https://docs.rs/curve25519-dalek/latest/curve25519_dalek/scalar/struct.Scalar.html) newtype.
pub struct ScalarS(pub(crate) Scalar);

cfg_if::cfg_if! {
    if #[cfg(any(feature = "openssl_core", feature="openssl_full"))] {

#[cfg(feature = "rayon")]
use rayon::prelude::*;
use crate::util::Par;

impl RistrettoCtx {
    fn generators_shake(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<RistrettoPointS>, StrandError> {
        let seed_ = seed.to_vec();

        let reader = crate::hash::hash_xof(64 * size, &seed_)?;
        let mut uniform_bytes = [0u8; 64];
        let mut bytes = vec![];
        for _ in 0..size {
            let bytes_read = std::io::Read::read(&mut reader.as_slice(), &mut uniform_bytes)
                .expect("impossible: we are reading from a byte slice, any out of bounds programming error should panic");
            assert_eq!(bytes_read, 64);
            bytes.push(uniform_bytes);
        }

        let ret: Vec<RistrettoPointS> = bytes.par().map(|b| {
            let g = RistrettoPoint::from_uniform_bytes(&b);
            RistrettoPointS(g)
        }).collect();

        Ok(ret)
    }
}
} else {

use crate::hash::{ExtendableOutput, Update, XofReader};

#[cfg(feature = "rayon")]
use rayon::prelude::*;
use crate::util::Par;

impl RistrettoCtx {
    // https://docs.rs/bulletproofs/4.0.0/src/bulletproofs/generators.rs.html
    fn generators_shake(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<RistrettoPointS>, StrandError> {
        let seed_ = seed.to_vec();

        // let mut ret: Vec<RistrettoPointS> = Vec::with_capacity(size);
        let mut shake = crate::hash::hasher_xof();
        shake.update(&seed_);

        let mut reader = shake.finalize_xof();
        let mut uniform_bytes = [0u8; 64];
        let mut bytes = vec![];

        for _ in 0..size {
            reader.read(&mut uniform_bytes);
            // let g = RistrettoPoint::from_uniform_bytes(&uniform_bytes);
            // ret.push(RistrettoPointS(g));
            bytes.push(uniform_bytes);
        }

        let ret: Vec<RistrettoPointS> = bytes.par().map(|b| {
            let g = RistrettoPoint::from_uniform_bytes(&b);
            RistrettoPointS(g)
        }).collect();

        Ok(ret)
    }
}
}
}

impl Ctx for RistrettoCtx {
    type E = RistrettoPointS;
    type X = ScalarS;
    type P = [u8; 30];
    type R = StrandRng;

    #[inline(always)]
    fn generator(&self) -> &Self::E {
        &RistrettoPointS(RISTRETTO_BASEPOINT_POINT)
    }
    #[inline(always)]
    fn gmod_pow(&self, other: &ScalarS) -> Self::E {
        RistrettoPointS(&other.0 * RISTRETTO_BASEPOINT_TABLE)
    }
    #[inline(always)]
    fn emod_pow(&self, base: &Self::E, exponent: &Self::X) -> Self::E {
        RistrettoPointS(base.0 * exponent.0)
    }
    #[inline(always)]
    // identity
    fn modulo(&self, value: &Self::E) -> Self::E {
        value.clone()
    }
    #[inline(always)]
    // identity
    fn exp_modulo(&self, value: &Self::X) -> Self::X {
        value.clone()
    }
    #[inline(always)]
    fn exp_sub_mod(&self, value: &Self::X, other: &Self::X) -> Self::X {
        value.sub(other)
    }

    #[inline(always)]
    fn get_rng(&self) -> StrandRng {
        StrandRng
    }
    #[inline(always)]
    fn rnd(&self, rng: &mut Self::R) -> Self::E {
        let mut uniform_bytes = [0u8; 64];
        rng.fill_bytes(&mut uniform_bytes);

        RistrettoPointS(RistrettoPoint::from_uniform_bytes(&uniform_bytes))
    }
    #[inline(always)]
    fn rnd_exp(&self, rng: &mut Self::R) -> Self::X {
        let mut uniform_bytes = [0u8; 64];
        rng.fill_bytes(&mut uniform_bytes);

        ScalarS(Scalar::from_bytes_mod_order_wide(&uniform_bytes))
    }
    fn rnd_plaintext(&self, rng: &mut Self::R) -> Self::P {
        let mut value = [0u8; 30];
        rng.fill_bytes(&mut value);

        value
    }
    fn hash_to_exp(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        /* let mut hasher = crate::hash::hasher();
        Digest::update(&mut hasher, bytes);

        ScalarS(Scalar::from_hash(hasher))*/

        let bytes = crate::hash::hash_to_array(bytes)?;
        Ok(ScalarS(Scalar::from_bytes_mod_order_wide(&bytes)))
    }
    // see https://github.com/dalek-cryptography/curve25519-dalek/issues/322
    // see https://github.com/hdevalence/ristretto255-data-encoding/blob/master/src/main.rs
    fn encode(&self, data: &[u8; 30]) -> Result<Self::E, StrandError> {
        let mut bytes = [0u8; 32];
        bytes[1..1 + data.len()].copy_from_slice(data);
        for j in 0..64 {
            bytes[31] = j as u8;
            for i in 0..128 {
                bytes[0] = 2 * i as u8;
                if let Some(point) = CompressedRistretto(bytes).decompress() {
                    return Ok(RistrettoPointS(point));
                }
            }
        }
        Err(StrandError::Generic(
            "Failed to encode into ristretto point".to_string(),
        ))
    }
    fn decode(&self, element: &Self::E) -> Self::P {
        let compressed = element.0.compress();
        // the 30 bytes of data are placed in the range 1-30
        let slice = &compressed.as_bytes()[1..31];
        to_ristretto_plaintext_array(slice)
            .expect("impossible, passed slice is size 30")
    }
    fn element_from_bytes(&self, bytes: &[u8]) -> Result<Self::E, StrandError> {
        let b32 = to_ristretto_point_array(bytes)?;
        CompressedRistretto(b32)
            .decompress()
            .map(RistrettoPointS)
            .ok_or(StrandError::Generic(
                "Failed constructing ristretto point".to_string(),
            ))
    }
    fn exp_from_bytes(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        let b32 = to_ristretto_point_array(bytes)?;
        let opt: Option<Self::X> =
            Scalar::from_canonical_bytes(b32).map(ScalarS).into();
        opt.ok_or(StrandError::Generic(
            "Failed constructing scalar".to_string(),
        ))
    }
    fn exp_from_u64(&self, value: u64) -> Self::X {
        let val_bytes = value.to_le_bytes();
        let mut bytes = [0u8; 32];
        let mut vec = val_bytes.to_vec();
        vec.resize(32, 0);
        bytes.copy_from_slice(&vec);
        let scalar = Scalar::from_bytes_mod_order(bytes);

        ///// FIXME remove this sanity check
        let mut scalar_bytes = scalar.as_bytes().to_vec();
        scalar_bytes.resize(8, 0);
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&scalar_bytes);
        let check = u64::from_le_bytes(bytes);
        assert_eq!(value, check);
        /////

        ScalarS(scalar)
    }

    fn encrypt_exp(
        &self,
        exp: &Self::X,
        pk: PublicKey<Self>,
    ) -> Result<Vec<u8>, StrandError> {
        let bytes = exp.0.to_bytes();
        let mut blank = vec![0; 30];
        blank[0..16].copy_from_slice(&bytes[0..16]);
        let first_array = to_ristretto_plaintext_array(&blank)?;
        let first = self.encode(&first_array);
        blank[0..16].copy_from_slice(&bytes[16..32]);
        let second_array = to_ristretto_plaintext_array(&blank)?;
        let second = self.encode(&second_array);
        let first_c = pk.encrypt(&first?);
        let second_c = pk.encrypt(&second?);

        vec![first_c, second_c].strand_serialize()
    }
    fn decrypt_exp(
        &self,
        bytes: &[u8],
        sk: PrivateKey<Self>,
    ) -> Result<Self::X, StrandError> {
        let vector = Vec::<Ciphertext<Self>>::strand_deserialize(bytes)?;
        if vector.len() == 2 {
            let first = self.decode(&sk.decrypt(&vector[0]));
            let second = self.decode(&sk.decrypt(&vector[1]));

            let mut concat = first[0..16].to_vec();
            concat.extend_from_slice(&second[0..16]);

            let ret = self.exp_from_bytes(&concat)?;

            Ok(ret)
        } else {
            Err(StrandError::Generic(
                "Ristretto encrypted exponent vector should have length 2"
                    .to_string(),
            ))
        }
    }
    fn generators(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<Self::E>, StrandError> {
        self.generators_shake(size, seed)
    }
}

impl Default for RistrettoCtx {
    fn default() -> RistrettoCtx {
        RistrettoCtx
    }
}

impl Element<RistrettoCtx> for RistrettoPointS {
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        RistrettoPointS(self.0 + other.0)
    }
    #[inline(always)]
    fn div(&self, other: &Self, _modulus: &Self) -> Self {
        RistrettoPointS(self.0 + other.inv(_modulus).0)
    }
    #[inline(always)]
    fn inv(&self, _modulus: &Self) -> Self {
        RistrettoPointS(-self.0)
    }
    #[inline(always)]
    fn mod_pow(&self, other: &ScalarS, _modulus: &Self) -> Self {
        RistrettoPointS(self.0 * other.0)
    }
    #[inline(always)]
    fn modulo(&self, _modulus: &Self) -> Self {
        self.clone()
    }
    #[inline(always)]
    fn modp(&self, _ctx: &RistrettoCtx) -> Self {
        self.clone()
    }
    #[inline(always)]
    fn divp(&self, other: &Self, ctx: &RistrettoCtx) -> Self {
        RistrettoPointS(self.0 + other.invp(ctx).0)
    }
    #[inline(always)]
    fn invp(&self, _ctx: &RistrettoCtx) -> Self {
        RistrettoPointS(-self.0)
    }
    fn mul_identity() -> Self {
        RistrettoPointS(RistrettoPoint::identity())
    }
}

impl Exponent<RistrettoCtx> for ScalarS {
    #[inline(always)]
    fn add(&self, other: &Self) -> Self {
        ScalarS(self.0 + other.0)
    }
    #[inline(always)]
    fn sub(&self, other: &Self) -> Self {
        ScalarS(self.0 - other.0)
    }
    #[inline(always)]
    fn sub_mod(&self, other: &Self, ctx: &RistrettoCtx) -> Self {
        ctx.exp_sub_mod(self, other)
    }
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        ScalarS(self.0 * other.0)
    }
    #[inline(always)]
    fn div(&self, other: &ScalarS, _modulus: &ScalarS) -> ScalarS {
        ScalarS(self.0 * other.0.invert())
    }
    #[inline(always)]
    fn inv(&self, _modulus: &Self) -> Self {
        ScalarS(self.0.invert())
    }
    #[inline(always)]
    fn modulo(&self, _modulus: &Self) -> Self {
        self.clone()
    }
    #[inline(always)]
    fn modq(&self, _ctx: &RistrettoCtx) -> Self {
        self.clone()
    }
    #[inline(always)]
    fn divq(&self, other: &ScalarS, _ctx: &RistrettoCtx) -> ScalarS {
        ScalarS(self.0 * other.0.invert())
    }
    #[inline(always)]
    fn invq(&self, _ctx: &RistrettoCtx) -> Self {
        ScalarS(self.0.invert())
    }
    fn add_identity() -> Self {
        ScalarS(Scalar::ZERO)
    }
    fn mul_identity() -> Self {
        ScalarS(Scalar::ONE)
    }
}

impl Plaintext for [u8; 30] {}

impl BorshSerialize for RistrettoPointS {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.compress().to_bytes();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for RistrettoPointS {
    /// Deserializes the given bytes into a point, checking for membership.
    #[inline]
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <[u8; 32]>::deserialize_reader(reader)?;
        // We duplicate this code in order to avoid the copying in
        // ctx.element_from_bytes Note we are passing the [u8; 32]
        // directly instead of passing through
        // to_ristretto_point_array(bytes) which takes a slice
        CompressedRistretto(bytes)
            .decompress()
            .map(RistrettoPointS)
            .ok_or(Error::new(
                ErrorKind::Other,
                "Failed to decode ristretto point",
            ))

        /* let ctx = RistrettoCtx::default();

        ctx.element_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))*/
    }
}

impl BorshSerialize for ScalarS {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_bytes();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for ScalarS {
    #[inline]
    /// Deserializes the given bytes into a scalar, checking for membership.
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        // We duplicate this code in order to avoid the copying in
        // ctx.exp_from_bytes Note we are passing the [u8; 32] directly
        // instead of passing through to_ristretto_point_array(bytes)
        // which takes a slice
        let bytes = <[u8; 32]>::deserialize_reader(reader)?;
        let opt: Option<ScalarS> =
            Scalar::from_canonical_bytes(bytes).map(ScalarS).into();
        opt.ok_or(Error::new(ErrorKind::Other, "Failed to decode scalar"))
        /* let ctx = RistrettoCtx::default();

        ctx.exp_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))*/
    }
}

impl std::fmt::Debug for RistrettoPointS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RistrettoPointS {{ value={} }}",
            &hex::encode(self.0.compress().as_bytes())[0..10]
        )
    }
}

pub(crate) fn to_ristretto_point_array(
    input: &[u8],
) -> Result<[u8; 32], StrandError> {
    util::to_u8_array(input)
}
pub(crate) fn to_ristretto_plaintext_array(
    input: &[u8],
) -> Result<[u8; 30], StrandError> {
    util::to_u8_array(input)
}

#[cfg(test)]
mod tests {
    use crate::backend::ristretto::*;
    use crate::backend::tests::*;
    use crate::keymaker::tests::*;
    use crate::serialization::tests::*;
    use crate::threshold::tests::test_threshold_generic;

    fn to_plaintext_array(input: &[u8]) -> [u8; 30] {
        super::to_ristretto_plaintext_array(input).unwrap()
    }

    #[test]
    fn test_elgamal() {
        let mut csprng = StrandRng;

        let ctx = RistrettoCtx;
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let plaintext = to_plaintext_array(&fill.to_vec());
        test_elgamal_generic(&ctx, plaintext);
    }

    #[test]
    fn test_elgamal_enc_pok() {
        let mut csprng = StrandRng;

        let ctx = RistrettoCtx;
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let plaintext = to_plaintext_array(&fill.to_vec());
        test_elgamal_enc_pok_generic(&ctx, plaintext);
    }

    #[test]
    fn test_encrypt_exp() {
        let ctx = RistrettoCtx;
        test_encrypt_exp_generic(&ctx);
    }

    #[test]
    fn test_schnorr() {
        let ctx = RistrettoCtx;
        test_schnorr_generic(&ctx);
    }

    #[test]
    fn test_chaumpedersen() {
        let ctx = RistrettoCtx;
        test_chaumpedersen_generic(&ctx);
    }

    #[test]
    fn test_rerand() {
        let ctx = RistrettoCtx;
        test_rerand_generic(&ctx);
    }

    #[test]
    fn test_vdecryption() {
        let mut csprng = StrandRng;

        let ctx = RistrettoCtx;
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let plaintext = to_plaintext_array(&fill.to_vec());
        test_vdecryption_generic(&ctx, plaintext);
    }

    #[test]
    fn test_distributed() {
        let mut csprng = StrandRng;

        let ctx = RistrettoCtx;
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let plaintext = to_plaintext_array(&fill.to_vec());
        test_distributed_generic(&ctx, plaintext);
    }

    #[test]
    fn test_distributed_serialization() {
        let mut csprng = StrandRng;

        let ctx = RistrettoCtx;
        let mut ps = vec![];
        for _ in 0..10 {
            let mut fill = [0u8; 30];
            csprng.fill_bytes(&mut fill);
            let p = to_plaintext_array(&fill.to_vec());
            ps.push(p);
        }
        test_distributed_serialization_generic(&ctx, ps);
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn test_shuffle() {
        let ctx = RistrettoCtx;
        test_shuffle_generic(&ctx);
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    pub(crate) fn test_shuffle_with_perm_rng() {
        use std::time::Instant;
        use rand_chacha::{ChaCha12Rng, rand_core::SeedableRng};

        use crate::shuffler::Shuffler;
        
        let ctx = &RistrettoCtx;
        let mut csprng = ctx.get_rng();

        let mut ps = vec![];
        for _ in 0..1000 {
            let mut fill = [0u8; 30];
            csprng.fill_bytes(&mut fill);
            let p = to_plaintext_array(&fill.to_vec());
            ps.push(p);
        }        
        
        let sk = PrivateKey::gen(ctx);
        let pk = sk.get_pk();
        println!("Computing ciphertexts..");
        let es = ps.iter()
            .map(|p| {
                let encoded = ctx.encode(p).unwrap();
                pk.encrypt(&encoded)
            })
            .collect::<Vec<_>>();
        
        let seed = vec![];
        let now = Instant::now(); println!("* generators..");
        let hs = ctx.generators(es.len() + 1, &seed).unwrap();
        println!("* generators {}", now.elapsed().as_millis());
        let shuffler = Shuffler {
            pk: &pk,
            ctx: (*ctx).clone(),
        };

        let beg = Instant::now();

        let rng_seed = [0u8; 32];
        let mut rng = ChaCha12Rng::from_seed(rng_seed);

        let now = Instant::now(); println!("* gen shuffle..");
        let (e_primes, rs, perm) = shuffler.gen_shuffle_with_perm_rng(&es, &mut rng);
        println!("* gen shuffle {}", now.elapsed().as_millis());
        let now = Instant::now();println!("* gen proof..");
        let proof = shuffler.gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &[]).unwrap();
        println!("* gen proof {}", now.elapsed().as_millis());
        let now = Instant::now(); println!("* check proof..");
        let ok = shuffler.check_proof(&proof, es.clone(), e_primes.clone(), hs, &[]).unwrap();
        assert!(ok);
        println!("* check proof {}", now.elapsed().as_millis());

        println!("All shuffle {}", beg.elapsed().as_millis());

        let ds = e_primes.iter()
            .map(|c| {
                let decrypted = sk.decrypt(c);
                ctx.decode(&decrypted)
            })
            .collect::<Vec<_>>();

        assert_eq!(ds.len(), ps.len());
        for d in &ds {
            assert!(ps.contains(d));
        }

        let seed = vec![];
        let now = Instant::now(); println!("* generators..");
        let hs = ctx.generators(es.len() + 1, &seed).unwrap();
        println!("* generators {}", now.elapsed().as_millis());
        let shuffler = Shuffler {
            pk: &pk,
            ctx: (*ctx).clone(),
        };

        let beg = Instant::now();
        let rng_seed = [0u8; 32];
        let mut rng = ChaCha12Rng::from_seed(rng_seed);

        let now = Instant::now(); println!("* gen shuffle..");
        let (e_primes, rs, perm) = shuffler.gen_shuffle_with_perm_rng(&es, &mut rng);
        println!("* gen shuffle {}", now.elapsed().as_millis());
        let now = Instant::now();println!("* gen proof..");
        let proof = shuffler.gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &[]).unwrap();
        println!("* gen proof {}", now.elapsed().as_millis());
        let now = Instant::now(); println!("* check proof..");
        let ok = shuffler.check_proof(&proof, es, e_primes.clone(), hs, &[]).unwrap();
        println!("* check proof {}", now.elapsed().as_millis());

        println!("All shuffle {}", beg.elapsed().as_millis());

        let ds2 = e_primes.iter()
            .map(|c| {
                let decrypted = sk.decrypt(c);
                ctx.decode(&decrypted)
            })
            .collect::<Vec<_>>();

        assert_eq!(ds2.len(), ps.len());
        for d in &ds2 {
            assert!(ps.contains(d));
        }

        assert_eq!(ds, ds2);

        assert!(ok);
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn test_product_shuffle() {
        let ctx = RistrettoCtx;
        test_product_shuffle_generic(&ctx);
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn test_shuffle_serialization() {
        let ctx = RistrettoCtx;
        test_shuffle_serialization_generic(&ctx);
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn test_product_shuffle_serialization() {
        let ctx = RistrettoCtx;
        test_product_shuffle_serialization_generic(&ctx);
    }

    use rand::Rng;

    #[test]
    fn test_threshold() {
        let mut csprng = StrandRng;

        let trustees = rand::rng().random_range(2..11);
        let threshold = rand::rng().random_range(2..trustees + 1);
        let ctx = RistrettoCtx;
        let mut fill = [0u8; 30];
        csprng.fill_bytes(&mut fill);
        let plaintext = to_plaintext_array(&fill.to_vec());

        test_threshold_generic(&ctx, trustees, threshold, plaintext);
    }

    #[test]
    fn test_element_borsh() {
        let ctx = RistrettoCtx;
        test_borsh_element(&ctx);
    }

    #[test]
    fn test_elements_borsh() {
        let ctx = RistrettoCtx;
        test_borsh_elements(&ctx);
    }

    #[test]
    fn test_exponent_borsh() {
        let ctx = RistrettoCtx;
        test_borsh_exponent(&ctx);
    }

    #[test]
    fn test_ciphertext_borsh() {
        let ctx = RistrettoCtx;
        test_ciphertext_borsh_generic(&ctx);
    }

    #[test]
    fn test_key_borsh() {
        let ctx = RistrettoCtx;
        test_key_borsh_generic(&ctx);
    }

    #[test]
    fn test_schnorr_borsh() {
        let ctx = RistrettoCtx;
        test_schnorr_borsh_generic(&ctx);
    }

    #[test]
    fn test_cp_borsh() {
        let ctx = RistrettoCtx;
        test_cp_borsh_generic(&ctx);
    }
}
