// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to obtain a context to use the malachite backend.
//! use strand::context::{Ctx, Element};
//! use strand::backend::malachite::{MalachiteCtx, P2048};
//! use strand::backend::malachite::NaturalE;
//! let ctx = MalachiteCtx::<P2048>::default();
//! let mut rng = ctx.get_rng();
//! // do some stuff..
//! let g = ctx.generator();
//! let a = ctx.rnd_exp(&mut rng);
//! let b = ctx.rnd_exp(&mut rng);
//! let g_ab = ctx.emod_pow(&ctx.emod_pow(g, &a), &b);
//! let g_ba = ctx.emod_pow(&ctx.emod_pow(g, &b), &a);
//! assert_eq!(g_ab, g_ba);
//! ```
use std::fmt::Debug;
use std::io::{Error, ErrorKind};
use std::marker::PhantomData;
use std::ops::Rem;

use borsh::{BorshDeserialize, BorshSerialize};
use malachite::natural::random::uniform_random_natural_inclusive_range;
use malachite::num::arithmetic::traits::LegendreSymbol;
use malachite::num::arithmetic::traits::ModInverse;
use malachite::num::arithmetic::traits::ModPow;
use malachite::num::conversion::traits::Digits;
use malachite::num::conversion::traits::{FromStringBase, ToStringBase};
use malachite::random::Seed;
use malachite::Natural;

use rand::Rng;

use crate::backend::constants::*;
use crate::context::{Ctx, Element, Exponent, Plaintext};
use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util::StrandError;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct NaturalE<P: MalachiteCtxParams>(
    pub(crate) Natural,
    PhantomData<MalachiteCtx<P>>,
);
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct NaturalX<P: MalachiteCtxParams>(
    pub(crate) Natural,
    PhantomData<MalachiteCtx<P>>,
);

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct NaturalP(pub(crate) Natural);

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct MalachiteCtx<P: MalachiteCtxParams> {
    params: P,
}

impl<P: MalachiteCtxParams> MalachiteCtx<P> {
    // https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-4.pdf A.2.3
    fn generators_fips(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<NaturalE<P>>, StrandError> {
        let mut ret = Vec::with_capacity(size);
        let two = Natural::from(2u32);

        let mut prefix = seed.to_vec();
        prefix.extend("ggen".to_string().into_bytes());

        let mut index: u64 = 0;
        for _ in 0..size {
            index += 1;
            let mut next = prefix.clone();
            let mut count: u64 = 0;
            loop {
                count += 1;
                assert!(count != 0);
                next.extend(index.to_le_bytes());
                next.extend(count.to_le_bytes());
                let elem: Natural = self.hash_to_element(&next)?;
                let g = elem
                    .mod_pow(self.params.co_factor(), &self.params.modulus().0);
                if g >= two {
                    ret.push(NaturalE::new(g));
                    break;
                }
            }
        }

        Ok(ret)
    }

    fn hash_to_element(&self, bytes: &[u8]) -> Result<Natural, StrandError> {
        let hashed = crate::hash::hash(bytes)?;
        let u16s = hashed.into_iter().map(|b| b as u16);
        let num = Natural::from_digits_desc(&256u16, u16s).expect("impossible");
        Ok(num.rem(&self.params.modulus().0))
    }

    pub fn element_from_natural(
        &self,
        natural: Natural,
    ) -> Result<NaturalE<P>, StrandError> {
        let one: Natural = Natural::from(1u8);
        if (natural < one) || natural >= self.params.modulus().0 {
            Err(StrandError::Generic("Out of range".to_string()))
        } else if natural.clone().legendre_symbol(&self.params.modulus().0) != 1
        {
            Err(StrandError::Generic("Not a quadratic residue".to_string()))
        } else {
            Ok(NaturalE::new(natural))
        }
    }

    pub fn element_from_string_radix(
        &self,
        string: &str,
        radix: u8,
    ) -> Result<NaturalE<P>, StrandError> {
        let natural = Natural::from_string_base(radix, string)
            .ok_or(StrandError::Generic("Failed to parse".to_string()))?;

        self.element_from_natural(natural)
    }

    #[inline(always)]
    fn get_random_seed(&self, rng: &mut StrandRng) -> Seed {
        let mut seed_bytes = [0u8; 32];
        rng.fill(&mut seed_bytes);
        Seed::from_bytes(seed_bytes)
    }
}

impl<P: MalachiteCtxParams> Ctx for MalachiteCtx<P> {
    type E = NaturalE<P>;
    type X = NaturalX<P>;
    type P = NaturalP;
    type R = StrandRng;

