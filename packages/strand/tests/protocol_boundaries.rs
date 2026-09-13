// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Small protocol examples make failures easy to inspect. Expected arithmetic
//! is written out, and every tampering test first verifies the original proof.

use strand::backend::ristretto::RistrettoCtx;
use strand::context::{Ctx, Element, Exponent};
use strand::elgamal::{Ciphertext, PrivateKey};
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::shuffler_product::{Shuffler, StrandRectangle};
use strand::zkp::Zkp;
use strand::{threshold, util};

type Scalar = <RistrettoCtx as Ctx>::X;
type Point = <RistrettoCtx as Ctx>::E;
const CONTEXT: &[u8] = b"fixture-election/contest";

#[test]
fn multiplying_exponential_ciphertexts_adds_the_encoded_votes() {
    let ctx = RistrettoCtx;
    let secret = PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
    let public = secret.get_pk();
    let first = public.encrypt_exponential(&ctx.exp_from_u64(3));
    let second = public.encrypt_exponential(&ctx.exp_from_u64(5));
    assert_eq!(
        secret.decrypt(&first.mul(&second)),
        ctx.gmod_pow(&ctx.exp_from_u64(8))
    );

    // Re-encryption changes the ciphertext while preserving its message.
    let rerandomized = first.mul(&public.one(&ctx.exp_from_u64(11)));
    assert_ne!(rerandomized, first);
    assert_eq!(
        secret.decrypt(&rerandomized),
        ctx.gmod_pow(&ctx.exp_from_u64(3))
    );
    assert_eq!(first.mhr(), &first.mhr);
    assert_eq!(first.gr(), &first.gr);
    assert_eq!(secret.pk_element(), public.element());
}

#[test]
fn encrypted_exponents_round_trip_and_require_two_ciphertexts() {
    let ctx = RistrettoCtx;
    let exponent = ctx.exp_from_u64(123456789);
    let secret = || PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
    let encoded = ctx.encrypt_exp(&exponent, secret().get_pk()).unwrap();
    assert_eq!(ctx.decrypt_exp(&encoded, secret()).unwrap(), exponent);
    let ciphertexts =
        Vec::<Ciphertext<RistrettoCtx>>::strand_deserialize(&encoded).unwrap();
    assert_eq!(ciphertexts.len(), 2);
    for length in [0, 1, 3] {
        let malformed = vec![ciphertexts[0].clone(); length]
            .strand_serialize()
            .unwrap();
        assert!(ctx.decrypt_exp(&malformed, secret()).is_err());
    }
    assert!(ctx
        .decrypt_exp(b"not an encoded ciphertext", secret())
        .is_err());
}

#[test]
fn group_and_scalar_decoders_enforce_canonical_fixed_width_values() {
    let ctx = RistrettoCtx;
    let point = ctx.gmod_pow(&ctx.exp_from_u64(7));
    let scalar = ctx.exp_from_u64(11);
    assert_eq!(
        ctx.element_from_bytes(&point.strand_serialize().unwrap())
            .unwrap(),
        point
    );
    assert_eq!(
        ctx.exp_from_bytes(&scalar.strand_serialize().unwrap())
            .unwrap(),
        scalar
    );
    for length in [0, 1, 31, 33, 64] {
        assert!(ctx.element_from_bytes(&vec![0; length]).is_err());
        assert!(ctx.exp_from_bytes(&vec![0; length]).is_err());
    }
    assert!(ctx.element_from_bytes(&[0xff; 32]).is_err());
    assert!(ctx.exp_from_bytes(&[0xff; 32]).is_err());
    assert!(util::to_hash_array(&[0; 63]).is_err());
    assert_eq!(util::to_hash_array(&[42; 64]).unwrap(), [42; 64]);
}

