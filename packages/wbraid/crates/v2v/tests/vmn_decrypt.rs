// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Converting braid's DKG and decryption output into Verificatum's form.
//!
//! The load-bearing check is that the polynomial in the exponent derived from
//! braid's per-dealer commitments has `Γ_0` equal to the DKG's own joint public
//! key. That is exactly what Verificatum's Algorithm 24 cross-checks, and it is
//! verifiable here without any Verificatum involvement because `y` comes from a
//! completely different code path — the recipient's share verification — than
//! the commitments the product is taken over.


use v2v::decrypt;
use cryptography::context::{Context, P256Ctx};
use cryptography::dkgd::dealer::Dealer;
use cryptography::dkgd::recipient::{ParticipantPosition, Recipient};
use cryptography::groups::p256::element::P256Element;
use cryptography::traits::groups::{CryptographicGroup, GroupElement, GroupScalar};

const T: usize = 2;
const P: usize = 3;

/// Run a real DKG and return each dealer's coefficient commitments alongside the
/// joint public key the recipients derive.
fn run_dkg() -> (Vec<Vec<P256Element>>, P256Element) {
    const TEST_DKG_CTX: &[u8] = b"vmn decrypt test dkg";
    let dealers: Vec<Dealer<P256Ctx, T, P>> = (0..P).map(|_| Dealer::generate()).collect();
    let all_shares: Vec<_> = dealers
        .iter()
        .map(|d| d.get_verifiable_shares(TEST_DKG_CTX).expect("dealing must succeed"))
        .collect();

    let commitments: Vec<Vec<P256Element>> = all_shares
        .iter()
        .map(|s| s.checking_values.iter().map(|cv| cv.value).collect())
        .collect();

    // Recipient 1 verifies its dealings from every dealer and derives the joint key.
    use cryptography::dkgd::dealer::VerifiableShare;
    let shares_for_first: [VerifiableShare<P256Ctx, T>; P] = std::array::from_fn(|d| {
        VerifiableShare::new(
            all_shares[d].shares[0].clone(),
            all_shares[d].checking_values.clone(),
        )
    });
    let position = ParticipantPosition::from_usize(1);
    let (_recipient, joint_pk, _vks) =
        Recipient::<P256Ctx, T, P>::from_shares(position, &shares_for_first, TEST_DKG_CTX)
            .expect("shares must verify");

    (commitments, joint_pk.inner.y)
}

/// Γ_0 must be the joint public key. This is Algorithm 24's cross-check, and the
/// reason supplying a wrong polynomial would be worse than supplying none.
#[test]
fn gamma_zero_is_the_joint_public_key() {
    for _ in 0..5 {
        let (commitments, joint_pk) = run_dkg();
        let gamma = decrypt::polynomial_in_exponent(&commitments).expect("derive gamma");

        assert_eq!(gamma.len(), T, "one coefficient per threshold");
        assert!(
            gamma[0].equals(&joint_pk),
            "Gamma_0 must equal the DKG's joint public key"
        );
    }
}

#[test]
fn mismatched_dealer_commitments_are_rejected() {
    let (mut commitments, _) = run_dkg();
    commitments[1].pop();
    assert!(
        decrypt::polynomial_in_exponent(&commitments).is_err(),
        "dealers must all commit to the same number of coefficients"
    );
    assert!(decrypt::polynomial_in_exponent(&[]).is_err());
}

/// `f_VMN = (f_braid)^{-1/alpha}`, checked by recovering braid's factor from
/// Verificatum's: raising by `-alpha` must invert the conversion.
#[test]
fn factor_conversion_is_the_documented_exponent() {
    let mut rng = P256Ctx::get_rng();
    const W: usize = 2;
    let k = 3;

    let braid_factor: [P256Element; W] =
        std::array::from_fn(|_| <P256Ctx as Context>::G::random_element(&mut rng));
    let converted = decrypt::to_vmn_factor(
        &braid_factor,
        &decrypt::negated_inverse_alpha(k).unwrap(),
    );

    // Undo it: alpha, negated. (x^{-1/a})^{-a} = x.
    let alpha_scalar = {
        let alpha = v2v::wire::lagrange::alpha(k).to_bytes_be();
        let mut fixed = [0u8; 32];
        fixed[32 - alpha.len()..].copy_from_slice(&alpha);
        cryptography::groups::p256::scalar::P256Scalar::from_bytes_reduced(&fixed)
    };
    for i in 0..W {
        let recovered = converted[i].exp(&alpha_scalar.neg());
        assert!(
            recovered.equals(&braid_factor[i]),
            "component {i} must round-trip through the alpha conversion"
        );
    }
}

/// A non-participating party still needs a factor file, of the same shape, all
/// identity — otherwise the verifier halts on the missing file.
#[test]
fn inactive_factors_are_all_identity() {
    const W: usize = 2;
    let factors = decrypt::inactive_factors::<W>(7);
    assert_eq!(factors.len(), 7, "one per ciphertext");
    for factor in &factors {
        for component in factor {
            assert!(component.is_identity(), "every component is the group identity");
        }
    }
}