    #[inline(always)]
    fn generator(&self) -> &Self::E {
        self.params.generator()
    }
    #[inline(always)]
    fn gmod_pow(&self, other: &Self::X) -> Self::E {
        NaturalE::new(
            self.generator()
                .0
                .clone()
                .mod_pow(&other.0, &self.params.modulus().0),
        )
    }
    #[inline(always)]
    fn emod_pow(&self, base: &Self::E, exponent: &Self::X) -> Self::E {
        NaturalE::new(
            base.0
                .clone()
                .mod_pow(&exponent.0, &self.params.modulus().0),
        )
    }
    #[inline(always)]
    fn modulo(&self, value: &Self::E) -> Self::E {
        value.modulo(self.params.modulus())
    }
    #[inline(always)]
    fn exp_modulo(&self, value: &Self::X) -> Self::X {
        value.modulo(self.params.exp_modulus())
    }
    #[inline(always)]
    fn exp_sub_mod(&self, value: &Self::X, other: &Self::X) -> Self::X {
        if value.0 > other.0 {
            value.sub(other).modulo(self.params.exp_modulus())
        } else {
            // Natural cannot hold negative numbers, so we add exp_modulus first
            value
                .add(self.params.exp_modulus())
                .sub(other)
                .modulo(self.params.exp_modulus())
        }
    }

    // Malachite does not support using _our_ rng, the best we can do is to
    // seed _its_ internal rng (chacha20) with random data generated with our
    // rng See https://docs.rs/malachite-base/0.4.0/src/malachite_base/random/mod.rs.html#46
    #[inline(always)]
    fn get_rng(&self) -> Self::R {
        StrandRng
    }
    #[inline(always)]
    fn rnd(&self, rng: &mut Self::R) -> Self::E {
        let seed = self.get_random_seed(rng);
        let one: Natural = Natural::from(1u8);
        let num = uniform_random_natural_inclusive_range(
            seed,
            Natural::from(0u8),
            &self.params.exp_modulus().0 - one,
        )
        .next()
        .expect("impossible: uniform_random_natural_inclusive_range implementation never returns None");

        let unencoded = NaturalP(num);

        self.encode(&unencoded)
            .expect("0..(q-1) should always be encodable")
    }
    #[inline(always)]
    fn rnd_exp(&self, rng: &mut Self::R) -> Self::X {
        let seed = self.get_random_seed(rng);
        let num = uniform_random_natural_inclusive_range(
            seed,
            Natural::from(0u8),
            self.params.exp_modulus().0.clone(),
        )
        .next()
        .expect("impossible: uniform_random_natural_inclusive_range implementation never returns None");

        NaturalX::new(num)
    }
    fn rnd_plaintext(&self, rng: &mut Self::R) -> Self::P {
        NaturalP(self.rnd_exp(rng).0)
    }

