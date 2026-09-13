// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the boundary between database records and the ballot sent to the
//! voter. Identity, ordering and election-wide encoding are part of that contract.

#![cfg(feature = "default_features")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

use sequent_core::ballot::{
    BallotStyle, MultiContestEncodingMode, StringifiedPeriodDates,
    TieBreakingPolicy,
};
use sequent_core::ballot_style::{create_ballot_style, parse_i18n_field};
use sequent_core::services::translations::{Alias, Name};
use sequent_core::types::hasura::core::{Contest, Election, ElectionEvent};
use serde_json::{json, Value};
use std::sync::Once;

const TENANT: &str = "fixture-tenant";
const EVENT: &str = "fixture-event";
const ELECTION: &str = "fixture-election";
const DEMO_KEY: &str = "synthetic-demo-public-key";

fn fixture() -> Value {
    json!({
        "area": {"id": "north", "tenant_id": TENANT, "election_event_id": EVENT},
        "event": {"id": EVENT, "tenant_id": TENANT, "is_archived": false, "encryption_protocol": "test"},
        "election": {"id": ELECTION, "tenant_id": TENANT, "election_event_id": EVENT,
            "presentation": {"language_conf": {"default_language_code": "fr"}}},
        "contests": [
            {"id": "contest-b", "tenant_id": TENANT, "election_event_id": EVENT, "election_id": ELECTION,
                "counting_algorithm": "plurality-at-large"},
            {"id": "contest-a", "tenant_id": TENANT, "election_event_id": EVENT, "election_id": ELECTION,
                "counting_algorithm": "plurality-at-large", "min_votes": 1, "max_votes": 2,
                "is_encrypted": true, "winning_candidates_num": 2,
                "presentation": {"i18n": {"fr": {"name": "Conseil", "alias": "Équipe"}}},
                "tally_configuration": {"tie_breaking_policy": "external-procedure"}},
            {"id": "foreign", "tenant_id": TENANT, "election_event_id": EVENT, "election_id": "other-election",
                "counting_algorithm": "plurality-at-large"}
        ],
        "candidates": [
            {"id": "candidate-b", "tenant_id": TENANT, "election_event_id": EVENT, "contest_id": "contest-a"},
            {"id": "candidate-a", "tenant_id": TENANT, "election_event_id": EVENT, "contest_id": "contest-a",
                "presentation": {"i18n": {"fr": {"name": "Camille", "alias": "C."}}}},
            {"id": "foreign-candidate", "tenant_id": TENANT, "election_event_id": EVENT, "contest_id": "foreign"}
        ]
    })
}

fn build(
    input: &Value,
    public_key: Option<String>,
) -> anyhow::Result<BallotStyle> {
    // Every test in this binary uses the same public, synthetic fallback.
    // Set it once before construction; never race environment changes between tests.
    static ENVIRONMENT: Once = Once::new();
    ENVIRONMENT.call_once(|| std::env::set_var("DEMO_PUBLIC_KEY", DEMO_KEY));
    let contests: Vec<Contest> =
        serde_json::from_value(input["contests"].clone())?;
    let all_contests = input
        .get("all_contests")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()?
        .unwrap_or_else(|| contests.clone());

    create_ballot_style(
        "style-north".into(),
        serde_json::from_value(input["area"].clone())?,
        serde_json::from_value(
            input
                .get("event")
                .ok_or_else(|| anyhow::anyhow!("fixture event is missing"))?
                .clone(),
        )?,
        serde_json::from_value(
            input
                .get("election")
                .ok_or_else(|| anyhow::anyhow!("fixture election is missing"))?
                .clone(),
        )?,
        contests,
        &all_contests,
        serde_json::from_value(input["candidates"].clone())?,
        StringifiedPeriodDates::default(),
        public_key,
    )
}

