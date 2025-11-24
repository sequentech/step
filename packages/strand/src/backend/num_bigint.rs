// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to obtain a context to use the num_bigint backend.
//! use strand::context::{Ctx, Element};
//! use strand::backend::num_bigint::{BigintCtx, P2048};
//! use strand::backend::num_bigint::BigUintE;
//! let ctx = BigintCtx::<P2048>::default();
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

use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::RandBigInt;
use num_bigint::{BigUint, ParseBigIntError};
use num_integer::Integer;
use num_modular::{ModularSymbols, ModularUnaryOps};
use num_traits::{Num, One, Zero};

use crate::backend::constants::*;
use crate::context::{Ctx, Element, Exponent, Plaintext};
use crate::elgamal::{Ciphertext, PrivateKey, PublicKey};
use crate::rng::StrandRng;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::util::StrandError;

pub trait SerializeNumber {
    fn to_str_radix(&self, radix: u32) -> String;
}

pub trait DeserializeNumber: Sized {
    fn from_str_radix(s: &str, radix: u32) -> Result<Self, ParseBigIntError>;
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BigUintE<P: BigintCtxParams>(
    pub(crate) BigUint,
    PhantomData<BigintCtx<P>>,
);
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BigUintX<P: BigintCtxParams>(
    pub(crate) BigUint,
    PhantomData<BigintCtx<P>>,
);

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct BigUintP(pub(crate) BigUint);

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct BigUintCoFactor<P: BigintCtxParams>(
    pub(crate) BigUint,
    PhantomData<BigintCtx<P>>,
);

impl DeserializeNumber for BigUintP {
    fn from_str_radix(
        s: &str,
        radix: u32,
    ) -> Result<BigUintP, ParseBigIntError> {
        let val = BigUint::from_str_radix(s, radix)?;
        Ok(BigUintP(val))
    }
}

#[derive(Eq, PartialEq, Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct BigintCtx<P: BigintCtxParams> {
    params: P,
}

impl<P: BigintCtxParams> SerializeNumber for BigUintE<P> {
    fn to_str_radix(&self, radix: u32) -> String {
        self.0.to_str_radix(radix)
    }
}

impl<P: BigintCtxParams> SerializeNumber for BigUintX<P> {
    fn to_str_radix(&self, radix: u32) -> String {
        self.0.to_str_radix(radix)
    }
}

impl<P: BigintCtxParams> BigintCtx<P> {
    // https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-4.pdf A.2.3
    fn generators_fips(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<BigUintE<P>>, StrandError> {
        let mut ret = Vec::with_capacity(size);
        let two = BigUint::from(2u32);

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
                let elem: BigUint = self.hash_to_element(&next)?;
                let g = elem
                    .modpow(self.params.co_factor(), &self.params.modulus().0);
                if g >= two {
                    ret.push(BigUintE::new(g));
                    break;
                }
            }
        }

        Ok(ret)
    }

    fn hash_to_element(&self, bytes: &[u8]) -> Result<BigUint, StrandError> {
        let hashed = crate::hash::hash(bytes)?;

        let num = BigUint::from_bytes_le(&hashed);
        Ok(num.mod_floor(&self.params.modulus().0))
    }

    pub fn element_from_biguint(
        &self,
        biguint: BigUint,
    ) -> Result<BigUintE<P>, StrandError> {
        let one: BigUint = One::one();
        if (biguint < one) || biguint >= self.params.modulus().0 {
            Err(StrandError::Generic("Out of range".to_string()))
        } else if biguint.legendre(&self.params.modulus().0) != 1 {
            Err(StrandError::Generic("Not a quadratic residue".to_string()))
        } else {
            Ok(BigUintE::new(biguint))
        }
    }

    pub fn element_from_string_radix(
        &self,
        string: &str,
        radix: u32,
    ) -> Result<BigUintE<P>, StrandError> {
        let biguint: Result<BigUint, StrandError> =
            BigUint::from_str_radix(string, radix).map_err(|e| e.into());

        self.element_from_biguint(biguint?)
    }
}

impl<P: BigintCtxParams> Ctx for BigintCtx<P> {
    type E = BigUintE<P>;
    type X = BigUintX<P>;
    type P = BigUintP;
    type R = StrandRng;

