// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Cross-implementation verification of a proof of a shuffle.
//!
//! braid and Verificatum implement the same Terelius–Wikström proof but derive
//! their Fiat–Shamir challenges differently. `vsvmn::challenges` supplies
//! Verificatum's derivation to vsc's prover/verifier through the
//! `ShuffleChallenges` seam. If that is right, then **braid's verifier should
//! accept a proof Verificatum produced** — algebra, encoding and transcript all
//! agreeing at once, with no proof-directory writer needed.
//!
//! That is what this test does: it reads a real VMN shuffle proof off disk,
//! rebuilds it as vsc types, and verifies it with vsc's own code.
//!
//! Runs against the in-repo reference corpus (`testdata/verificatum/`), or set
//! `VCOMPAT_CORPUS` to point at a freshly generated one.


use std::path::PathBuf;

use vsvmn::{challenges::VmnChallenges, encode, generators::vmn_generators};
use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::PublicKey;
use cryptography::zkp::shuffle::{Responses, ShuffleCommitments, ShuffleProof, Shuffler};
use vsvmn::wire::bytetree::ByteTree;
use vsvmn::wire::crypto::{global_prefix, Hashfunction, PrefixParams};

const W: usize = 2;
const N_R: usize = 100;
const N_E: usize = 256;
const N_V: usize = 256;

/// `<pgroup>` from the reference `protInfo.xml`, verbatim.
const PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";

/// The reference proof directory: the in-repo corpus by default, overridable
/// with `VCOMPAT_CORPUS`. See `testdata/verificatum/README.md`.
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("VCOMPAT_CORPUS") {
        let path = PathBuf::from(raw);
        assert!(path.is_dir(), "VCOMPAT_CORPUS is not a directory");
        return Some(path);
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum/nizkp");
    path.is_dir().then_some(path)
}

fn reference_rho() -> Vec<u8> {
    global_prefix(
        Hashfunction::Sha256,
        &PrefixParams {
            version: "3.1.0".into(),
            sid: "braidpoc".into(),
            auxsid: "default".into(),
            n_r: N_R as u32,
            n_v: N_V as u32,
            n_e: N_E as u32,
            prg: "SHA-256".into(),
            pgroup: PGROUP.into(),
            rohash: "SHA-256".into(),
        },
    )
}

fn read_tree(dir: &PathBuf, name: &str) -> ByteTree {
    ByteTree::from_bytes(&std::fs::read(dir.join(name)).expect("read corpus file"))
        .expect("parse byte tree")
}

/// **The interop result.** Verify Verificatum's own proof of a shuffle using
/// braid's cryptography.
#[test]
fn braid_verifies_a_verificatum_shuffle_proof() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory");
        return;
    };

    // --- statement -------------------------------------------------------
    let pk_tree = read_tree(&dir, "FullPublicKey.bt");
    let y = encode::tree_to_element(&pk_tree.as_node_of(2).unwrap()[1]).expect("decode y");
    let pk = PublicKey::<P256Ctx>::new(y);

    let w = encode::tree_to_ciphertexts::<W>(&read_tree(&dir, "Ciphertexts.bt"))
        .expect("decode input ciphertexts");
    let w_prime =
        encode::tree_to_ciphertexts::<W>(&read_tree(&dir, "proofs/Ciphertexts01.bt"))
            .expect("decode shuffled ciphertexts");
    assert_eq!(w.len(), w_prime.len());

    // --- proof -----------------------------------------------------------
    let u_n = encode::tree_to_elements(&read_tree(&dir, "proofs/PermutationCommitment01.bt"))
        .expect("decode permutation commitment");

    let tau = read_tree(&dir, "proofs/PoSCommitment01.bt");
    let tau = tau.as_node_of(6).expect("tau^pos has 6 components");
    let commitments = ShuffleCommitments::<P256Ctx, W>::new(
        encode::tree_to_elements(&tau[0]).expect("B"),
        encode::tree_to_element(&tau[1]).expect("A'"),
        encode::tree_to_elements(&tau[2]).expect("B'"),
        encode::tree_to_element(&tau[3]).expect("C'"),
        encode::tree_to_element(&tau[4]).expect("D'"),
        encode::tree_to_ciphertexts::<W>(&ByteTree::node(vec![
            // F' is a single ciphertext; reuse the array decoder by wrapping it
            // in the transposed one-element form.
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
        .remove(0),
        u_n,
    );

    let sigma = read_tree(&dir, "proofs/PoSReply01.bt");
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

    let proof = ShuffleProof::new(commitments, responses);

    // --- verify ----------------------------------------------------------
    let rho = reference_rho();
    let generators =
        vmn_generators(Hashfunction::Sha256, &rho, N_R, w.len()).expect("derive generators");

    let shuffler = Shuffler::<P256Ctx, W>::new(generators, pk);
    let challenges = VmnChallenges::new(Hashfunction::Sha256, rho, N_E, N_V, W);

    let ok = shuffler
        .verify_with(&w, &w_prime, &proof, &[], &challenges)
        .expect("verification must not error");

    assert!(
        ok,
        "braid must accept a proof of a shuffle produced by Verificatum"
    );
    eprintln!("braid verified a Verificatum-produced shuffle proof (N={})", w.len());

    // --- negative controls ------------------------------------------------
    // A test that cannot fail proves nothing, so check that the verifier is
    // actually discriminating: each of these perturbs exactly one input that
    // the transcript commits to, and each must be rejected.

    // 1. Generators derived under a different prefix.
    let wrong_rho = {
        let mut r = reference_rho();
        r[0] ^= 0x01;
        r
    };
    let wrong_generators =
        vmn_generators(Hashfunction::Sha256, &wrong_rho, N_R, w.len()).unwrap();
    let wrong_shuffler =
        Shuffler::<P256Ctx, W>::new(wrong_generators, PublicKey::<P256Ctx>::new(y));
    let challenges_right_rho = VmnChallenges::new(Hashfunction::Sha256, reference_rho(), N_E, N_V, W);
    assert!(
        !wrong_shuffler
            .verify_with(&w, &w_prime, &proof, &[], &challenges_right_rho)
            .unwrap_or(false),
        "wrong independent generators must be rejected"
    );

    // 2. Correct generators, but a prefix that does not match the proof.
    let generators2 =
        vmn_generators(Hashfunction::Sha256, &reference_rho(), N_R, w.len()).unwrap();
    let shuffler2 = Shuffler::<P256Ctx, W>::new(generators2, PublicKey::<P256Ctx>::new(y));
    let challenges_wrong_rho = VmnChallenges::new(Hashfunction::Sha256, wrong_rho, N_E, N_V, W);
    assert!(
        !shuffler2
            .verify_with(&w, &w_prime, &proof, &[], &challenges_wrong_rho)
            .unwrap_or(false),
        "a mismatched random-oracle prefix must be rejected"
    );

    // 3. The output ciphertexts swapped, so the claimed permutation is wrong.
    let mut swapped = w_prime.clone();
    swapped.swap(0, 1);
    let generators3 =
        vmn_generators(Hashfunction::Sha256, &reference_rho(), N_R, w.len()).unwrap();
    let shuffler3 = Shuffler::<P256Ctx, W>::new(generators3, PublicKey::<P256Ctx>::new(y));
    let challenges_for_swap = VmnChallenges::new(Hashfunction::Sha256, reference_rho(), N_E, N_V, W);
    assert!(
        !shuffler3
            .verify_with(&w, &swapped, &proof, &[], &challenges_for_swap)
            .unwrap_or(false),
        "tampered output ciphertexts must be rejected"
    );

    eprintln!("negative controls rejected as expected");
}
