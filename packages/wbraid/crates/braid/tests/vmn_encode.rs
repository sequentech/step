// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Does braid's own P-256 arithmetic encode to the bytes Verificatum writes?
//!
//! `vcompat` is already validated against a real VMN corpus, but only as a
//! format library — it never touched a vsc group element. These tests close that
//! link: they take elements produced by braid's cryptography and check the
//! resulting byte trees against VMN's own output.
//!
//! Corpus-backed checks use the in-repo reference corpus (`testdata/verificatum/`,
//! overridable with `VCOMPAT_CORPUS`); the rest are self-contained.

#![cfg(feature = "native")]

use std::path::PathBuf;

use braid::vmn::encode;
use cryptography::context::{Context, P256Ctx};
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::groups::p256::element::P256Element;
use cryptography::traits::groups::{CryptographicGroup, GroupElement};
use vcompat::bytetree::ByteTree;

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

#[test]
fn generator_encodes_to_verificatum_bytes() {
    // vcompat's hardcoded P-256 generator was checked against VMN's output;
    // braid's group arithmetic must agree with it independently.
    let from_vsc = encode::element_to_tree(&P256Element::generator()).unwrap();
    let from_vcompat = vcompat::marshal::p256::generator();
    assert_eq!(
        from_vsc, from_vcompat,
        "vsc's generator must encode exactly as vcompat's"
    );
}

#[test]
fn elements_round_trip_through_byte_trees() {
    let mut rng = P256Ctx::get_rng();
    for _ in 0..20 {
        let element = <P256Ctx as Context>::G::random_element(&mut rng);
        let tree = encode::element_to_tree(&element).unwrap();
        let back = encode::tree_to_element(&tree).unwrap();
        assert!(element.equals(&back), "element round-trip must be exact");
        assert_eq!(encode::element_to_tree(&back).unwrap(), tree);
    }
}

#[test]
fn identity_maps_to_the_point_at_infinity() {
    // VMNV §6.5 encodes the point at infinity as node(leaf(-1), leaf(-1)).
    let identity = P256Element::one();
    let tree = encode::element_to_tree(&identity).unwrap();
    let coords = tree.as_node_of(2).unwrap();
    assert!(coords[0].as_leaf().unwrap().iter().all(|b| *b == 0xFF));
    assert!(encode::tree_to_element(&tree).unwrap().is_identity());
}

#[test]
fn ciphertext_arrays_are_transposed_not_listed() {
    // The §6.6 trap: N ciphertexts of width W serialize as two sides, each of W
    // arrays of N components -- not N tuples of W.
    const W: usize = 2;
    const N: usize = 4;
    let keypair: KeyPair<P256Ctx> = KeyPair::generate();
    let ciphertexts: Vec<_> = (0..N)
        .map(|_| {
            let m: [<P256Ctx as Context>::Element; W] =
                std::array::from_fn(|_| P256Ctx::random_element());
            keypair.encrypt(&m)
        })
        .collect();

    let tree = encode::ciphertexts_to_tree(&ciphertexts).unwrap();
    let sides = tree.as_node_of(2).expect("(u, v)");
    for side in sides {
        let components = side.as_node_of(W).expect("W component arrays");
        for component in components {
            assert_eq!(component.as_node().unwrap().len(), N, "N entries per component");
        }
    }
}

/// The strongest link in the chain: parse VMN's own `FullPublicKey.bt`, decode
/// its `y` into a vsc element, re-encode it, and require the bytes back.
#[test]
fn corpus_public_key_decodes_into_vsc_and_back() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory");
        return;
    };
    let original = std::fs::read(dir.join("FullPublicKey.bt")).unwrap();
    let tree = ByteTree::from_bytes(&original).unwrap();
    let parts = tree.as_node_of(2).unwrap();

    // The g half must equal braid's own generator.
    let g = encode::tree_to_element(&parts[0]).unwrap();
    assert!(
        g.equals(&P256Element::generator()),
        "VMN's g must decode to braid's generator"
    );

    // And a full re-encode must reproduce the file byte for byte.
    let y = encode::tree_to_element(&parts[1]).unwrap();
    let rebuilt = encode::public_key_to_tree(&y).unwrap();
    assert_eq!(
        rebuilt.to_bytes(),
        original,
        "re-encoding VMN's public key through vsc must be byte-identical"
    );
}

/// Every group element in the corpus must survive a decode/encode round trip
/// through vsc's arithmetic.
#[test]
fn corpus_generators_round_trip_through_vsc() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory");
        return;
    };
    let original = std::fs::read(dir.join("proofs/PermutationCommitment01.bt")).unwrap();
    let tree = ByteTree::from_bytes(&original).unwrap();

    let elements = encode::tree_to_elements(&tree).expect("decode commitments");
    assert!(!elements.is_empty());
    let rebuilt = encode::elements_to_tree(&elements).unwrap();
    assert_eq!(
        rebuilt.to_bytes(),
        original,
        "permutation commitment must survive a vsc round trip byte-identically"
    );
}
