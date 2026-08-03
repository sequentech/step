// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Cross-implementation verification against corpora generated on demand.
//!
//! The other ingest tests read fixed corpora out of `testdata/`, so they check
//! three shapes: one single-party session and two three-party ones. These
//! generate their own input by running Verificatum, so the shape is a parameter
//! rather than an artifact of whatever someone once produced by hand.
//!
//! The property being tested is the strong form of the interop claim:
//!
//! > for any session shape, whatever Verificatum produces, we verify.
//!
//! `#[ignore]` because it runs VMN's *prover*, which needs a JVM and a Unix host
//! — rather more than the verifier does; see `tests/common/mod.rs`. On Windows
//! it goes through WSL, and skips if that is unavailable.
//!
//! ```text
//! cargo test -p vsvmn --test vmn_generated -- --ignored --nocapture
//! ```

mod common;

use common::Shape;
use std::path::PathBuf;

use vsvmn::session;
use vsvmn::wire::protinfo::ProtocolInfo;

/// Generate a corpus for `shape` and verify every proof in it.
///
/// Returns `false` when Verificatum cannot be run here, so the caller skips
/// rather than failing.
fn generate_and_verify(shape: Shape) -> bool {
    let Some(corpus) = common::generate(&shape) else {
        return false;
    };

    let xml = std::fs::read_to_string(&corpus.protinfo).expect("read protInfo.xml");
    let info = ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    assert!(info.is_consistent());
    assert_eq!(info.parties, shape.parties, "k must be what we asked for");
    assert_eq!(info.threshold, shape.threshold, "lambda likewise");

    let meta = session::read_metadata(&corpus.nizkp).expect("read the proof metadata");
    assert_eq!(meta.width, shape.width, "omega likewise");

    // Width is a const generic, so dispatch on what the *proof* declares rather
    // than on what we asked for -- a mismatch then fails here instead of being
    // silently assumed.
    match meta.width {
        1 => check::<1>(&corpus.nizkp, &info, &meta),
        2 => check::<2>(&corpus.nizkp, &info, &meta),
        3 => check::<3>(&corpus.nizkp, &info, &meta),
        w => panic!("unsupported width {w}; add an arm"),
    }

    eprintln!(
        "verified a generated corpus: k={}, lambda={}, omega={}, N={}",
        info.parties, info.threshold, meta.width, shape.ciphertexts
    );
    let _ = std::fs::remove_dir_all(corpus.nizkp.parent().unwrap_or(&PathBuf::new()));
    true
}

fn check<const W: usize>(nizkp: &std::path::Path, info: &ProtocolInfo, meta: &session::ProofMetadata) {
    let outcome = session::verify_session::<W>(nizkp, info, meta)
        .expect("the directory must be well formed")
        .expect("every proof in a generated session must verify");

    // Skipping absent mixer slots is what lets a valid proof through; it must
    // not let a shorter chain through unnoticed.
    assert!(
        outcome.mixers_verified >= info.threshold,
        "only {} mixers were verified for a threshold of {}",
        outcome.mixers_verified,
        info.threshold
    );
    assert!(
        outcome.plaintexts.is_some(),
        "a mixing session must yield plaintexts"
    );
}

/// **Whatever Verificatum produces, we verify** — over shapes we choose rather
/// than shapes we happen to have artifacts for.
#[test]
#[ignore = "runs VMN's prover: needs a JVM and a Unix host (WSL on Windows)"]
fn we_verify_generated_corpora_across_shapes() {
    let shapes = [
        // The degenerate case, where alpha = 1 and the combination is trivial.
        Shape::new(1, 1, 2, 4),
        // Threshold below the party count: alpha and the coefficients bite.
        Shape::new(3, 2, 2, 4),
        // A width we have no checked-in corpus for at all.
        Shape::new(2, 2, 3, 4),
        // A party that takes no part, so VMN writes its placeholder material
        // *and* leaves a gap in the mixer slots.
        Shape::new(3, 2, 2, 4).with_active(vec![1, 3]),
    ];

    let mut ran = 0;
    for shape in shapes {
        if generate_and_verify(shape) {
            ran += 1;
        } else {
            eprintln!("skipping: Verificatum's prover cannot be run here");
            return;
        }
    }
    assert!(ran > 0, "no corpus was generated");
    eprintln!("verified {ran} generated corpora");
}