    fn encode(&self, plaintext: &Self::P) -> Result<Self::E, StrandError> {
        let one: Natural = Natural::from(1u8);

        if plaintext.0 >= (&self.params.exp_modulus().0 - &one) {
            return Err(StrandError::Generic(
                "Failed to encode, out of range".to_string(),
            ));
        }
        let notzero: Natural = plaintext.0.clone() + one;
        let legendre =
            notzero.clone().legendre_symbol(&self.params.modulus().0);
        if legendre == 0 {
            return Err(StrandError::Generic(
                "Failed to encode, legendre = 0".to_string(),
            ));
        }
        let result = if legendre == 1 {
            notzero
        } else {
            &self.params.modulus().0 - notzero
        };
        Ok(NaturalE::new(Natural::rem(
            result,
            &self.params.modulus().0,
        )))
    }
    fn decode(&self, element: &Self::E) -> Self::P {
        let one: Natural = Natural::from(1u8);
        if element.0 > self.params.exp_modulus().0 {
            NaturalP((&self.params.modulus().0 - &element.0) - one)
        } else {
            NaturalP(&element.0 - one)
        }
    }
    fn element_from_bytes(&self, bytes: &[u8]) -> Result<Self::E, StrandError> {
        let u16s = bytes.iter().map(|b| *b as u16);
        let num = Natural::from_digits_desc(&256, u16s).ok_or(
            StrandError::Generic("from_digits_desc returned None".to_string()),
        )?;
        self.element_from_natural(num)
    }
    fn exp_from_bytes(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        let u16s = bytes.iter().map(|b| *b as u16);
        let ret = Natural::from_digits_desc(&256u16, u16s).ok_or(
            StrandError::Generic("from_digits_desc returned None".to_string()),
        )?;
        let zero: Natural = Natural::from(0u8);
        if (ret < zero) || ret >= self.params.exp_modulus().0 {
            Err(StrandError::Generic("Out of range".to_string()))
        } else {
            Ok(NaturalX::new(ret))
        }
    }
    fn exp_from_u64(&self, value: u64) -> Self::X {
        NaturalX::new(Natural::from(value))
    }
    fn hash_to_exp(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        let hashed = crate::hash::hash(bytes)?;
        let u16s = hashed.into_iter().map(|b| b as u16);

        let num = Natural::from_digits_desc(&256, u16s).expect("impossible");
        Ok(NaturalX::new(num.rem(&self.params.exp_modulus().0)))
    }
    fn encrypt_exp(
        &self,
        exp: &Self::X,
        pk: PublicKey<Self>,
    ) -> Result<Vec<u8>, StrandError> {
        let encrypted = pk.encrypt(&self.encode(&NaturalP(exp.0.clone()))?);
        encrypted.strand_serialize()
    }
    fn decrypt_exp(
        &self,
        bytes: &[u8],
        sk: PrivateKey<Self>,
    ) -> Result<Self::X, StrandError> {
        let encrypted = Ciphertext::<Self>::strand_deserialize(bytes)?;
        let decrypted = sk.decrypt(&encrypted);
        Ok(NaturalX(self.decode(&decrypted).0, PhantomData))
    }

