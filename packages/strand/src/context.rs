// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//!
//! The interface is composed of three abstractions on which other functionality
//! is built:
//! - [Ctx](crate::context::Ctx): a cryptographic context most closely
//!   corresponds to the underlying
//! group in which modular arithmetic operations take place, and where discrete
//! log assumptions hold.
//! - [Element](crate::context::Element): An element of the underlying group
//!   whose main operations are multiplication and modular
//! exponentiation. If the group is an elliptic curve, the corresponding terms
//! are addition and multiplication.
//! - [Exponent](crate::context::Exponent): A member of the "exponent ring",
//!   used in modular exponentiation (or scalar multiplication
//! for elliptic curves).
//!
//! # Examples
//!
//! ```
//! // This example shows how to obtain a context instance,
//! // generate an ElGamal keypair, and encrypt/decrypt.
//! use strand::context::Ctx;
//! use strand::backend::ristretto::RistrettoCtx;
//! use strand::elgamal::{PrivateKey, PublicKey};
//!
//! let ctx = RistrettoCtx;
//! let mut rng = ctx.get_rng();
//! // generate an ElGamal keypair
//! let sk = PrivateKey::gen(&ctx);
//! let pk = sk.get_pk();
//! // encrypt and decrypt
//! let plaintext = ctx.rnd_plaintext(&mut rng);
//! let encoded = ctx.encode(&plaintext).unwrap();
//! let ciphertext = pk.encrypt(&encoded);
//! let decrypted = sk.decrypt(&ciphertext);
//! let plaintext_ = ctx.decode(&decrypted);
//! assert_eq!(plaintext, plaintext_);
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
// use crate::zkp::Zkp;
use crate::{
    elgamal::{PrivateKey, PublicKey},
    util::StrandError,
};
use std::{
    fmt::Debug,
    marker::{Send, Sync},
};

/// A cryptographic context loosely corresponds to the underlying modular
/// arithmetic groups.
pub trait Ctx:
    Send
    + Sync
    + Sized
    + Clone
    + Default
    + Debug
    + BorshSerialize
    + BorshDeserialize
{
    /// The type of group elements (or points).
    type E: Element<Self>;
    /// The type of ring elements (or scalars).
    type X: Exponent<Self>;
    /// The type of plaintexts.
    type P: Plaintext;
    /// The random number generator type associated to this backend.
    type R: Send + Sync;

    /// Returns the generator of group (or basepoint).
    fn generator(&self) -> &Self::E;
    /// Returns modular exponentiation with the default generator as base.
    fn gmod_pow(&self, other: &Self::X) -> Self::E;
    /// Returns the modular exponentiation with the supplied element as base.
    fn emod_pow(&self, base: &Self::E, exponent: &Self::X) -> Self::E;
    /// Returns the modular subtraction of the given ring elements (or scalars).
    fn exp_sub_mod(&self, value: &Self::X, other: &Self::X) -> Self::X;
    /// Returns the result of applying the modulo operation using the group
    /// modulus.
    fn modulo(&self, value: &Self::E) -> Self::E;
    /// Returns the result of applying the modulo operation using the ring
    /// modulus.
    fn exp_modulo(&self, value: &Self::X) -> Self::X;

    /// Returns the random number generator associated with this backend.
    fn get_rng(&self) -> Self::R;
    /// Returns a random group element (or point).
    fn rnd(&self, rng: &mut Self::R) -> Self::E;
    /// Returns a random ring element (or scalar).
    fn rnd_exp(&self, rng: &mut Self::R) -> Self::X;
    /// Returns a random plaintext.
    fn rnd_plaintext(&self, rng: &mut Self::R) -> Self::P;

    /// Returns the encoding of the given plaintext into a group element (or
    /// point).
    fn encode(&self, plaintext: &Self::P) -> Result<Self::E, StrandError>;
    /// Returns the plaintext corresponding to the given group element (or
    /// point).
    fn decode(&self, element: &Self::E) -> Self::P;

    // Needed to perform context dependent validation on incoming bytes
    /// Constructs a group element (or point) from the given bytes. The bytes
    /// must have been produced by a call to strand_serialize() on the type
    /// Self::E.
    fn element_from_bytes(&self, bytes: &[u8]) -> Result<Self::E, StrandError>;
    // Needed to perform context dependent validation on incoming bytes
    /// Constructs a ring element (or scalar) from the given bytes. The bytes
    /// must have been produced by a call to strand_serialize() on the type
    /// Self::X.
    fn exp_from_bytes(&self, bytes: &[u8]) -> Result<Self::X, StrandError>;
    // Used to convert exponents in threshold cryptography
    /// Constructs a ring element (or scalar) from the given unsigned integer.
    fn exp_from_u64(&self, value: u64) -> Self::X;
    /// Returns the result of hashing the supplied bytes into a ring element (or
    /// scalar).
    // Used to compute challenges in zk proofs
    /// The bytes are hashed using hash::hash_to_array.
    fn hash_to_exp(&self, bytes: &[u8]) -> Result<Self::X, StrandError>;
    // In braid, used to encrypt shares (evaluations of the polynomial in Zp)
    /// Returns the serialization of an ElGamal encryption of
    /// the given ring element (or scalar) under the given public key.
    fn encrypt_exp(
        &self,
        exp: &Self::X,
        pk: PublicKey<Self>,
    ) -> Result<Vec<u8>, StrandError>;
    /// Returns the ring element (or scalar) decrypted from an ElGamal
    /// encryption resulting from the deserialization of the given bytes.
    /// Decryption is performed with the given private key.
    fn decrypt_exp(
        &self,
        bytes: &[u8],
        sk: PrivateKey<Self>,
    ) -> Result<Self::X, StrandError>;
    /// Returns independent generators for use in shuffling. See [FIPS 186-4 A.2.3](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.186-4.pdf)
    fn generators(
        &self,
        size: usize,
        seed: &[u8],
    ) -> Result<Vec<Self::E>, StrandError>;
}

