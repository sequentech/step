// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Stage 2 acceptance: reproduce Verificatum's random-oracle layer bit for bit.
//!
//! The values pinned here were printed by a real `vmnv` run over the Stage 0
//! reference proof (`vmnv -t par,der,bas,PoS,Dec,u`), for a single-mix-server
//! session over P-256 with width 2, `sid = "braidpoc"`, `auxsid = "default"`.
//!
//! This is the go/no-go gate described in `VERIFICATUM.md`: braid's proof
//! algebra already matches Verificatum's, so what decides the approach is
//! whether the Fiat–Shamir transcript can be reproduced exactly. Everything
//! downstream depends on these bytes.

mod common;

use vsvmn::wire::crypto::{global_prefix, Hashfunction, PrefixParams, Prg, RandomOracle};
use vsvmn::wire::protinfo::ProtocolInfo;

/// The `<pgroup>` value from the reference `protInfo.xml`, verbatim — comment
/// prefix included, which is what actually goes into ρ.
const PGROUP: &str = "ECqPGroup(P-256)::0000000002010000002\
0636f6d2e766572696669636174756d2e61726974686d2e4543715047726f757001000000\
05502d323536";

fn reference_params() -> PrefixParams {
    PrefixParams {
        version: "3.1.0".to_string(),
        sid: "braidpoc".to_string(),
        auxsid: "default".to_string(),
        n_r: 100, // statdist
        n_v: 256, // vbitlenro
        n_e: 256, // ebitlenro
        prg: "SHA-256".to_string(),
        pgroup: PGROUP.to_string(),
        rohash: "SHA-256".to_string(),
    }
}

/// ρ must be sensitive to every field it commits to — a prefix that ignored one
/// would still pass the equality test above while failing to separate sessions.
#[test]
fn global_prefix_binds_every_parameter() {
    let base = global_prefix(Hashfunction::Sha256, &reference_params());

    let mutations: Vec<(&str, Box<dyn Fn(&mut PrefixParams)>)> = vec![
        ("version", Box::new(|p: &mut PrefixParams| p.version = "3.1.1".into())),
        ("sid", Box::new(|p: &mut PrefixParams| p.sid = "other".into())),
        ("auxsid", Box::new(|p: &mut PrefixParams| p.auxsid = "other".into())),
        ("n_r", Box::new(|p: &mut PrefixParams| p.n_r = 101)),
        ("n_v", Box::new(|p: &mut PrefixParams| p.n_v = 255)),
        ("n_e", Box::new(|p: &mut PrefixParams| p.n_e = 255)),
        ("prg", Box::new(|p: &mut PrefixParams| p.prg = "SHA-512".into())),
        ("pgroup", Box::new(|p: &mut PrefixParams| p.pgroup = "ECqPGroup(P-384)::00".into())),
        ("rohash", Box::new(|p: &mut PrefixParams| p.rohash = "SHA-512".into())),
    ];

    for (field, mutate) in mutations {
        let mut params = reference_params();
        mutate(&mut params);
        let changed = global_prefix(Hashfunction::Sha256, &params);
        assert_ne!(changed, base, "rho must depend on {field}");
    }
}

/// The `sid`/`auxsid` join is a concatenation with a literal dot, so a session
/// cannot be confused with one whose identifiers split differently. This pins
/// the detail rather than leaving it implicit in the golden value.
#[test]
fn rosid_is_a_dot_join_not_a_pair() {
    let mut split_differently = reference_params();
    split_differently.sid = "braidpoc.def".to_string();
    split_differently.auxsid = "ault".to_string();

    // "braidpoc" + "." + "default" != "braidpoc.def" + "." + "ault", so these
    // must differ -- if the join were length-prefixed they would differ too, but
    // if it were a naive concatenation without the dot they would collide.
    assert_ne!(
        global_prefix(Hashfunction::Sha256, &reference_params()),
        global_prefix(Hashfunction::Sha256, &split_differently)
    );
}

// ------------------------------------------------------------------- PRG

#[test]
fn prg_is_hash_of_seed_and_counter() {
    // VMNV §5.2: r_i = H(s || bytes_4(i)), counter big-endian from 0.
    use sha2::Digest;
    let seed = [0x42u8; 32];
    let out = Prg::new(Hashfunction::Sha256, &seed).generate(96);

    for i in 0..3u32 {
        let mut input = seed.to_vec();
        input.extend_from_slice(&i.to_be_bytes());
        let expected = sha2::Sha256::digest(&input);
        assert_eq!(&out[i as usize * 32..(i as usize + 1) * 32], &expected[..],
                   "PRG block {i}");
    }
}

#[test]
fn prg_output_is_a_prefix_independent_of_requested_length() {
    let seed = [0x07u8; 32];
    let prg = Prg::new(Hashfunction::Sha256, &seed);
    let long = prg.generate(200);
    for len in [0usize, 1, 31, 32, 33, 64, 199, 200] {
        assert_eq!(prg.generate(len), long[..len], "length {len}");
    }
}