    fn generators(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<Self::E>, StrandError> {
        self.generators_fips(size, seed)
    }
}

impl<P: MalachiteCtxParams> Default for MalachiteCtx<P> {
    fn default() -> MalachiteCtx<P> {
        let params = P::new();
        MalachiteCtx { params }
    }
}

impl<P: MalachiteCtxParams + Eq> Element<MalachiteCtx<P>> for NaturalE<P> {
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        NaturalE::new(&self.0 * &other.0)
    }
    #[inline(always)]
    fn div(&self, other: &Self, modulus: &Self) -> Self {
        let inverse = Element::<MalachiteCtx<P>>::inv(other, modulus);
        NaturalE::new(&self.0 * inverse.0)
    }
    #[inline(always)]
    fn inv(&self, modulus: &Self) -> Self {
        let inverse = (&self.0).mod_inverse(&modulus.0);
        NaturalE::new(inverse.expect("there is always an inverse for prime p"))
    }
    #[inline(always)]
    fn mod_pow(&self, other: &NaturalX<P>, modulus: &Self) -> Self {
        NaturalE::new(self.0.clone().mod_pow(&other.0, &modulus.0))
    }
    #[inline(always)]
    fn modulo(&self, modulus: &Self) -> Self {
        NaturalE::new(self.0.clone().rem(&modulus.0))
    }
    #[inline(always)]
    fn modp(&self, ctx: &MalachiteCtx<P>) -> Self {
        NaturalE::new(ctx.modulo(self).0)
    }
    #[inline(always)]
    fn divp(&self, other: &Self, ctx: &MalachiteCtx<P>) -> Self {
        self.div(other, ctx.params.modulus())
    }
    #[inline(always)]
    fn invp(&self, ctx: &MalachiteCtx<P>) -> Self {
        self.inv(ctx.params.modulus())
    }
    fn mul_identity() -> Self {
        NaturalE::new(Natural::from(1u8))
    }
}

impl<P: MalachiteCtxParams + Eq> Exponent<MalachiteCtx<P>> for NaturalX<P> {
    #[inline(always)]
    fn add(&self, other: &Self) -> Self {
        NaturalX::new(&self.0 + &other.0)
    }
    #[inline(always)]
    fn sub(&self, other: &Self) -> Self {
        NaturalX::new(&self.0 - &other.0)
    }
    #[inline(always)]
    fn sub_mod(&self, other: &Self, ctx: &MalachiteCtx<P>) -> Self {
        ctx.exp_sub_mod(self, other)
    }
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        NaturalX::new(&self.0 * &other.0)
    }
    #[inline(always)]
    fn div(&self, other: &Self, modulus: &Self) -> Self {
        let inverse = Exponent::<MalachiteCtx<P>>::inv(other, modulus);
        NaturalX::new(&self.0 * inverse.0)
    }
    #[inline(always)]
    fn inv(&self, modulus: &Self) -> Self {
        let inverse = (&self.0).mod_inverse(&modulus.0);
        NaturalX::new(inverse.expect("there is always an inverse for prime p"))
    }
    #[inline(always)]
    fn modulo(&self, modulus: &Self) -> Self {
        NaturalX::new(self.0.clone().rem(&modulus.0))
    }
    #[inline(always)]
    fn modq(&self, ctx: &MalachiteCtx<P>) -> Self {
        NaturalX::new(ctx.exp_modulo(self).0)
    }
    #[inline(always)]
    fn divq(&self, other: &Self, ctx: &MalachiteCtx<P>) -> Self {
        self.div(other, ctx.params.exp_modulus())
    }
    #[inline(always)]
    fn invq(&self, ctx: &MalachiteCtx<P>) -> Self {
        self.inv(ctx.params.exp_modulus())
    }
    fn add_identity() -> Self {
        NaturalX::new(Natural::from(0u8))
    }
    fn mul_identity() -> Self {
        NaturalX::new(Natural::from(1u8))
    }
}

impl Plaintext for NaturalP {}

pub trait MalachiteCtxParams: Clone + Eq + Send + Sync + Debug {
    fn generator(&self) -> &NaturalE<Self>;
    fn modulus(&self) -> &NaturalE<Self>;
    fn exp_modulus(&self) -> &NaturalX<Self>;
    fn co_factor(&self) -> &Natural;
    fn new() -> Self;
}
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct P2048 {
    generator: NaturalE<Self>,
    modulus: NaturalE<Self>,
    exp_modulus: NaturalX<Self>,
    co_factor: Natural,
}

impl MalachiteCtxParams for P2048 {
    #[inline(always)]
    fn generator(&self) -> &NaturalE<Self> {
        &self.generator
    }
    #[inline(always)]
    fn modulus(&self) -> &NaturalE<Self> {
        &self.modulus
    }
    #[inline(always)]
    fn exp_modulus(&self) -> &NaturalX<Self> {
        &self.exp_modulus
    }
    #[inline(always)]
    fn co_factor(&self) -> &Natural {
        &self.co_factor
    }
    fn new() -> P2048 {
        let p = NaturalE::new(
            Natural::from_string_base(10, P_VERIFICATUM_STR_2048).unwrap(),
        );
        let q = NaturalX::new(
            Natural::from_string_base(10, Q_VERIFICATUM_STR_2048).unwrap(),
        );
        let g = NaturalE::new(
            Natural::from_string_base(10, G_VERIFICATUM_STR_2048).unwrap(),
        );
        let co_factor =
            Natural::from_string_base(16, SAFEPRIME_COFACTOR).unwrap();

        assert!(g.0.clone().legendre_symbol(&p.0) == 1);

        P2048 {
            generator: g,
            modulus: p,
            exp_modulus: q,
            co_factor,
        }
    }
}

