// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Conformance tests taken from the worked examples in `vmnv-3.1.0.md` (VMNV
//! §4, §6) plus the structural size predictions validated against a real
//! VMN-generated proof directory.
//!
//! These are self-contained: no corpus needed. The corpus round-trip lives in
//! `corpus_roundtrip.rs`.

use v2v::wire::arithm;
use v2v::wire::bytetree::ByteTree;
use v2v::wire::marshal;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------- byte trees

#[test]
fn leaf_and_node_wire_format() {
    // VMNV §4.2: leaf = 01 || len_4 || data, node = 00 || count_4 || children.
    assert_eq!(
        hex(&ByteTree::leaf(vec![0x01, 0x07]).to_bytes()),
        "010000000201 07".replace(' ', "")
    );
    assert_eq!(
        hex(
            &ByteTree::node(vec![ByteTree::leaf(vec![0xaa]), ByteTree::leaf(vec![0xbb])])
                .to_bytes()
        ),
        "00000000020100000001aa0100000001bb"
    );
}

#[test]
fn example_5_integer_263() {
    // VMNV §6.1 Example 5: 263 is leaf(0107).
    let tree = ByteTree::leaf(arithm::encode_nonneg_minimal(&[0x01, 0x07]));
    assert_eq!(hex(&tree.to_bytes()), "01000000020107");
}

#[test]
fn signed_encoding_adds_a_leading_zero() {
    // The trap: 128 (0x80) has its top bit set, so as a NON-NEGATIVE value it
    // must be encoded in two bytes, 0080 -- not one. VMNV §6.1's own -263 =>
    // FEF9 example is what establishes the encoding is signed.
    assert_eq!(arithm::minimal_signed_width(&[0x80]), 2);
    assert_eq!(arithm::encode_nonneg_minimal(&[0x80]), vec![0x00, 0x80]);
    // Whereas 127 fits in one byte.
    assert_eq!(arithm::minimal_signed_width(&[0x7f]), 1);
    assert_eq!(arithm::encode_nonneg_minimal(&[0x7f]), vec![0x7f]);
    // Zero occupies a single byte.
    assert_eq!(arithm::minimal_signed_width(&[0x00]), 1);
}

#[test]
fn example_9_and_10_field_elements_are_fixed_width() {
    // VMNV §6.2 Examples 9/10: in Z_263 (modulus needs 9 bits => width 2),
    // 258 is leaf(0102) and 5 is leaf(0005) -- same width, zero padded.
    let width = arithm::fixed_width_for_modulus_bits(9);
    assert_eq!(width, 2);
    assert_eq!(
        hex(&arithm::field_element(&[0x01, 0x02], width)
            .unwrap()
            .to_bytes()),
        "01000000020102"
    );
    assert_eq!(
        hex(&arithm::field_element(&[0x05], width).unwrap().to_bytes()),
        "01000000020005"
    );
}

#[test]
fn example_11_array_of_field_elements() {
    // VMNV §6.2 Example 11: the array (1,2,3) over Z_263.
    let width = 2;
    let tree = arithm::array(
        [1u8, 2, 3]
            .iter()
            .map(|v| arithm::field_element(&[*v], width).unwrap())
            .collect(),
    );
    assert_eq!(
        hex(&tree.to_bytes()),
        "0000000003010000000200010100000002000201000000020003"
    );
}

#[test]
fn example_14_product_arrays_are_transposed() {
    // VMNV §6.3 Example 14: the array ((1,4),(2,5),(3,6)) is stored as
    // node( node(1,2,3), node(4,5,6) ) -- component-major, not element-major.
    let w = 2;
    let f = |v: u8| arithm::field_element(&[v], 2).unwrap();
    let rows = vec![vec![f(1), f(4)], vec![f(2), f(5)], vec![f(3), f(6)]];
    let tree = arithm::product_array(&rows, w).unwrap();

    let cols = tree.as_node_of(2).unwrap();
    let first = cols[0].as_node_of(3).unwrap();
    assert_eq!(first[0].as_leaf().unwrap(), &[0x00, 0x01]);
    assert_eq!(first[1].as_leaf().unwrap(), &[0x00, 0x02]);
    assert_eq!(first[2].as_leaf().unwrap(), &[0x00, 0x03]);
    let second = cols[1].as_node_of(3).unwrap();
    assert_eq!(second[0].as_leaf().unwrap(), &[0x00, 0x04]);

    // And the inverse recovers element-major order.
    assert_eq!(arithm::product_array_rows(&tree).unwrap(), rows);
}

// ------------------------------------------------------------------- P-256

#[test]
fn p256_width_is_33_not_32() {
    // The headline trap: p and q are 256-bit with the top bit set, so the signed
    // encoding needs a 33rd byte. A 32-byte assumption yields a 163-byte
    // FullPublicKey.bt where VMN actually writes 167.
    assert_eq!(marshal::p256::WIDTH, 33);
}

#[test]
fn example_18_p256_group_descriptor_matches_real_vmn_output() {
    // Ground truth: the exact string emitted by
    //     vog -gen ECqPGroup -name P-256
    // in the Stage 0 run (VMNV §6.7 Example 18 describes the same structure, but
    // its printed hex is garbled by OCR, so we pin the real tool output).
    const REAL: &str = "0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";
    let tree = marshal::p256::group_tree();
    assert_eq!(hex(&tree.to_bytes()), REAL);
    assert_eq!(marshal::curve_name(&tree).unwrap(), "P-256");

    // And it round-trips through the comment::hex marshalling.
    let s = marshal::marshal("ECqPGroup(P-256)", &tree);
    assert!(s.starts_with("ECqPGroup(P-256)::"));
    assert_eq!(marshal::unmarshal(&s).unwrap(), tree);
}

