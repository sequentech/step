// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Joint-Feldman distributed key generation: recipient

#![allow(clippy::type_complexity)]
use crate::context::Context;
use crate::cryptosystem::elgamal::PublicKey;
use crate::cryptosystem::elgamal::{Ciphertext, KeyPair};
use crate::dkgd::dealer::VerifiableShare;
use crate::traits::groups::DistGroupOps;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::utils::error::Error;
use crate::zkp::dlogeq::DlogEqProof;
use std::array;
use vser_derive::VSerializable;

/**
 * A recipient in the Joint-Feldman distributed key generation (DKG) protocol.
 *
 * * NOTE: this API does not represent private shares as encrypted values.
 *   In the messaging layer, private shares should be encrypted with the recipient's
 *   public key.
 *
 * In the Joint-Feldman DKG, a recipient receives a secret share from each of the
 * `P` participants, including itself. The recipient must verify that these shares
 * are correct using the dealer's public checking values.
 *
 * In the Joint-Feldman DKG:
 *
 * - Only the recipients can compute their secret share of the joint public key with
 *   their `P` private shares.
 *
 * - Anyone can compute the joint public key from public data.
 *
 * - Anyone can compute the recipients' verification keys from public data.
 *
 * At least `T` of the `P` recipients are needed to decrypt ciphertexts encrypted
 * with the joint public key. To do this, each recipient computes a partial decryption
 * of the ciphertext using their secret. These partial values are then combined.
 * Recipients can prove correctness of their partial decryptions with respect to their
 * verification key, using an equality of discrete logs proof. The correctness of the
 * aggregated decrypted value is verifiable from the partial decryptions, which
 * are public data.
 *
 * See `EVS`: Protocol 16.20
 *
 * See also [Dealer][`crate::dkgd::dealer::Dealer`]
 *
 * # Examples
 *
 * ```
 * use std::array;
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::groups::ristretto255::RistrettoElement;
 * use cryptography::dkgd::dealer::{VerifiableShare, Dealer};
 * use cryptography::dkgd::recipient::{combine, Recipient, DkgPublicKey, ParticipantPosition, AttributedDecryption};
 *
 * const P: usize = 3;
 * const T: usize = 2;
 * const W: usize = 2;
 *
 * // Simulates the DKG protocol
 *
 * let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
 *
 * let recipients: [(Recipient<RCtx, T, P>, DkgPublicKey<RCtx, T>); P] = array::from_fn(|i| {
 *     let position = ParticipantPosition::from_usize(i + 1);
 *
 *     let verifiable_shares: [VerifiableShare<RCtx, T>; P] = dealers
 *         .clone()
 *         .map(|d| d.get_verifiable_shares().for_recipient(&position));
 *
 *     Recipient::from_shares(position, &verifiable_shares).unwrap()
 * });
 *
 * // Simulates distributed decryption
 *
 * // the joint public key is returned from Recipient::from_shares, but can also
 * // be obtained via [`Recipient::joint_public_key`]
 * let (recipient, pk) = &recipients[0];
 * // encrypt a message of width `W`
 * let message: [RistrettoElement; W] = array::from_fn(|_| RCtx::random_element());
 * let encrypted = vec![pk.encrypt(&message)];
 *
 * // partial decryption: factors for every ciphertext plus one proof covering
 * // them all, attributed to its author. In a real execution the position comes
 * // from the authenticated message and the key from the DKG public key --
 * // never from the contribution itself.
 * let contributions: [AttributedDecryption<RCtx, W, P>; P] = recipients.map(|r| {
 *     let partial = r.0.partial_decrypt(&encrypted, &vec![]).unwrap();
 *     AttributedDecryption::new(
 *         partial,
 *         r.0.get_position().clone(),
 *         r.0.get_verification_key().clone(),
 *     )
 * });
 *
 * let threshold: &[AttributedDecryption<RCtx, W, P>; T] =
 *     contributions[0..T].try_into().expect("slice matches array: T == T");
 *
 * // combine the decryption factors into the plaintext
 * let decrypted = combine(&encrypted, threshold, &vec![]).unwrap();
 *
 * assert!(message == decrypted[0]);
 * ```
 */
