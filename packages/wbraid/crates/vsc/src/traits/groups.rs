// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Traits related to cryptographic groups
//!
//! This includes traits for group elements, group scalars (exponents),
//! and the cryptographic group itself.
//!
//! By cryptographic group we mean a _cyclic group_ of a large (typically
//! prime) order that can be securely used in cryptographic algorithms and
//! protocol implementations.
use crate::utils::error::Error;
use crate::utils::hash;
use crate::utils::rng;

use std::fmt::Debug;

// -------------------------------------------------------------------------
// Trait for a Cryptographic Group
// -------------------------------------------------------------------------

/**
 * Trait for a cryptographic group
 *
 * By cryptographic group we mean a _cyclic group_ of a large (typically
 * prime) order that can be securely used in cryptographic algorithms and
 * protocol implementations.
 *
 * This trait aggregates various types relevant to working with the group,
 * providing the type-based notions of group element, group scalar (exponents),
 * encoding/decoding from/to a plain text message type, and hashing.
 */
// @todo
pub trait CryptographicGroup {
    /// The element type for this group
    type Element: GroupElement<Scalar = Self::Scalar>;

    /// The scalar type for this group
    type Scalar: GroupScalar;

    /// The hashing function used for hash to curve and hash to scalar
    type Hasher: hash::Hasher;

    /// The default generator
    fn generator() -> Self::Element;

    /// Exponentiation with the default generator as a base
    ///
    /// In some implementations this operations is accelerated by precomputed tables.
    fn g_exp(scalar: &Self::Scalar) -> Self::Element;

    /// Hash bytes into a `Scalar`.
    ///
    /// The returning Scalar is uniformly distributed for uniformly distributed input.
    ///
    /// # Errors
    ///
    /// - `HashToScalarError` if using `P256Group` and `NistP256::hash_to_scalar` returns error
    #[crate::warning("Verify the claim of uniformity is correct for implementations")]
    fn hash_to_scalar(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Scalar, Error>;

    /// Hash bytes into an `Element`.
    ///
    /// The returning Scalar is uniformly distributed for uniformly distributed input.
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if using `P256Group` and `NistP256::hash_from_bytes` returns error
    #[crate::warning("Verify the claim of uniformity is correct for implementations.")]
    fn hash_to_element(input_slices: &[&[u8]], ds_tags: &[&[u8]]) -> Result<Self::Element, Error>;

    /// Returns a random `Element`
    fn random_element<R: rng::CRng>(rng: &mut R) -> Self::Element;

    /// Returns a random `Scalar`
    fn random_scalar<R: rng::CRng>(rng: &mut R) -> Self::Scalar;

    /// Encode bytes into the given number of group elements
    ///
    /// Both backends carry 30 payload bytes per element, so `O` must equal
    /// `I.div_ceil(30)`; each checks this at compile time.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if a point was not found for the input, with negligible probability
    fn encode_bytes<const I: usize, const O: usize>(
        bytes: &[u8; I],
    ) -> Result<[Self::Element; O], Error>;

    /// Decode bytes from the given number of group elements
    ///
    /// # Errors
    ///
    /// - `EncodingError` if using `P256Group` and an element is the identity,
    ///   which carries no payload; infallible for `Ristretto255Group`
    fn decode_bytes<const I: usize, const O: usize>(
        element: &[Self::Element; I],
    ) -> Result<[u8; O], Error>;

    /// Returns `count` independent generators of the group
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if using `P256Group` and `NistP256::hash_from_bytes` returns error
    #[crate::warning("Verify implementations are correct")]
    fn ind_generators(count: usize, label: &[u8]) -> Result<Vec<Self::Element>, Error>;

    /// Encrypt a scalar with ElGamal encryption using the given public key
    ///
    /// This encodes the scalar into group elements and encrypts them, returning
    /// the serialized ciphertext as a byte vector. Used for encrypting DKG shares.
    ///
    /// # Errors
    ///
    /// - `EncodingError` if the scalar cannot be encoded into elements
    /// - `SerializationError` if the ciphertext cannot be serialized
    fn encrypt_scalar(scalar: &Self::Scalar, public_key: &Self::Element) -> Result<Vec<u8>, Error>;

    /// Decrypt a scalar from ElGamal-encrypted serialized ciphertext
    ///
    /// This deserializes the ciphertext, decrypts it, and decodes the result
    /// back into a scalar. Used for decrypting DKG shares.
    ///
    /// # Errors
    ///
    /// - `DeserializationError` if the ciphertext bytes cannot be deserialized
    /// - `ScalarDecodeError` if the decrypted elements cannot be decoded into a scalar
    fn decrypt_scalar(ciphertext: &[u8], secret_key: &Self::Scalar) -> Result<Self::Scalar, Error>;
}

// -------------------------------------------------------------------------
// Traits for Group Elements
// -------------------------------------------------------------------------

/**
 * Trait for group elements of a cryptographic group
 *
 * @traceability Cryptol type `T` in [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
 *
 * Group elements ought be valid by construction. Hence, no validity
 * predicate such as `G` in [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry) is specified by the trait.
 */
pub trait GroupElement: Sized + Debug + Eq + std::hash::Hash {
    /// The scalar type associated to this group
    type Scalar: GroupScalar;