#[test]
fn full_public_key_layout_matches_real_vmn_output() {
    // Ground truth: the first 16 bytes of the real FullPublicKey.bt from the
    // Stage 0 corpus, whose generator point is the standard P-256 base point.
    // pk = (g, y); this checks the g half byte for byte.
    let g = marshal::p256::generator();
    let bytes = g.to_bytes();
    // node(2) || leaf(33) || 00 || Gx ...
    assert_eq!(&hex(&bytes)[..14], "00000000020100");
    assert_eq!(bytes[5], 0x01); // leaf tag
    assert_eq!(&bytes[6..10], &[0, 0, 0, 33]); // 33-byte coordinate
    assert_eq!(bytes[10], 0x00); // the sign byte
    assert_eq!(&bytes[11..43], &marshal::p256::GENERATOR_X);
}

// ------------------------------------------------- structural size predictions

/// Sizes of the files in a real VMN proof directory for N=10, width=2, P-256.
/// Every one of these was confirmed against the Stage 0 corpus, so they pin the
/// whole structural model: the encoding, the transposition, and the component
/// lists of the shuffle proof objects.
#[test]
fn predicted_sizes_match_the_real_corpus() {
    const N: usize = 10;
    const W: usize = 2;
    let width = marshal::p256::WIDTH;

    let point = || arithm::curve_point(&[1], &[2], width).unwrap();
    let scalar = || arithm::field_element(&[1], width).unwrap();

    let elem_len = point().serialized_len();
    assert_eq!(elem_len, 81, "curve point = node(leaf(33), leaf(33))");

    let arr = ByteTree::node(vec![point(); N]);
    let m_kw = ByteTree::node(vec![arr.clone(); W]);
    let ciphertexts = ByteTree::node(vec![m_kw.clone(), m_kw.clone()]);

    assert_eq!(
        ByteTree::node(vec![point(), point()]).serialized_len(),
        167,
        "FullPublicKey.bt"
    );
    assert_eq!(ciphertexts.serialized_len(), 3275, "Ciphertexts.bt");
    assert_eq!(
        m_kw.serialized_len(),
        1635,
        "Plaintexts.bt / DecryptionFactors01.bt"
    );
    assert_eq!(arr.serialized_len(), 815, "PermutationCommitment01.bt");

    // tau^pos = node(B, A', B', C', D', F') -- VMNV §8.3, and field for field
    // braid's ShuffleCommitments.
    let f_prime = ByteTree::node(vec![
        ByteTree::node(vec![point(); W]),
        ByteTree::node(vec![point(); W]),
    ]);
    let tau_pos = ByteTree::node(vec![
        arr.clone(), // B
        point(),     // A'
        arr.clone(), // B'
        point(),     // C'
        point(),     // D'
        f_prime,     // F'
    ]);
    assert_eq!(tau_pos.serialized_len(), 2217, "PoSCommitment01.bt");

    // sigma^pos = node(k_A, k_B, k_C, k_D, k_E, k_F) -- braid's Responses.
    let scalar_arr = ByteTree::node(vec![scalar(); N]);
    let sigma_pos = ByteTree::node(vec![
        scalar(),                          // k_A
        scalar_arr.clone(),                // k_B
        scalar(),                          // k_C
        scalar(),                          // k_D
        scalar_arr,                        // k_E
        ByteTree::node(vec![scalar(); W]), // k_F
    ]);
    assert_eq!(sigma_pos.serialized_len(), 970, "PoSReply01.bt");
}

// -------------------------------------------------------------- strictness

#[test]
fn parsing_is_strict() {
    let good = ByteTree::leaf(vec![1, 2, 3]).to_bytes();
    assert!(ByteTree::from_bytes(&good).is_ok());

    // Trailing bytes are rejected: the encoding must be unique, because these
    // bytes are hashed into Fiat-Shamir transcripts.
    let mut trailing = good.clone();
    trailing.push(0x00);
    assert!(ByteTree::from_bytes(&trailing).is_err());

    // Truncated input.
    assert!(ByteTree::from_bytes(&good[..good.len() - 1]).is_err());
    // Bad tag.
    assert!(ByteTree::from_bytes(&[0x02, 0, 0, 0, 0]).is_err());
    // A node claiming more children than could possibly fit.
    assert!(ByteTree::from_bytes(&[0x00, 0xff, 0xff, 0xff, 0xff]).is_err());
}

#[test]
fn roundtrip_is_byte_identical() {
    let tree = ByteTree::node(vec![
        ByteTree::leaf(vec![]),
        ByteTree::node(vec![]),
        ByteTree::node(vec![ByteTree::leaf(vec![0xff; 300])]),
    ]);
    let bytes = tree.to_bytes();
    let parsed = ByteTree::from_bytes(&bytes).unwrap();
    assert_eq!(parsed, tree);
    assert_eq!(parsed.to_bytes(), bytes);
    assert_eq!(bytes.len(), tree.serialized_len());
}
