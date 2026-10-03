// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A slate is a shortcut for marking candidates. The election carries its
//! definition as configuration, and the ballot carries candidate marks only:
//! there is no slate vote to encode, cast, verify or count.

#![cfg(feature = "default_features")]

use sequent_core::ballot::{
    BallotStyle, Candidate, Contest, HashableBallot, SignedHashableBallot,
};
use sequent_core::ballot_codec::multi_ballot::BallotChoices;
use sequent_core::ballot_codec::PlaintextCodec;
use sequent_core::encrypt::{
    encode_to_plaintext_decoded_multi_contest, encrypt_decoded_contest,
    encrypt_decoded_multi_contest, hash_ballot_style,
    DEFAULT_PUBLIC_KEY_RISTRETTO_STR,
};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::plaintext::{
    map_decoded_ballot_choices_to_decoded_contests, map_to_decoded_contest,
    map_to_decoded_multi_contest, DecodedVoteChoice, DecodedVoteContest,
};
use sequent_core::types::ceremonies::CountingAlgType;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use strand::backend::ristretto::RistrettoCtx;

const SLATES_ANNOTATION: &str = "sequent.slates";
const ELECTION: &str = "officers";

const UNITED: &str = "slate-united";
const FORWARD: &str = "slate-forward";
const VOICES: &str = "slate-voices";

/// The ballot style hash of [`style`] without slates. It changes only if the
/// hashed ballot style gains, loses or reorders a field, which would stop
/// ballots already cast in elections without slates from verifying.
const HASH_WITHOUT_SLATES: &str =
    "f1f95e228ca6db5948226cf56a6062787138f252662a79954793ee13c6f11549";

/// Candidate marks per contest, the only thing a ballot says.
type Marks = BTreeMap<String, BTreeSet<String>>;