    /**
     * Identity element of the group
     *
     * @traceability Cryptol constant `idendity` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
     */
    fn one() -> Self;

    /**
     * Binary group operation
     *
     * @traceability Cryptol function `gop` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
     */
    #[must_use]
    fn mul(&self, other: &Self) -> Self;

    /**
     * Group inverse operation
     *
     * @traceability Cryptol function `inv` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
     */
    #[must_use]
    fn inv(&self) -> Self;

    /**
     * Exponentation within the group
     *
     * Exponentation shall obey the usual expected properties:
     * - `x.exp(zero()) = x`
     * - `x.exp(s.add(one())) = x.mul(x.exp(s))`
     * - `a.exp(s.neg()) = x.exp(s).inv()`
     *
     * @traceability Cryptol function `exp` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry) and [`CyclicGroup_Axioms.cry`](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroup_Axioms.cry)
     */
    #[must_use]
    fn exp(&self, scalar: &Self::Scalar) -> Self;

    /**
     * Multi-exponentiation: the product `∏_i bases[i]^{exponents[i]}`
     *
     * Returns [`one`](GroupElement::one) for empty input.
     *
     * # Why this is a trait method
     *
     * The default body below is the naive one — a full exponentiation per base,
     * multiplied together — and is only a correct placeholder, not a fast one.
     * Backends are expected to override it: a windowed or Pippenger
     * implementation costs roughly `n/log(n)` exponentiations rather than `n`,
     * and for a group whose backing crate already provides one, overriding is a
     * few lines. This is the reason the operation is expressed here at all
     * rather than being written out at each call site.
     *
     * # Timing
     *
     * Implementations must be constant-time in the exponents, exactly as
     * [`exp`](GroupElement::exp) is — callers may pass secret scalars. A
     * variable-time variant would be a *separate* method, since its safety
     * depends on the caller's inputs being public.
     *
     * # Why `bases` is a slice of references
     *
     * So that a product group can gather one component column out of
     * `&[[T; N]]` without copying points, and thereby delegate to `T`'s
     * specialized implementation — see
     * [`dist_multi_exp`](DistGroupOps::dist_multi_exp). Taking `&[Self]` would
     * require a `Clone` bound on `GroupElement`, which it does not currently
     * carry. Callers holding a `&[Self]` can `.iter().collect()`, which copies
     * pointers rather than group elements.
     *
     * # Errors
     *
     * - `MismatchedMultiExpLength` if `bases` and `exponents` differ in length.
     */
    fn multi_exp(bases: &[&Self], exponents: &[Self::Scalar]) -> Result<Self, Error> {
        if bases.len() != exponents.len() {
            return Err(Error::MismatchedMultiExpLength(
                bases.len(),
                exponents.len(),
            ));
        }
        Ok(bases
            .iter()
            .zip(exponents)
            .fold(Self::one(), |acc, (base, exponent)| {
                acc.mul(&base.exp(exponent))
            }))
    }

    /**
     * Equality test between two group elements
     *
     * @traceability Cryptol predicate `T'eq` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
     */
    fn equals(&self, other: &Self) -> bool;

