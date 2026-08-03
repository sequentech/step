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
use std::path::{Path, PathBuf};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::PublicKey;
use cryptography::groups::p256::element::P256Element;
use cryptography::traits::groups::GroupElement;
use cryptography::zkp::shuffle::Shuffler;
use vsvmn::verify::{verify_decryption, PartyContribution, SessionParams};
use vsvmn::wire::bytetree::ByteTree;
use vsvmn::wire::crypto::{global_prefix, Hashfunction};
use vsvmn::wire::protinfo::ProtocolInfo;
use vsvmn::{challenges::VmnChallenges, decrypt::BatchedDecryptionProof, encode,
            generators::vmn_generators};

fn read_tree(dir: &Path, name: &str) -> ByteTree {
    ByteTree::from_bytes(&std::fs::read(dir.join(name)).expect("read a proof file"))
        .expect("parse byte tree")
}

fn read_text(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name))
        .expect("read a proof file")
        .trim()
        .to_string()
}

/// Verify every proof in a generated corpus: each mixer's shuffle, then the
/// decryption, then that the plaintexts follow.
///
/// Width is a const generic, so the caller dispatches on the value read from the
/// protocol info file.
fn verify_corpus<const W: usize>(nizkp: &Path, info: &ProtocolInfo) {
    let rho = global_prefix(
        Hashfunction::Sha256,
        &info.prefix_params(&read_text(nizkp, "auxsid")),
    );

    let pk_tree = read_tree(nizkp, "FullPublicKey.bt");
    let y = encode::tree_to_element(&pk_tree.as_node_of(2).unwrap()[1]).expect("decode y");

    let mixers: usize = read_text(nizkp, "proofs/activethreshold")
        .parse()
        .expect("activethreshold is a number");
    assert!(
        mixers >= info.threshold,
        "lambda_a >= lambda by construction"
    );

    // --- the shuffle chain ------------------------------------------------
    let mut current = encode::tree_to_ciphertexts::<W>(&read_tree(nizkp, "Ciphertexts.bt"))
        .expect("decode the input ciphertexts");
    let generators = vmn_generators(Hashfunction::Sha256, &rho, info.n_r as usize, current.len())
        .expect("derive generators");

    // Mixer slots are numbered by *party index*, not sequentially, and a party
    // that took no part leaves a gap: with the active set {1,3} the directory
    // holds PermutationCommitment01 and 03 but no 02, while `activethreshold`
    // is 3 — the highest active index, not the count.
    //
    // `vmnv` decides a slot is active purely by whether its proof file exists
    // (`getPoSCActive`: `return file.exists()`), so a missing one is skipped in
    // silence rather than rejected. We match that to accept valid proofs, but
    // count what was actually checked instead of inheriting the silence.
    let mut verified = 0;
    for mixer in 1..=mixers {
        if !nizkp
            .join(format!("proofs/PermutationCommitment{mixer:02}.bt"))
            .is_file()
        {
            eprintln!("  mixer slot {mixer} has no proof; party took no part");
            continue;
        }

        let output = encode::tree_to_ciphertexts::<W>(&read_tree(
            nizkp,
            &format!("proofs/Ciphertexts{mixer:02}.bt"),
        ))
        .expect("decode a mixer's output");

        let proof = read_shuffle_proof::<W>(nizkp, mixer);
        let shuffler =
            Shuffler::<P256Ctx, W>::new(generators.clone(), PublicKey::<P256Ctx>::new(y));
        let challenges = VmnChallenges::new(
            Hashfunction::Sha256,
            rho.clone(),
            info.n_e as usize,
            info.n_v as usize,
            W,
        );
        assert!(
            shuffler
                .verify_with(&current, &output, &proof, &[], &challenges)
                .expect("shuffle verification must not error"),
            "mixer {mixer} of {mixers} must verify"
        );
        current = output;
        verified += 1;
    }

    // The point of skipping absent slots is to accept valid proofs, not to let
    // a chain shrink unnoticed. A session claiming a threshold must actually
    // have that many mixers behind its output.
    assert!(
        verified >= info.threshold,
        "only {verified} mixers were verified for a threshold of {}",
        info.threshold
    );

    // --- decryption --------------------------------------------------------
    let gamma = encode::tree_to_elements(&read_tree(nizkp, "proofs/PolynomialInExponent.bt"))
        .expect("decode gamma");
    assert!(gamma[0].equals(&y), "Gamma_0 must be the joint public key");

    let correct =
        vsvmn::wire::arithm::bool_array_values(&read_tree(nizkp, "proofs/CorrectIndices.bt"))
            .expect("decode CorrectIndices");

    let held: Vec<_> = (1..=info.parties).map(|l| read_contribution::<W>(nizkp, l)).collect();
    let contributions: Vec<PartyContribution<W>> = held
        .iter()
        .map(|(factors, proof)| PartyContribution { factors, proof })
        .collect();

    let params = SessionParams {
        rho,
        hash: Hashfunction::Sha256,
        n_e: info.n_e as usize,
        n_v: info.n_v as usize,
        parties: info.parties,
        threshold: info.threshold,
    };

    let plaintexts = verify_decryption(&params, &gamma, &current, &contributions, &correct)
        .expect("the statement must be well formed")
        .expect("the decryption proof must verify");

    let published = encode::tree_to_component_array::<W>(&read_tree(nizkp, "Plaintexts.bt"))
        .expect("decode the published plaintexts");
    assert_eq!(plaintexts, published, "plaintexts must match Plaintexts.bt");
}

