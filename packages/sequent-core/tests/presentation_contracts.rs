// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Presentation fallbacks must retain an election's identity when a translation
//! is absent. These tests distinguish missing text from an explicitly empty name.

#![cfg(feature = "default_features")]

use sequent_core::ballot::*;
use sequent_core::services::translations::{Alias, Name};
use sequent_core::types::hasura::core;
use sequent_core::util::voting_screen::get_contest_plurality;
use serde_json::json;

#[test]
fn contest_names_fall_back_from_requested_language_to_english_to_base_text() {
    let mut contest = get_contest_plurality(
        EOverVotePolicy::ALLOWED,
        EBlankVotePolicy::ALLOWED,
        InvalidVotePolicy::ALLOWED,
        Some(1),
    );
    contest.name = Some("City council".into());
    contest.alias = None;
    contest.name_i18n = None;
    contest.alias_i18n = None;
    assert_eq!(contest.get_name("fr"), "City council");
    contest.name_i18n = Some(
        [("en".into(), Some("Council".into())), ("fr".into(), None)].into(),
    );
    assert_eq!(contest.get_name("fr"), "Council");
    contest
        .name_i18n
        .as_mut()
        .unwrap()
        .insert("fr".into(), Some("Conseil".into()));
    assert_eq!(contest.get_name("fr"), "Conseil");
    contest.alias = Some("Short title".into());
    assert_eq!(contest.get_name("fr"), "Short title");
    contest.alias_i18n =
        Some([("en".into(), Some("Short".into())), ("fr".into(), None)].into());
    assert_eq!(contest.get_name("fr"), "Short");
    contest
        .alias_i18n
        .as_mut()
        .unwrap()
        .insert("fr".into(), Some("Bref".into()));
    assert_eq!(contest.get_name("fr"), "Bref");
}

#[test]
fn election_names_aliases_and_languages_have_independent_fallbacks() {
    let mut election: core::Election = serde_json::from_value(json!({"id": "city", "tenant_id": "north", "election_event_id": "mayor"})).unwrap();
    assert_eq!(election.get_name("fr"), "-");
    assert_eq!(election.get_alias("fr").as_deref(), Some("-"));
    assert_eq!(election.get_default_language(), "en");
    assert_eq!(election.get_presentation(), None);
    election.presentation = Some(
        json!({"i18n": {"en": {"name": "Council", "alias": "City"}, "fr": {"name": "Conseil"}},
        "language_conf": {"default_language_code": "fr"}}),
    );
    assert_eq!(election.get_name("fr"), "Conseil");
    assert_eq!(election.get_name("de"), "Council");
    assert_eq!(election.get_alias("fr").as_deref(), Some("City"));
    assert_eq!(election.get_default_language(), "fr");
    assert!(election.get_presentation().is_some());
    election.presentation = Some(json!("bad presentation"));
    assert_eq!(election.get_default_language(), "en");
    assert_eq!(election.get_name("fr"), "-");
    assert!(election.get_presentation().is_none());
}

#[test]
fn event_policy_getters_respect_explicit_configuration_and_safe_defaults() {
    let mut event: core::ElectionEvent = serde_json::from_value(json!({"id": "mayor", "tenant_id": "north", "is_archived": false, "encryption_protocol": "ELGAMAL"})).unwrap();
    assert_eq!(event.get_default_language(), "en");
    assert_eq!(event.get_name("fr"), "-");
    assert_eq!(
        event.get_contest_encryption_policy(),
        ContestEncryptionPolicy::default()
    );
    assert_eq!(
        event.get_decoded_ballots_inclusion_policy(),
        DecodedBallotsInclusionPolicy::default()
    );
    assert_eq!(
        event.get_delegated_voting_policy(),
        DelegatedVotingPolicy::default()
    );
    assert_eq!(
        event.get_weighted_voting_policy(),
        WeightedVotingPolicy::default()
    );
    assert_eq!(
        event.get_language_detection_policy(),
        LanguageDetectionPolicy::default()
    );
    // Build typed configuration so this test checks accessor routing separately
    // from the wire spelling tests for policy enums.
    let mut presentation = ElectionEventPresentation::default();
    presentation.i18n = Some(
        [(
            "en".into(),
            [("name".into(), Some("Mayoral election".into()))].into(),
        )]
        .into(),
    );
    presentation.contest_encryption_policy =
        Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS);
    presentation.decoded_ballot_inclusion_policy =
        Some(DecodedBallotsInclusionPolicy::INCLUDED);
    presentation.delegated_voting_policy = Some(DelegatedVotingPolicy::ENABLED);
    presentation.weighted_voting_policy =
        Some(WeightedVotingPolicy::AREAS_WEIGHTED_VOTING);
    presentation.language_conf = Some(serde_json::from_value(json!({"default_language_code": "fr", "language_detection_policy": "force-default"})).unwrap());
    event.presentation = Some(serde_json::to_value(presentation).unwrap());
    assert_eq!(event.get_name("fr"), "Mayoral election");
    assert_eq!(event.get_default_language(), "fr");
    assert_eq!(
        event.get_contest_encryption_policy(),
        ContestEncryptionPolicy::MULTIPLE_CONTESTS
    );
    assert_eq!(
        event.get_decoded_ballots_inclusion_policy(),
        DecodedBallotsInclusionPolicy::INCLUDED
    );
    assert_eq!(
        event.get_delegated_voting_policy(),
        DelegatedVotingPolicy::ENABLED
    );
    assert_eq!(
        event.get_weighted_voting_policy(),
        WeightedVotingPolicy::AREAS_WEIGHTED_VOTING
    );
    assert_eq!(
        event.get_language_detection_policy(),
        LanguageDetectionPolicy::FORCE_DEFAULT
    );
}