pub struct Recipient<C: Context, const T: usize, const P: usize> {
    /// This recipient's position in the protocol, from 1 to `P`
    position: ParticipantPosition<P>,
    /// This recipient's verification key, used to prove correctness of partial decryptions
    verification_key: C::Element,
    /// This recipient's share of the secret key, used to partially decrypt ciphertexts
    sk: C::Scalar,
}

impl<C: Context, const T: usize, const P: usize> Recipient<C, T, P> {
    /// compile-time checks for recipient const parameters
    #[crate::warning(
        "Ensure choice of threshold parameter is secure. See https://eprint.iacr.org/2024/915.pdf section 2.4"
    )]
    const CHECK: () = {
        assert!(P < 100);
        assert!(P > 0);
        assert!(T <= P);
        assert!(T > 0);
    };

    /// Construct a `Recipient` with the given values.
    ///
    /// The standard way to create a `Recipient` is through the
    /// [`from_shares`][`Self::from_shares`] function, passing in this recipient's
    /// [shares][VerifiableShare] which are then verified. Use this constructor
    /// instead if the required argument values are available from a previously
    /// created `Recipient` as a result of a call to [`from_shares`][`Self::from_shares`].
    pub fn new(
        position: ParticipantPosition<P>,
        verification_key: C::Element,
        sk: C::Scalar,
    ) -> Self {
        #[allow(path_statements)]
        Self::CHECK;

        Self {
            position,
            verification_key,
            sk,
        }
    }

    /// Returns a reference to this recipient's verification key.
    pub fn get_verification_key(&self) -> &C::Element {
        &self.verification_key
    }

    /// Returns a reference to this recipient's position.
    ///
    /// Note this is the recipient's *own* view of its position. A verifier must
    /// not take a participant's position from anything the participant sends —
    /// see [`PartialDecryption`] — so this is for a participant reasoning about
    /// itself, not for attributing someone else's contribution.
    pub fn get_position(&self) -> &ParticipantPosition<P> {
        &self.position
    }

    /// Construct a `Recipient` from its shares.
    ///
    /// The supplied shares will be verified by this function, using
    /// the [dealer's][`crate::dkgd::dealer::Dealer`] checking values.
    ///
    /// # Examples
    ///
    /// ```
    /// use cryptography::dkgd::recipient::{Recipient, DkgPublicKey, ParticipantPosition};
    /// use cryptography::dkgd::dealer::{VerifiableShare, Dealer, DealerShares};
    /// use cryptography::context::RistrettoCtx as RCtx;
    /// use std::array;
    ///
    /// const P: usize = 3;
    /// const T: usize = 2;
    ///
    /// // simulates P dealers with threshold T
    /// let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
    ///
    //  // simulates P recipients with threshold T
    /// let recipients: [(Recipient<RCtx, T, P>, DkgPublicKey<RCtx, T>); P] = array::from_fn(|i| {
    ///     let position = ParticipantPosition::from_usize(i + 1);
    ///
    ///     // gather the shares for recipient at position from all dealers
    ///     let verifiable_shares: [VerifiableShare<RCtx, T>; P] = dealers
    ///         .clone()
    ///         .map(|d| d.get_verifiable_shares().for_recipient(&position));
    ///
    ///     // constructs the recipient, this includes verifying its shares
    ///     Recipient::from_shares(position, &verifiable_shares).unwrap()
    /// });
    ///
    /// ```
    /// Returns a tuple with the constructed `Recipient` and the protocol's joint public key
    ///
    /// # Parameters
    ///
    /// - `position`: the position of the recipient
    /// - `shares`: the set of shares assigned to this participant from all `P` dealers, in any order
    ///
    /// # Errors
    ///
    /// - `ShareVerificationFailed` if the shares do not verify.
    pub fn from_shares(
        position: ParticipantPosition<P>,
        shares: &[VerifiableShare<C, T>; P],
    ) -> Result<(Self, DkgPublicKey<C, T>), Error> {
        let (joint_pk, verification_key, sk) = Self::verify_shares(&position, shares)?;
        let inner = PublicKey { y: joint_pk };
        let joint_pk_ret = DkgPublicKey::from_public_key(&inner);

        let recipient = Self::new(position, verification_key, sk);

        Ok((recipient, joint_pk_ret))
    }

    /// Verify the given shares for a `Recipient` at `position`.
    ///
    /// The supplied shares will be verified by this function, using
    /// the [dealer's][`crate::dkgd::dealer::Dealer`] checking values.
    ///
    /// Returns a tuple with the protocol's [joint public key][`DkgPublicKey`], the
    /// recipient's verification key, and their secret key. If you are constructing
    /// a `Recipient` using [`from_shares`][`Self::from_shares`], you do
    /// not need to separately call this function, it is called automatically.
    ///
    /// # Parameters
    ///
    /// - `position`: the position of the recipient
    /// - `shares`: the set of shares assigned to this participant from all `P` dealers, in any order
    ///
    /// # Errors
    ///
    /// - `ShareVerificationFailed` if the shares do not verify.
    pub fn verify_shares(
        position: &ParticipantPosition<P>,
        shares: &[VerifiableShare<C, T>; P],
    ) -> Result<(C::Element, C::Element, C::Scalar), Error> {
        let mut verification_key = C::Element::one();
        let mut joint_pk = C::Element::one();
        let mut sk = C::Scalar::zero();

        for verifiable_share in shares {
            let (pk_factor, vk_factor, sk_summand) =
                Self::verify_share(verifiable_share, position)?;
            joint_pk = joint_pk.mul(&pk_factor);
            verification_key = verification_key.mul(&vk_factor);
            sk = sk.add(&sk_summand);
        }

        Ok((joint_pk, verification_key, sk))
    }

    /// Compute the verification key for a `Recipient` at `position`.
    ///
    /// # Parameters
    ///
    /// - `position`: the position of the recipient
    /// - `all_checking_values`: an array of checking values provided by each of `P` dealers, in any dealer order
    ///
    /// Allows computing a verification key without constructing a `Recipient`,
    /// using the checking values for all dealers.
    pub fn verification_key(
        position: &ParticipantPosition<P>,
        all_checking_values: &[[C::Element; T]; P],
    ) -> C::Element {
        let mut verification_key = C::Element::one();

        for cv in all_checking_values {
            let vk_factor = Self::vk_factor(cv, position);
            verification_key = verification_key.mul(&vk_factor);
        }

        verification_key
    }

    /// Compute the joint public key.
    ///
    /// # Parameters
    ///
    /// - `all_checking_values`: an array of checking values provided by each of `P` dealers, in any dealer order
    ///
    /// Allows computing the joint public key without constructing a `Recipient`,
    /// using the checking values for all dealers.
    pub fn joint_public_key(all_checking_values: &[[C::Element; T]; P]) -> DkgPublicKey<C, T> {
        let mut joint_public_key = C::Element::one();

        for cv in all_checking_values {
            joint_public_key = joint_public_key.mul(&cv[0]);
        }
        let inner = PublicKey::new(joint_public_key);
        DkgPublicKey::from_public_key(&inner)
    }

    /// Compute this recipient's partial decryptions for the given ciphertexts.
    ///
    /// At least `T` partial decryptions are needed to decrypt ciphertexts encrypted with the
    /// DKG's joint public key. Partial decryptions can be combined to compute the plaintext using the
    /// [`combine`] function.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::array;
    /// use cryptography::context::Context;
    /// use cryptography::context::RistrettoCtx as RCtx;
    /// use cryptography::groups::ristretto255::RistrettoElement;
    /// use cryptography::dkgd::dealer::{VerifiableShare, Dealer};
    /// use cryptography::dkgd::recipient::{combine, Recipient, DkgPublicKey, ParticipantPosition, AttributedDecryption};
    ///
    /// const P: usize = 3;
    /// const T: usize = 2;
    /// const W: usize = 2;
    ///
    /// let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
    ///
    /// let recipients: [(Recipient<RCtx, T, P>, DkgPublicKey<RCtx, T>); P] = array::from_fn(|i| {
    ///    let position = ParticipantPosition::from_usize(i + 1);
    ///
    ///    let verifiable_shares: [VerifiableShare<RCtx, T>; P] = dealers
    ///        .clone()
    ///        .map(|d| d.get_verifiable_shares().for_recipient(&position));
    ///
    ///    Recipient::from_shares(position, &verifiable_shares).unwrap()
    /// });
    ///
    /// let (recipient, pk) = &recipients[0];
    /// let message: [RistrettoElement; W] = array::from_fn(|_| RCtx::random_element());
    /// let encrypted = vec![pk.encrypt(&message)];
    ///
    /// // each recipient computes its partial decryption: one factor per
    /// // ciphertext, and a single proof covering all of them
    /// let contributions: [AttributedDecryption<RCtx, W, P>; P] = recipients.map(|r| {
    ///     let partial = r.0.partial_decrypt(&encrypted, &vec![]).unwrap();
    ///     AttributedDecryption::new(
    ///         partial,
    ///         r.0.get_position().clone(),
    ///         r.0.get_verification_key().clone(),
    ///     )
    /// });
    ///
    /// // select the first T contributions
    /// let threshold: &[AttributedDecryption<RCtx, W, P>; T] =
    ///     contributions[0..T].try_into().expect("slice matches array: T == T");
    ///
    /// // combine the decryption factors into the plaintext
    /// let decrypted = combine(&encrypted, threshold, &vec![]).unwrap();
    ///
    /// assert!(message == decrypted[0]);
    /// ```
    ///
    /// # Parameters
    ///
    /// - `ciphertexts`: the ciphertexts to decrypt
    /// - `proof_context`: proof context label (ZKP CONTEXT)
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    /// - `MismatchedMultiExpLength` if batching is given inconsistent lengths
    ///
    /// Returns a [`PartialDecryption`]: one factor per input ciphertext, in the
    /// ciphertexts' order, and **one** proof covering all of them. It carries no
    /// position — see [`PartialDecryption`] for why that must come from the
    /// authenticated envelope instead.
    pub fn partial_decrypt<const W: usize>(
        &self,
        ciphertexts: &[DkgCiphertext<C, W, T>],
        proof_context: &[u8],
    ) -> Result<PartialDecryption<C, W>, Error> {
        let factors: Vec<[C::Element; W]> = ciphertexts
            .iter()
            .map(|c| c.u().dist_exp(&self.sk))
            .collect();

        let bases: Vec<[C::Element; W]> = ciphertexts.iter().map(|c| c.u().clone()).collect();
        let exponents =
            batching_exponents::<C, W>(&self.verification_key, &bases, &factors, proof_context)?;

        // The batched statement: `A = ∏ u_i^{e_i}` and `B = ∏ f_i^{e_i}`. Since
        // every `f_i = u_i^{sk}`, `B = A^{sk}` — the same discrete-log equality
        // the per-ciphertext proofs asserted, over one pair of bases instead of
        // `N`.
        let a = <[C::Element; W]>::dist_multi_exp(&bases, &exponents)?;
        let b = <[C::Element; W]>::dist_multi_exp(&factors, &exponents)?;

        let proof = DlogEqProof::<C, W>::prove(
            &self.sk,
            &C::generator(),
            &self.verification_key,
            &a,
            &b,
            proof_context,
        )?;

        Ok(PartialDecryption { factors, proof })
    }

    /// Compute a factor of the verification key for a `Recipient` at `position`.
    ///
    /// # Parameters
    ///
    /// - `checking_values`: the checking values provided by the dealer
    /// - `position`: the position of the recipient
    ///
    /// This function is used during share [verification][`Self::verify_share`].
    fn vk_factor(
        checking_values: &[C::Element; T],
        position: &ParticipantPosition<P>,
    ) -> C::Element {
        let exponents: [C::Scalar; T] = array::from_fn(|i| {
            let exp: u32 = i.try_into().expect("T <= P < 100 < u32::MAX");
            let exp = position.0.pow(exp);
            exp.into()
        });
        let big_a_n_j = checking_values.exp(&exponents);

        big_a_n_j
            .iter()
            .fold(C::Element::one(), |acc, next| acc.mul(next))
    }

    /// Verify a single share for a `Recipient` at `position`.
    ///
    /// Returns a tuple with the public key factor, verification key factor,
    /// and secret key summand if the share is valid.
    ///
    /// # Parameters
    ///
    /// - `verifiable_share`: the share to verify
    /// - `position`: the position of the recipient
    ///
    /// # Errors
    ///
    /// - `ShareVerificationFailed` if the shares do not verify.
    fn verify_share(
        verifiable_share: &VerifiableShare<C, T>,
        position: &ParticipantPosition<P>,
    ) -> Result<(C::Element, C::Element, C::Scalar), Error> {
        let g = C::generator();
        let share = &verifiable_share.value;
        let checking_values = &verifiable_share.checking_values;
        let lhs = g.exp(share);
        let rhs = Self::vk_factor(checking_values, position);

        if lhs != rhs {
            return Err(Error::ShareVerificationFailed(
                "Failed to verify share".into(),
            ));
        }

        // pk_factor, vk_factor, sk_summand
        Ok((checking_values[0].clone(), rhs, share.clone()))
    }
}