    #[inline(always)]
    fn generator(&self) -> &Self::E {
        self.params.generator()
    }
    #[inline(always)]
    fn gmod_pow(&self, other: &Self::X) -> Self::E {
        BigUintE::new(
            self.generator()
                .0
                .modpow(&other.0, &self.params.modulus().0),
        )
    }
    #[inline(always)]
    fn emod_pow(&self, base: &Self::E, exponent: &Self::X) -> Self::E {
        BigUintE::new(base.0.modpow(&exponent.0, &self.params.modulus().0))
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
            // BigUint cannot hold negative numbers, so we add exp_modulus first
            value
                .add(self.params.exp_modulus())
                .sub(other)
                .modulo(self.params.exp_modulus())
        }
    }

    #[inline(always)]
    fn get_rng(&self) -> StrandRng {
        StrandRng
    }
    #[inline(always)]
    fn rnd(&self, rng: &mut Self::R) -> Self::E {
        let one: BigUint = One::one();
        let unencoded = BigUintP(
            rng.gen_biguint_below(&(&self.params.exp_modulus().0 - one)),
        );

        self.encode(&unencoded)
            .expect("0..(q-1) should always be encodable")
    }
    #[inline(always)]
    fn rnd_exp(&self, rng: &mut Self::R) -> Self::X {
        BigUintX::new(rng.gen_biguint_below(&self.params.exp_modulus().0))
    }
    fn rnd_plaintext(&self, rng: &mut Self::R) -> Self::P {
        BigUintP(self.rnd_exp(rng).0)
    }

    fn encode(&self, plaintext: &Self::P) -> Result<Self::E, StrandError> {
        let one: BigUint = One::one();

        if plaintext.0 >= (&self.params.exp_modulus().0 - &one) {
            return Err(StrandError::Generic(
                "Failed to encode, out of range".to_string(),
            ));
        }
        let notzero: BigUint = plaintext.0.clone() + one;
        let legendre = notzero.legendre(&self.params.modulus().0);
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
        Ok(BigUintE::new(BigUint::mod_floor(
            &result,
            &self.params.modulus().0,
        )))
    }
    fn decode(&self, element: &Self::E) -> Self::P {
        let one: BigUint = One::one();
        if element.0 > self.params.exp_modulus().0 {
            BigUintP((&self.params.modulus().0 - &element.0) - one)
        } else {
            BigUintP(&element.0 - one)
        }
    }
    fn element_from_bytes(&self, bytes: &[u8]) -> Result<Self::E, StrandError> {
        let biguint = BigUint::from_bytes_le(bytes);
        self.element_from_biguint(biguint)
    }
    fn exp_from_bytes(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        let ret = BigUint::from_bytes_le(bytes);
        let zero: BigUint = Zero::zero();
        if (ret < zero) || ret >= self.params.exp_modulus().0 {
            Err(StrandError::Generic("Out of range".to_string()))
        } else {
            Ok(BigUintX::new(ret))
        }
    }
    fn exp_from_u64(&self, value: u64) -> Self::X {
        BigUintX::new(BigUint::from(value))
    }
    fn hash_to_exp(&self, bytes: &[u8]) -> Result<Self::X, StrandError> {
        let hashed = crate::hash::hash(bytes)?;

        let num = BigUint::from_bytes_le(&hashed);
        Ok(BigUintX::new(num.mod_floor(&self.params.exp_modulus().0)))
    }
    fn encrypt_exp(
        &self,
        exp: &Self::X,
        pk: PublicKey<Self>,
    ) -> Result<Vec<u8>, StrandError> {
        let encrypted = pk.encrypt(&self.encode(&BigUintP(exp.0.clone()))?);
        encrypted.strand_serialize()
    }
    fn decrypt_exp(
        &self,
        bytes: &[u8],
        sk: PrivateKey<Self>,
    ) -> Result<Self::X, StrandError> {
        let encrypted = Ciphertext::<Self>::strand_deserialize(bytes)?;
        let decrypted = sk.decrypt(&encrypted);
        Ok(BigUintX(self.decode(&decrypted).0, PhantomData))
    }

    fn generators(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<Self::E>, StrandError> {
        self.generators_fips(size, seed)
    }
}

impl<P: BigintCtxParams> Default for BigintCtx<P> {
    fn default() -> BigintCtx<P> {
        let params = P::new();
        BigintCtx { params }
    }
}

impl<P: BigintCtxParams + Eq> Element<BigintCtx<P>> for BigUintE<P> {
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        BigUintE::new(&self.0 * &other.0)
    }
    #[inline(always)]
    fn div(&self, other: &Self, modulus: &Self) -> Self {
        let inverse = Element::<BigintCtx<P>>::inv(other, modulus);
        BigUintE::new(&self.0 * inverse.0)
    }
    #[inline(always)]
    fn inv(&self, modulus: &Self) -> Self {
        let inverse = (&self.0).invm(&modulus.0);
        BigUintE::new(inverse.expect("there is always an inverse for prime p"))
    }
    #[inline(always)]
    fn mod_pow(&self, other: &BigUintX<P>, modulus: &Self) -> Self {
        BigUintE::new(self.0.modpow(&other.0, &modulus.0))
    }
    #[inline(always)]
    fn modulo(&self, modulus: &Self) -> Self {
        BigUintE::new(self.0.mod_floor(&modulus.0))
    }
    #[inline(always)]
    fn modp(&self, ctx: &BigintCtx<P>) -> Self {
        BigUintE::new(ctx.modulo(self).0)
    }
    #[inline(always)]
    fn divp(&self, other: &Self, ctx: &BigintCtx<P>) -> Self {
        self.div(other, ctx.params.modulus())
    }
    #[inline(always)]
    fn invp(&self, ctx: &BigintCtx<P>) -> Self {
        self.inv(ctx.params.modulus())
    }
    fn mul_identity() -> Self {
        BigUintE::new(One::one())
    }
}