    /**
     * Generation of a random group element
     *
     * @traceability Cryptol function `gen` of [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
     *
     * Note that `gen` is using a random bitstring of length `rngbits` as
     * seed rather than a Rust RNG of type [`rng::CRng`].
     */
    fn random<R: rng::CRng>(rng: &mut R) -> Self;
}

/**
 * Trait for distributed application of the group operation over the
 * components of a product group
 *
 * E.g., for a product group implementation using arrays, the trait methods
 * shall obey:
 * - `[g1, g2, ...].dist_op(h) = [g1.op(h), g2.op(h), ...]` for `op` being
 *   [`mul`](GroupElement::mul) or [`exp`](GroupElement::exp);
 * - `[g1, g2, ...].dist_equals(h) = g1.equals(h) && g2.equals(h) && ...`
 *   for [`equals`](GroupElement::equals).
 */
pub trait DistGroupOps<Rhs: GroupElement>: GroupElement {
    /**
     * Result type of the operation, typically same as `Self`
     *
     * It is expected to be some aggregate type like an array or vector.
     */
    type Result;

    /**
     * Distributed application of multiplication with `other` over the
     * components of a product group element `self`
     */
    fn dist_mul(&self, other: &Rhs) -> Self::Result;

    /**
     * Distributed application of exponentiation with `other` over the
     * components of a product group element `self`
     */
    fn dist_exp(&self, other: &Rhs::Scalar) -> Self::Result;

    /**
     * Distributed multi-exponentiation: `∏_i bases[i]^{exponents[i]}`, with one
     * **base-group** scalar per base, broadcast across the components.
     *
     * This is to [`multi_exp`](GroupElement::multi_exp) exactly what
     * [`dist_exp`](DistGroupOps::dist_exp) is to [`exp`](GroupElement::exp), and
     * the distinction is the same one:
     *
     * ```text
     * exp            [g1, g2].exp([s1, s2])          one scalar per component
     * dist_exp       [g1, g2].dist_exp(s)            one scalar, broadcast
     *
     * multi_exp      bases: &[[T; N]]  exps: &[[T::Scalar; N]]   per component
     * dist_multi_exp bases: &[[T; N]]  exps: &[T::Scalar]        broadcast
     * ```
     *
     * The broadcast form is the one batching needs: a ciphertext of width `ω`
     * is raised to a *single* batching exponent, not to `ω` independent ones.
     *
     * There is no `repl` counterpart, since replicating one element across many
     * exponents degenerates to `g^{Σ e_i}`.
     *
     * Implementations should delegate per component to
     * [`multi_exp`](GroupElement::multi_exp) so that a backend's specialized
     * implementation is inherited here rather than reimplemented.
     *
     * # Errors
     *
     * - `MismatchedMultiExpLength` if `bases` and `exponents` differ in length.
     */
    fn dist_multi_exp(bases: &[Self], exponents: &[Rhs::Scalar]) -> Result<Self::Result, Error>;

    /**
     * Conjunctive evaluation of equality of the components of a product
     * group element `self` with respect to `other`
     */
    fn dist_equals(&self, other: &Rhs) -> bool;
}

/**
 * Trait for pointwise application of a list/product of group elements
 * to a single replicated group element
 *
 * E.g., for a product implementation using arrays, the trait methods
 * shall obey:
 * - `g.op([h1, h2, ...]) = [g.op(h1), g.op(h2), ...]` for `op` being
 *   [`mul`](GroupElement::mul) or [`exp`](GroupElement::exp);
 * - `g.dist_equals([h1, h2, ...]) = g.equals(h1) && g.equals(h2) && ...`
 *   for [`equals`](GroupElement::equals).
 */
pub trait ReplGroupOps<Rhs: GroupElement>: GroupElement {
    /**
     * Result type of the operation, typically `Self` lifted to a some
     * aggregate type like an array or vector.
     */
    type Result;

    /**
     * Component-wise application of multiplication of replicated `self`
     * with the elements of `other`
     */
    fn repl_mul(&self, other: &Rhs) -> Self::Result;

    /**
     * Component-wise application of exponentation of replicated `self`
     * with the elements of `other`
     */
    fn repl_exp(&self, other: &Rhs::Scalar) -> Self::Result;