/**
 * One participant's contribution to decrypting a list of ciphertexts: a factor
 * `u_i^{sk}` for every ciphertext, and a **single** proof covering all of them.
 *
 * This is what a participant publishes. It is produced by
 * [`Recipient::partial_decrypt`] and consumed, once attributed to its author, by
 * [`combine`].
 *
 * # There is deliberately no position field
 *
 * A participant must not be able to assert *which* participant it is in data it
 * controls — otherwise it could claim another's position and be checked against
 * the wrong verification key. The position is recovered from the authenticated
 * envelope this is carried in (for braid, the signed message's sender), and
 * attached separately in [`AttributedDecryption`]. Adding a `source` field here
 * would move a security-critical value into attacker-controlled data.
 *
 * # One proof, not `N`
 *
 * The proof is over a random linear combination of the ciphertexts rather than
 * over each one: with `A = ∏ u_i^{e_i}` and `B = ∏ f_i^{e_i}` for exponents
 * derived from the factors themselves, `B = A^{sk}` holds if and only if every
 * `f_i = u_i^{sk}`, except with negligible probability. The exponents must be
 * derived *after* the factors are fixed, which
 * the batching-exponent derivation enforces by hashing them.
 */
#[derive(Debug, Clone, VSerializable, PartialEq)]
pub struct PartialDecryption<C: Context, const W: usize> {
    /// The partial decryption of each ciphertext, in the ciphertexts' order
    pub factors: Vec<[C::Element; W]>,
    /// A single proof that every factor was computed with the same secret key,
    /// and that this key matches the author's verification key
    pub proof: DlogEqProof<C, W>,
}

