// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Distributed key generation and decryption module tests

use crate::cryptosystem::elgamal::{Ciphertext, PublicKey};
use crate::dkgd::dealer::{CheckingValue, Dealer, VerifiableShare};
use crate::dkgd::recipient::combine;
use crate::dkgd::recipient::{
    AttributedDecryption, PartialDecryption, ParticipantPosition, Recipient,
};
use crate::traits::groups::DistGroupOps;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::utils::error::Error;
use std::array;

use crate::context::Context;
use crate::context::RistrettoCtx as RCtx;
use crate::context::RistrettoCtx as PCtx;
use rand::seq::SliceRandom;

/// Proof context under which every dealer in these tests proves knowledge of
/// its checking-value exponents.
const DKG_PROOF_CTX: &[u8] = b"dkgd test proof context";

#[test]
fn test_joint_pkey_ristretto() {
    test_joint_pkey::<RCtx, 2, 2, 2>();
    test_joint_pkey::<RCtx, 2, 3, 2>();
    test_joint_pkey::<RCtx, 3, 4, 2>();
}

#[test]
fn test_joint_pkey_p256() {
    test_joint_pkey::<PCtx, 2, 2, 2>();
    test_joint_pkey::<PCtx, 2, 3, 2>();
    test_joint_pkey::<PCtx, 3, 4, 2>();
}

#[test]
fn test_dkgd_ristretto() {
    test_dkgd::<RCtx, 2, 2, 2>();
    test_dkgd::<RCtx, 2, 3, 2>();
    test_dkgd::<RCtx, 3, 4, 2>();
    test_dkgd_all_participants::<RCtx, 1, 1, 2>();
}

#[test]
fn test_dkgd_p256() {
    test_dkgd::<PCtx, 2, 2, 2>();
    test_dkgd::<PCtx, 2, 3, 2>();
    test_dkgd::<PCtx, 3, 4, 2>();
    test_dkgd_all_participants::<PCtx, 1, 1, 2>();
}

#[test]
fn test_dkgd_non_t_ristretto() {
    test_dkgd_all_participants::<RCtx, 2, 2, 2>();
    test_dkgd_all_participants::<RCtx, 2, 3, 2>();
    test_dkgd_all_participants::<RCtx, 3, 3, 2>();
    test_dkgd_all_participants::<RCtx, 3, 4, 2>();
    test_dkgd_all_participants::<RCtx, 1, 1, 2>();
}

#[test]
fn test_dkgd_non_t_p256() {
    test_dkgd_all_participants::<PCtx, 2, 2, 2>();
    test_dkgd_all_participants::<PCtx, 2, 3, 2>();
    test_dkgd_all_participants::<PCtx, 3, 3, 2>();
    test_dkgd_all_participants::<PCtx, 3, 4, 2>();
    test_dkgd_all_participants::<PCtx, 1, 1, 2>();
}

