// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Round-trip every byte tree in a real Verificatum proof directory.
//!
//! This is the acceptance test for Stage 1 of the plan in `VERIFICATUM.md`: we
//! must be able to parse VMN's own `.bt` files and re-emit them **byte for
//! byte**. Anything less means our encoder and VMN's disagree somewhere, which
//! would surface later as an unverifiable proof with no useful diagnostic.
//!
//! Runs against the in-repo reference corpus at `testdata/verificatum/nizkp`;
//! `VCOMPAT_CORPUS` points it at a different one. `testdata/verificatum/README.md`
//! documents how that corpus was generated and which constants below are pinned
//! to it.

use std::path::{Path, PathBuf};

use vcompat::arithm;
use vcompat::bytetree::ByteTree;
use vcompat::marshal;

/// The reference proof directory: the in-repo corpus by default, overridable
/// with `VCOMPAT_CORPUS` to point at a freshly generated one.
///
/// See `testdata/verificatum/README.md` for how the corpus was produced and
/// which constants in these tests are pinned to it.
fn corpus_dir() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("VCOMPAT_CORPUS") {
        let path = PathBuf::from(raw);
        assert!(
            path.is_dir(),
            "VCOMPAT_CORPUS is set but {} is not a directory",
            path.display()
        );
        return Some(path);
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/verificatum/nizkp");
    path.is_dir().then_some(path)
}

/// Every `.bt` file under `dir`, recursively.
fn byte_tree_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "bt").unwrap_or(false) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
fn every_corpus_byte_tree_roundtrips_exactly() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };

    let files = byte_tree_files(&dir);
    assert!(!files.is_empty(), "no .bt files under {}", dir.display());

    for path in &files {
        let original = std::fs::read(path).expect("read corpus file");
        let tree = ByteTree::from_bytes(&original)
            .unwrap_or_else(|e| panic!("{}: parse failed: {e}", path.display()));

        assert_eq!(
            tree.serialized_len(),
            original.len(),
            "{}: serialized_len disagrees with the file size",
            path.display()
        );
        assert_eq!(
            tree.to_bytes(),
            original,
            "{}: re-encoding is not byte-identical",
            path.display()
        );
    }

    eprintln!("round-tripped {} byte trees from {}", files.len(), dir.display());
}

/// Structural checks against the documented layout (VMNV §9.1), not just byte
/// equality: this is what catches a parser that round-trips garbage faithfully.
#[test]
fn corpus_structures_match_the_specification() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };
    let width = marshal::p256::WIDTH;

    // FullPublicKey.bt is pk = (g, y), and g must be the standard P-256 base
    // point -- an end-to-end check of the coordinate encoding.
    let pk = ByteTree::from_bytes(&std::fs::read(dir.join("FullPublicKey.bt")).unwrap()).unwrap();
    let parts = pk.as_node_of(2).expect("pk = (g, y)");
    assert_eq!(parts[0], marshal::p256::generator(), "pk's g is the P-256 generator");
    let y = parts[1].as_node_of(2).expect("y is an affine point");
    assert_eq!(y[0].as_leaf().unwrap().len(), width);

    // Ciphertexts.bt is an array of width-2 ciphertexts: node(u_arrays, v_arrays),
    // each side transposed into `width` component arrays of N elements.
    let ciphs =
        ByteTree::from_bytes(&std::fs::read(dir.join("Ciphertexts.bt")).unwrap()).unwrap();
    let sides = ciphs.as_node_of(2).expect("ciphertext = (u, v)");
    let u_rows = arithm::product_array_rows(&sides[0]).expect("u side transposes");
    let v_rows = arithm::product_array_rows(&sides[1]).expect("v side transposes");
    assert_eq!(u_rows.len(), v_rows.len(), "u and v have the same count");
    let n = u_rows.len();
    assert!(n > 0);
    assert_eq!(u_rows[0].len(), 2, "width 2");

    // tau^pos: node(B, A', B', C', D', F') with |B| = |B'| = N (VMNV §8.3).
    let tau = ByteTree::from_bytes(
        &std::fs::read(dir.join("proofs/PoSCommitment01.bt")).unwrap(),
    )
    .unwrap();
    let tau = tau.as_node_of(6).expect("tau^pos has 6 components");
    assert_eq!(tau[0].as_node().unwrap().len(), n, "B has N entries");
    assert_eq!(tau[2].as_node().unwrap().len(), n, "B' has N entries");

    // sigma^pos: node(k_A, k_B, k_C, k_D, k_E, k_F) with |k_B| = |k_E| = N,
    // scalars at the fixed width, and k_F of width omega.
    let sigma =
        ByteTree::from_bytes(&std::fs::read(dir.join("proofs/PoSReply01.bt")).unwrap()).unwrap();
    let sigma = sigma.as_node_of(6).expect("sigma^pos has 6 components");
    assert_eq!(sigma[0].as_leaf().unwrap().len(), width, "k_A is a fixed-width scalar");
    assert_eq!(sigma[1].as_node().unwrap().len(), n, "k_B has N entries");
    assert_eq!(sigma[4].as_node().unwrap().len(), n, "k_E has N entries");
    assert_eq!(sigma[5].as_node().unwrap().len(), 2, "k_F has omega entries");

    // The permutation commitment is a flat array of N group elements.
    let mu = ByteTree::from_bytes(
        &std::fs::read(dir.join("proofs/PermutationCommitment01.bt")).unwrap(),
    )
    .unwrap();
    assert_eq!(mu.as_node().unwrap().len(), n, "mu has N Pedersen commitments");

    // CorrectIndices.bt is a boolean array of length k+1 whose true entries are
    // the set Delta (VMNV §9.1 point 20).
    let indices = ByteTree::from_bytes(
        &std::fs::read(dir.join("proofs/CorrectIndices.bt")).unwrap(),
    )
    .unwrap();
    let flags = arithm::bool_array_values(&indices).expect("boolean array");
    assert!(flags.iter().any(|&b| b), "at least one party decrypted");

    eprintln!("structural checks passed for N={n}, width=2, P-256");
}