/// **Negative control.** A test that cannot fail proves nothing, so corrupt each
/// published piece of a real Verificatum decryption in turn and require
/// rejection.
///
/// Corruption is by *group multiplication*, not by flipping bytes: the result
/// must still be a well-formed element, or we would only be testing that the
/// decoder rejects garbage rather than that the verifier rejects a wrong proof.
#[test]
#[ignore = "runs VMN's prover: needs a JVM and a Unix host (WSL on Windows)"]
fn a_tampered_verificatum_proof_is_rejected() {
    use cryptography::groups::p256::element::P256Element;
    use cryptography::groups::p256::scalar::P256Scalar;
    use cryptography::traits::groups::{GroupElement, GroupScalar};
    use vsvmn::verify::{verify_decryption, PartyContribution, SessionParams};

    let Some(corpus) = common::shared() else {
        return common::skip("Verificatum is unavailable");
    };
    let dir = &corpus.nizkp;

    let xml = std::fs::read_to_string(&corpus.protinfo).expect("read protInfo.xml");
    let info = ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    let meta = session::read_metadata(dir).expect("read the metadata");

    let rho = vsvmn::wire::crypto::global_prefix(
        vsvmn::wire::crypto::Hashfunction::Sha256,
        &info.prefix_params(&meta.auxsid),
    );
    let params = SessionParams {
        rho,
        hash: vsvmn::wire::crypto::Hashfunction::Sha256,
        n_e: info.n_e as usize,
        n_v: info.n_v as usize,
        parties: info.parties,
        threshold: info.threshold,
    };

    let read = |name: &str| {
        vsvmn::wire::bytetree::ByteTree::from_bytes(
            &std::fs::read(dir.join(name)).expect("read a proof file"),
        )
        .expect("parse byte tree")
    };
    let gamma = vsvmn::encode::tree_to_elements(&read("proofs/PolynomialInExponent.bt")).unwrap();
    let correct =
        vsvmn::wire::arithm::bool_array_values(&read("proofs/CorrectIndices.bt")).unwrap();
    let mixed = vsvmn::encode::tree_to_ciphertexts::<2>(&read(&format!(
        "proofs/Ciphertexts{:02}.bt",
        meta.active_threshold
    )))
    .unwrap();

    // Party 1 is in Delta for every shape we generate, so tampering with it is
    // guaranteed to reach the combination.
    for case in ["factor", "commitment", "reply"] {
        let mut held: Vec<_> = (1..=info.parties)
            .map(|l| read_party::<2>(dir, l))
            .collect();
        match case {
            "factor" => {
                held[0].0[0][0] = held[0].0[0][0].mul(&P256Element::generator());
            }
            "commitment" => {
                held[0].1.y_prime = held[0].1.y_prime.mul(&P256Element::generator());
            }
            "reply" => held[0].1.k_x = held[0].1.k_x.add(&P256Scalar::one()),
            _ => unreachable!(),
        }

        let contributions: Vec<PartyContribution<2>> = held
            .iter()
            .map(|(factors, proof)| PartyContribution { factors, proof })
            .collect();
        let verdict = verify_decryption(&params, &gamma, &mixed, &contributions, &correct)
            .expect("the statement is still well formed");
        assert!(
            verdict.is_none(),
            "a proof with a corrupted {case} must be rejected"
        );
    }
}

/// **The inactive party's placeholder material, byte for byte.**
///
/// A party that takes no part still occupies a slot, and what fills it is not
/// inert: the seed commits to every party's factors and the challenge to every
/// party's commitment, including the excluded one's. Get it wrong and the
/// *participating* parties' proofs fail.
///
/// Verifying a session with an inactive party already exercises that
/// indirectly. This asserts the values themselves, so a change in the
/// convention is reported as such rather than as an unexplained challenge
/// mismatch somewhere downstream.
#[test]
#[ignore = "runs VMN's prover: needs a JVM and a Unix host (WSL on Windows)"]
fn an_inactive_party_gets_identity_factors_and_a_zero_reply() {
    use cryptography::traits::groups::{GroupElement, GroupScalar};

    let shape = Shape::new(3, 2, 2, 4).with_active(vec![1, 3]);
    let Some(corpus) = common::generate(&shape) else {
        return common::skip("Verificatum is unavailable");
    };
    let dir = &corpus.nizkp;

    let correct = vsvmn::wire::arithm::bool_array_values(
        &vsvmn::wire::bytetree::ByteTree::from_bytes(
            &std::fs::read(dir.join("proofs/CorrectIndices.bt")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(!correct[2], "party 2 sat out, so CorrectIndices must say so");

    let (factors, proof) = read_party::<2>(dir, 2);
    assert!(
        factors.iter().flatten().all(|e| e.is_identity()),
        "an absent party's factors are the all-identity array"
    );
    assert!(
        proof.y_prime.is_identity() && proof.b_prime.iter().all(|e| e.is_identity()),
        "and its commitment is node(1, 1^omega)"
    );
    assert!(
        proof.k_x.equals(&cryptography::groups::p256::scalar::P256Scalar::zero()),
        "and its reply is the zero scalar"
    );

    // The array is present at full length, not omitted or shortened: the
    // verifier's readArray halts on a missing file.
    let active = read_party::<2>(dir, 1).0;
    assert_eq!(
        factors.len(),
        active.len(),
        "the placeholder array is as long as a real one"
    );

    let _ = std::fs::remove_dir_all(dir.parent().unwrap());
}

/// Read one party's published decryption contribution.
fn read_party<const W: usize>(
    dir: &std::path::Path,
    party: usize,
) -> (
    Vec<[cryptography::groups::p256::element::P256Element; W]>,
    vsvmn::decrypt::BatchedDecryptionProof<W>,
) {
    let read = |name: String| {
        vsvmn::wire::bytetree::ByteTree::from_bytes(
            &std::fs::read(dir.join(name)).expect("read a proof file"),
        )
        .expect("parse byte tree")
    };
    let factors = vsvmn::encode::tree_to_component_array::<W>(&read(format!(
        "proofs/DecryptionFactors{party:02}.bt"
    )))
    .expect("decode factors");
    let tau = read(format!("proofs/DecrFactCommitment{party:02}.bt"));
    let tau = tau.as_node_of(2).expect("node(y', B')");
    (
        factors,
        vsvmn::decrypt::BatchedDecryptionProof::<W> {
            y_prime: vsvmn::encode::tree_to_element(&tau[0]).unwrap(),
            b_prime: vsvmn::encode::tree_to_elements(&tau[1])
                .unwrap()
                .try_into()
                .expect("omega components"),
            k_x: vsvmn::encode::tree_to_scalar(&read(format!(
                "proofs/DecrFactReply{party:02}.bt"
            )))
            .unwrap(),
        },
    )
}