#[test]
fn scalar_and_group_operations_preserve_their_arithmetic_identities() {
    let ctx = RistrettoCtx;
    let three = ctx.exp_from_u64(3);
    let seven = ctx.exp_from_u64(7);
    assert_eq!(seven.sub(&three), ctx.exp_from_u64(4));
    assert_eq!(
        ctx.exp_sub_mod(&three, &seven).add(&seven).modq(&ctx),
        three
    );
    assert_eq!(seven.div(&seven, &three), Scalar::mul_identity());
    assert_eq!(
        seven.mul(&seven.inv(&three)).modq(&ctx),
        Scalar::mul_identity()
    );
    assert_eq!(ctx.exp_modulo(&seven), seven);

    let point = ctx.gmod_pow(&seven);
    let generator = ctx.generator();
    assert_eq!(point.div(&point, generator), Point::mul_identity());
    assert_eq!(point.mul(&point.inv(generator)), Point::mul_identity());
    assert_eq!(
        point.mod_pow(&three, generator),
        ctx.gmod_pow(&ctx.exp_from_u64(21))
    );
    assert_eq!(ctx.modulo(&point), point);
    assert_eq!(point.modulo(generator), point);
}

#[test]
fn schnorr_and_equality_proofs_bind_the_statement_and_context() {
    let ctx = RistrettoCtx;
    let zkp = Zkp::new(&ctx);
    let secret = ctx.exp_from_u64(7);
    let public = ctx.gmod_pow(&secret);
    let proof = zkp.schnorr_prove(&secret, &public, None, CONTEXT).unwrap();
    assert!(zkp.schnorr_verify(&public, None, &proof, CONTEXT));
    assert!(!zkp.schnorr_verify(&public, None, &proof, b"other election"));
    assert!(!zkp.schnorr_verify(
        &ctx.gmod_pow(&ctx.exp_from_u64(8)),
        None,
        &proof,
        CONTEXT
    ));

    let second_base = ctx.gmod_pow(&ctx.exp_from_u64(11));
    let second_public = ctx.emod_pow(&second_base, &secret);
    let equality = zkp
        .cp_prove(
            &secret,
            &public,
            &second_public,
            None,
            &second_base,
            CONTEXT,
        )
        .unwrap();
    assert!(zkp.cp_verify(
        &public,
        &second_public,
        None,
        &second_base,
        &equality,
        CONTEXT
    ));
    assert!(!zkp.cp_verify(
        &public,
        &second_public,
        None,
        &second_base,
        &equality,
        b"other election"
    ));
    assert!(!zkp.cp_verify(
        &public,
        &public,
        None,
        &second_base,
        &equality,
        CONTEXT
    ));
}

#[test]
fn decryption_proofs_reject_a_changed_factor_ciphertext_or_context() {
    let ctx = RistrettoCtx;
    let zkp = Zkp::new(&ctx);
    let share = ctx.exp_from_u64(7);
    let secret = PrivateKey::from(&share, &ctx);
    let public = secret.get_pk();
    let plaintext = ctx.gmod_pow(&ctx.exp_from_u64(3));
    let ciphertext = public.encrypt(&plaintext);
    let (factor, proof) = threshold::decryption_factor(
        &ciphertext,
        &share,
        public.element(),
        CONTEXT,
        &zkp,
        &ctx,
    )
    .unwrap();
    assert_eq!(factor, secret.decryption_factor(&ciphertext));
    assert!(threshold::verify_decryption_factor(
        &ciphertext,
        public.element(),
        &factor,
        &proof,
        CONTEXT,
        &zkp
    )
    .unwrap());
    assert!(!threshold::verify_decryption_factor(
        &ciphertext,
        public.element(),
        &Point::mul_identity(),
        &proof,
        CONTEXT,
        &zkp
    )
    .unwrap());
    assert!(!threshold::verify_decryption_factor(
        &ciphertext,
        public.element(),
        &factor,
        &proof,
        b"other election",
        &zkp
    )
    .unwrap());
    let mut changed = ciphertext.clone();
    changed.mhr = changed.mhr.mul(ctx.generator());
    assert!(!threshold::verify_decryption_factor(
        &changed,
        public.element(),
        &factor,
        &proof,
        CONTEXT,
        &zkp
    )
    .unwrap());
    let (decrypted, direct_proof) =
        secret.decrypt_and_prove(&ciphertext, CONTEXT).unwrap();
    assert_eq!(decrypted, plaintext);
    assert!(threshold::verify_decryption_factor(
        &ciphertext,
        public.element(),
        &factor,
        &direct_proof,
        CONTEXT,
        &zkp
    )
    .unwrap());
}