#[test]
fn a_style_keeps_only_its_election_and_orders_candidates_by_identity(
) -> TestResult {
    let style =
        build(&fixture(), Some("synthetic-election-public-key".into()))?;
    assert_eq!(style.tenant_id, TENANT);
    assert_eq!(style.election_id, ELECTION);
    assert_eq!(style.area_id, "north");
    assert_eq!(
        style
            .contests
            .iter()
            .map(|contest| contest.id.as_str())
            .collect::<Vec<_>>(),
        vec!["contest-a", "contest-b"]
    );
    let council = style.contests.first().ok_or("council contest is missing")?;
    assert_eq!(
        council
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        vec!["candidate-a", "candidate-b"]
    );
    assert_eq!(council.name.as_deref(), Some("Conseil"));
    assert_eq!(council.alias.as_deref(), Some("Équipe"));
    assert_eq!(
        council
            .candidates
            .first()
            .ok_or("council candidate is missing")?
            .name
            .as_deref(),
        Some("Camille")
    );
    assert_eq!(
        council
            .candidates
            .first()
            .ok_or("council candidate is missing")?
            .alias
            .as_deref(),
        Some("C.")
    );
    assert_eq!(
        (
            council.min_votes,
            council.max_votes,
            council.winning_candidates_num
        ),
        (1, 2, 2)
    );
    assert_eq!(
        council.tie_breaking_policy,
        Some(TieBreakingPolicy::EXTERNAL_PROCEDURE)
    );
    assert!(council.is_encrypted);
    assert!(style
        .contests
        .get(1)
        .ok_or("second contest is missing")?
        .candidates
        .is_empty());
    assert_eq!(
        style
            .contests
            .get(1)
            .ok_or("second contest is missing")?
            .winning_candidates_num,
        1
    );
    let key = style
        .public_key
        .ok_or("ballot style is missing its public key")?;
    assert!(!key.is_demo);
    assert_eq!(key.public_key, "synthetic-election-public-key");
    Ok(())
}

#[test]
fn fallback_key_is_clearly_marked_as_demo() -> TestResult {
    let key = build(&fixture(), None)?
        .public_key
        .ok_or("ballot style is missing its public key")?;
    assert!(key.is_demo);
    assert_eq!(key.public_key, DEMO_KEY);
    Ok(())
}

#[test]
fn encoding_capacity_is_resolved_for_the_whole_election_not_just_this_area(
) -> TestResult {
    for (policy, expected) in [
        ("allowed", MultiContestEncodingMode::EXPANDED_CAPACITY),
        (
            "allowed-with-msg",
            MultiContestEncodingMode::EXPANDED_CAPACITY,
        ),
        (
            "allowed-with-msg-and-alert",
            MultiContestEncodingMode::EXPANDED_CAPACITY,
        ),
        (
            "not-allowed-with-msg-and-alert",
            MultiContestEncodingMode::LEGACY,
        ),
        (
            "not-allowed-with-msg-and-disable",
            MultiContestEncodingMode::LEGACY,
        ),
    ] {
        let mut input = fixture();
        let mut elsewhere = input
            .pointer("/contests/0")
            .and_then(Value::as_object)
            .ok_or("fixture contest must be an object")?
            .clone();
        elsewhere.insert("id".into(), json!("contest-in-another-area"));
        elsewhere
            .insert("presentation".into(), json!({"over_vote_policy": policy}));
        input
            .as_object_mut()
            .ok_or("fixture must be an object")?
            .insert("all_contests".into(), json!([elsewhere]));
        assert_eq!(
            build(&input, None)?.multi_contest_encoding_mode,
            Some(expected),
            "{policy}"
        );

        // An overvote rule in a different election must not change this one's bytes.
        *input
            .pointer_mut("/all_contests/0/election_id")
            .ok_or("fixture election is missing")? = json!("other-election");
        assert_eq!(
            build(&input, None)?.multi_contest_encoding_mode,
            Some(MultiContestEncodingMode::LEGACY)
        );
    }
    Ok(())
}