impl<C: Context, const W: usize> PartialDecryption<C, W> {
    /// Constructs a new [`PartialDecryption`] from the given values.
    ///
    /// The standard way to produce one is [`Recipient::partial_decrypt`].
    #[must_use]
    pub fn new(factors: Vec<[C::Element; W]>, proof: DlogEqProof<C, W>) -> Self {
        Self { factors, proof }
    }
}

/**
 * A [`PartialDecryption`] together with everything needed to check it, none of
 * which the author supplied.
 *
 * [`combine`] takes `T` of these. Bundling the three per-participant values —
 * rather than passing parallel arrays — means they cannot be misaligned against
 * each other, which was previously the caller's responsibility to get right.
 */
#[derive(Debug, Clone, PartialEq)]
pub struct AttributedDecryption<C: Context, const W: usize, const P: usize> {
    /// The published contribution
    pub partial: PartialDecryption<C, W>,
    /// Who published it, from the authenticated envelope rather than the body
    pub source: ParticipantPosition<P>,
    /// That participant's verification key, from the DKG public key
    pub verification_key: C::Element,
}

impl<C: Context, const W: usize, const P: usize> AttributedDecryption<C, W, P> {
    /// Constructs a new [`AttributedDecryption`] from the given values.
    #[must_use]
    pub fn new(
        partial: PartialDecryption<C, W>,
        source: ParticipantPosition<P>,
        verification_key: C::Element,
    ) -> Self {
        Self {
            partial,
            source,
            verification_key,
        }
    }
}