/// End-to-end Fiat–Shamir check: derive the shuffle proof's challenge `v` the
/// way VMN does and compare against the value `vmnv` printed for this very
/// proof.
///
/// This is the strongest available evidence that our transcript layer agrees
/// with Verificatum's, because it combines three independently-derived pieces:
/// the global prefix ρ we compute ourselves, the golden batching seed `s`, and
/// the real `PoSCommitment01.bt` bytes parsed from disk. If any of the byte-tree
/// encoding, the oracle construction, or the query framing were wrong, `v` would
/// not match.
#[test]
fn shuffle_challenge_matches_vmn() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };

    // Golden values printed by `vmnv -t PoS.s,PoS.v` for this proof.
    const GOLDEN_POS_S: &str =
        "78e66e9d0099c4322d9d18579254ae92e779ad2f5b3a7120cd21c2c84bfa49f5";
    const GOLDEN_POS_V: &str =
        "412cddba831caaecce9fc71e7ba6896c8f9761e1d86878e7117c997fe9bef70c";

    let rho = reference_rho();
    let seed = hex_bytes(GOLDEN_POS_S);

    let tau_pos = ByteTree::from_bytes(
        &std::fs::read(dir.join("proofs/PoSCommitment01.bt")).unwrap(),
    )
    .unwrap();

    let v = vcompat::crypto::pos_challenge(
        vcompat::crypto::Hashfunction::Sha256,
        256, // n_v
        &rho,
        &seed,
        &tau_pos,
    );

    assert_eq!(
        hex_string(&v),
        GOLDEN_POS_V,
        "shuffle challenge v must match vmnv -t PoS.v"
    );
    eprintln!("shuffle challenge v reproduced exactly");
}

