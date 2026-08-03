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
use vsvmn::decrypt::BatchedDecryptionProof;
use vsvmn::verify::{verify_decryption, PartyContribution, SessionParams};
use cryptography::groups::p256::element::P256Element;
use cryptography::traits::groups::{GroupElement, GroupScalar};

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

// -------------------------------------------------------------------------
// Decryption
// -------------------------------------------------------------------------

/// Read one party's contribution out of a proof directory.
fn read_contribution(dir: &PathBuf, party: usize) -> (Vec<[P256Element; W]>, BatchedDecryptionProof<W>) {
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

/// **The other half of the interop result.** Verify Verificatum's own proof of
/// correct decryption using vsc's cryptography, and check that the plaintexts it
/// implies are the ones the proof publishes.
///
/// Until this existed the decryption interop ran one way only: `vmnv` accepted
/// proofs we emitted, but nothing here checked a proof Verificatum produced. The
/// transcript was already pinned by `corpus_roundtrip::decryption_transcript_matches_vmn`
/// — that reproduces `Dec.s` and `Dec.v` — but reproducing a transcript is not
/// verifying a proof; the verification equations were never evaluated against
/// real VMN output.
///
/// The corpus is a single-party session, so `k = lambda = 1`, `alpha = 1` and
/// `c_1 = 1`: the Lagrange combination is the identity and is *not* exercised
/// here. `vmn_decrypt::our_verifier_accepts_a_three_party_decryption_with_an_inactive_party`
/// covers that, against a construction `vmnv` also accepts.
#[test]
fn braid_verifies_a_verificatum_decryption_proof() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory");
        return;
    };

    // The list decryption was applied to is the final mixer's output, not the
    // session input.
    let mixed = encode::tree_to_ciphertexts::<W>(&read_tree(&dir, "proofs/Ciphertexts01.bt"))
        .expect("decode the mixed ciphertexts");
    let gamma = encode::tree_to_elements(&read_tree(&dir, "proofs/PolynomialInExponent.bt"))
        .expect("decode the polynomial in the exponent");

    // Algorithm 24's cross-check: the polynomial's constant term is the joint key.
    let pk_tree = read_tree(&dir, "FullPublicKey.bt");
    let y = encode::tree_to_element(&pk_tree.as_node_of(2).unwrap()[1]).expect("decode y");
    assert!(gamma[0].equals(&y), "Gamma_0 must be the joint public key");

    let correct = vsvmn::wire::arithm::bool_array_values(&read_tree(&dir, "proofs/CorrectIndices.bt"))
        .expect("decode CorrectIndices");
    let parties = correct.len() - 1;

    let contributions: Vec<_> = (1..=parties).map(|l| read_contribution(&dir, l)).collect();
    let contributions: Vec<PartyContribution<W>> = contributions
        .iter()
        .map(|(factors, proof)| PartyContribution { factors, proof })
        .collect();

    let params = SessionParams {
        rho: reference_rho(),
        hash: Hashfunction::Sha256,
        n_e: N_E,
        n_v: N_V,
        parties,
        threshold: gamma.len(),
    };

    let plaintexts = verify_decryption(&params, &gamma, &mixed, &contributions, &correct)
        .expect("the statement must be well formed")
        .expect("Verificatum's decryption proof must verify");

    // And the plaintexts it implies must be the ones the proof publishes --
    // Algorithm 28 keeps these separate, and so does vmnv.
    let published = encode::tree_to_component_array::<W>(&read_tree(&dir, "Plaintexts.bt"))
        .expect("decode the published plaintexts");
    assert_eq!(
        plaintexts, published,
        "the computed plaintexts must match Plaintexts.bt"
    );
}