/// Domain separation tags for the batching seed.
const BATCH_SEED_TAGS: [&[u8]; 4] = [
    b"batch_verification_key",
    b"batch_ciphertexts",
    b"batch_factors",
    b"batch_proof_context",
];

/// Domain separation tags for each batching exponent.
const BATCH_EXPONENT_TAGS: [&[u8]; 2] = [b"batch_seed", b"batch_index"];

/// The exponents `e_i` batching a participant's decryption factors into a single
/// discrete-log equality statement.
///
/// # Why these must be derived, not chosen
///
/// Batching is only sound if the prover cannot pick its factors after learning
/// the exponents. Hashing the factors into the seed fixes them first, which is
/// the Fiat–Shamir analogue of the verifier sending `e` after receiving them. A
/// caller supplying its own exponents could produce a proof for factors that are
/// individually wrong but happen to satisfy the combination.
///
/// The verification key is included so a proof cannot be replayed as another
/// participant's, and the ciphertexts so it cannot be replayed onto a different
/// list.
///
/// # Two stages, deliberately
///
/// The transcript is hashed **once** into a seed, and the `N` exponents are then
/// derived from `(seed, i)`. Hashing the whole transcript per exponent would be
/// quadratic in the number of ciphertexts.
///
/// # Errors
///
/// - `MismatchedMultiExpLength` if `ciphertexts` and `factors` differ in length
/// - `HashToElementError` if scalar derivation fails
fn batching_exponents<C: Context, const W: usize>(
    verification_key: &C::Element,
    ciphertexts: &[[C::Element; W]],
    factors: &[[C::Element; W]],
    proof_context: &[u8],
) -> Result<Vec<C::Scalar>, Error> {
    use crate::traits::groups::CryptographicGroup;
    use crate::utils::hash::{update_hasher, Hasher};
    use crate::utils::serialization::VSerializable as _;
    use sha3::Digest as _;

    if ciphertexts.len() != factors.len() {
        return Err(Error::MismatchedMultiExpLength(
            ciphertexts.len(),
            factors.len(),
        ));
    }

    let seed_input = [
        verification_key.ser(),
        ciphertexts.to_vec().ser(),
        factors.to_vec().ser(),
        proof_context.to_vec(),
    ];
    let slices: Vec<&[u8]> = seed_input.iter().map(Vec::as_slice).collect();
    let mut hasher = C::Hasher::hasher();
    update_hasher(&mut hasher, &slices, &BATCH_SEED_TAGS);
    let seed = hasher.finalize();

    (0..factors.len())
        .map(|index| {
            let index: u64 = index.try_into().expect("length fits in u64");
            C::G::hash_to_scalar(&[&seed, &index.to_be_bytes()], &BATCH_EXPONENT_TAGS)
        })
        .collect()
}

