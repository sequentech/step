// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use strum::IntoEnumIterator;

/// The values the platform will accept, from the enum the Admin Portal uses.
///
/// Written out rather than derived, because the whole claim of this module is
/// that its variants *are* those values. Deriving the expectation from the code
/// under test would assert nothing.
const OVER_VOTE: &[&str] = &[
    "allowed",
    "allowed-with-msg",
    "allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-alert",
    "not-allowed-with-msg-and-disable",
];
const BLANK_VOTE: &[&str] =
    &["allowed", "warn", "warn-only-in-review", "not-allowed"];
const UNDER_VOTE: &[&str] =
    &["allowed", "warn", "warn-only-in-review", "warn-and-alert"];
const INVALID_VOTE: &[&str] = &[
    "allowed",
    "warn",
    "warn-invalid-implicit-and-explicit",
    "not-allowed",
];
const DUPLICATED_RANK: &[&str] =
    &["allowed-warn-and-dialog", "not-allowed-warn-and-dialog"];
const CANDIDATES_ORDER: &[&str] = &["custom", "alphabetical", "random"];

fn rendered<T: PolicyValue + IntoEnumIterator>() -> Vec<&'static str> {
    T::iter().map(PolicyValue::as_str).collect()
}

// -- the claim this module makes -------------------------------------------

#[test]
fn every_variant_renders_as_a_value_the_platform_has() {
    assert_eq!(rendered::<OverVote>(), OVER_VOTE);
    assert_eq!(rendered::<BlankVote>(), BLANK_VOTE);
    assert_eq!(rendered::<UnderVote>(), UNDER_VOTE);
    assert_eq!(rendered::<InvalidVote>(), INVALID_VOTE);
    assert_eq!(rendered::<DuplicatedRank>(), DUPLICATED_RANK);
    assert_eq!(rendered::<PreferenceGaps>(), DUPLICATED_RANK);
    assert_eq!(rendered::<CandidatesOrder>(), CANDIDATES_ORDER);
}

/// The three mistakes the previous implementations made, each now unwritable.
#[test]
fn the_values_those_mappings_invented_are_not_in_the_value_space() {
    assert!(
        !rendered::<UnderVote>().contains(&"not-allowed"),
        "an under-vote cannot be refused; mapping 'restricted' onto it \
         produced a value the platform does not have"
    );
    assert!(
        !rendered::<OverVote>().contains(&"warn-only-in-review"),
        "an over-vote has no review-only warning"
    );
    assert!(
        rendered::<CandidatesOrder>().contains(&"alphabetical")
            && !rendered::<CandidatesOrder>().contains(&"alphabetic"),
        "the platform spells it 'alphabetical'"
    );
}

#[test]
fn a_value_round_trips_through_serde_as_its_wire_string() {
    let text = serde_json::to_string(&UnderVote::WarnOnlyInReview).unwrap();
    assert_eq!(text, "\"warn-only-in-review\"");
    let read: UnderVote = serde_json::from_str(&text).unwrap();
    assert_eq!(read, UnderVote::WarnOnlyInReview);
}

#[test]
fn something_that_is_not_a_value_will_not_deserialize() {
    assert!(serde_json::from_str::<UnderVote>("\"not-allowed\"").is_err());
}

/// One of the three copies of the value space killed at no cost.
///
/// `contest.hbs` carries the platform's defaults and this module carries the
/// plan's; they have to be the same, or a plan that says nothing about a policy
/// would compile to something other than the template's default.
#[test]
fn the_template_defaults_and_the_plan_defaults_agree() {
    let template: serde_json::Value = {
        let source = include_str!("templates/contest.hbs");
        // Past the handlebars comment, whose `{{!--` would otherwise look like
        // the start of the object.
        let body = source
            .split_once("--}}")
            .map(|(_, rest)| rest)
            .unwrap_or(source);
        serde_json::from_str(body.trim())
            .expect("contest.hbs is JSON once its comment is gone")
    };
    let presentation = &template["presentation"];

    let defaults = Policies::default();
    for (column, cell) in defaults.columns() {
        let key = column
            .strip_prefix("presentation.")
            .expect("every policy is a presentation column");
        let ours = match cell {
            Cell::Text(text) => text,
            other => panic!("a policy should render as text, got {other:?}"),
        };
        assert_eq!(
            presentation[key],
            serde_json::json!(ours),
            "contest.hbs and Policies::default() disagree about {key}"
        );
    }
}

// -- resolving ---------------------------------------------------------------

#[test]
fn an_empty_patch_changes_nothing() {
    let base = Policies::strict();
    assert_eq!(base.apply(&PolicyPatch::default()), base);
    assert!(PolicyPatch::default().is_empty());
}

#[test]
fn a_patch_replaces_only_what_it_names() {
    let patched = Policies::standard().apply(&PolicyPatch {
        over_vote: Some(OverVote::Allowed),
        ..Default::default()
    });

    assert_eq!(patched.over_vote, OverVote::Allowed);
    assert_eq!(
        patched.blank_vote,
        Policies::standard().blank_vote,
        "everything the patch did not name is untouched"
    );
}