/// The result above is only meaningful if a wrong proof is rejected, so corrupt
/// each of the three published pieces in turn.
#[test]
fn a_tampered_verificatum_decryption_proof_is_rejected() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory");
        return;
    };

    let mixed = encode::tree_to_ciphertexts::<W>(&read_tree(&dir, "proofs/Ciphertexts01.bt"))
        .expect("decode the mixed ciphertexts");
    let gamma = encode::tree_to_elements(&read_tree(&dir, "proofs/PolynomialInExponent.bt"))
        .expect("decode gamma");
    let correct =
        vsvmn::wire::arithm::bool_array_values(&read_tree(&dir, "proofs/CorrectIndices.bt"))
            .expect("decode CorrectIndices");
    let parties = correct.len() - 1;
    let params = SessionParams {
        rho: reference_rho(),
        hash: Hashfunction::Sha256,
        n_e: N_E,
        n_v: N_V,
        parties,
        threshold: gamma.len(),
    };

    for case in ["factor", "commitment", "reply"] {
        let (mut factors, mut proof) = read_contribution(&dir, 1);
        match case {
            // A different but well-formed group element: the proof must fail,
            // not merely fail to parse.
            "factor" => factors[0][0] = factors[0][0].mul(&P256Element::generator()),
            "commitment" => proof.y_prime = proof.y_prime.mul(&P256Element::generator()),
            "reply" => proof.k_x = proof.k_x.add(&cryptography::groups::p256::scalar::P256Scalar::one()),
            _ => unreachable!(),
        }

        let contributions = vec![PartyContribution::<W> {
            factors: &factors,
            proof: &proof,
        }];
        let verdict = verify_decryption(&params, &gamma, &mixed, &contributions, &correct)
            .expect("the statement is still well formed");
        assert!(
            verdict.is_none(),
            "a proof with a corrupted {case} must be rejected"
        );
    }
}

// -------------------------------------------------------------------------
// A genuine multi-party Verificatum session
// -------------------------------------------------------------------------

/// The three-party corpus: `k = 3`, `λ = 2`, produced by VMN's own demo.
fn multiparty_corpus() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/verificatum-3party");
    dir.join("nizkp").is_dir().then_some(dir)
}

/// **Cross-implementation verification of a multi-party decryption.**
///
/// The single-party corpus cannot exercise the parts of Algorithm 22 that
/// matter most: with `k = λ = 1` the modified Lagrange coefficients are all `1`,
/// `α = 1`, and Δ is everything. Here `α = lcm(1,2,3)² = 36`, the coefficients
/// over Δ are `72` and `−36`, and the seed commits to all three parties'
/// factors while only two are combined.
///
/// # What this does *not* establish
///
/// Not the "Δ is the first λ true flags" rule, despite three parties being
/// marked correct against a threshold of two. All three published *valid*
/// factors, so any 2-subset interpolates to the same `u^{−x}` — Δ = {2,3} gives
/// coefficients `108, −72` and reconstructs exactly as well as {1,2} does.
/// Verified by mutation: selecting the *last* λ instead of the first leaves this
/// test passing. Δ selection only becomes observable when an excluded party's
/// factors are absent or all-identity, which needs a corpus generated with
/// `./sact` restricting the active set.
///
/// Nothing here is hardcoded: every session parameter, including `sid`, comes
/// from this corpus's own `protInfo.xml`. Before the reader existed this proof
/// could not have been checked at all, because our constants said `braidpoc` and
/// the demo's say `MyDemo`, and ρ commits to it.
#[test]
fn braid_verifies_a_multiparty_verificatum_decryption_proof() {
    let Some(root) = multiparty_corpus() else {
        eprintln!("skipping: no three-party corpus");
        return;
    };
    let dir = root.join("nizkp");

    let xml = std::fs::read_to_string(root.join("protInfo.xml")).expect("read protInfo.xml");
    let info = vsvmn::wire::protinfo::ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    assert!(info.is_consistent());
    assert_eq!(info.parties, 3, "k");
    assert_eq!(info.threshold, 2, "lambda");

    let auxsid = std::fs::read_to_string(dir.join("auxsid")).expect("read auxsid");
    let rho = global_prefix(Hashfunction::Sha256, &info.prefix_params(auxsid.trim()));

    // The decryption input is the last mixer's output.
    let mixers: usize = std::fs::read_to_string(dir.join("proofs/activethreshold"))
        .expect("read activethreshold")
        .trim()
        .parse()
        .expect("activethreshold is a number");
    let mixed = encode::tree_to_ciphertexts::<W>(&read_tree(
        &dir,
        &format!("proofs/Ciphertexts{mixers:02}.bt"),
    ))
    .expect("decode the mixed ciphertexts");

    let gamma = encode::tree_to_elements(&read_tree(&dir, "proofs/PolynomialInExponent.bt"))
        .expect("decode gamma");
    let pk_tree = read_tree(&dir, "FullPublicKey.bt");
    let y = encode::tree_to_element(&pk_tree.as_node_of(2).unwrap()[1]).expect("decode y");
    assert!(gamma[0].equals(&y), "Gamma_0 must be the joint public key");

    let correct =
        vsvmn::wire::arithm::bool_array_values(&read_tree(&dir, "proofs/CorrectIndices.bt"))
            .expect("decode CorrectIndices");
    assert_eq!(
        correct.iter().skip(1).filter(|c| **c).count(),
        3,
        "VMN marks all three parties correct even though the threshold is two"
    );

    let contributions: Vec<_> = (1..=info.parties).map(|l| read_contribution(&dir, l)).collect();
    let contributions: Vec<PartyContribution<W>> = contributions
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

    let plaintexts = verify_decryption(&params, &gamma, &mixed, &contributions, &correct)
        .expect("the statement must be well formed")
        .expect("Verificatum's multi-party decryption proof must verify");

    let published = encode::tree_to_component_array::<W>(&read_tree(&dir, "Plaintexts.bt"))
        .expect("decode the published plaintexts");
    assert_eq!(
        plaintexts, published,
        "the computed plaintexts must match Plaintexts.bt"
    );
}