/**
 * The joint public key computed as output of the Joint-Feldman DKG protocol.
 *
 * This type is a newtype wrapper around a plain `ElGamal` public key, parameterized by
 * the threshold `T`. Ciphertexts encrypted with this joint public key will require
 * a threshold `T` of participants to decrypt.
 */
#[derive(Debug, VSerializable, PartialEq, Clone)]
pub struct DkgPublicKey<C: Context, const T: usize> {
    /// the underlying `ElGamal` public key
    pub inner: PublicKey<C>,
}

impl<C: Context, const T: usize> DkgPublicKey<C, T> {
    /// Constructs a new [`DkgPublicKey`] from the given `ElGamal` public key.
    pub fn from_public_key(public_key: &PublicKey<C>) -> Self {
        let inner = public_key.clone();
        Self { inner }
    }

    /// Constructs a new [`DkgPublicKey`] from the given `ElGamal` keypair.
    pub fn from_keypair(keypair: &KeyPair<C>) -> Self {
        let inner = PublicKey {
            y: keypair.pkey.y.clone(),
        };
        Self { inner }
    }

    /// Encrypts a message and returns a [`DkgCiphertext`], a ciphertext parameterized by `T`.
    ///
    /// See [`elgamal::PublicKey::encrypt`][`crate::cryptosystem::elgamal::PublicKey::encrypt`]
    pub fn encrypt<const W: usize>(&self, message: &[C::Element; W]) -> DkgCiphertext<C, W, T> {
        let mut rng = C::get_rng();
        let r = <[C::Scalar; W]>::random(&mut rng);
        self.encrypt_with_r(message, &r)
    }

