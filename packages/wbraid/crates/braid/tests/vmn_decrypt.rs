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

#![cfg(feature = "native")]

use braid::vmn::decrypt;
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
    let dealers: Vec<Dealer<P256Ctx, T, P>> = (0..P).map(|_| Dealer::generate()).collect();
    let all_shares: Vec<_> = dealers.iter().map(|d| d.get_verifiable_shares()).collect();

    let commitments: Vec<Vec<P256Element>> = all_shares
        .iter()
        .map(|s| s.checking_values.to_vec())
        .collect();

    // Recipient 1 verifies its shares from every dealer and derives the joint key.
    use cryptography::dkgd::dealer::VerifiableShare;
    let shares_for_first: [VerifiableShare<P256Ctx, T>; P] = std::array::from_fn(|d| {
        VerifiableShare::new(
            all_shares[d].shares[0].clone(),
            all_shares[d].checking_values.clone(),
        )
    });
    let position = ParticipantPosition::from_usize(1);
    let (joint_pk, _vk, _sk) =
        Recipient::<P256Ctx, T, P>::verify_shares(&position, &shares_for_first)
            .expect("shares must verify");

    (commitments, joint_pk)
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
        let alpha = vcompat::lagrange::alpha(k).to_bytes_be();
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

/// End-to-end shape of the batched proof: several parties each prove over their
/// own share, the pieces are Lagrange-combined, and the two equations `vmnv`
/// checks must hold.
///
/// This is self-consistency, not agreement with Verificatum — `vmnv` adjudicates
/// that once a mixing proof is emitted. What it does establish is that the
/// derivation documented on `prove_decryption` closes: the alpha cancellation,
/// the sign of `k_x`, and the Lagrange combination all have to line up, or the
/// equations fail.
#[test]
fn combined_batched_proof_satisfies_the_verification_equations() {
    use braid::vmn::decrypt::{batch, prove_decryption};
    use cryptography::groups::p256::scalar::P256Scalar;

    const W: usize = 2;
    const N: usize = 4;
    let k = 3usize;
    let threshold = 2usize;

    let mut rng = P256Ctx::get_rng();
    let g = P256Element::generator();

    let scalar_from = |z: u64| {
        let mut b = [0u8; 32];
        b[24..].copy_from_slice(&z.to_be_bytes());
        P256Scalar::from_bytes_reduced(&b)
    };

    // A degree threshold-1 sharing; the secret is the constant term.
    let coefficients: Vec<P256Scalar> =
        (0..threshold).map(|_| P256Scalar::random(&mut rng)).collect();
    let evaluate = |z: u64| {
        let point = scalar_from(z);
        let mut acc = P256Scalar::zero();
        let mut power = P256Scalar::one();
        for c in &coefficients {
            acc = acc.add(&c.mul(&power));
            power = power.mul(&point);
        }
        acc
    };
    let x = coefficients[0].clone();
    let y = g.exp(&x);

    // Delta = {1, 3}, so the Lagrange coefficients are not trivial.
    let delta = vec![1usize, 3];
    let alpha_c: Vec<P256Scalar> =
        vcompat::lagrange::p256_modified_lagrange_coefficients(&delta, k)
            .into_iter()
            .map(|(negative, magnitude)| {
                let s = P256Scalar::from_bytes_reduced(&magnitude);
                if negative {
                    s.neg()
                } else {
                    s
                }
            })
            .collect();
    // c_l alone, for combining the proof pieces (the factors use alpha * c_l).
    let inv_alpha = decrypt::negated_inverse_alpha(k).unwrap().neg();
    let c: Vec<P256Scalar> = alpha_c.iter().map(|ac| ac.mul(&inv_alpha)).collect();

    // Ciphertext first components, and each party's factors u^{-x_l/alpha}.
    let u: Vec<[P256Element; W]> = (0..N)
        .map(|_| std::array::from_fn(|_| <P256Ctx as Context>::G::random_element(&mut rng)))
        .collect();
    let factors: Vec<Vec<[P256Element; W]>> = delta
        .iter()
        .map(|&l| {
            let z = evaluate(l as u64).mul(&inv_alpha).neg();
            u.iter()
                .map(|ui| std::array::from_fn(|i| ui[i].exp(&z)))
                .collect()
        })
        .collect();

    // Batched values.
    let e: Vec<P256Scalar> = (0..N).map(|_| P256Scalar::random(&mut rng)).collect();
    let a = batch(&u, &e).unwrap();
    let combined: Vec<[P256Element; W]> = (0..N)
        .map(|i| {
            let mut acc: [P256Element; W] = std::array::from_fn(|_| P256Element::one());
            for position in 0..delta.len() {
                for w in 0..W {
                    acc[w] = acc[w].mul(&factors[position][i][w].exp(&alpha_c[position]));
                }
            }
            acc
        })
        .collect();
    let b = batch(&combined, &e).unwrap();

    // Each party proves over its own unscaled share.
    let v = P256Scalar::random(&mut rng);
    let randomizers: Vec<P256Scalar> =
        delta.iter().map(|_| P256Scalar::random(&mut rng)).collect();
    let proofs: Vec<_> = delta
        .iter()
        .enumerate()
        .map(|(position, &l)| {
            prove_decryption::<W>(&evaluate(l as u64), &a, &v, &randomizers[position])
        })
        .collect();

    // Combine by Lagrange coefficient.
    let mut y_prime = P256Element::one();
    let mut b_prime: [P256Element; W] = std::array::from_fn(|_| P256Element::one());
    let mut k_x = P256Scalar::zero();
    for (position, proof) in proofs.iter().enumerate() {
        y_prime = y_prime.mul(&proof.y_prime.exp(&c[position]));
        for w in 0..W {
            b_prime[w] = b_prime[w].mul(&proof.b_prime[w].exp(&c[position]));
        }
        k_x = k_x.add(&c[position].mul(&proof.k_x));
    }

    assert!(
        y.exp(&v.neg()).mul(&y_prime).equals(&g.exp(&k_x)),
        "y^-v . y' = g^k_x must hold"
    );
    for w in 0..W {
        assert!(
            b[w].exp(&v).mul(&b_prime[w]).equals(&a[w].exp(&k_x)),
            "B^v . B' = A^k_x must hold for component {w}"
        );
    }
}