#[test]
fn polynomial_shares_match_independent_values_and_reject_wrong_trustees() {
    let ctx = RistrettoCtx;
    // f(x) = 5 + 2x + 3x², evaluated at trustee coordinates 1, 2 and 3.
    let coefficients = vec![
        ctx.exp_from_u64(5),
        ctx.exp_from_u64(2),
        ctx.exp_from_u64(3),
    ];
    let commitments: Vec<_> = coefficients
        .iter()
        .map(|value| ctx.gmod_pow(value))
        .collect();
    let expected = [10, 21, 38];
    for (trustee, expected_share) in expected.into_iter().enumerate() {
        let share =
            threshold::compute_peer_share(trustee, 3, &coefficients, &ctx);
        assert_eq!(share, ctx.exp_from_u64(expected_share));
        let verification =
            threshold::verification_key_factor(&commitments, 3, trustee, &ctx);
        assert!(threshold::verify_share(&share, &verification, &ctx));
        assert!(!threshold::verify_share(
            &share.add(&Scalar::mul_identity()),
            &verification,
            &ctx
        ));
        let other_trustee = threshold::verification_key_factor(
            &commitments,
            3,
            trustee + 1,
            &ctx,
        );
        assert!(!threshold::verify_share(&share, &other_trustee, &ctx));
    }

    // Interpolation at zero must recover f(0), not one trustee's share.
    let recovered = [1, 2, 3]
        .into_iter()
        .zip(expected)
        .fold(Scalar::add_identity(), |sum, (trustee, share)| {
            sum.add(&ctx.exp_from_u64(share).mul(&threshold::lagrange(
                trustee,
                &[1, 2, 3],
                &ctx,
            )))
        })
        .modq(&ctx);
    assert_eq!(recovered, ctx.exp_from_u64(5));
}

#[test]
fn product_shuffle_proofs_bind_every_column_and_the_election_context() {
    let ctx = RistrettoCtx;
    let secret = PrivateKey::from(&ctx.exp_from_u64(7), &ctx);
    let public = secret.get_pk();
    let rows = [vec![1, 2], vec![3, 4], vec![5, 6]]
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|value| {
                    public.encrypt_exponential(&ctx.exp_from_u64(value))
                })
                .collect()
        })
        .collect();
    let original = StrandRectangle::new(rows).unwrap();
    let generators = ctx.generators(4, b"fixture generators").unwrap();
    let shuffler = Shuffler::new(&public, &generators, &ctx);
    let (shuffled, randomness, permutation) = shuffler.gen_shuffle(&original);
    let proof = shuffler
        .gen_proof(&original, &shuffled, randomness, &permutation, CONTEXT)
        .unwrap();
    assert!(shuffler
        .check_proof(&proof, &original, &shuffled, CONTEXT)
        .unwrap());
    assert!(!shuffler
        .check_proof(&proof, &original, &shuffled, b"other election")
        .unwrap());

    for column in 0..2 {
        let mut altered = shuffled.rows().clone();
        altered[0][column].mhr = altered[0][column].mhr.mul(ctx.generator());
        let altered = StrandRectangle::new(altered).unwrap();
        assert!(
            !shuffler
                .check_proof(&proof, &original, &altered, CONTEXT)
                .unwrap(),
            "accepted a changed ciphertext in column {column}"
        );
    }
}