#[test]
fn malformed_presentations_and_annotations_do_not_create_partial_styles(
) -> TestResult {
    let cases = [
        ("/event/presentation", json!({"language_conf": 42})),
        ("/event/annotations", json!({"name": []})),
        ("/election/presentation", json!({"language_conf": 42})),
        ("/election/annotations", json!({"name": []})),
        ("/area/presentation", json!({"allow_early_voting": 42})),
        ("/area/annotations", json!({"weight": "many"})),
        (
            "/contests/0/presentation",
            json!({"over_vote_policy": "typo"}),
        ),
        ("/contests/0/annotations", json!({"name": []})),
        ("/contests/0/counting_algorithm", json!("unknown-algorithm")),
        ("/candidates/0/presentation", json!({"is_write_in": "yes"})),
        ("/candidates/0/annotations", json!({"name": []})),
    ];
    for (path, invalid) in cases {
        let mut input = fixture();
        let (parent, field) = path
            .rsplit_once('/')
            .ok_or("fixture path must contain a parent")?;
        input
            .pointer_mut(parent)
            .ok_or("fixture parent is missing")?[field] = invalid;
        assert!(
            build(&input, None).is_err(),
            "accepted invalid field at {path}"
        );
    }
    Ok(())
}

#[test]
fn invalid_election_wide_presentation_is_rejected_even_outside_the_area(
) -> TestResult {
    let mut input = fixture();
    let mut malformed_contest = input
        .pointer("/contests/0")
        .and_then(Value::as_object)
        .ok_or("fixture contest must be an object")?
        .clone();
    malformed_contest
        .insert("presentation".into(), json!({"over_vote_policy": "typo"}));
    input
        .as_object_mut()
        .ok_or("fixture must be an object")?
        .insert("all_contests".into(), json!([malformed_contest]));
    assert!(build(&input, None).is_err());
    Ok(())
}

#[test]
fn translations_fall_back_to_english_and_aliases_fall_back_to_names(
) -> TestResult {
    let input = fixture();
    let mut election: Election = serde_json::from_value(
        input
            .get("election")
            .ok_or("fixture election is missing")?
            .clone(),
    )?;
    election.presentation = Some(json!({"i18n": {
        "fr": {"name": "Conseil", "alias": "Équipe"},
        "en": {"name": "Council", "alias": "Team"}
    }}));
    assert_eq!(election.get_name("fr"), "Conseil");
    assert_eq!(election.get_alias("fr").as_deref(), Some("Équipe"));
    assert_eq!(election.get_name("de"), "Council");
    assert_eq!(election.get_alias("de").as_deref(), Some("Team"));
    election.presentation = Some(json!({"i18n": {"en": {"name": "Council"}}}));
    assert_eq!(election.get_alias("fr").as_deref(), Some("Council"));

    let mut event: ElectionEvent = serde_json::from_value(
        input
            .get("event")
            .ok_or("fixture event is missing")?
            .clone(),
    )?;
    event.presentation = election.presentation.clone();
    assert_eq!(event.get_name("fr"), "Council");
    assert_eq!(event.get_default_language(), "en");
    event.presentation = Some(json!(false));
    assert_eq!(event.get_name("fr"), "-");
    assert_eq!(event.get_default_language(), "en");
    election.presentation = None;
    assert_eq!(election.get_name("fr"), "-");
    assert_eq!(election.get_default_language(), "en");
    Ok(())
}

#[test]
fn extracting_a_translation_preserves_explicit_nulls_and_omits_missing_fields(
) -> TestResult {
    let translations = serde_json::from_value(json!({
        "en": {"name": "Council"}, "fr": {"name": null}, "de": {"alias": "Team"}
    }))?;
    let names = parse_i18n_field(&Some(translations), "name")
        .ok_or("translation fixture should produce a language map")?;
    assert_eq!(names.get("en").and_then(Option::as_deref), Some("Council"));
    assert_eq!(names.get("fr"), Some(&None));
    assert!(!names.contains_key("de"));
    assert!(parse_i18n_field(&None, "name").is_none());
    Ok(())
}