fn test_dkgd<C: Context, const T: usize, const P: usize, const W: usize>() {
    assert!(T <= P);

    let dealers: [Dealer<C, T, P>; P] = array::from_fn(|_| Dealer::generate());

    let mut recipients: [(Recipient<C, T, P>, PublicKey<C>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);

        let verifiable_shares: [VerifiableShare<C, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_PROOF_CTX)
                .unwrap()
                .for_recipient(&position)
        });

        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &verifiable_shares, DKG_PROOF_CTX).unwrap();
        (recipient, joint_pk)
    });

    // check different ways of computing verification keys match
    let verification_keys: [C::Element; T] =
        array::from_fn(|i| recipients[i].0.get_verification_key().clone());

    let all_checking_values: [[<C as Context>::Element; T]; P] = dealers.clone().map(|d| {
        d.get_verifiable_shares(DKG_PROOF_CTX)
            .unwrap()
            .checking_values
            .map(|cv| cv.value)
    });
    let verification_keys_2: [C::Element; T] = array::from_fn(|i| {
        let position: ParticipantPosition<P> = ParticipantPosition::from_usize(i + 1);
        Recipient::<C, T, P>::verification_key(&position, &all_checking_values)
    });
    assert_eq!(verification_keys, verification_keys_2);

    let mut rng = C::get_rng();
    recipients.shuffle(&mut rng);

    let pk: &PublicKey<C> = &recipients[0].1;

    let message: [C::Element; W] = array::from_fn(|_| C::random_element());
    let encrypted = vec![pk.encrypt(&message)];

    // Each contribution carries its own author and key, so shuffling the
    // recipients cannot misalign them against the factors.
    let contributions: [AttributedDecryption<C, W, P>; P] = recipients.map(|r| {
        let partial = r.0.partial_decrypt(&encrypted, &vec![]).unwrap();
        AttributedDecryption::new(
            partial,
            r.0.get_position().clone(),
            r.0.get_verification_key().clone(),
        )
    });

    let threshold: &[AttributedDecryption<C, W, P>; T] = contributions[0..T]
        .try_into()
        .expect("slice matches array: T == T");
    let decrypted = combine(&encrypted, threshold, &vec![]);
    assert!(message == decrypted.unwrap()[0]);
}

fn test_dkgd_all_participants<C: Context, const T: usize, const P: usize, const W: usize>() {
    assert!(T <= P);

    let dealers: [Dealer<C, T, P>; P] = array::from_fn(|_| Dealer::generate());

    let recipients: [(Recipient<C, T, P>, PublicKey<C>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);

        let verifiable_shares: [VerifiableShare<C, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_PROOF_CTX)
                .unwrap()
                .for_recipient(&position)
        });

        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &verifiable_shares, DKG_PROOF_CTX).unwrap();
        (recipient, joint_pk)
    });

    let pk: &PublicKey<C> = &recipients[0].1;

    let message: [C::Element; W] = array::from_fn(|_| C::random_element());
    let encrypted = vec![pk.encrypt(&message)];

    let mut dfactors: [(PartialDecryption<C, W>, ParticipantPosition<P>); P] =
        recipients.map(|r| {
            let partial = r.0.partial_decrypt(&encrypted, &vec![]).unwrap();
            (partial, r.0.get_position().clone())
        });
    let mut rng = C::get_rng();
    dfactors.shuffle(&mut rng);

    // using all participants, not just T of them
    assert_eq!(dfactors.len(), P);

    let decrypted = untyped_combine(&encrypted, &dfactors);
    assert!(message == decrypted[0]);
}

fn test_joint_pkey<C: Context, const T: usize, const P: usize, const W: usize>() {
    assert!(T <= P);

    let dealers: [Dealer<C, T, P>; P] = array::from_fn(|_| Dealer::generate());

    let recipients: [(Recipient<C, T, P>, PublicKey<C>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);

        let verifiable_shares: [VerifiableShare<C, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_PROOF_CTX)
                .unwrap()
                .for_recipient(&position)
        });

        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &verifiable_shares, DKG_PROOF_CTX).unwrap();
        (recipient, joint_pk)
    });

    // all computed joint public keys are equal
    let equal = recipients.windows(2).all(|w| w[0].1 == w[1].1);
    assert!(equal);

    let lhs = Recipient::<C, T, P>::joint_public_key(&dealers.map(|d| {
        d.get_verifiable_shares(DKG_PROOF_CTX)
            .unwrap()
            .checking_values
            .map(|cv| cv.value)
    }));
    let rhs: PublicKey<C> = recipients[0].1.clone();

    assert_eq!(lhs, rhs);
}