/// Closes the Fiat–Shamir loop: derive the batching **seed** `s` from the
/// statement itself and check it against `vmnv -t PoS.s`.
///
/// Where `shuffle_challenge_matches_vmn` takes `s` as given, this one computes
/// it from `node(g, h, u, pk, w, w')` — so it additionally pins the *order* and
/// framing of that six-element query, and the reconstruction of group elements
/// from raw coordinates. Together the two tests cover the whole transcript path
/// for a proof of a shuffle.
///
/// Needs `testvectors.txt` (for `bas.h`, the independent generators) alongside
/// the corpus directory; skipped if absent.
#[test]
fn shuffle_seed_matches_vmn() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };
    let vectors_path = match dir.parent().map(|p| p.join("testvectors.txt")) {
        Some(p) if p.is_file() => p,
        _ => {
            eprintln!("skipping: testvectors.txt not found next to the corpus directory");
            return;
        }
    };

    const GOLDEN_POS_S: &str =
        "78e66e9d0099c4322d9d18579254ae92e779ad2f5b3a7120cd21c2c84bfa49f5";

    let text = std::fs::read_to_string(&vectors_path).unwrap();

    // Diagnostic: `bas.pk` is printed in the same point-list format AND stored
    // on disk as FullPublicKey.bt, so parsing it and comparing against the file
    // isolates the parser from everything else.
    let parsed_pk = parse_point_list(&text, "bas.pk").expect("bas.pk in test vectors");
    let file_pk =
        ByteTree::from_bytes(&std::fs::read(dir.join("FullPublicKey.bt")).unwrap()).unwrap();
    assert_eq!(
        parsed_pk, file_pk,
        "point-list parser must reproduce FullPublicKey.bt exactly"
    );

    let h = parse_point_list(&text, "bas.h").expect("bas.h in test vectors");
    eprintln!("parsed {} independent generators", h.as_node().unwrap().len());

    let read_tree = |name: &str| {
        ByteTree::from_bytes(&std::fs::read(dir.join(name)).unwrap()).unwrap()
    };

    // The key is WIDENED to omega before entering the query -- not the stored
    // FullPublicKey.bt as VMNV §8.3's "pk in C_kappa" would suggest.
    let wide_pk =
        vcompat::crypto::wide_public_key(&read_tree("FullPublicKey.bt"), 2).unwrap();

    let seed = vcompat::crypto::pos_seed(
        vcompat::crypto::Hashfunction::Sha256,
        &reference_rho(),
        &marshal::p256::generator(),        // g
        &h,                                 // h, the independent generators
        &read_tree("proofs/PermutationCommitment01.bt"), // u
        &wide_pk,                           // pk, widened to omega
        &read_tree("Ciphertexts.bt"),       // w   = L_0
        &read_tree("proofs/Ciphertexts01.bt"), // w' = L_1
    );

    assert_eq!(
        hex_string(&seed),
        GOLDEN_POS_S,
        "batching seed s must match vmnv -t PoS.s"
    );
    eprintln!("shuffle batching seed s reproduced exactly");
}

/// The decryption transcript, checked against `vmnv -t Dec.s,Dec.v`.
///
/// The first step of the decryption work (VERIFICATUM.md Stage 4), and
/// deliberately the cheapest: it settles whether we have the *transcript* right
/// before any of the batched-proof algebra is written. Everything here comes
/// from the corpus, which is a full `mixing` proof and so carries the decryption
/// artifacts.
#[test]
fn decryption_transcript_matches_vmn() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };

    // Golden values printed by `vmnv -t Dec.s,Dec.v` for this proof.
    const GOLDEN_DEC_S: &str =
        "b3b0803472e0f921e6ec1efe0207b34a23a44c717f079406f7efd32441d56734";
    const GOLDEN_DEC_V: &str =
        "9393f014a1ddd07120e5c9f474c0371186162e52f7883a018496a0f9b2c82940";

    let read_tree = |name: &str| {
        ByteTree::from_bytes(&std::fs::read(dir.join(name)).expect("read corpus file"))
            .expect("parse byte tree")
    };
    let rho = reference_rho();

    // The list being decrypted is the final shuffled output, not the input; with
    // one mixer that is Ciphertexts01.bt. `g` enters unwidened.
    let seed = vcompat::crypto::dec_seed(
        vcompat::crypto::Hashfunction::Sha256,
        &rho,
        &marshal::p256::generator(),
        &read_tree("proofs/Ciphertexts01.bt"),
        &read_tree("proofs/PolynomialInExponent.bt"),
        &[read_tree("proofs/DecryptionFactors01.bt")],
    );
    assert_eq!(
        hex_string(&seed),
        GOLDEN_DEC_S,
        "decryption batching seed must match vmnv -t Dec.s"
    );

    let v = vcompat::crypto::dec_challenge(
        vcompat::crypto::Hashfunction::Sha256,
        256, // n_v
        &rho,
        &seed,
        &[read_tree("proofs/DecrFactCommitment01.bt")],
    );
    assert_eq!(
        hex_string(&v),
        GOLDEN_DEC_V,
        "decryption challenge must match vmnv -t Dec.v"
    );
    eprintln!("decryption seed and challenge reproduced exactly");
}

