// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Step's Tracking ID writing and reading against Vitrine's vectors, copied
//! unchanged from its `testdata/tracking-id/tracking-id.json` (Apache-2.0, see
//! `REUSE.toml`): a 32-byte `ballot_id` as Crockford base32 in groups of four,
//! its short ID, and which typed starts are valid and which ballots they match.
//!
//! One read of a typed start differs on purpose. Vitrine's profile takes the
//! characters of an ID only, so `49SJ 006D` is no start there; Step reads what
//! a voter types or pastes, and ignores spaces as it ignores hyphens.

#![cfg(feature = "default_features")]

use sequent_core::ballot_receipt::{
    format_tracking_id, normalize_tracking_id, short_tracking_id,
    tracking_id_starts_with, TRACKING_ID_BYTES,
};
use serde_json::Value;

/// The one vector whose typed text Step reads although Vitrine's profile
/// does not: see the module documentation.
const READ_BY_STEP_ONLY: &str = "prefix-space";

const VECTORS: &str =
    include_str!("vitrine-vectors/tracking-id/tracking-id.json");
const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn vectors(kind: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(VECTORS).unwrap();
    let selected: Vec<Value> = all["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|vector| vector["kind"] == kind)
        .cloned()
        .collect();
    assert!(!selected.is_empty(), "no {kind} vectors");
    selected
}

fn digest(ballot_id: &str) -> [u8; TRACKING_ID_BYTES] {
    hex::decode(ballot_id).unwrap().try_into().unwrap()
}

fn name(vector: &Value) -> &str {
    vector["name"].as_str().unwrap()
}

/// The bits of the characters, most significant first, padded with zeros to
/// whole bytes, as lowercase hex: the `bits` of a prefix vector.
fn bits_of(characters: &str) -> String {
    let mut bits: Vec<bool> = characters
        .chars()
        .flat_map(|character| {
            let value = ALPHABET.find(character).unwrap();
            (0..5).rev().map(move |shift| (value >> shift) & 1 == 1)
        })
        .collect();
    while bits.len() % 8 != 0 {
        bits.push(false);
    }
    let bytes: Vec<u8> = bits
        .chunks(8)
        .map(|byte| byte.iter().fold(0u8, |sum, bit| (sum << 1) | *bit as u8))
        .collect();
    hex::encode(bytes)
}

#[test]
fn a_ballot_id_is_written_in_groups_of_four_with_its_short_id_first() {
    for vector in vectors("encode") {
        let full =
            format_tracking_id(&digest(vector["ballot_id"].as_str().unwrap()));
        assert_eq!(full, vector["full"].as_str().unwrap(), "{}", name(&vector));
        assert_eq!(
            short_tracking_id(&full),
            vector["short"].as_str().unwrap(),
            "{}",
            name(&vector)
        );
        assert_eq!(full.len(), 52 + 12, "{}", name(&vector));
    }
}

#[test]
fn a_typed_start_is_valid_when_vitrine_says_so() {
    for vector in vectors("prefix") {
        let typed = vector["typed"].as_str().unwrap();
        let read = normalize_tracking_id(typed);
        if name(&vector) == READ_BY_STEP_ONLY {
            assert!(!vector["valid"].as_bool().unwrap());
            assert_eq!(read.as_deref(), Some("49SJ-006D"));
            continue;
        }
        if vector["valid"].as_bool().unwrap() {
            let read =
                read.unwrap_or_else(|| panic!("{} is valid", name(&vector)));
            assert_eq!(
                read.replace('-', ""),
                vector["chars"].as_str().unwrap(),
                "{}",
                name(&vector)
            );
        } else {
            assert_eq!(read, None, "{}", name(&vector));
        }
    }
}

#[test]
fn a_valid_start_has_the_bits_vitrine_gives_it() {
    for vector in vectors("prefix") {
        if !vector["valid"].as_bool().unwrap() {
            continue;
        }
        let chars = vector["chars"].as_str().unwrap();
        assert_eq!(
            vector["bit_length"].as_u64().unwrap() as usize,
            chars.len() * 5,
            "{}",
            name(&vector)
        );
        assert_eq!(
            bits_of(chars),
            vector["bits"].as_str().unwrap(),
            "{}",
            name(&vector)
        );
    }
}

#[test]
fn a_start_matches_the_ballots_vitrine_lists_and_no_other() {
    for vector in vectors("prefix") {
        if !vector["valid"].as_bool().unwrap() {
            continue;
        }
        let typed = normalize_tracking_id(vector["typed"].as_str().unwrap())
            .unwrap_or_else(|| panic!("{} is valid", name(&vector)));
        let matching: Vec<String> = vector["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|candidate| candidate.as_str().unwrap().to_string())
            .filter(|candidate| {
                tracking_id_starts_with(
                    &format_tracking_id(&digest(candidate)),
                    &typed,
                )
            })
            .collect();
        let listed: Vec<String> = vector["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|listed| listed.as_str().unwrap().to_string())
            .collect();
        assert_eq!(matching, listed, "{}", name(&vector));
    }
}

#[test]
fn two_ballots_may_share_a_short_id_and_a_longer_start_tells_them_apart() {
    let ids: Vec<String> = vectors("encode")
        .iter()
        .map(|vector| {
            format_tracking_id(&digest(vector["ballot_id"].as_str().unwrap()))
        })
        .collect();
    let shared: Vec<&String> = ids
        .iter()
        .filter(|id| short_tracking_id(id) == "49SJ-006D")
        .collect();
    assert_eq!(shared.len(), 2);
    assert!(shared
        .iter()
        .all(|id| tracking_id_starts_with(id, "49SJ-006D")));
    let longer = &shared[0][..14];
    assert_eq!(
        shared
            .iter()
            .filter(|id| tracking_id_starts_with(id, longer))
            .count(),
        1
    );
}