/// The three-party corpus with party 2 **genuinely inactive**: `./sact '{1,3}'`
/// before mixing, so VMN wrote its own placeholder decryption material.
fn inactive_party_corpus() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum-3party-inactive");
    dir.join("nizkp").is_dir().then_some(dir)
}

/// **The placeholder convention, checked against Verificatum rather than
/// against ourselves.**
///
/// A party that takes no part still occupies a slot: `DecryptionFactors<l>.bt`
/// holds an all-identity array of full size, `DecrFactCommitment<l>.bt` holds
/// `node(1, 1^ω)`, and `DecrFactReply<l>.bt` holds the zero scalar. We derived
/// that by reading `DistrElGamalSessionBasic`'s fallbacks, and every test of it
/// so far has been against our own construction of the same values — which
/// proves consistency, not correctness.
///
/// It matters because those values are *not* inert. Every party's factors are
/// hashed into the batching seed and every party's commitment into the
/// challenge, including the excluded one's. Get the placeholder wrong and `v`
/// moves, so the two *participating* parties' proofs fail. This corpus is the
/// first thing that can catch that.
#[test]
fn braid_verifies_a_decryption_with_a_genuinely_inactive_party() {
    let Some(root) = inactive_party_corpus() else {
        eprintln!("skipping: no inactive-party corpus");
        return;
    };
    let dir = root.join("nizkp");

    let xml = std::fs::read_to_string(root.join("protInfo.xml")).expect("read protInfo.xml");
    let info = vsvmn::wire::protinfo::ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    let auxsid = std::fs::read_to_string(dir.join("auxsid")).expect("read auxsid");
    let rho = global_prefix(Hashfunction::Sha256, &info.prefix_params(auxsid.trim()));

    let correct =
        vsvmn::wire::arithm::bool_array_values(&read_tree(&dir, "proofs/CorrectIndices.bt"))
            .expect("decode CorrectIndices");
    assert_eq!(correct.len(), info.parties + 1);
    assert!(
        !correct[2],
        "this corpus exists because party 2 sat out; CorrectIndices must say so"
    );

    // Party 2's array is present and full length even though it contributed
    // nothing -- the file cannot be omitted or shortened.
    let absent = encode::tree_to_component_array::<W>(&read_tree(
        &dir,
        "proofs/DecryptionFactors02.bt",
    ))
    .expect("decode the inactive party's factors");
    assert!(
        absent.iter().flatten().all(|e: &P256Element| e.is_identity()),
        "VMN writes an all-identity array for a party that took no part"
    );

    let mixers: usize = std::fs::read_to_string(dir.join("proofs/activethreshold"))
        .expect("read activethreshold")
        .trim()
        .parse()
        .expect("a number");
    let mixed = encode::tree_to_ciphertexts::<W>(&read_tree(
        &dir,
        &format!("proofs/Ciphertexts{mixers:02}.bt"),
    ))
    .expect("decode the mixed ciphertexts");
    let gamma = encode::tree_to_elements(&read_tree(&dir, "proofs/PolynomialInExponent.bt"))
        .expect("decode gamma");

    let contributions: Vec<_> = (1..=info.parties).map(|l| read_contribution(&dir, l)).collect();
    let contributions: Vec<PartyContribution<W>> = contributions
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

    let plaintexts = verify_decryption(&params, &gamma, &mixed, &contributions, &correct)
        .expect("well formed")
        .expect("a decryption with an inactive party must verify");

    let published = encode::tree_to_component_array::<W>(&read_tree(&dir, "Plaintexts.bt"))
        .expect("decode the published plaintexts");
    assert_eq!(plaintexts, published);
}