fn read_contribution<const W: usize>(
    dir: &Path,
    party: usize,
) -> (Vec<[P256Element; W]>, BatchedDecryptionProof<W>) {
    let factors = encode::tree_to_component_array::<W>(&read_tree(
        dir,
        &format!("proofs/DecryptionFactors{party:02}.bt"),
    ))
    .expect("decode decryption factors");

    let tau = read_tree(dir, &format!("proofs/DecrFactCommitment{party:02}.bt"));
    let tau = tau.as_node_of(2).expect("tau^dec = node(y', B')");
    let proof = BatchedDecryptionProof::<W> {
        y_prime: encode::tree_to_element(&tau[0]).expect("y'"),
        b_prime: encode::tree_to_elements(&tau[1])
            .expect("B'")
            .try_into()
            .expect("B' has omega components"),
        k_x: encode::tree_to_scalar(&read_tree(
            dir,
            &format!("proofs/DecrFactReply{party:02}.bt"),
        ))
        .expect("k_x"),
    };
    (factors, proof)
}

fn read_shuffle_proof<const W: usize>(
    dir: &Path,
    mixer: usize,
) -> cryptography::zkp::shuffle::ShuffleProof<P256Ctx, W> {
    use cryptography::zkp::shuffle::{Responses, ShuffleCommitments, ShuffleProof};

    let u_n = encode::tree_to_elements(&read_tree(
        dir,
        &format!("proofs/PermutationCommitment{mixer:02}.bt"),
    ))
    .expect("decode the permutation commitment");

    let tau = read_tree(dir, &format!("proofs/PoSCommitment{mixer:02}.bt"));
    let tau = tau.as_node_of(6).expect("tau^pos has 6 components");
    // F' is a single ciphertext; wrap it in the transposed one-element form so
    // the array decoder can be reused.
    let f_prime = encode::tree_to_ciphertexts::<W>(&ByteTree::node(vec![
        ByteTree::node(
            tau[5].as_node_of(2).expect("F' = (u, v)")[0]
                .as_node()
                .expect("u components")
                .iter()
                .map(|c| ByteTree::node(vec![c.clone()]))
                .collect::<Vec<_>>(),
        ),
        ByteTree::node(
            tau[5].as_node_of(2).expect("F' = (u, v)")[1]
                .as_node()
                .expect("v components")
                .iter()
                .map(|c| ByteTree::node(vec![c.clone()]))
                .collect::<Vec<_>>(),
        ),
    ]))
    .expect("F'")
    .remove(0);

    let commitments = ShuffleCommitments::<P256Ctx, W>::new(
        encode::tree_to_elements(&tau[0]).expect("B"),
        encode::tree_to_element(&tau[1]).expect("A'"),
        encode::tree_to_elements(&tau[2]).expect("B'"),
        encode::tree_to_element(&tau[3]).expect("C'"),
        encode::tree_to_element(&tau[4]).expect("D'"),
        f_prime,
        u_n,
    );

    let sigma = read_tree(dir, &format!("proofs/PoSReply{mixer:02}.bt"));
    let sigma = sigma.as_node_of(6).expect("sigma^pos has 6 components");
    let k_f: [_; W] = encode::tree_to_scalars(&sigma[5])
        .expect("k_F")
        .try_into()
        .expect("k_F has omega entries");
    let responses = Responses::<P256Ctx, W>::new(
        encode::tree_to_scalar(&sigma[0]).expect("k_A"),
        encode::tree_to_scalars(&sigma[1]).expect("k_B"),
        encode::tree_to_scalar(&sigma[2]).expect("k_C"),
        encode::tree_to_scalar(&sigma[3]).expect("k_D"),
        encode::tree_to_scalars(&sigma[4]).expect("k_E"),
        k_f,
    );

    ShuffleProof::new(commitments, responses)
}

/// Generate a corpus for `shape` and verify every proof in it.
fn generate_and_verify(shape: Shape) -> bool {
    let Some(corpus) = common::generate(&shape) else {
        return false;
    };

    let xml = std::fs::read_to_string(&corpus.protinfo).expect("read protInfo.xml");
    let info = ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    assert!(info.is_consistent());
    assert_eq!(info.parties, shape.parties, "k must be what we asked for");
    assert_eq!(info.threshold, shape.threshold, "lambda likewise");

    // Width is a const generic; dispatch on the value the file declares rather
    // than on the one we asked for, so a mismatch shows up as a failure here
    // instead of being silently assumed.
    match info.width {
        1 => verify_corpus::<1>(&corpus.nizkp, &info),
        2 => verify_corpus::<2>(&corpus.nizkp, &info),
        3 => verify_corpus::<3>(&corpus.nizkp, &info),
        w => panic!("unsupported width {w}; add an arm"),
    }

    eprintln!(
        "verified a generated corpus: k={}, lambda={}, omega={}, N={}",
        info.parties, info.threshold, info.width, shape.ciphertexts
    );
    let _ = std::fs::remove_dir_all(corpus.nizkp.parent().unwrap_or(&PathBuf::new()));
    true
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