    /// Encrypts a message into a [`DkgCiphertext`], a ciphertext parameterized by `T`.
    ///
    /// See [`elgamal::PublicKey::encrypt_with_r`][`crate::cryptosystem::elgamal::PublicKey::encrypt_with_r`]
    pub fn encrypt_with_r<const W: usize>(
        &self,
        message: &[C::Element; W],
        r: &[C::Scalar; W],
    ) -> DkgCiphertext<C, W, T> {
        DkgCiphertext::<C, W, T>(self.inner.encrypt_with_r(message, r))
    }
}

/**
 * A newtype around a plain `ElGamal` ciphertext, parameterized by the threshold `T`.
 *
 * Ciphertexts can be partially decrypted using the [`Recipient::partial_decrypt`] method,
 * which produces a [`PartialDecryption`] covering the whole list. Given
 * `T` of these, the plaintext can be computed using the [`combine`]
 * function.
 */
#[derive(Debug, PartialEq, VSerializable)]
pub struct DkgCiphertext<C: Context, const W: usize, const T: usize>(pub Ciphertext<C, W>);
impl<C: Context, const W: usize, const T: usize> DkgCiphertext<C, W, T> {
    /// Returns the `u` component of the ciphertext.
    pub fn u(&self) -> &[C::Element; W] {
        self.0.u()
    }
    /// Returns the `v` component of the ciphertext.
    pub fn v(&self) -> &[C::Element; W] {
        self.0.v()
    }
}

/**
 * A participant's position in the DKG protocol.
 *
 * Participants of the DKG protocol play both the role of [Dealer][`crate::dkgd::dealer::Dealer`]
 * and [Recipient][`Recipient`]. Each participant is assigned a 1-based index;
 * the first participant is assigned position 1, and so on up to participant `P`.
 */
#[derive(Clone, Debug, VSerializable, PartialEq)]
pub struct ParticipantPosition<const P: usize>(pub u32);

impl<const P: usize> ParticipantPosition<P> {
    /// Creates a new [`ParticipantPosition`] with the given 1-based index
    /// as a u32.
    ///
    /// The supplied position must be an integer greater than zero and
    /// smaller than or equal to `P`.
    ///
    /// # Panics
    ///
    /// Panics if the position is not in the range [1, P].
    #[must_use]
    pub fn new(position: u32) -> Self {
        #[crate::warning("Possibly avoidable panics")]
        assert!(position > 0);
        assert!(position as usize <= P);

        ParticipantPosition(position)
    }
    /// Creates a new [`ParticipantPosition`] with the given 1-based index
    /// as a usize.
    ///
    /// The supplied position must be an integer greater than zero.
    /// The number of participants in a DKG protocol is usually below 10,
    /// and never exceeds the capacity of a u32; the conversion from
    /// usize to u32 cannot fail unless this assumption is violated.
    ///
    /// # Panics
    ///
    /// Panics if the position is not in the range [1, P].
    #[must_use]
    pub fn from_usize(position: usize) -> Self {
        #[crate::warning("Possibly avoidable panics")]
        assert!(position > 0);
        assert!(position <= P);

        let p_u32: u32 = position.try_into().expect("position <= P < 100 < u32::MAX");

        Self::new(p_u32)
    }
}

