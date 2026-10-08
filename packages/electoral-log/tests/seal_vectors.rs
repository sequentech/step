// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seal manifest test vectors (`tests/data/ballot_box_seal_v1.json`).
//! Each vector gives the manifest's header, its entries (unsorted, with the
//! ballot content they hash) and the expected Borsh bytes and SHA-512. A
//! change to the manifest's encoding or ordering fails here.
//!
//! Regenerate after a deliberate format change (a new format version, never
//! v1) with `cargo test -p electoral-log --test seal_vectors -- --ignored`.

use electoral_log::seal::*;
use serde::{Deserialize, Serialize};

const VECTORS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/ballot_box_seal_v1.json"
);

#[derive(Serialize, Deserialize)]
struct VectorFile {
    format: String,
    about: String,
    vectors: Vec<Vector>,
}

#[derive(Serialize, Deserialize)]
struct VectorEntry {
    /// The stored `cast_vote.content`; `ballot_hash` is its SHA-512.
    content: String,
    #[serde(flatten)]
    entry: SealEntry,
}

#[derive(Serialize, Deserialize)]
struct Vector {
    name: String,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    closed_at: u64,
    grace_deadline: u64,
    sealed_at: u64,
    close_request_id: Option<String>,
    eligible_voters: u64,
    /// In the order the sealer might read them; `build` sorts.
    entries: Vec<VectorEntry>,
    /// The sorted entries' Ballot IDs, in manifest order.
    sorted_ballot_ids: Vec<String>,
    manifest_borsh_hex: String,
    seal_hash: String,
}

impl Vector {
    fn manifest(&self) -> BallotBoxSealManifest {
        BallotBoxSealManifest {
            format: SEAL_FORMAT_V1.to_string(),
            tenant_id: self.tenant_id.clone(),
            election_event_id: self.election_event_id.clone(),
            election_id: self.election_id.clone(),
            area_id: self.area_id.clone(),
            closed_at: self.closed_at,
            grace_deadline: self.grace_deadline,
            sealed_at: self.sealed_at,
            close_request_id: self.close_request_id.clone(),
            eligible_voters: self.eligible_voters,
            entries: self
                .entries
                .iter()
                .map(|vector| vector.entry.clone())
                .collect(),
        }
    }
}

fn input(
    content: &str,
    ballot_id: &str,
    disposition: SealDisposition,
    weight: u64,
    channel: &str,
) -> VectorEntry {
    VectorEntry {
        content: content.to_string(),
        entry: SealEntry {
            ballot_hash: ballot_hash(content).unwrap(),
            ballot_id: ballot_id.to_string(),
            disposition,
            weight,
            channel: channel.to_string(),
        },
    }
}

fn vector(
    name: &str,
    close_request_id: Option<&str>,
    eligible_voters: u64,
    entries: Vec<VectorEntry>,
) -> Vector {
    let mut vector = Vector {
        name: name.to_string(),
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
        election_event_id: "4f1c2d7e-8b3a-4c55-9e21-6a0d5b7c3e90".to_string(),
        election_id: "dffbd1e9-2858-4cc7-81bc-217c18eda7db".to_string(),
        area_id: "7c797ca2-1972-402c-8e38-3042c5fbbd7a".to_string(),
        closed_at: 1_841_396_472,
        grace_deadline: 1_841_397_372,
        sealed_at: 1_841_397_374,
        close_request_id: close_request_id.map(str::to_string),
        eligible_voters,
        entries,
        sorted_ballot_ids: Vec::new(),
        manifest_borsh_hex: String::new(),
        seal_hash: String::new(),
    };
    let built = build(vector.manifest()).unwrap();
    vector.sorted_ballot_ids = built
        .manifest
        .entries
        .iter()
        .map(|entry| entry.ballot_id.clone())
        .collect();
    vector.manifest_borsh_hex = hex::encode(&built.bytes);
    vector.seal_hash = hex::encode(built.hash);
    vector
}