/// Combine without a threshold bound and without verifying proofs, to check that
/// all `P` participants reconstruct the same secret that `T` of them do.
fn untyped_combine<C: Context, const P: usize, const W: usize>(
    ciphertexts: &[Ciphertext<C, W>],
    dfactors: &[(PartialDecryption<C, W>, ParticipantPosition<P>)],
) -> Vec<[C::Element; W]> {
    // get the participants
    let present: Vec<ParticipantPosition<P>> =
        dfactors.iter().map(|(_, source)| source.clone()).collect();

    let mut divisors_acc = vec![<[C::Element; W]>::one(); ciphertexts.len()];

    for (partial, source) in dfactors {
        let iter = partial.factors.iter();
        let lagrange = untyped_lagrange::<C, P>(source, &present);

        let raised = iter.map(|factor| factor.dist_exp(&lagrange));

        divisors_acc = divisors_acc
            .iter()
            .zip(raised)
            .map(|(r, divisor)| r.mul(&divisor))
            .collect();
    }

    let ret: Vec<[C::Element; W]> = divisors_acc
        .iter()
        .zip(ciphertexts.iter())
        .map(|(d, c)| c.v().mul(&d.inv()))
        .collect();

    ret
}

fn untyped_lagrange<C: Context, const P: usize>(
    trustee: &ParticipantPosition<P>,
    present: &[ParticipantPosition<P>],
) -> C::Scalar {
    let mut numerator = C::Scalar::one();
    let mut denominator = C::Scalar::one();
    let trustee_exp: C::Scalar = trustee.0.into();

    for p in present {
        if p.0 == trustee.0 {
            continue;
        }

        let present_exp: C::Scalar = p.0.into();
        let diff_exp = present_exp.sub(&trustee_exp);

        numerator = numerator.mul(&present_exp);
        denominator = denominator.mul(&diff_exp);
    }

    numerator.mul(&denominator.inv().unwrap())
}

// -------------------------------------------------------------------------
// The batched proof must actually reject
// -------------------------------------------------------------------------

#[test]
fn test_batched_proof_rejects_ristretto() {
    test_batched_proof_rejects::<RCtx, 2, 3, 2>();
}

#[test]
fn test_batched_proof_rejects_p256() {
    test_batched_proof_rejects::<PCtx, 2, 3, 2>();
}

/// A single wrong factor anywhere in the array must sink the whole
/// contribution, and the error must name its author.
///
/// Batching makes this the load-bearing test. The success tests only show that
/// honest factors combine to the right plaintext, which a proof that always
/// verified would also satisfy — so without this, a batching bug (exponents not
/// bound to the factors, a stale transcript, a mis-derived `A` or `B`) would
/// pass the whole suite unnoticed. It corrupts *every* position in turn because
/// batching is exactly the step that could be blind to one of them.
fn test_batched_proof_rejects<C: Context, const T: usize, const P: usize, const W: usize>() {
    const N: usize = 4;

    let dealers: [Dealer<C, T, P>; P] = array::from_fn(|_| Dealer::generate());
    let recipients: [(Recipient<C, T, P>, PublicKey<C>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);
        let verifiable_shares: [VerifiableShare<C, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_PROOF_CTX)
                .unwrap()
                .for_recipient(&position)
        });
        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &verifiable_shares, DKG_PROOF_CTX).unwrap();
        (recipient, joint_pk)
    });

    let pk: &PublicKey<C> = &recipients[0].1;
    let encrypted: Vec<_> = (0..N)
        .map(|_| {
            let message: [C::Element; W] = array::from_fn(|_| C::random_element());
            pk.encrypt(&message)
        })
        .collect();

    let contributions: Vec<AttributedDecryption<C, W, P>> = recipients
        .iter()
        .map(|r| {
            let partial = r.0.partial_decrypt(&encrypted, &vec![]).unwrap();
            AttributedDecryption::new(
                partial,
                r.0.get_position().clone(),
                r.0.get_verification_key().clone(),
            )
        })
        .collect();

    // Sanity: untampered, this set decrypts.
    let clean: [AttributedDecryption<C, W, P>; T] = contributions[0..T]
        .to_vec()
        .try_into()
        .expect("slice matches array");
    assert!(combine(&encrypted, &clean, &vec![]).is_ok());

    let mut rng = C::get_rng();
    for corrupt_at in 0..N {
        let mut tampered = contributions[0..T].to_vec();
        tampered[0].partial.factors[corrupt_at] = <[C::Element; W]>::random(&mut rng);

        let tampered: [AttributedDecryption<C, W, P>; T] =
            tampered.try_into().expect("slice matches array");
        let result = combine(&encrypted, &tampered, &vec![]);

        match result {
            Err(Error::DecryptProofFailed(message)) => {
                assert!(
                    message.contains(&format!("{}", contributions[0].source.0)),
                    "the failure must name the participant responsible, got: {message}"
                );
            }
            Err(other) => panic!("expected a proof failure at {corrupt_at}, got {other:?}"),
            Ok(_) => panic!("a corrupted factor at position {corrupt_at} was accepted"),
        }
    }

    // Swapping two factors is the sharper case, and the reason the exponents
    // have to be distinct rather than merely present: it leaves the *product*
    // of the factors unchanged, so an implementation that batched with an
    // all-ones vector would accept it while being trivially unsound. Only
    // genuinely different `e_i` separate `e_1·f_1 + e_2·f_2` from
    // `e_1·f_2 + e_2·f_1`.
    let mut swapped = contributions[0..T].to_vec();
    swapped[0].partial.factors.swap(0, 1);
    let swapped: [AttributedDecryption<C, W, P>; T] =
        swapped.try_into().expect("slice matches array");
    assert!(
        matches!(
            combine(&encrypted, &swapped, &vec![]),
            Err(Error::DecryptProofFailed(_))
        ),
        "swapping two factors must be detected, not just corrupting one"
    );
}

