// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Joint-Feldman distributed key generation: dealer

use std::array;

use crate::context::Context;
use crate::dkgd::recipient::ParticipantPosition;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::zkp::schnorr::SchnorrProof;
use crate::utils::error::Error;
use vser_derive::VSerializable;

/**
 * A dealer in the Joint-Feldman distributed key generation (DKG) protocol.
 *
 * * NOTE: this API does not represent private shares as encrypted values.
 *   In the messaging layer, private shares should be encrypted with the recipient's
 *   public key.
 *
 * In the Joint-Feldman DKG, the dealer generates a random polynomial `f` of degree
 * `T - 1` and distributes `P` shares of its secret, `f(0)`, to all participants,
 * including itself. The dealer also publishes `T` checking values that allow the
 * participants to verify their shares.
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
 * At least `T` of the `P` participants are needed to decrypt ciphertexts encrypted
 * with the joint public key.
 *
 * See `EVS`: Protocol 16.20
 *
 * See also [Recipient][`crate::dkgd::recipient::Recipient`]
 *
 * # Examples
 *
 * ```
 * use std::array;
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::groups::ristretto255::RistrettoElement;
 * use cryptography::dkgd::dealer::{VerifiableShare, Dealer};
 * use cryptography::dkgd::recipient::{combine, Recipient, ParticipantPosition, AttributedDecryption};
 * use cryptography::cryptosystem::elgamal::PublicKey;
 *
 * const P: usize = 3;
 * const T: usize = 2;
 * const W: usize = 2;
 *
 * // Simulates the DKG protocol
 *
 * let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
 *
 * let recipients: [(Recipient<RCtx, T, P>, PublicKey<RCtx>); P] = array::from_fn(|i| {
 *     let position = ParticipantPosition::from_usize(i + 1);
 *
 *     let verifiable_shares: [VerifiableShare<RCtx, T>; P] = dealers
 *         .clone()
 *         .map(|d| d.get_verifiable_shares(b"dkg proof context").unwrap().for_recipient(&position));
 *
 *     let (recipient, joint_pk, _vks) =
 *         Recipient::from_shares(position, &verifiable_shares, b"dkg proof context").unwrap();
 *     (recipient, joint_pk)
 * });
 *
 * // Simulates distributed decryption
 *
 * // the joint public key is returned from Recipient::from_shares, but can also
 * // be obtained via [`Recipient::joint_public_key`]
 * let (recipient, pk) = &recipients[0];
 * // encrypt a message of width `W
 * let message: [RistrettoElement; W] = array::from_fn(|_| RCtx::random_element());
 * let encrypted = vec![pk.encrypt(&message)];
 *
 * // partial decryption: each participant publishes its factors with one proof
 * // covering all of them, attributed to its author. In a real execution the
 * // position and key come from the authenticated message and the DKG public
 * // key, never from the contribution itself.
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

#[derive(Clone)]
pub struct Dealer<C: Context, const T: usize, const P: usize> {
    /// The polynomial used by this dealer to share their secret.
    pub(crate) polynomial: Polynomial<C, T>,
}

impl<C: Context, const T: usize, const P: usize> Dealer<C, T, P> {
    /// compile-time checks for dealer const parameters
    const CHECK: () = {
        assert!(P < 100);
        assert!(P > 0);
        assert!(T <= P);
        assert!(T > 0);
    };

    /// Construct a new [`Dealer`] by randomly generating a `T - 1` degree polynomial.
    ///
    /// At least `T` of the `P` participants will be needed to decrypt ciphertexts
    /// encrypted with the joint public key.
    #[must_use]
    pub fn generate() -> Self {
        #[allow(path_statements)]
        Self::CHECK;

        let polynomial = Polynomial::<C, T>::generate();
        Self { polynomial }
    }

    /// Compute the `P` shares distributed by this dealer, and its `T` checking
    /// values with Schnorr proofs of knowledge of their exponents.
    ///
    /// The proofs prevent rogue-key-style attacks in which a dealer chooses its
    /// checking values as a function of other dealers' values without knowing
    /// the corresponding coefficients (see
    /// <https://eprint.iacr.org/2024/915.pdf> section 2.4). Recipients must
    /// verify every proof (via [`CheckingValue::verify`]) under the same
    /// `proof_context`.
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    ///
    /// Returns a [`DealerShares`] instance containing the shares and checking values.
    pub fn get_verifiable_shares(
        &self,
        proof_context: &[u8],
    ) -> Result<DealerShares<C, T, P>, Error> {
        let shares = self.get_shares();

        Ok(DealerShares::new(
            shares,
            self.get_checking_values_proofs(proof_context)?,
        ))
    }

    /// Compute the `P` shares distributed by this dealer.
    ///
    /// Each share is computed as `f(i)` for `i = 1, ..., P`.
    /// Use [`Self::get_verifiable_shares`] to obtain the shares [along
    /// with][`DealerShares`] their checking values.
    pub(crate) fn get_shares(&self) -> [C::Scalar; P] {
        array::from_fn(|p| {
            // p + 1 cannot overflow, P < 100 is compile-time checked
            let recipient: u32 = p.checked_add(1).expect("P < 100")
                .try_into().expect("P < 100 < u32::MAX");
            let recipient: C::Scalar = recipient.into();
            self.polynomial.eval(&recipient)
        })
    }

    /// Compute the `T` checking values for this dealer's polynomial, with Schnorr proofs.
    ///
    /// See <https://eprint.iacr.org/2024/915.pdf> section 2.4:
    /// 
    /// "Common mitigations include an initial round during which every trustee
    /// commits to its Ki,j values before opening them and resuming the protocol, or
    /// requiring every trustee to provide a Schnorr proof that it knows the discrete
    /// logarithms of its Ki,j values w.r.t. g" 
    /// 
    /// Each checking value is computed as `g^polynomial_coefficient`.
    /// Use [`Self::get_verifiable_shares`] to obtain the shares [along
    /// with][`DealerShares`] their checking values.
    pub(crate) fn get_checking_values_proofs(&self, proof_context: &[u8]) -> Result<[CheckingValue<C>; T], Error> {
        let g = C::generator();
        let values: [Result<CheckingValue<C>, Error>; T]  = self.polynomial.0.clone().map(|v| {
            let value = g.exp(&v);
            let proof = SchnorrProof::<C>::prove(&g, &value, &v, proof_context);
            let cv = CheckingValue::new(value, proof?);
            Ok(cv)
        });
        let values: Vec<CheckingValue<C>> = values
            .into_iter()
            .collect::<Result<Vec<CheckingValue<C>>, Error>>()?;
        let values: [CheckingValue<C>; T] = values.try_into().expect("Vec length matches array length T");
        
        Ok(values)
    }
}

/**
 * A polynomial of degree `T - 1` over the scalar field of the elliptic curve group, `C::G`.
 *
 * This polynomial is used by the dealer to generate shares and checking values
 * for the participants in the DKG protocol. The polynomial is defined by `T` coefficients
 * of type `C::Scalar`, as are its arguments `x` and values `f(x)`.
 */