/// An element of the underlying group.
///
/// Operations depend on the backend and are given below for multiplicative
/// groups / elliptic curves.
pub trait Element<C: Ctx>:
    Clone + Eq + Send + Sync + BorshSerialize + BorshDeserialize + Debug
{
    /// Multiplication (point addition).
    fn mul(&self, other: &C::E) -> C::E;
    /// Division (a div b = a * b^1) (point subtraction).
    fn div(&self, other: &C::E, modulus: &C::E) -> C::E;
    /// Modular inverse  (point negation).
    fn inv(&self, modulus: &C::E) -> C::E;
    /// Modular exponentiation (scalar multiplication).
    fn mod_pow(&self, exp: &C::X, modulus: &C::E) -> C::E;
    /// Modulo operation using the given modulus (N/A).
    fn modulo(&self, modulus: &C::E) -> C::E;
    /// Modulo operation using group order (N/A).
    fn modp(&self, ctx: &C) -> C::E;
    /// Division (a div b = a * b^-1) using group order (point subtraction).
    /// This operation does _not_ take the modulus you need to call that
    /// explicitly (eg .modp())
    fn divp(&self, other: &C::E, ctx: &C) -> C::E;
    /// Modular inverse using group order (point negation).
    fn invp(&self, ctx: &C) -> C::E;

    /// Multiplicative identity (point at infinity).
    fn mul_identity() -> C::E;
}

/// A member of the "exponent ring" associated to the element group, or scalar
/// ring for elliptic curves.
pub trait Exponent<C: Ctx>:
    Clone + Eq + Send + Sync + BorshSerialize + BorshDeserialize + Debug
{
    // Addition.
    fn add(&self, other: &C::X) -> C::X;
    // Subtraction.
    fn sub(&self, other: &C::X) -> C::X;
    // Multiplication.
    fn mul(&self, other: &C::X) -> C::X;
    // Division (a div b = a * b^-1).
    fn div(&self, other: &C::X, modulus: &C::X) -> C::X;
    /// Modular inverse.
    fn inv(&self, modulus: &C::X) -> C::X;
    /// Modulo operation using the given modulus (N/A).
    fn modulo(&self, modulus: &C::X) -> C::X;
    /// Modulo operation using ring order (N/A).
    fn modq(&self, ctx: &C) -> C::X;
    // Division using ring order (a div b = a * b^-1).
    fn divq(&self, other: &C::X, ctx: &C) -> C::X;
    /// Modular inverse using ring order.
    fn invq(&self, ctx: &C) -> C::X;

    // Modular subtraction.
    fn sub_mod(&self, other: &C::X, ctx: &C) -> C::X;

    /// Additive identity.
    fn add_identity() -> C::X;
    /// Multiplicative identity.
    fn mul_identity() -> C::X;
}

/// The type of plaintext data. This type must be encoded into a group member
/// before it can be encrypted.
pub trait Plaintext:
    Send
    + Sync
    + Eq
    + Debug
    + BorshSerialize
    + BorshDeserialize
    + std::hash::Hash
    + Clone
{
}