// -------------------------------------------------------------------------
// from_shares is the single verification gate: both parts must reject
// -------------------------------------------------------------------------

/// `from_shares` performs round 2's complete verification — the
/// checking-value proofs and the algebraic share check. Each part must
/// reject on its own, and the error must name the dealer responsible.
#[test]
fn test_from_shares_rejects() {
    const T: usize = 2;
    const P: usize = 3;

    let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
    let position = ParticipantPosition::from_usize(1);
    let shares: [VerifiableShare<RCtx, T>; P] = dealers.clone().map(|d| {
        d.get_verifiable_shares(DKG_PROOF_CTX)
            .unwrap()
            .for_recipient(&position)
    });

    // Untampered: verifies.
    assert!(Recipient::<RCtx, T, P>::from_shares(position.clone(), &shares, DKG_PROOF_CTX).is_ok());

    // A swapped checking-value proof (valid, but for another dealer's value)
    // must reject, naming dealer 2.
    let mut tampered = dealers.clone().map(|d| {
        d.get_verifiable_shares(DKG_PROOF_CTX)
            .unwrap()
            .for_recipient(&position)
    });
    let foreign_proof = tampered[0].checking_values[0].proof.clone();
    tampered[1].checking_values[0].proof = foreign_proof;
    match Recipient::<RCtx, T, P>::from_shares(position.clone(), &tampered, DKG_PROOF_CTX) {
        Err(Error::ShareVerificationFailed(msg)) => {
            assert!(
                msg.contains("checking-value proof") && msg.contains("dealer 2"),
                "must name the failing part and dealer, got: {msg}"
            );
        }
        Err(other) => panic!("expected ShareVerificationFailed, got {other:?}"),
        Ok(_) => panic!("a swapped proof must reject"),
    }

    // A tampered share (proofs intact) must reject the algebraic check,
    // naming dealer 3.
    let mut tampered = dealers.map(|d| {
        d.get_verifiable_shares(DKG_PROOF_CTX)
            .unwrap()
            .for_recipient(&position)
    });
    tampered[2].value = tampered[2].value.add(&<RCtx as Context>::Scalar::one());
    match Recipient::<RCtx, T, P>::from_shares(position, &tampered, DKG_PROOF_CTX) {
        Err(Error::ShareVerificationFailed(msg)) => {
            assert!(
                msg.contains("share") && msg.contains("dealer 3"),
                "must name the failing part and dealer, got: {msg}"
            );
        }
        Err(other) => panic!("expected ShareVerificationFailed, got {other:?}"),
        Ok(_) => panic!("a tampered share must reject"),
    }
}