fn office(id: &str, seats: i64, candidates: &[&str]) -> Contest {
    Contest {
        id: id.into(),
        election_id: ELECTION.into(),
        min_votes: 0,
        max_votes: seats,
        winning_candidates_num: seats,
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        candidates: candidates
            .iter()
            .map(|candidate| Candidate {
                id: (*candidate).into(),
                contest_id: id.into(),
                election_id: ELECTION.into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

/// Five contests electing seven people: four single-seat offices and three
/// trustees. Each office also has candidates outside every slate.
fn contests() -> Vec<Contest> {
    vec![
        office(
            "president",
            1,
            &["pres-united", "pres-forward", "pres-independent"],
        ),
        office("vice-president", 1, &["vp-united", "vp-forward"]),
        office(
            "secretary-treasurer",
            1,
            &["st-united", "st-forward", "st-independent"],
        ),
        office("recording-secretary", 1, &["rs-united", "rs-forward"]),
        office(
            "trustees",
            3,
            &[
                "tr-united-1",
                "tr-united-2",
                "tr-united-3",
                "tr-forward-1",
                "tr-forward-2",
                "tr-forward-3",
                "tr-voices-1",
                "tr-voices-2",
                "tr-independent",
            ],
        ),
    ]
}

/// Two full slates and one that only runs trustees.
fn slates() -> Value {
    json!({
        "version": 1,
        "slates": [
            {
                "id": UNITED,
                "name": {"en": "United Members"},
                "members": {
                    "president": ["pres-united"],
                    "vice-president": ["vp-united"],
                    "secretary-treasurer": ["st-united"],
                    "recording-secretary": ["rs-united"],
                    "trustees": ["tr-united-1", "tr-united-2", "tr-united-3"]
                }
            },
            {
                "id": FORWARD,
                "name": {"en": "Forward Together"},
                "members": {
                    "president": ["pres-forward"],
                    "vice-president": ["vp-forward"],
                    "secretary-treasurer": ["st-forward"],
                    "recording-secretary": ["rs-forward"],
                    "trustees": ["tr-forward-1", "tr-forward-2", "tr-forward-3"]
                }
            },
            {
                "id": VOICES,
                "name": {"en": "Independent Voices"},
                "members": {"trustees": ["tr-voices-1", "tr-voices-2"]}
            }
        ]
    })
}

fn style(annotations: Option<Value>) -> BallotStyle {
    serde_json::from_value(json!({
        "id": "style", "tenant_id": "tenant", "election_event_id": "event",
        "election_id": ELECTION, "area_id": "area", "contests": contests(),
        "election_presentation": {},
        "election_annotations": annotations,
        "public_key": {
            "public_key": DEFAULT_PUBLIC_KEY_RISTRETTO_STR, "is_demo": true
        }
    }))
    .unwrap()
}

fn style_without_slates() -> BallotStyle {
    style(None)
}

fn style_with_slates() -> BallotStyle {
    style(Some(json!({SLATES_ANNOTATION: slates().to_string()})))
}

/// The members of a slate, read from the published ballot style as a portal
/// would read them.
fn members(style: &BallotStyle, slate_id: &str) -> Marks {
    let annotation =
        &style.election_annotations.as_ref().unwrap()[SLATES_ANNOTATION];
    let configuration: Value = serde_json::from_str(annotation).unwrap();
    let slate = configuration["slates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slate| slate["id"] == slate_id)
        .unwrap();
    serde_json::from_value(slate["members"].clone()).unwrap()
}

fn marks(entries: &[(&str, &[&str])]) -> Marks {
    entries
        .iter()
        .map(|(contest, candidates)| {
            (
                (*contest).to_string(),
                candidates.iter().map(|id| (*id).to_string()).collect(),
            )
        })
        .collect()
}

/// The voter's selection as the Voting Portal holds it: one entry per
/// contest, listing every candidate as marked or not.
fn selection(style: &BallotStyle, marks: &Marks) -> Vec<DecodedVoteContest> {
    style
        .contests
        .iter()
        .map(|contest| DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .map(|candidate| DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: if marks
                        .get(&contest.id)
                        .is_some_and(|marked| marked.contains(&candidate.id))
                    {
                        0
                    } else {
                        -1
                    },
                    write_in_text: None,
                })
                .collect(),
        })
        .collect()
}

/// What a decoded ballot says, reduced to the marked candidates per contest.
fn marked(decoded: &[DecodedVoteContest]) -> Marks {
    decoded
        .iter()
        .map(|contest| {
            assert!(!contest.is_explicit_invalid);
            assert!(contest.invalid_errors.is_empty());
            (
                contest.contest_id.clone(),
                contest
                    .choices
                    .iter()
                    .filter(|choice| choice.is_selected())
                    .map(|choice| choice.id.clone())
                    .collect(),
            )
        })
        .collect()
}

/// The same marks with an empty entry for every contest left alone.
fn on_every_contest(style: &BallotStyle, marks: &Marks) -> Marks {
    style
        .contests
        .iter()
        .map(|contest| {
            (
                contest.id.clone(),
                marks.get(&contest.id).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

/// Every kind of ballot the slate feature can produce.
fn ballots(style: &BallotStyle) -> Vec<Marks> {
    let mut mixed = members(style, UNITED);
    mixed.extend(marks(&[
        ("president", &["pres-independent"]),
        ("vice-president", &[]),
        (
            "trustees",
            &["tr-united-2", "tr-forward-1", "tr-independent"],
        ),
    ]));
    vec![
        members(style, UNITED),
        members(style, FORWARD),
        members(style, VOICES),
        mixed,
        Marks::new(),
    ]
}

fn keys(value: &impl serde::Serialize) -> BTreeSet<String> {
    serde_json::to_value(value)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

fn names(fields: &[&str]) -> BTreeSet<String> {
    fields.iter().map(|field| (*field).to_string()).collect()
}

#[test]
fn a_slate_choice_is_the_selection_of_its_members() {
    let style = style_with_slates();

    // Choosing United and marking its seven candidates one by one, in any
    // order, leave the voting screen holding the same selection.
    let by_hand = marks(&[
        ("trustees", &["tr-united-3", "tr-united-1", "tr-united-2"]),
        ("recording-secretary", &["rs-united"]),
        ("president", &["pres-united"]),
        ("secretary-treasurer", &["st-united"]),
        ("vice-president", &["vp-united"]),
    ]);
    assert_eq!(
        selection(&style, &members(&style, UNITED)),
        selection(&style, &by_hand)
    );

    // A partial slate leaves the contests it does not cover unmarked.
    let voices = selection(&style, &members(&style, VOICES));
    assert_eq!(
        marked(&voices),
        on_every_contest(
            &style,
            &marks(&[("trustees", &["tr-voices-1", "tr-voices-2"])])
        )
    );
}

#[test]
fn the_selection_and_the_encoded_ballot_have_no_slate_field() {
    let style = style_with_slates();
    let united = selection(&style, &members(&style, UNITED));

    assert_eq!(
        keys(&united[0]),
        names(&[
            "contest_id",
            "is_explicit_invalid",
            "is_decline_to_vote",
            "is_blank_ballot",
            "invalid_errors",
            "invalid_alerts",
            "choices",
        ])
    );
    assert_eq!(
        keys(&united[0].choices[0]),
        names(&["id", "selected", "write_in_text"])
    );

    let (_, encoded) =
        encode_to_plaintext_decoded_multi_contest(&united, &style).unwrap();
    assert_eq!(
        keys(&encoded),
        names(&[
            "is_explicit_invalid",
            "is_blank_ballot",
            "choices",
            "counting_algorithm",
        ])
    );
    assert_eq!(
        keys(&encoded.choices[0]),
        names(&["contest_id", "choices", "is_explicit_invalid"])
    );
    assert_eq!(
        keys(&encoded.choices[0].choices[0]),
        names(&["candidate_id", "selected"])
    );
}

#[test]
fn configuring_slates_does_not_change_how_a_contest_is_encoded() {
    let plain = style_without_slates();
    let configured = style_with_slates();

    for marks in ballots(&configured) {
        let votes = selection(&configured, &marks);

        for (index, vote) in votes.iter().enumerate() {
            let code = configured.contests[index]
                .encode_plaintext_contest(vote)
                .unwrap();
            assert_eq!(
                code,
                plain.contests[index]
                    .encode_plaintext_contest(vote)
                    .unwrap()
            );
            let decoded = plain.contests[index]
                .decode_plaintext_contest(&code)
                .unwrap();
            assert_eq!(marked(&[decoded]), marked(&[vote.clone()]));
        }

        let (code, _) =
            encode_to_plaintext_decoded_multi_contest(&votes, &configured)
                .unwrap();
        assert_eq!(
            code,
            encode_to_plaintext_decoded_multi_contest(&votes, &plain)
                .unwrap()
                .0
        );
        let decoded = map_decoded_ballot_choices_to_decoded_contests(
            BallotChoices::decode_from_30_bytes(&code, &plain).unwrap(),
            &plain.contests,
        )
        .unwrap();
        assert_eq!(marked(&decoded), on_every_contest(&plain, &marks));
    }
}

#[test]
fn a_cast_ballot_has_one_ciphertext_per_contest_and_decodes_to_candidates() {
    let style = style_with_slates();
    let contest_ids: Vec<String> = style
        .contests
        .iter()
        .map(|contest| contest.id.clone())
        .collect();

    for marks in ballots(&style) {
        let votes = selection(&style, &marks);
        let audit =
            encrypt_decoded_contest(&RistrettoCtx, &votes, &style).unwrap();

        // The ballot verifier shows what the review screen showed.
        let decoded = map_to_decoded_contest::<RistrettoCtx>(&audit).unwrap();
        assert_eq!(marked(&decoded), on_every_contest(&style, &marks));

        // What is cast names the contests and nothing else.
        let signed = SignedHashableBallot::try_from(&audit).unwrap();
        assert_eq!(
            signed
                .deserialize_contests::<RistrettoCtx>()
                .unwrap()
                .iter()
                .map(|contest| contest.contest_id.clone())
                .collect::<Vec<_>>(),
            contest_ids
        );
        assert_eq!(
            keys(&signed),
            names(&[
                "version",
                "issue_date",
                "contests",
                "config",
                "ballot_style_hash",
                "voter_signing_pk",
                "voter_ballot_signature",
            ])
        );
        let hashable = HashableBallot::try_from(&signed).unwrap();
        assert_eq!(hashable.config, style.id);
        assert_eq!(
            hashable.ballot_style_hash,
            hash_ballot_style(&style).unwrap()
        );
    }
}

#[test]
fn a_cast_multi_contest_ballot_decodes_to_candidates_too() {
    let style = style_with_slates();
    let contest_ids: Vec<String> = style
        .contests
        .iter()
        .map(|contest| contest.id.clone())
        .collect();

    for marks in ballots(&style) {
        let votes = selection(&style, &marks);
        let audit =
            encrypt_decoded_multi_contest(&RistrettoCtx, &votes, &style)
                .unwrap();

        let decoded =
            map_to_decoded_multi_contest::<RistrettoCtx>(&audit).unwrap();
        assert_eq!(marked(&decoded), on_every_contest(&style, &marks));

        let hashable = HashableMultiBallot::try_from(&audit).unwrap();
        assert_eq!(
            hashable
                .deserialize_contests::<RistrettoCtx>()
                .unwrap()
                .contest_ids,
            contest_ids
        );
        assert_eq!(
            keys(&hashable),
            names(&[
                "version",
                "issue_date",
                "contests",
                "config",
                "ballot_style_hash",
            ])
        );
    }
}

#[test]
fn elections_without_slates_keep_their_ballot_style_hash() {
    let plain = style_without_slates();
    assert_eq!(hash_ballot_style(&plain).unwrap(), HASH_WITHOUT_SLATES);

    // Slates reach the ballot style only through the election annotations, so
    // configuring them is a new publication with its own hash.
    let configured = style_with_slates();
    assert_ne!(hash_ballot_style(&configured).unwrap(), HASH_WITHOUT_SLATES);
    assert_eq!(
        BallotStyle {
            election_annotations: None,
            ..configured
        },
        plain
    );
}