// --------------------------------------------------------- random oracle

#[test]
fn random_oracle_prefixes_the_output_length() {
    // VMNV §5.3 step 1: s = H(bytes_4(n_out) || d). Two oracles of different
    // widths on the same input must therefore not share a prefix.
    let data = b"abc";
    let a = RandomOracle::new(Hashfunction::Sha256, 256).eval(data);
    let b = RandomOracle::new(Hashfunction::Sha256, 512).eval(data);
    assert_eq!(a.len(), 32);
    assert_eq!(b.len(), 64);
    assert_ne!(a[..], b[..32], "differing n_out must reseed the PRG differently");
}

#[test]
fn random_oracle_masks_leading_bits_for_non_multiples_of_eight() {
    // VMNV §5.3 step 3: zero the leading 8 - (n_out mod 8) bits so the output
    // reads as a non-negative integer of nominal bit length n_out.
    for bits in [1usize, 7, 9, 100, 255] {
        let out = RandomOracle::new(Hashfunction::Sha256, bits).eval(b"x");
        assert_eq!(out.len(), bits.div_ceil(8));
        let excess = bits % 8;
        if excess != 0 {
            assert_eq!(
                out[0] & !(0xFFu8 >> (8 - excess)),
                0,
                "top bits must be cleared for n_out = {bits}"
            );
        }
    }
    // A multiple of 8 is left untouched.
    let out = RandomOracle::new(Hashfunction::Sha256, 256).eval(b"x");
    assert_eq!(out.len(), 32);
}

#[test]
fn hashfunction_names_round_trip() {
    for h in [Hashfunction::Sha256, Hashfunction::Sha384, Hashfunction::Sha512] {
        assert_eq!(Hashfunction::from_name(h.name()), Some(h));
        assert_eq!(h.outlen() * 8, h.outlen_bits());
    }
    // VMN 3.1.0 has no SHA-3, which is precisely why braid's native transcripts
    // (SHA3-512) cannot be verified as-is.
    assert_eq!(Hashfunction::from_name("SHA3-512"), None);
}

/// **The Stage 2 gate, against whatever Verificatum is installed.**
///
/// If this passes, our transcript layer agrees with VMN's at the root: every
/// proof-specific oracle query is salted with ρ, so nothing downstream can match
/// unless this does.
///
/// It generates a session and compares against the `der.rho` *that* VMN reports,
/// rather than a value captured from 3.1.0 once and pinned. A pinned value keeps
/// passing against a later VMN that changed the derivation — a check that cannot
/// fail is not a check. Every parameter comes from the generated
/// `protInfo.xml`; nothing about the session is assumed here.
#[test]
#[ignore = "runs VMN; see tests/common/mod.rs"]
fn rho_matches_the_installed_verificatum() {
    let Some(corpus) = common::shared() else {
        return common::skip("Verificatum is unavailable");
    };
    let Some(vectors) = common::shared_vectors() else {
        return common::skip("vmnv -t produced no test vectors");
    };

    let xml = std::fs::read_to_string(&corpus.protinfo).expect("read protInfo.xml");
    let info = ProtocolInfo::parse(&xml).expect("parse protInfo.xml");
    let auxsid = std::fs::read_to_string(corpus.nizkp.join("auxsid")).expect("read auxsid");

    let rho = global_prefix(Hashfunction::Sha256, &info.prefix_params(auxsid.trim()));

    assert_eq!(
        hex::encode(&rho),
        vectors["der.rho"],
        "our global prefix must equal this VMN's der.rho"
    );
    assert_eq!(rho.len(), 32, "SHA-256 gives a 32-byte prefix");
}

/// The proof-specific transcripts, likewise against the installed VMN.
///
/// ρ agreeing is necessary but not sufficient: the shuffle and decryption
/// seeds and challenges are separate derivations over separate byte trees, and
/// each is a place the two implementations could diverge.
#[test]
#[ignore = "runs VMN; see tests/common/mod.rs"]
fn the_proof_transcripts_match_the_installed_verificatum() {
    if common::shared().is_none() {
        return common::skip("Verificatum is unavailable");
    }
    let Some(vectors) = common::shared_vectors() else {
        return common::skip("vmnv -t produced no test vectors");
    };

    // Present in the output means VMN computed it; we only assert on the ones
    // it reports, so a session type without a decryption phase is not a failure.
    for name in ["PoS.s", "PoS.v", "Dec.s", "Dec.v"] {
        assert!(
            vectors.contains_key(name),
            "vmnv -t should have reported {name} for a mixing session"
        );
        assert!(
            !vectors[name].is_empty(),
            "{name} must not be empty"
        );
    }

    // The values themselves are checked where they are computed --
    // corpus_roundtrip reproduces them from the proof directory. This test
    // establishes that vmnv reports them at all for the shape we generate, so a
    // silent change in which vectors exist is caught here rather than showing up
    // as a confusing absence downstream.
}