fn generated() -> VectorFile {
    use SealDisposition::*;
    VectorFile {
        format: SEAL_FORMAT_V1.to_string(),
        about: "Ballot box seal manifest vectors: build(manifest) sorts the entries, \
                manifest_borsh_hex is the manifest's Borsh bytes and seal_hash their SHA-512. \
                ballot_hash is the SHA-512 of content's UTF-8 bytes."
            .to_string(),
        vectors: vec![
            vector("empty box", None, 120, vec![]),
            vector(
                "one ballot",
                Some("b821a9c1-a465-4217-847d-a0158c7c8aae"),
                1,
                vec![input("{\"ballot\":1}", "6a0b0e11", Counted, 1, "ONLINE")],
            ),
            vector(
                "repeated ballots",
                None,
                3,
                vec![
                    input("{\"ballot\":\"same\"}", "1f00aa01", Counted, 1, "ONLINE"),
                    input("{\"ballot\":\"other\"}", "2e00bb02", Counted, 1, "ONLINE"),
                    input("{\"ballot\":\"same\"}", "1f00aa01", Counted, 1, "ONLINE"),
                    input("{\"ballot\":\"same\"}", "0a00cc03", Replaced, 0, "ONLINE"),
                ],
            ),
            vector(
                "every disposition",
                Some("b821a9c1-a465-4217-847d-a0158c7c8aae"),
                4,
                vec![
                    input("{\"ballot\":\"counted\"}", "c0c0c0c0", Counted, 1, "ONLINE"),
                    input(
                        "{\"ballot\":\"delegated\"}",
                        "d0d0d0d0",
                        Counted,
                        3,
                        "KIOSK",
                    ),
                    input(
                        "{\"ballot\":\"replaced\"}",
                        "e0e0e0e0",
                        Replaced,
                        0,
                        "ONLINE",
                    ),
                    input(
                        "{\"ballot\":\"not-eligible\"}",
                        "f0f0f0f0",
                        NotEligible,
                        0,
                        "ONLINE",
                    ),
                    input(
                        "{\"ballot\":\"discarded\"}",
                        "a0a0a0a0",
                        Discarded,
                        0,
                        "ONLINE",
                    ),
                ],
            ),
        ],
    }
}

#[test]
fn the_committed_vectors_still_hold() {
    let text = std::fs::read_to_string(VECTORS_PATH).unwrap();
    let file: VectorFile = serde_json::from_str(&text).unwrap();
    assert_eq!(file.format, SEAL_FORMAT_V1);
    let names: Vec<&str> = file
        .vectors
        .iter()
        .map(|vector| vector.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "empty box",
            "one ballot",
            "repeated ballots",
            "every disposition"
        ]
    );
    for vector in &file.vectors {
        for input in &vector.entries {
            assert_eq!(
                input.entry.ballot_hash,
                ballot_hash(&input.content).unwrap(),
                "{}",
                vector.name
            );
        }
        let built = build(vector.manifest()).unwrap();
        assert_eq!(
            hex::encode(&built.bytes),
            vector.manifest_borsh_hex,
            "{}",
            vector.name
        );
        assert_eq!(hex::encode(built.hash), vector.seal_hash, "{}", vector.name);
        let ids: Vec<&str> = built
            .manifest
            .entries
            .iter()
            .map(|entry| entry.ballot_id.as_str())
            .collect();
        assert_eq!(ids, vector.sorted_ballot_ids, "{}", vector.name);
        let decoded =
            BallotBoxSealManifest::from_bytes(&hex::decode(&vector.manifest_borsh_hex).unwrap())
                .unwrap();
        assert_eq!(decoded, built.manifest, "{}", vector.name);
    }
    let every = &file.vectors[3];
    for disposition in [
        SealDisposition::Counted,
        SealDisposition::Replaced,
        SealDisposition::NotEligible,
        SealDisposition::Discarded,
    ] {
        assert!(every
            .entries
            .iter()
            .any(|input| input.entry.disposition == disposition));
    }
}

/// The empty box's bytes, assembled field by field without the serializer.
#[test]
fn the_empty_box_is_encoded_field_by_field() {
    let text = |value: &str| {
        let mut bytes = (value.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(value.as_bytes());
        bytes
    };
    let vector = &generated().vectors[0];
    let mut expected = text(SEAL_FORMAT_V1);
    expected.extend(text(&vector.tenant_id));
    expected.extend(text(&vector.election_event_id));
    expected.extend(text(&vector.election_id));
    expected.extend(text(&vector.area_id));
    expected.extend(vector.closed_at.to_le_bytes());
    expected.extend(vector.grace_deadline.to_le_bytes());
    expected.extend(vector.sealed_at.to_le_bytes());
    expected.push(0);
    expected.extend(vector.eligible_voters.to_le_bytes());
    expected.extend(0_u32.to_le_bytes());
    assert_eq!(hex::encode(expected), vector.manifest_borsh_hex);
}

/// One entry: the raw 64-byte hash, the Ballot ID, the disposition tag,
/// the weight and the channel.
#[test]
fn an_entry_is_encoded_field_by_field() {
    let entry = SealEntry {
        ballot_hash: [9; 64],
        ballot_id: "id".to_string(),
        disposition: SealDisposition::NotEligible,
        weight: 0,
        channel: "ONLINE".to_string(),
    };
    let mut expected = vec![9u8; 64];
    expected.extend(2_u32.to_le_bytes());
    expected.extend(b"id");
    expected.push(2);
    expected.extend(0_u64.to_le_bytes());
    expected.extend(6_u32.to_le_bytes());
    expected.extend(b"ONLINE");
    assert_eq!(borsh::to_vec(&entry).unwrap(), expected);
}

#[test]
#[ignore = "writes tests/data/ballot_box_seal_v1.json"]
fn write_vectors() {
    let text = serde_json::to_string_pretty(&generated()).unwrap();
    std::fs::write(VECTORS_PATH, text + "\n").unwrap();
}