/// Derive the independent generators ourselves and check them against
/// `vmnv -t bas.h` (VMNV §6.8).
///
/// This closes the last gap in the shuffle transcript: with `h` derived rather
/// than borrowed, every input to the batching seed can be produced from the
/// protocol parameters alone.
#[test]
fn independent_generators_match_vmn() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };
    let vectors_path = match dir.parent().map(|p| p.join("testvectors.txt")) {
        Some(p) if p.is_file() => p,
        _ => {
            eprintln!("skipping: testvectors.txt not found next to the corpus directory");
            return;
        }
    };
    let text = std::fs::read_to_string(&vectors_path).unwrap();
    let expected = parse_point_list(&text, "bas.h").expect("bas.h in test vectors");
    let count = expected.as_node().unwrap().len();

    let derived = vcompat::generators::independent_generators(
        vcompat::crypto::Hashfunction::Sha256,
        &reference_rho(),
        &vcompat::generators::CurveParams::p256(),
        100, // n_r = statdist
        count,
    )
    .expect("derive generators");

    assert_eq!(derived, expected, "derived generators must match vmnv -t bas.h");
    eprintln!("derived {count} independent generators matching VMN exactly");
}

/// With `h` derived rather than taken from a test vector, the batching seed is
/// reproducible from the protocol parameters and the proof files alone — which
/// is the position a real emitter is in.
#[test]
fn shuffle_seed_from_fully_derived_inputs() {
    let Some(dir) = corpus_dir() else {
        eprintln!("skipping: set VCOMPAT_CORPUS to a VMN nizkp directory to run this test");
        return;
    };
    const GOLDEN_POS_S: &str =
        "78e66e9d0099c4322d9d18579254ae92e779ad2f5b3a7120cd21c2c84bfa49f5";

    let read_tree = |name: &str| {
        ByteTree::from_bytes(&std::fs::read(dir.join(name)).unwrap()).unwrap()
    };
    let w = read_tree("Ciphertexts.bt");
    // N is the number of ciphertexts, which also fixes how many generators the
    // shuffle proof needs.
    let n = arithm::product_array_rows(&w.as_node_of(2).unwrap()[0]).unwrap().len();

    let rho = reference_rho();
    let h = vcompat::generators::independent_generators(
        vcompat::crypto::Hashfunction::Sha256,
        &rho,
        &vcompat::generators::CurveParams::p256(),
        100,
        n,
    )
    .unwrap();

    let seed = vcompat::crypto::pos_seed(
        vcompat::crypto::Hashfunction::Sha256,
        &rho,
        &marshal::p256::generator(),
        &h,
        &read_tree("proofs/PermutationCommitment01.bt"),
        &vcompat::crypto::wide_public_key(&read_tree("FullPublicKey.bt"), 2).unwrap(),
        &w,
        &read_tree("proofs/Ciphertexts01.bt"),
    );

    assert_eq!(hex_string(&seed), GOLDEN_POS_S);
    eprintln!("batching seed reproduced from fully derived inputs (N={n})");
}

/// Parse a `vmnv -t` point-list vector -- `((x, y),(x, y),...)` in hex -- into a
/// byte tree array of affine points at P-256's fixed width.
///
/// The printed coordinates are plain integers with leading zeros trimmed, so
/// they are neither fixed-width nor necessarily even-length; [`hex_bytes`]
/// left-pads accordingly.
fn parse_point_list(text: &str, name: &str) -> Option<ByteTree> {
    let marker = format!("{name} - ");
    let start = text.find(&marker)?;
    let line = text[start..].lines().nth(1)?.trim();

    // Strip the grouping punctuation and read the flat sequence of coordinates.
    let flat: String = line.chars().filter(|c| *c != '(' && *c != ')').collect();
    let coords: Vec<&str> = flat
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_hexdigit()))
        .collect();
    if coords.is_empty() || coords.len() % 2 != 0 {
        return None;
    }

    let points = coords
        .chunks(2)
        .map(|xy| arithm::curve_point(&hex_bytes(xy[0]), &hex_bytes(xy[1]), marshal::p256::WIDTH))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(ByteTree::node(points))
}

/// The reference session's global prefix, recomputed from its protocol info
/// parameters rather than hardcoded, so this test also re-exercises ρ.
fn reference_rho() -> Vec<u8> {
    use vcompat::crypto::{global_prefix, Hashfunction, PrefixParams};
    const PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";
    global_prefix(
        Hashfunction::Sha256,
        &PrefixParams {
            version: "3.1.0".into(),
            sid: "braidpoc".into(),
            auxsid: "default".into(),
            n_r: 100,
            n_v: 256,
            n_e: 256,
            prg: "SHA-256".into(),
            pgroup: PGROUP.into(),
            rohash: "SHA-256".into(),
        },
    )
}

/// Hex to bytes, left-padding an odd-length string with a leading zero nibble
/// (the printed test vectors trim leading zeros).
fn hex_bytes(s: &str) -> Vec<u8> {
    let padded;
    let s = if s.len() % 2 == 0 {
        s
    } else {
        padded = format!("0{s}");
        &padded
    };
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
