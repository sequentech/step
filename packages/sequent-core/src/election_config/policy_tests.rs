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

/// The tie-breaking policy is a real decision with a considered default.
///
/// `external-procedure`, not `random`. A random tie-break is defensible and it
/// is also a result nobody can derive from the ballots, which is precisely the
/// result that gets challenged — so the default leaves the tie in the open for
/// somebody to settle under their own rules.
#[test]
fn a_tie_is_left_for_a_person_to_settle_unless_asked_otherwise() {
    assert_eq!(Tally::default().tie_breaking_policy, "external-procedure");

    let random = Tally::default().apply(&TallyPatch {
        tie_breaking_policy: Some("random".to_string()),
        ..TallyPatch::default()
    });
    assert_eq!(random.tie_breaking_policy, "random");
}

/// It is written where the platform reads it.
///
/// `tally_configuration.tie_breaking_policy`, not a bare column: the Admin
/// Portal keeps it under the contest's tally configuration, and a flat
/// `tie_breaking_policy` would land somewhere nothing reads.
#[test]
fn the_tie_breaking_column_is_nested_where_the_contest_keeps_it() {
    let columns = Tally::default().columns();
    assert!(columns
        .iter()
        .any(|(name, _)| *name == "tally_configuration.tie_breaking_policy"));
    assert!(!columns
        .iter()
        .any(|(name, _)| *name == "tie_breaking_policy"));
}

/// One column and nothing collapsible: an ordinary list of candidates.
///
/// These defaults matter more than most. Every contest the wizard has ever built
/// carried whatever `contest.hbs` said, and this is the first time the plan has
/// an opinion — so the opinion has to be the shape almost every ballot wants.
#[test]
fn a_ballot_is_one_plain_column_unless_asked_otherwise() {
    let layout = Layout::default();
    assert_eq!(layout.columns, 1);
    assert_eq!(layout.collapsible_lists, "disabled");
    assert_eq!(layout.enable_checkable_lists, "disabled");
    assert_eq!(layout.max_selections_per_type, 0);
}

/// Every layout value lands under the contest's presentation.
#[test]
fn the_layout_writes_only_presentation_columns() {
    let columns = Layout::default().columns_for_sheet();
    assert_eq!(columns.len(), 4);
    for (name, _) in &columns {
        assert!(
            name.starts_with("presentation."),
            "{name} is not on the presentation"
        );
    }
}

/// A patch replaces what it names and leaves the rest.
#[test]
fn a_layout_patch_touches_only_what_it_names() {
    let two = Layout::default().apply(&LayoutPatch {
        columns: Some(2),
        ..LayoutPatch::default()
    });
    assert_eq!(two.columns, 2);
    // Untouched, rather than reset to the type's own default — which is the same
    // value here, so this is asserted through a level that changed it.
    let three = two.apply(&LayoutPatch {
        collapsible_lists: Some("enabled-collapsed".to_string()),
        ..LayoutPatch::default()
    });
    assert_eq!(three.columns, 2);
    assert_eq!(three.collapsible_lists, "enabled-collapsed");
}

/// An empty layout patch means "whatever the level above decided".
///
/// `Overrides::is_empty` decides whether an election has claimed a decision, and
/// a third group that never reported itself empty would make every election look
/// as though it had.
#[test]
fn a_level_that_says_nothing_about_the_layout_says_nothing_at_all() {
    assert!(Overrides::default().is_empty());
    assert!(!Overrides {
        layout: LayoutPatch {
            columns: Some(2),
            ..LayoutPatch::default()
        },
        ..Overrides::default()
    }
    .is_empty());
}

/// The three groups all reach the sheet.
#[test]
fn a_contest_writes_its_rules_its_counting_and_its_layout() {
    let columns = Behaviour::default().columns();
    let names: Vec<&str> = columns.iter().map(|(name, _)| *name).collect();
    assert!(names.contains(&"presentation.over_vote_policy"));
    assert!(names.contains(&"counting_algorithm"));
    assert!(names.contains(&"presentation.columns"));
}

/// Every field of every group reaches a sheet column.
///
/// The one way these hand-written groups can go wrong quietly. `apply` is a
/// struct literal, so a field added and forgotten there will not compile — but
/// `columns()` builds a `Vec`, and a field left out of it is a setting somebody
/// chose on screen that never reaches the bundle. Nothing else would notice.
///
/// Counted through serde rather than listed, so adding a field to either struct
/// fails this test until its column exists.
#[test]
fn no_setting_is_carried_on_screen_and_dropped_on_the_way_out() {
    fn field_count<T: Serialize>(value: &T) -> usize {
        serde_json::to_value(value)
            .expect("serialisable")
            .as_object()
            .expect("a struct")
            .len()
    }

    let tally = Tally::default();
    assert_eq!(
        field_count(&tally),
        tally.columns().len(),
        "Tally has a field with no column"
    );

    let layout = Layout::default();
    assert_eq!(
        field_count(&layout),
        layout.columns_for_sheet().len(),
        "Layout has a field with no column"
    );

    let policies = Policies::default();
    assert_eq!(
        field_count(&policies),
        policies.columns().len(),
        "Policies has a field with no column"
    );
}

/// The counting rules a `TallyPatch` carries, by name.
///
/// The Election Architect's *Contest Ballot Rules* card names four of these in
/// its client-profile entry — every one except `min_votes`, which the Ballot
/// screen draws in the contest's own row as *Minimum Choices* and which the
/// card therefore must not switch off. That list is written out in
/// `beyond/packages/election-architect/src/profile/sections.ts` as
/// `CONTEST_TALLY_RULES`, and nothing over there can see this struct.
///
/// So a sixth rule added here fails this test, which is where somebody is told
/// the card would silently stop covering it.
#[test]
fn tally_patch_carries_exactly_the_rules_the_architect_lists() {
    let patch = TallyPatch {
        voting_type: Some("plurality-at-large".to_string()),
        counting_algorithm: Some("plurality-at-large".to_string()),
        min_votes: Some(1),
        is_encrypted: Some(true),
        tie_breaking_policy: Some("random".to_string()),
    };
    // Every field set, so `skip_serializing_if` hides none of them.
    let value = serde_json::to_value(&patch).expect("a patch serializes");
    let mut names: Vec<&str> = value
        .as_object()
        .expect("a struct")
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();

    assert_eq!(
        names,
        vec![
            "counting_algorithm",
            "is_encrypted",
            "min_votes",
            "tie_breaking_policy",
            "voting_type",
        ],
        "TallyPatch changed: update CONTEST_TALLY_RULES in sections.ts, and \
         decide whether the new rule belongs on the Contest Ballot Rules card"
    );
}