impl<P: MalachiteCtxParams> NaturalE<P> {
    fn new(value: Natural) -> NaturalE<P> {
        NaturalE(value, PhantomData)
    }

    pub fn to_string_radix(&self, radix: u8) -> String {
        self.0.to_string_base(radix)
    }
}
impl<P: MalachiteCtxParams> NaturalX<P> {
    fn new(value: Natural) -> NaturalX<P> {
        NaturalX(value, PhantomData)
    }

    pub fn to_string_radix(&self, radix: u8) -> String {
        self.0.to_string_base(radix)
    }
}

impl<P: MalachiteCtxParams> BorshSerialize for NaturalE<P> {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_digits_desc(&256u16);
        let u8s: Vec<u8> = bytes.into_iter().map(|b| b as u8).collect();
        u8s.serialize(writer)
    }
}

impl<P: MalachiteCtxParams> BorshDeserialize for NaturalE<P> {
    #[inline]
    fn deserialize(bytes: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <Vec<u8>>::deserialize(bytes)?;
        let ctx: MalachiteCtx<P> = Default::default();

        ctx.element_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

impl<P: MalachiteCtxParams> BorshSerialize for NaturalX<P> {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_digits_desc(&256u16);
        let u8s: Vec<u8> = bytes.into_iter().map(|b| b as u8).collect();
        u8s.serialize(writer)
    }
}

impl<P: MalachiteCtxParams> BorshDeserialize for NaturalX<P> {
    #[inline]
    fn deserialize(bytes: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <Vec<u8>>::deserialize(bytes)?;
        let ctx = MalachiteCtx::<P>::default();

        ctx.exp_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

impl BorshSerialize for NaturalP {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_digits_desc(&256u16);
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for NaturalP {
    #[inline]
    fn deserialize(bytes: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <Vec<u16>>::deserialize(bytes)?;

        let num = Natural::from_digits_desc(&256u16, bytes.into_iter()).ok_or(
            Error::new(ErrorKind::Other, "from_digits_desc returned None"),
        )?;

        Ok(NaturalP(num))
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::malachite::*;
    use crate::backend::tests::*;
    use crate::context::Ctx;
    use crate::serialization::tests::*;

    #[test]
    fn test_elgamal() {
        let ctx = MalachiteCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_elgamal_generic(&ctx, plaintext);
    }

    #[test]
    fn test_elgamal_enc_pok() {
        let ctx = MalachiteCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_elgamal_enc_pok_generic(&ctx, plaintext);
    }

    #[test]
    fn test_encrypt_exp() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_encrypt_exp_generic(&ctx);
    }

    #[test]
    fn test_schnorr() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_schnorr_generic(&ctx);
    }

    #[test]
    fn test_chaumpedersen() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_chaumpedersen_generic(&ctx);
    }

    #[test]
    fn test_rerand() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_rerand_generic(&ctx);
    }

    #[test]
    fn test_vdecryption() {
        let ctx = MalachiteCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_vdecryption_generic(&ctx, plaintext);
    }

    #[test]
    fn test_element_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_borsh_element(&ctx);
    }

    #[test]
    fn test_elements_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_borsh_elements(&ctx);
    }

    #[test]
    fn test_exponent_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_borsh_exponent(&ctx);
    }

    #[test]
    fn test_ciphertext_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_ciphertext_borsh_generic(&ctx);
    }

    #[test]
    fn test_key_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_key_borsh_generic(&ctx);
    }

    #[test]
    fn test_schnorr_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_schnorr_borsh_generic(&ctx);
    }

    #[test]
    fn test_cp_borsh() {
        let ctx = MalachiteCtx::<P2048>::default();
        test_cp_borsh_generic(&ctx);
    }

    #[test]
    fn test_encode_err() {
        let ctx = MalachiteCtx::<P2048>::default();
        let one: Natural = Natural::from(1u8);
        let result = ctx.encode(&NaturalP(&ctx.params.exp_modulus().0 - one));
        assert!(result.is_err())
    }
}