// -------------------------------------------------------------------------
// The checking-value proofs must actually reject
// -------------------------------------------------------------------------

/// A checking value's Schnorr proof must verify only for the value it was
/// produced for, and only under the context it was produced under. Without
/// this, a dealer could post checking values whose exponents it does not
/// know (the rogue-key pattern of eprint 2024/915 §2.4) and the proofs
/// would be decoration.
#[test]
fn test_checking_value_proofs_reject() {
    const T: usize = 3;
    const P: usize = 4;

    let g = RCtx::generator();
    let dealer: Dealer<RCtx, T, P> = Dealer::generate();
    let shares = dealer.get_verifiable_shares(DKG_PROOF_CTX).unwrap();

    // Untampered: every proof verifies.
    for cv in &shares.checking_values {
        assert!(cv.verify(&g, DKG_PROOF_CTX).unwrap());
    }

    // A different context must not verify: the proof is bound to its domain.
    for cv in &shares.checking_values {
        assert!(!cv.verify(&g, b"some other proof context").unwrap());
    }

    // A proof attached to a different value must not verify: a dealer cannot
    // present a value it did not choose with a proof it holds for another.
    let other: Dealer<RCtx, T, P> = Dealer::generate();
    let other_shares = other.get_verifiable_shares(DKG_PROOF_CTX).unwrap();
    let forged = CheckingValue::new(
        other_shares.checking_values[0].value.clone(),
        shares.checking_values[0].proof.clone(),
    );
    assert!(!forged.verify(&g, DKG_PROOF_CTX).unwrap());
}

/// Swapping the proof between two participants must fail: the batching
/// exponents commit to the verification key, so a contribution cannot be
/// replayed as somebody else's.
#[test]
fn test_batched_proof_is_bound_to_its_author() {
    const T: usize = 2;
    const P: usize = 3;
    const W: usize = 2;

    let dealers: [Dealer<RCtx, T, P>; P] = array::from_fn(|_| Dealer::generate());
    let recipients: [(Recipient<RCtx, T, P>, PublicKey<RCtx>); P] = array::from_fn(|i| {
        let position = ParticipantPosition::from_usize(i + 1);
        let verifiable_shares: [VerifiableShare<RCtx, T>; P] = dealers.clone().map(|d| {
            d.get_verifiable_shares(DKG_PROOF_CTX)
                .unwrap()
                .for_recipient(&position)
        });
        let (recipient, joint_pk, _vks) =
            Recipient::from_shares(position, &verifiable_shares, DKG_PROOF_CTX).unwrap();
        (recipient, joint_pk)
    });

    let pk = &recipients[0].1;
    let message: [<RCtx as Context>::Element; W] = array::from_fn(|_| RCtx::random_element());
    let encrypted = vec![pk.encrypt(&message)];

    let first = recipients[0]
        .0
        .partial_decrypt(&encrypted, &vec![])
        .unwrap();
    let second = recipients[1]
        .0
        .partial_decrypt(&encrypted, &vec![])
        .unwrap();

    // Party 1's factors, party 2's proof, checked against party 1's key.
    let forged = PartialDecryption::new(first.factors.clone(), second.proof.clone());
    let contributions: [AttributedDecryption<RCtx, W, P>; T] = [
        AttributedDecryption::new(
            forged,
            recipients[0].0.get_position().clone(),
            recipients[0].0.get_verification_key().clone(),
        ),
        AttributedDecryption::new(
            second,
            recipients[1].0.get_position().clone(),
            recipients[1].0.get_verification_key().clone(),
        ),
    ];

    assert!(
        matches!(
            combine(&encrypted, &contributions, &vec![]),
            Err(Error::DecryptProofFailed(_))
        ),
        "a proof from another participant must not verify"
    );
}