/// Combine `T` participants' partial decryptions and apply them to the
/// ciphertexts to yield the plaintexts.
///
/// Each contribution's batched proof is verified before its factors are used, so
/// a failure names the participant responsible rather than only reporting that
/// the set as a whole is bad — which is why each participant proves separately
/// instead of the `T` proofs being combined into one.
///
/// # Parameters
///
/// - `ciphertexts`: the ciphertexts to decrypt, marked with matching `T` parameters
/// - `contributions`: the `T` participants' partial decryptions, each carrying its
///   own author and verification key
/// - `proof_context`: proof context label (ZKP CONTEXT)
///
/// # Errors
///
/// - `MismatchedMultiExpLength` if a contribution has a factor count other than
///   the number of ciphertexts
/// - `HashToElementError` if any challenge generation for [`DlogEqProof`] verification returns error
/// - `DecryptProofFailed` if any of the decryption proofs fail to verify.
pub fn combine<C: Context, const T: usize, const P: usize, const W: usize>(
    ciphertexts: &[DkgCiphertext<C, W, T>],
    contributions: &[AttributedDecryption<C, W, P>; T],
    proof_context: &[u8],
) -> Result<Vec<[C::Element; W]>, Error> {
    // get the participants
    let present: [ParticipantPosition<P>; T] = array::from_fn(|i| contributions[i].source.clone());
    let bases: Vec<[C::Element; W]> = ciphertexts.iter().map(|c| c.u().clone()).collect();
    let mut divisors_acc: Vec<[C::Element; W]> = vec![<[C::Element; W]>::one(); ciphertexts.len()];

    #[crate::warning("Ensure that the contributions are from distinct participants.")]
    for contribution in contributions {
        let factors = &contribution.partial.factors;
        if factors.len() != ciphertexts.len() {
            return Err(Error::MismatchedMultiExpLength(
                ciphertexts.len(),
                factors.len(),
            ));
        }

        // Rebuild the batched statement from the published factors. The
        // exponents are a function of those factors, so a participant cannot
        // have chosen them to suit a set of wrong ones.
        let exponents = batching_exponents::<C, W>(
            &contribution.verification_key,
            &bases,
            factors,
            proof_context,
        )?;
        let a = <[C::Element; W]>::dist_multi_exp(&bases, &exponents)?;
        let b = <[C::Element; W]>::dist_multi_exp(factors, &exponents)?;

        let proof_ok = contribution.partial.proof.verify(
            &C::generator(),
            &contribution.verification_key,
            &a,
            &b,
            proof_context,
        )?;
        if !proof_ok {
            return Err(Error::DecryptProofFailed(format!(
                "Failed to verify decryption proof of participant {}",
                contribution.source.0
            )));
        }

        let lagrange = lagrange::<C, T, P>(&contribution.source, &present);
        for (divisor, factor) in divisors_acc.iter_mut().zip(factors) {
            *divisor = divisor.mul(&factor.dist_exp(&lagrange));
        }
    }

    Ok(divisors_acc
        .iter()
        .zip(ciphertexts.iter())
        .map(|(d, c)| c.v().mul(&d.inv()))
        .collect())
}

#[crate::warning("Rustdoc needs a reference to lagrange coeff. calculation")]
/// Compute the Lagrange coefficient for the given participant.
///
/// # Parameters
///
/// - `position`: the participant for whom to compute the Lagrange coefficient
/// - `present`: the set of participants currently present (those selected for decryption), in any order
///
/// For participant j in a set of participants K, the Lagrange coefficient
/// is computed as:
///
/// `lambda_j` = \prod_{j \neq K} \frac{k}{k - j}
pub(crate) fn lagrange<C: Context, const T: usize, const P: usize>(
    position: &ParticipantPosition<P>,
    present: &[ParticipantPosition<P>; T],
) -> C::Scalar {
    let mut numerator = C::Scalar::one();
    let mut denominator = C::Scalar::one();
    let position_exp: C::Scalar = position.0.into();

    for p in present {
        if p.0 == position.0 {
            continue;
        }

        let present_exp: C::Scalar = p.0.into();
        let diff_exp = present_exp.sub(&position_exp);

        numerator = numerator.mul(&present_exp);
        denominator = denominator.mul(&diff_exp);
    }

    numerator.mul(
        &denominator
            .inv()
            .expect("denominator != 0: the denominator is a product of non-zero elements"),
    )
}
