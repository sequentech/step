// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Step's encoders against Vitrine's profile vectors, copied from Vitrine's
//! `profiles/election/v1/vectors` and `profiles/events/v1/vectors`.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use electoral_log::vitrine::note::split_note;
use electoral_log::vitrine::{
    leaf_hash, ConfigStatement, Demo, ElectionEntry, EventKind, EventsEntry, Hash, MilestoneKind,
    MilestoneStatement, NoteSigner, Release, Subject, Visibility,
};
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;

/// The vectors' authority: `board.example`, with the Ed25519 seed of 32
/// bytes 7.
const BOARD: &str = "board.example";
const BOARD_SEED: [u8; 32] = [7; 32];

fn vectors(profile: &str) -> Vec<(String, Value)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vitrine-vectors")
        .join(profile);
    let mut out: Vec<(String, Value)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| {
            let path = e.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                serde_json::from_str(&text).unwrap(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "no vectors in {}", dir.display());
    out
}

/// Each entry of a vector with its `fields`, by name; `None` for an entry that
/// doesn't parse.
type NamedFields = Vec<(String, Value)>;

fn entries(vector: &Value) -> Vec<(Vec<u8>, Option<NamedFields>)> {
    let entries = vector["entries"].as_array().unwrap();
    let fields = vector["fields"].as_array().unwrap();
    assert_eq!(entries.len(), fields.len());
    entries
        .iter()
        .zip(fields)
        .map(|(e, f)| {
            let bytes = STANDARD.decode(e.as_str().unwrap()).unwrap();
            let named = f.as_array().map(|list| {
                list.iter()
                    .map(|x| (x["name"].as_str().unwrap().to_string(), x["value"].clone()))
                    .collect()
            });
            (bytes, named)
        })
        .collect()
}

fn field<'a>(fields: &'a [(String, Value)], name: &str) -> &'a Value {
    &fields.iter().find(|(n, _)| n == name).unwrap().1
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}

fn hash(hex_text: &str) -> Hash {
    hex::decode(hex_text).unwrap().try_into().unwrap()
}

fn b64_hash(text: &str) -> Hash {
    STANDARD.decode(text).unwrap().try_into().unwrap()
}

#[test]
fn election_entries_match_the_vectors() {
    let mut checked = 0;
    for (name, vector) in vectors("election-v1") {
        for (bytes, fields) in entries(&vector) {
            let Some(fields) = fields else {
                assert!(ElectionEntry::decode(&bytes).is_err(), "{name}: parsed");
                continue;
            };
            let entry = ElectionEntry {
                event_id: text(field(&fields, "event_id")).into(),
                election_id: text(field(&fields, "election_id")).into(),
                ballot_id: hash(text(field(&fields, "ballot_id"))),
                accepted_at: text(field(&fields, "accepted_at")).parse().unwrap(),
            };
            assert_eq!(entry.encode().unwrap(), bytes, "{name}");
            assert_eq!(ElectionEntry::decode(&bytes).unwrap(), entry, "{name}");
            checked += 1;
        }
    }
    assert!(checked > 0);
}

fn subject(v: &str) -> Subject {
    let parts: Vec<&str> = v.split(' ').collect();
    match parts.as_slice() {
        ["none"] => Subject::Nothing,
        ["hash", h] => Subject::Hash(hash(h)),
        ["origin", o] => Subject::Origin(o.to_string()),
        ["ref", o, i] => Subject::Ref {
            origin: o.to_string(),
            index: i.parse().unwrap(),
        },
        _ => panic!("unknown subject {v}"),
    }
}

fn statement_lines(text: &str) -> Vec<(&str, &str)> {
    text.lines()
        .skip(1)
        .map(|l| l.split_once(' ').unwrap())
        .collect()
}

fn one<'a>(lines: &[(&str, &'a str)], key: &str) -> &'a str {
    lines.iter().find(|(k, _)| *k == key).unwrap().1
}