#[derive(Clone)]
pub(crate) struct Polynomial<C: Context, const T: usize>(pub(crate) [C::Scalar; T]);

impl<C: Context, const T: usize> Polynomial<C, T> {
    /// Generate a random polynomial of degree `T - 1` with `T` coefficients.
    ///
    /// Returns a new [`Polynomial`] instance, with inner type `[C::Scalar; T]`.
    #[must_use]
    pub(crate) fn generate() -> Self {
        let coefficients: [C::Scalar; T] = array::from_fn(|_| C::random_scalar());

        Self(coefficients)
    }

    /// Evaluate the polynomial at a given point `x`.
    ///
    /// Returns the scalar `k`, where `k = f(x)`.
    pub(crate) fn eval(&self, x: &C::Scalar) -> C::Scalar {
        let mut sum: C::Scalar = self.0[0].clone();
        let mut power = C::Scalar::one();

        for v in self.0.iter().skip(1) {
            power = power.mul(x);
            sum = sum.add(&v.mul(&power));
        }

        sum
    }
}

/**
 * The set of verifiable shares produced by one dealer in the DKG protocol.
 *
 * A [`DealerShares`] contains `P` shares for each of the `P` participants, together
 * with the dealer's `T` checking values. The set of *all* shares and checking
 * values for a protocol execution would be of type `[DealerShares; P]`
 *
 * # Examples
 *
 * ```
 * use std::array;
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::dkgd::dealer::{Dealer, DealerShares};
 *
 * const P: usize = 3;
 * const T: usize = 2;
 * const W: usize = 2;
 *
 * // Generates `P` shares for threshold `T`
 * let dealer: Dealer<RCtx, T, P> = Dealer::generate();
 * let shares = dealer.get_verifiable_shares(b"dkg proof context").unwrap();
 * ```
 */
#[derive(Debug, Clone, VSerializable, PartialEq)]
pub struct DealerShares<C: Context, const T: usize, const P: usize> {
    /// The shares distributed to each participant, offset by -1.
    /// For example, the share for participant 1 is stored at index 0.
    pub shares: [C::Scalar; P],
    /// The checking values for the dealer's shares, each carrying a Schnorr
    /// proof of knowledge of its exponent.
    pub checking_values: [CheckingValue<C>; T],
}