impl<P: BigintCtxParams + Eq> Exponent<BigintCtx<P>> for BigUintX<P> {
    #[inline(always)]
    fn add(&self, other: &Self) -> Self {
        BigUintX::new(&self.0 + &other.0)
    }
    #[inline(always)]
    fn sub(&self, other: &Self) -> Self {
        BigUintX::new(&self.0 - &other.0)
    }
    #[inline(always)]
    fn sub_mod(&self, other: &Self, ctx: &BigintCtx<P>) -> Self {
        ctx.exp_sub_mod(self, other)
    }
    #[inline(always)]
    fn mul(&self, other: &Self) -> Self {
        BigUintX::new(&self.0 * &other.0)
    }
    #[inline(always)]
    fn div(&self, other: &Self, modulus: &Self) -> Self {
        let inverse = Exponent::<BigintCtx<P>>::inv(other, modulus);
        BigUintX::new(&self.0 * inverse.0)
    }
    #[inline(always)]
    fn inv(&self, modulus: &Self) -> Self {
        let inverse = (&self.0).invm(&modulus.0);
        BigUintX::new(inverse.expect("there is always an inverse for prime p"))
    }
    #[inline(always)]
    fn modulo(&self, modulus: &Self) -> Self {
        BigUintX::new(self.0.div_rem(&modulus.0).1)
    }
    #[inline(always)]
    fn modq(&self, ctx: &BigintCtx<P>) -> Self {
        BigUintX::new(ctx.exp_modulo(self).0)
    }
    #[inline(always)]
    fn divq(&self, other: &Self, ctx: &BigintCtx<P>) -> Self {
        self.div(other, ctx.params.exp_modulus())
    }
    #[inline(always)]
    fn invq(&self, ctx: &BigintCtx<P>) -> Self {
        self.inv(ctx.params.exp_modulus())
    }
    fn add_identity() -> Self {
        BigUintX::new(Zero::zero())
    }
    fn mul_identity() -> Self {
        BigUintX::new(One::one())
    }
}

impl Plaintext for BigUintP {}

pub trait BigintCtxParams:
    Clone + Eq + Send + Sync + Debug + BorshSerialize + BorshDeserialize
{
    fn generator(&self) -> &BigUintE<Self>;
    fn modulus(&self) -> &BigUintE<Self>;
    fn exp_modulus(&self) -> &BigUintX<Self>;
    fn co_factor(&self) -> &BigUint;
    fn new() -> Self;
}
#[derive(Eq, PartialEq, Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct P2048 {
    generator: BigUintE<Self>,
    modulus: BigUintE<Self>,
    exp_modulus: BigUintX<Self>,
    co_factor: BigUintCoFactor<Self>,
}
impl BigintCtxParams for P2048 {
    #[inline(always)]
    fn generator(&self) -> &BigUintE<Self> {
        &self.generator
    }
    #[inline(always)]
    fn modulus(&self) -> &BigUintE<Self> {
        &self.modulus
    }
    #[inline(always)]
    fn exp_modulus(&self) -> &BigUintX<Self> {
        &self.exp_modulus
    }
    #[inline(always)]
    fn co_factor(&self) -> &BigUint {
        &self.co_factor.0
    }
    fn new() -> P2048 {
        let p = BigUintE::new(
            BigUint::from_str_radix(P_VERIFICATUM_STR_2048, 10).unwrap(),
        );
        let q = BigUintX::new(
            BigUint::from_str_radix(Q_VERIFICATUM_STR_2048, 10).unwrap(),
        );
        let g = BigUintE::new(
            BigUint::from_str_radix(G_VERIFICATUM_STR_2048, 10).unwrap(),
        );
        let co_factor = BigUintCoFactor(
            BigUint::from_str_radix(SAFEPRIME_COFACTOR, 16).unwrap(),
            PhantomData,
        );

        assert!(g.0.legendre(&p.0) == 1);

        P2048 {
            generator: g,
            modulus: p,
            exp_modulus: q,
            co_factor,
        }
    }
}