/// Rebuilds a statement's text with step's encoders.
fn rebuild(text: &str) -> Option<String> {
    let kind = text.lines().next().unwrap();
    let lines = match kind {
        "vitrine/config/v1" | "vitrine/milestone/v1" => statement_lines(text),
        _ => return None,
    };
    match kind {
        "vitrine/config/v1" => Some(
            ConfigStatement {
                event_id: one(&lines, "event").into(),
                election_id: one(&lines, "election").into(),
                key: b64_hash(one(&lines, "key")),
                styles: b64_hash(one(&lines, "styles")),
                opens: one(&lines, "opens").parse().unwrap(),
                closes: one(&lines, "closes").parse().unwrap(),
                policy_id: one(&lines, "policy").into(),
                releases: lines
                    .iter()
                    .filter(|(k, _)| *k == "release")
                    .map(|(_, v)| {
                        let (package, version) = v.split_once(' ').unwrap();
                        Release {
                            package: package.into(),
                            version: version.into(),
                        }
                    })
                    .collect(),
                transparency: Visibility::from_str(one(&lines, "transparency")).unwrap(),
                demo: Demo::from_str(one(&lines, "demo")).unwrap(),
                time: one(&lines, "time").parse().unwrap(),
            }
            .text()
            .unwrap(),
        ),
        "vitrine/milestone/v1" => Some(
            MilestoneStatement {
                event_id: one(&lines, "event").into(),
                kind: match one(&lines, "kind") {
                    MilestoneKind::VOTING_OPENED => MilestoneKind::VotingOpened,
                    MilestoneKind::VOTING_CLOSED => MilestoneKind::VotingClosed,
                    other => MilestoneKind::Other(other.into()),
                },
                time: one(&lines, "time").parse().unwrap(),
                configs: lines
                    .iter()
                    .filter(|(k, _)| *k == "config")
                    .map(|(_, v)| {
                        let (e, leaf) = v.split_once(' ').unwrap();
                        (e.to_string(), b64_hash(leaf))
                    })
                    .collect(),
                counts: lines
                    .iter()
                    .filter(|(k, _)| *k == "count")
                    .map(|(_, v)| {
                        let (e, n) = v.split_once(' ').unwrap();
                        (e.to_string(), n.parse().unwrap())
                    })
                    .collect(),
            }
            .text()
            .unwrap(),
        ),
        _ => None,
    }
}

#[test]
fn events_entries_and_statements_match_the_vectors() {
    let board = NoteSigner::from_seed(BOARD, &BOARD_SEED).unwrap();
    let (mut entries_checked, mut statements_signed, mut statements_rebuilt) = (0, 0, 0);
    for (name, vector) in vectors("events-v1") {
        for (bytes, fields) in entries(&vector) {
            let Some(fields) = fields else {
                assert!(EventsEntry::decode(&bytes).is_err(), "{name}: parsed");
                continue;
            };
            let statement = field(&fields, "statement").as_str().map(str::to_string);
            let entry = EventsEntry {
                kind: EventKind::from_str(text(field(&fields, "kind"))).unwrap(),
                time: text(field(&fields, "time")).parse().unwrap(),
                subject: subject(text(field(&fields, "subject"))),
                statement: statement.clone(),
                details: field(&fields, "details").as_str().map(hash),
            };
            assert_eq!(entry.encode().unwrap(), bytes, "{name}");
            assert_eq!(EventsEntry::decode(&bytes).unwrap(), entry, "{name}");
            entries_checked += 1;

            let Some(note) = statement else { continue };
            let (body, signatures) = split_note(&note).unwrap();
            if signatures.len() == 1 && signatures[0].starts_with(&format!("\u{2014} {BOARD} ")) {
                assert_eq!(board.sign(body).unwrap(), note, "{name}: signature");
                statements_signed += 1;
            }
            if let Some(rebuilt) = rebuild(body) {
                assert_eq!(rebuilt, body, "{name}: statement text");
                statements_rebuilt += 1;
            }
        }
    }
    assert!(entries_checked > 0 && statements_signed > 0 && statements_rebuilt > 0);
}

#[test]
fn milestones_name_the_config_entry_by_its_leaf_hash() {
    let (_, vector) = vectors("events-v1")
        .into_iter()
        .find(|(n, _)| n == "passing.json")
        .unwrap();
    let all = entries(&vector);
    let config = &all[0].0;
    let opened = EventsEntry::decode(&all[1].0).unwrap();
    assert_eq!(opened.kind, EventKind::VotingOpened);
    let statement = opened.statement.unwrap();
    let line = statement
        .lines()
        .find(|l| l.starts_with("config "))
        .unwrap();
    assert_eq!(
        line.rsplit(' ').next().unwrap(),
        STANDARD.encode(leaf_hash(config))
    );
}
