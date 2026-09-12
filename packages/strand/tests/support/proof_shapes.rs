// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Malformed proofs remain valid Rust/Borsh objects. Verification must check
//! their shape before indexing, including structures received from other trustees.

use crate::backend::ristretto::RistrettoCtx;
use crate::context::Ctx;
use crate::elgamal::PrivateKey;
use crate::serialization::{StrandDeserialize, StrandSerialize};
use crate::shuffler;
use crate::shuffler_product::{self, StrandRectangle};
use crate::util;

#[test]
fn single_shuffle_rejects_incomplete_proofs_and_mismatched_inputs() {
    type Proof = shuffler::ShuffleProof<RistrettoCtx>;
    let ctx = RistrettoCtx;
    let secret = PrivateKey::gen(&ctx);
    let public = secret.get_pk();
    let original = util::random_ciphertexts(3, &ctx);
    let generators = ctx.generators(4, b"fixture generators").unwrap();
    let shuffler = shuffler::Shuffler::new(&public, &ctx);
    let (shuffled, randomness, permutation) = shuffler.gen_shuffle(&original);
    let proof = shuffler
        .gen_proof(
            original.clone(),
            &shuffled,
            randomness,
            generators.clone(),
            permutation,
            b"fixture",
        )
        .unwrap();
    assert!(shuffler
        .check_proof(
            &proof,
            original.clone(),
            shuffled.clone(),
            generators.clone(),
            b"fixture"
        )
        .unwrap());

    let mutations: [(&str, fn(&mut Proof)); 6] = [
        ("missing permutation commitments", |p| p.cs.0.clear()),
        ("extra permutation commitment", |p| {
            p.cs.0.push(p.cs.0[0].clone())
        }),
        ("missing commitment chain", |p| p.c_hats.0.clear()),
        ("missing response chain", |p| p.s.s_hats.0.clear()),
        ("missing permutation responses", |p| p.s.s_primes.0.clear()),
        ("missing verification commitments", |p| p.t.t_hats.0.clear()),
    ];
    for (description, mutate) in mutations {
        let mut malformed = proof.clone();
        mutate(&mut malformed);
        // Serialization still succeeds: syntax validation alone cannot enforce
        // the relationship between proof dimensions and ciphertext count.
        let malformed =
            Proof::strand_deserialize(&malformed.strand_serialize().unwrap())
                .unwrap();
        assert!(
            shuffler
                .check_proof(
                    &malformed,
                    original.clone(),
                    shuffled.clone(),
                    generators.clone(),
                    b"fixture"
                )
                .is_err(),
            "{description}"
        );
    }
    for invalid_generators in [vec![], generators[..2].to_vec()] {
        assert!(shuffler
            .check_proof(
                &proof,
                original.clone(),
                shuffled.clone(),
                invalid_generators,
                b"fixture"
            )
            .is_err());
    }
    assert!(shuffler
        .check_proof(
            &proof,
            vec![],
            vec![],
            generators[..1].to_vec(),
            b"fixture"
        )
        .is_err());
    assert!(shuffler
        .check_proof(
            &proof,
            original.clone(),
            shuffled[..2].to_vec(),
            generators,
            b"fixture"
        )
        .is_err());
}

#[test]
fn product_shuffle_rejects_incomplete_proofs_rows_and_columns() {
    type Proof = shuffler_product::ShuffleProof<RistrettoCtx>;
    let ctx = RistrettoCtx;
    let secret = PrivateKey::gen(&ctx);
    let public = secret.get_pk();
    let original = util::random_product_ciphertexts(3, 2, &ctx);
    let generators = ctx.generators(4, b"fixture generators").unwrap();
    let shuffler = shuffler_product::Shuffler::new(&public, &generators, &ctx);
    let (shuffled, randomness, permutation) = shuffler.gen_shuffle(&original);
    let proof = shuffler
        .gen_proof(&original, &shuffled, randomness, &permutation, b"fixture")
        .unwrap();
    assert!(shuffler
        .check_proof(&proof, &original, &shuffled, b"fixture")
        .unwrap());

    let mutations: [(&str, fn(&mut Proof)); 9] = [
        ("missing permutation commitments", |p| p.cs.0.clear()),
        ("extra permutation commitment", |p| {
            p.cs.0.push(p.cs.0[0].clone())
        }),
        ("missing commitment chain", |p| p.c_hats.0.clear()),
        ("missing response chain", |p| p.s.s_hats.0.clear()),
        ("missing permutation responses", |p| p.s.s_primes.0.clear()),
        ("missing verification commitments", |p| p.t.t_hats.0.clear()),
        ("missing first column commitments", |p| p.t.t4_1s.clear()),
        ("missing second column commitments", |p| p.t.t4_2s.clear()),
        ("missing column responses", |p| p.s.s4s.clear()),
    ];
    for (description, mutate) in mutations {
        let mut malformed = proof.clone();
        mutate(&mut malformed);
        let malformed =
            Proof::strand_deserialize(&malformed.strand_serialize().unwrap())
                .unwrap();
        assert!(
            shuffler
                .check_proof(&malformed, &original, &shuffled, b"fixture")
                .is_err(),
            "{description}"
        );
    }
    for invalid_generators in [vec![], generators[..2].to_vec()] {
        let invalid =
            shuffler_product::Shuffler::new(&public, &invalid_generators, &ctx);
        assert!(invalid
            .check_proof(&proof, &original, &shuffled, b"fixture")
            .is_err());
    }
    let fewer_rows =
        StrandRectangle::new(shuffled.rows()[..2].to_vec()).unwrap();
    assert!(shuffler
        .check_proof(&proof, &original, &fewer_rows, b"fixture")
        .is_err());
    let fewer_columns = StrandRectangle::new(
        shuffled
            .rows()
            .iter()
            .map(|row| row[..1].to_vec())
            .collect(),
    )
    .unwrap();
    assert!(shuffler
        .check_proof(&proof, &original, &fewer_columns, b"fixture")
        .is_err());
    let empty = StrandRectangle::new_unchecked(vec![]);
    let empty_generators = generators[..1].to_vec();
    let empty_shuffler =
        shuffler_product::Shuffler::new(&public, &empty_generators, &ctx);
    assert!(empty_shuffler
        .check_proof(&proof, &empty, &empty, b"fixture")
        .is_err());
    let zero_width =
        StrandRectangle::new(vec![vec![], vec![], vec![]]).unwrap();
    assert!(shuffler
        .check_proof(&proof, &zero_width, &zero_width, b"fixture")
        .is_err());
}