impl<P: BigintCtxParams> BigUintE<P> {
    fn new(value: BigUint) -> BigUintE<P> {
        BigUintE(value, PhantomData)
    }

    pub fn to_string_radix(&self, radix: u32) -> String {
        self.0.to_str_radix(radix)
    }
}
impl<P: BigintCtxParams> BigUintX<P> {
    fn new(value: BigUint) -> BigUintX<P> {
        BigUintX(value, PhantomData)
    }

    pub fn to_string_radix(&self, radix: u32) -> String {
        self.0.to_str_radix(radix)
    }
}

impl<P: BigintCtxParams> BorshSerialize for BigUintE<P> {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_bytes_le();
        bytes.serialize(writer)
    }
}

impl<P: BigintCtxParams> BorshDeserialize for BigUintE<P> {
    #[inline]
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <Vec<u8>>::deserialize_reader(reader)?;

        let ctx: BigintCtx<P> = Default::default();
        ctx.element_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }

    /*fn deserialize(bytes: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = <Vec<u8>>::deserialize(bytes)?;
        let ctx: BigintCtx<P> = Default::default();

        ctx.element_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }*/
}

impl<P: BigintCtxParams> BorshSerialize for BigUintX<P> {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_bytes_le();
        bytes.serialize(writer)
    }
}

impl<P: BigintCtxParams> BorshDeserialize for BigUintX<P> {
    #[inline]
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <Vec<u8>>::deserialize_reader(reader)?;

        let ctx = BigintCtx::<P>::default();

        ctx.exp_from_bytes(&bytes)
            .map_err(|e| Error::new(ErrorKind::Other, e))
    }
}

impl BorshSerialize for BigUintP {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_bytes_le();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for BigUintP {
    #[inline]
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <Vec<u8>>::deserialize_reader(reader)?;

        let biguint = BigUint::from_bytes_le(&bytes);
        Ok(BigUintP(biguint))
    }
}

impl<P: BigintCtxParams> BorshSerialize for BigUintCoFactor<P> {
    #[inline]
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let bytes = self.0.to_bytes_le();
        bytes.serialize(writer)
    }
}

impl<P: BigintCtxParams> BorshDeserialize for BigUintCoFactor<P> {
    #[inline]
    fn deserialize_reader<R: std::io::Read>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        let bytes = <Vec<u8>>::deserialize_reader(reader)?;

        let biguint = BigUint::from_bytes_le(&bytes);
        Ok(BigUintCoFactor(biguint, PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::num_bigint::*;
    use crate::backend::tests::*;
    use crate::context::Ctx;
    use crate::serialization::tests::*;

    #[test]
    fn test_elgamal() {
        let ctx = BigintCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_elgamal_generic(&ctx, plaintext);
    }

    #[test]
    fn test_elgamal_enc_pok() {
        let ctx = BigintCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_elgamal_enc_pok_generic(&ctx, plaintext);
    }

    #[test]
    fn test_encrypt_exp() {
        let ctx = BigintCtx::<P2048>::default();
        test_encrypt_exp_generic(&ctx);
    }

    #[test]
    fn test_schnorr() {
        let ctx = BigintCtx::<P2048>::default();
        test_schnorr_generic(&ctx);
    }

    #[test]
    fn test_chaumpedersen() {
        let ctx = BigintCtx::<P2048>::default();
        test_chaumpedersen_generic(&ctx);
    }

    #[test]
    fn test_rerand() {
        let ctx = BigintCtx::<P2048>::default();
        test_rerand_generic(&ctx);
    }

    #[test]
    fn test_vdecryption() {
        let ctx = BigintCtx::<P2048>::default();
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        test_vdecryption_generic(&ctx, plaintext);
    }

    #[test]
    fn test_element_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_borsh_element(&ctx);
    }

    #[test]
    fn test_elements_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_borsh_elements(&ctx);
    }

    #[test]
    fn test_exponent_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_borsh_exponent(&ctx);
    }

    #[test]
    fn test_ciphertext_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_ciphertext_borsh_generic(&ctx);
    }

    #[test]
    fn test_key_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_key_borsh_generic(&ctx);
    }

    #[test]
    fn test_schnorr_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_schnorr_borsh_generic(&ctx);
    }

    #[test]
    fn test_cp_borsh() {
        let ctx = BigintCtx::<P2048>::default();
        test_cp_borsh_generic(&ctx);
    }

    #[test]
    fn test_encode_err() {
        let ctx = BigintCtx::<P2048>::default();
        let one: BigUint = One::one();
        let result = ctx.encode(&BigUintP(&ctx.params.exp_modulus().0 - one));
        assert!(result.is_err())
    }
}