impl<C: Context, const T: usize, const P: usize> DealerShares<C, T, P> {
    /// Construct a new [`DealerShares`] instance from the given values.
    ///
    /// The standard way to compute the shares distributed by a [`Dealer`] is
    /// through the [`Dealer::get_verifiable_shares`] method.
    pub(crate) fn new(shares: [C::Scalar; P], checking_values: [CheckingValue<C>; T]) -> Self {
        Self {
            shares,
            checking_values,
        }
    }

    /// Return the shares for the requested recipient as specified by the given [`ParticipantPosition`].
    ///
    /// This method will select the shares assigned to the required recipient from the set
    /// of all shares computed by the [`Dealer`].
    ///
    /// **Simulation convenience**: in a deployed protocol, shares reach their
    /// recipients encrypted over a message board (the caller encrypts
    /// `DealerShares::shares[i]` to recipient `i`, who decrypts it and builds
    /// its own [`VerifiableShare`]) — no production code hands a recipient its
    /// share in memory. This method exists for tests and examples that
    /// simulate the full ceremony in one process.
    ///
    /// # Panics
    ///
    /// Infallible: panics if position < 1 or position > `usize::MAX`.
    #[allow(clippy::missing_panics_doc)]
    pub fn for_recipient(&self, recipient: &ParticipantPosition<P>) -> VerifiableShare<C, T> {
        let index: usize = (recipient.0.checked_sub(1))
            .expect("ParticipantPosition is guaranteed to be in the range 1..=P")
            .try_into()
            .expect("ParticipantPosition(u32), u32 < usize::MAX");

        VerifiableShare::new(self.shares[index].clone(), self.checking_values.clone())
    }
}

/**
 * A checking value and its associated Schnorr proof.
 *
 * A [`CheckingValue`] contains an element `g^polynomial_coefficient` and a Schnorr proof of knowledge
 * of the exponent `polynomial_coefficient`.
 */
#[derive(Debug, Clone, VSerializable, PartialEq)]
pub struct CheckingValue<C: Context> {
    /// The checking value `g^polynomial_coefficient`.
    pub value: C::Element,
    /// The Schnorr proof of knowledge of the exponent `polynomial_coefficient`.
    pub proof: SchnorrProof<C>,
}
impl<C: Context> CheckingValue<C> {
    /// Construct a new [`CheckingValue`] from the given value and proof.
    pub fn new(value: C::Element, proof: SchnorrProof<C>) -> Self {
        Self { value, proof }
    }
    /// Verify the Schnorr proof of knowledge for this checking value.
    /// 
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    /// 
    /// Returns `true` if the proof is valid, `false` otherwise.
    pub fn verify(&self, g: &C::Element, proof_context: &[u8]) -> Result<bool, Error> {
        self.proof.verify(g, &self.value, proof_context)
    }
}

/**
 * One verifiable share distributed by one dealer to one recipient, in the DKG protocol.
 *
 * A [`VerifiableShare`] contains a secret scalar and the dealer's `T` checking values —
 * each carrying its Schnorr proof of knowledge — which together are everything needed
 * to verify the dealing: the proofs (against the proof context) and the share (against
 * the checking values), both performed by
 * [`Recipient::from_shares`][`crate::dkgd::recipient::Recipient::from_shares`]. The
 * secret share of the joint public key held by a recipient is the sum of the `P` secret
 * scalars it receives from all dealers (participants), including itself.
 *
 * * # Examples
 *
 * ```
 * use std::array;
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::dkgd::dealer::{Dealer, DealerShares, VerifiableShare};
 * use cryptography::dkgd::recipient::ParticipantPosition;
 *
 * const P: usize = 3;
 * const T: usize = 2;
 * const W: usize = 2;
 *
 * // Generates `P` shares for threshold `T`
 * let dealer: Dealer<RCtx, T, P> = Dealer::generate();
 * let shares = dealer.get_verifiable_shares(b"dkg proof context").unwrap();
 * // Get the shares for participant 1
 * let position = ParticipantPosition::from_usize(1);
 * let shares: VerifiableShare<RCtx, T> = shares.for_recipient(&position);
 * ```
 */
#[derive(Debug, VSerializable)]
pub struct VerifiableShare<C: Context, const T: usize> {
    /// the secret share as a raw scalar
    pub value: C::Scalar,
    /// the dealer's checking values, each carrying a Schnorr proof of
    /// knowledge of its exponent
    pub checking_values: [CheckingValue<C>; T],
}

impl<C: Context, const T: usize> VerifiableShare<C, T> {
    /// Construct a new [`VerifiableShare`] from the given values.
    ///
    /// The standard way to obtain verifiable shares for some recipient `P` is through
    /// the [`Dealer::get_verifiable_shares`] method combined with the [`DealerShares::for_recipient`]
    /// method.
    pub fn new(value: C::Scalar, checking_values: [CheckingValue<C>; T]) -> Self {
        Self {
            value,
            checking_values,
        }
    }
}