    /**
     * Conjunctive evaluation of equality of `self` with respect to the
     * elements of `other`
     */
    fn repl_equals(&self, other: &Rhs) -> bool;
}

// -------------------------------------------------------------------------
// Traits for Group Scalars
// -------------------------------------------------------------------------

/**
 * Trait for a scalar (exponent) of a cryptographic group
 *
 * Scalars are used as exponents by the [`GroupElement::exp`] method. Scalars
 * shall constitute a finite field (Galois Field) of the same size as the
 * underlying group's order. (Note that this restricts the group order to
 * be a either prime or a prime power.)
 *
 * @traceability type `E` in [CyclicGroupI.cry](../../../../../../../../../models/cryptography/cryptol/Algebra/CyclicGroupI.cry)
 */
pub trait GroupScalar: Sized + Debug + Eq {
    /**
     * Zero of the field (identity of addition)
     *
     * @traceability function `fromInteger` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    fn zero() -> Self;

    /**
     * One of the field (identity of multiplication)
     *
     * @traceability function `fromInteger` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    fn one() -> Self;

    /**
     * Field operation for addition
     *
     * @traceability operation `(+)` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    #[must_use]
    fn add(&self, other: &Self) -> Self;

    /**
     * Field operation for subtraction
     *
     * This method shall obey the law `x.sub(y) == x.add(y.neg())`.
     *
     * @traceability operation `(-)` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    #[must_use]
    fn sub(&self, other: &Self) -> Self;

    /**
     * Field operation for multiplication
     *
     * @traceability operation `(*)` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    #[must_use]
    fn mul(&self, other: &Self) -> Self;

    /**
     * Field operation for negation (additive inverse)
     *
     * @traceability function `negate` of the `Ring` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    #[must_use]
    fn neg(&self) -> Self;

    /**
     * Field operation for multiplicative inverse
     *
     * @traceability function `recip` of the `Field` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    #[must_use]
    fn inv(&self) -> Option<Self>;

    /**
     * Equality test between two field elements
     *
     * @traceability `Field` type class of Cryptol (@see [CryptolPrims.pdf](https://github.com/GaloisInc/cryptol/blob/master/docs/CryptolPrims.pdf))
     */
    fn equals(&self, other: &Self) -> bool;

    /**
     * Generation of a random field element
     *
     * @traceability `random` function of Cryptol (@see TODO)
     */
    fn random<R: rng::CRng>(rng: &mut R) -> Self;
}

/**
 * Distributed operations from many to one [`GroupScalar`] operands
 *
 * [s1, s2, ...].op(t) = [s1.op(t), s2.op(t), ...]
 *
 * Another term for these types of operations is [`Broadcasting`](https://numpy.org/doc/stable/user/basics.broadcasting.html)
 */
pub trait DistScalarOps<Rhs: GroupScalar>: GroupScalar {
    /// The output type of these operations
    type Output;

    /// Distributed addition
    fn dist_add(&self, other: &Rhs) -> Self::Output;

    /// Distributed subtraction
    fn dist_sub(&self, other: &Rhs) -> Self::Output;

    /// Distributed multiplication
    fn dist_mul(&self, other: &Rhs) -> Self::Output;

    /// Distributed equality
    ///
    /// Returns `true` only if _all_ comparisons return `true`
    fn dist_equals(&self, other: &Rhs) -> bool;
}

/**
 * Replicated operations from one to many [`GroupScalar`] operands
 *
 * s.op([t1, t2, ...]) = [s.op(t1), s.op(t2), ...]
 *
 * Another term for these types of operations is [`Broadcasting`](https://numpy.org/doc/stable/user/basics.broadcasting.html)
 */
pub trait ReplScalarOps<Rhs: GroupScalar>: GroupScalar {
    /// The output type of these operations
    type Output;

    /// Replicated addition
    fn repl_add(&self, other: &Rhs) -> Self::Output;

    /// Replicated subtraction
    fn repl_sub(&self, other: &Rhs) -> Self::Output;

    /// Replicated multiplication
    fn repl_mul(&self, other: &Rhs) -> Self::Output;

    /// Returns `true` only if _all_ comparisons return `true`
    fn repl_equals(&self, other: &Rhs) -> bool;
}