#[test]
fn the_presets_differ_where_they_should() {
    assert_eq!(Policies::permissive().over_vote, OverVote::Allowed);
    assert_eq!(
        Policies::strict().over_vote,
        OverVote::NotAllowedWithMsgAndDisable
    );
    assert_eq!(Policies::standard(), Policies::default());
}

/// The one place a "strict" reading has to bend, and it bends toward a value
/// that exists rather than toward one that would be rejected.
#[test]
fn strict_under_voting_warns_loudly_because_it_cannot_refuse() {
    assert_eq!(Policies::strict().under_vote, UnderVote::WarnAndAlert);
}

// -- what it writes ----------------------------------------------------------

#[test]
fn every_policy_writes_the_column_the_builder_reads() {
    let columns: Vec<&str> = Policies::default()
        .columns()
        .into_iter()
        .map(|(column, _)| column)
        .collect();

    assert_eq!(
        columns,
        vec![
            "presentation.over_vote_policy",
            "presentation.blank_vote_policy",
            "presentation.under_vote_policy",
            "presentation.invalid_vote_policy",
            "presentation.duplicated_rank_policy",
            "presentation.preference_gaps_policy",
            "presentation.candidates_order",
        ]
    );
}

/// A patch is what a level says; the resolved set is what a contest gets. The
/// macro generates both from one declaration so a new policy cannot be added to
/// one and forgotten in the other — this asserts they stayed the same size.
#[test]
fn the_patch_covers_exactly_the_policies_that_exist() {
    let resolved = serde_json::to_value(Policies::default()).unwrap();
    let full = PolicyPatch {
        over_vote: Some(OverVote::Allowed),
        blank_vote: Some(BlankVote::Allowed),
        under_vote: Some(UnderVote::Allowed),
        invalid_vote: Some(InvalidVote::Allowed),
        duplicated_rank: Some(DuplicatedRank::AllowedWarnAndDialog),
        preference_gaps: Some(PreferenceGaps::AllowedWarnAndDialog),
        candidates_order: Some(CandidatesOrder::Random),
    };
    let patch = serde_json::to_value(full).unwrap();

    // Key sets, not lengths. Lengths agree by construction — both come from one
    // macro invocation over one field list — so comparing them asserts nothing,
    // and would still pass if the two sides used different *names*.
    let names = |value: &serde_json::Value| -> Vec<String> {
        value.as_object().unwrap().keys().cloned().collect()
    };
    assert_eq!(names(&resolved), names(&patch));
}

/// The catalog hand-writes each plan field name — `kind::<OverVote>("over_vote")`
/// — and nothing else checks them against serde's.
///
/// A typo there makes the picker write `{overvot: "allowed"}`, which `Policies`
/// silently discards: every field is `#[serde(default)]` and unknown keys are
/// ignored. The choice vanishes between the dropdown and the bundle, with no
/// error anywhere.
#[test]
fn the_catalog_names_the_fields_serde_actually_reads() {
    // Sorted, because a `serde_json::Map`'s order is an implementation detail
    // and what is being checked is the *names*.
    let serialised = serde_json::to_value(Policies::default()).unwrap();
    let mut from_serde: Vec<String> =
        serialised.as_object().unwrap().keys().cloned().collect();
    from_serde.sort();

    // The same list `policy_catalog()` builds, in the same order. Kept here
    // rather than imported because `wasm.rs` is behind a feature `cargo test`
    // does not enable, so the catalog itself is never compiled by this suite.
    let mut from_catalog = vec![
        "over_vote".to_string(),
        "blank_vote".to_string(),
        "under_vote".to_string(),
        "invalid_vote".to_string(),
        "duplicated_rank".to_string(),
        "preference_gaps".to_string(),
        "candidates_order".to_string(),
    ];
    from_catalog.sort();

    assert_eq!(from_serde, from_catalog);
}

/// The catalog is what stops the value space acquiring a fourth copy, so it has
/// to actually carry every kind and every value.
#[test]
fn the_catalog_covers_every_policy_the_columns_write() {
    let columns: Vec<&str> = Policies::default()
        .columns()
        .into_iter()
        .map(|(column, _)| column)
        .collect();

    // One catalog entry per column written, or a picker would be missing a
    // control for something the bundle carries.
    let kinds = [
        OverVote::COLUMN,
        BlankVote::COLUMN,
        UnderVote::COLUMN,
        InvalidVote::COLUMN,
        DuplicatedRank::COLUMN,
        PreferenceGaps::COLUMN,
        CandidatesOrder::COLUMN,
    ];
    assert_eq!(columns, kinds);

    // And every kind names a translation namespace, or a front end has nothing
    // to label its values with and invents its own wording.
    for labels in [
        OverVote::LABELS,
        BlankVote::LABELS,
        UnderVote::LABELS,
        InvalidVote::LABELS,
        DuplicatedRank::LABELS,
        PreferenceGaps::LABELS,
        CandidatesOrder::LABELS,
    ] {
        assert!(!labels.is_empty());
    }
}
