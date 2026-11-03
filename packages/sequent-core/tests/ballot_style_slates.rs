// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A ballot style carries an election's slates only when they pass the rules.

#![cfg(feature = "default_features")]

use sequent_core::ballot::{BallotStyle, StringifiedPeriodDates};
use sequent_core::ballot_style::create_ballot_style;
use sequent_core::slates::{InvalidSlates, SLATES_ANNOTATION};
use sequent_core::types::hasura::core::Contest;
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
                "created_at": "2026-09-13T14:30:00+02:00",
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
        serde_json::from_value(input["event"].clone())?,
        serde_json::from_value(input["election"].clone())?,
        contests,
        &all_contests,
        serde_json::from_value(input["candidates"].clone())?,
        StringifiedPeriodDates::default(),
        public_key,
    )
}

fn with_slates(members: Value) -> Value {
    let slates = json!({
        "version": 1,
        "slates": [{
            "id": "conseil-uni",
            "name": {"fr": "Conseil uni"},
            "members": members
        }]
    });
    let mut input = fixture();
    input["election"]["annotations"] =
        json!({SLATES_ANNOTATION: slates.to_string(), "other": "kept"});
    input
}

#[test]
fn a_style_carries_valid_slates_and_drops_nothing() {
    let input = with_slates(
        json!({"contest-a": ["candidate-a", "candidate-b"], "foreign": ["x"]}),
    );
    let style = build(&input, None).unwrap();
    let annotations = style.election_annotations.unwrap();
    assert!(annotations[SLATES_ANNOTATION].contains("conseil-uni"));
    assert_eq!(annotations["other"], "kept");
}

#[test]
fn a_style_is_not_built_from_slates_that_break_the_rules() {
    for members in [
        json!({"contest-a": ["candidate-gone"]}),
        json!({"contest-b": ["candidate-a"]}),
        json!({"contest-b": ["candidate-a", "candidate-b"]}),
    ] {
        let error = build(&with_slates(members), None).unwrap_err();
        let invalid = error
            .downcast_ref::<InvalidSlates>()
            .expect("an invalid slates error");
        assert!(!invalid.problems.is_empty());
        assert!(error.to_string().contains("sequent.slates"));
    }

    let mut wrong_language = with_slates(json!({"contest-a": ["candidate-a"]}));
    wrong_language["election"]["presentation"] =
        json!({"language_conf": {"default_language_code": "de"}});
    assert!(build(&wrong_language, None).is_err());
}
