// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::election_config::architect::{
    PlannedCandidate, PlannedContest, PlannedElection, Translated, Trustee,
    BLUEPRINT_VERSION,
};
use crate::election_config::problem::Severity;

fn plan() -> Blueprint {
    Blueprint {
        version: BLUEPRINT_VERSION,
        external_id: "union-2027".to_string(),
        name: Translated::new("Union Election 2027"),
        trustees: vec![
            Trustee {
                name: "A".to_string(),
                email: "a@example.org".to_string(),
            },
            Trustee {
                name: "B".to_string(),
                email: "b@example.org".to_string(),
            },
        ],
        trustee_threshold: 2,
        elections: vec![PlannedElection {
            shared: None,
            external_id: "officers".to_string(),
            name: Translated::new("Officers"),
            contests: vec![
                PlannedContest {
                    external_id: "president".to_string(),
                    name: Translated::new("President"),
                    max_votes: 1,
                    winners: 1,
                    candidates: vec![PlannedCandidate {
                        external_id: "alice".to_string(),
                        name: Translated::new("Alice"),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                PlannedContest {
                    external_id: "board".to_string(),
                    name: Translated::new("Board"),
                    max_votes: 3,
                    winners: 3,
                    ..Default::default()
                },
            ],
        }],
        ..Default::default()
    }
}

fn profile_of(document: ClientProfile) -> Profile {
    match Profile::read(&document) {
        Ok(profile) => profile,
        Err(report) => panic!("expected a readable profile, got:\n{report}"),
    }
}

fn refused(document: ClientProfile) -> Report {
    Profile::read(&document).expect_err("this profile should be refused")
}

fn says(report: &Report, needle: &str) -> bool {
    report
        .problems
        .iter()
        .any(|problem| problem.message.contains(needle))
}

fn defaults(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(path, value)| (path.to_string(), value.clone()))
        .collect()
}

// -- reading a path --------------------------------------------------------

#[test]
fn a_path_may_name_a_field_a_list_and_a_field_inside_it() {
    let path = PlanPath::parse("elections[].contests[].max_votes")
        .expect("this is the shape the whole design rests on");
    assert_eq!(path.as_str(), "elections[].contests[].max_votes");
}

#[test]
fn an_index_is_refused_because_reordering_a_ballot_would_break_it() {
    let why = PlanPath::parse("elections[0].contests[].max_votes")
        .expect_err("an index should be refused");
    assert!(why.contains("reorders their ballot"), "{why}");
}

#[test]
fn a_pattern_is_refused() {
    assert!(PlanPath::parse("elections[*].max_votes").is_err());
}

#[test]
fn an_empty_segment_is_refused() {
    assert!(PlanPath::parse("elections..max_votes").is_err());
    assert!(PlanPath::parse("").is_err());
}

/// The failure worth catching: a profile with a typo configures nothing at all,
/// and does it silently.
#[test]
fn a_path_naming_a_field_no_plan_has_is_refused() {
    let report = refused(ClientProfile {
        id: "acme".to_string(),
        locked: vec!["trustee_threshhold".to_string()],
        ..Default::default()
    });
    assert!(says(&report, "names nothing a plan has"));
}

#[test]
fn a_path_reaching_into_a_contest_is_accepted() {
    profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[(
            "elections[].contests[].max_votes",
            Value::from(1),
        )]),
        locked: vec!["elections[].contests[].max_votes".to_string()],
        ..Default::default()
    });
}

#[test]
fn a_profile_needs_an_id() {
    let report = refused(ClientProfile::default());
    assert!(says(&report, "a profile needs an id"));
}

/// Locking a path with nothing to lock it *to* fixes it at whatever the plan
/// happens to say — which for a new plan is nothing. Odd rather than wrong, so
/// the profile still reads, and the warning travels with it.
#[test]
fn locking_something_with_no_default_is_a_warning_that_survives() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });

    assert!(!profile.warnings.has_errors());
    assert!(
        says(&profile.warnings, "has no default"),
        "the warning must reach whoever reads the report, not be computed and \
         dropped: {}",
        profile.warnings
    );
}

// -- applying one ----------------------------------------------------------

#[test]
fn a_locked_value_survives_a_plan_that_disagrees() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("trustee_threshold", Value::from(3))]),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });

    let mut disagrees = plan();
    disagrees.trustee_threshold = 1;

    let applied = apply_profile(&disagrees, &profile).expect("applies");
    assert_eq!(applied.trustee_threshold, 3, "the lock has to hold");
}

#[test]
fn a_hidden_value_is_forced_exactly_like_a_locked_one() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("notes", Value::from("set by the profile"))]),
        hidden: vec!["notes".to_string()],
        ..Default::default()
    });

    let mut disagrees = plan();
    disagrees.notes = "typed by hand into the saved file".to_string();

    let applied = apply_profile(&disagrees, &profile).expect("applies");
    assert_eq!(applied.notes, "set by the profile");
}

/// The reason paths are worth having. One entry reaches every contest, however
/// many there are and whatever they are called.
#[test]
fn one_path_with_every_element_reaches_every_contest() {
    let profile = profile_of(ClientProfile {
        id: "smart-td".to_string(),
        defaults: defaults(&[(
            "elections[].contests[].description",
            Value::from("Set for every contest"),
        )]),
        locked: vec!["elections[].contests[].description".to_string()],
        ..Default::default()
    });

    let applied = apply_profile(&plan(), &profile).expect("applies");
    let descriptions: Vec<&str> = applied.elections[0]
        .contests
        .iter()
        .map(|contest| contest.description.as_str())
        .collect();

    assert_eq!(
        descriptions,
        vec!["Set for every contest", "Set for every contest"]
    );
}

/// A default is a starting point, not an override — otherwise opening a saved
/// plan would quietly discard the answers somebody gave.
#[test]
fn an_unlocked_default_seeds_an_empty_field_and_leaves_an_answered_one() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[
            ("notes", Value::from("a starting note")),
            ("external_id", Value::from("should-not-win")),
        ]),
        ..Default::default()
    });

    let applied = apply_profile(&plan(), &profile).expect("applies");

    assert_eq!(applied.notes, "a starting note", "was empty, so seeded");
    assert_eq!(
        applied.external_id, "union-2027",
        "was answered, so left alone"
    );
}

/// Zero and false are answers. Treating them as unset is how a default quietly
/// overwrites a deliberate choice.
#[test]
fn a_zero_counts_as_an_answer_not_as_an_empty_field() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("trustee_threshold", Value::from(5))]),
        ..Default::default()
    });

    let mut chose_zero = plan();
    chose_zero.trustee_threshold = 0;

    let applied = apply_profile(&chose_zero, &profile).expect("applies");
    assert_eq!(applied.trustee_threshold, 0);
}

#[test]
fn a_profile_with_nothing_in_it_changes_nothing() {
    let profile = profile_of(ClientProfile {
        id: "default".to_string(),
        ..Default::default()
    });
    assert_eq!(apply_profile(&plan(), &profile).expect("applies"), plan());
}

/// A plan with no contests yet is not a plan the profile is wrong about.
#[test]
fn a_path_over_an_empty_list_reaches_nothing_and_says_nothing() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[(
            "elections[].contests[].max_votes",
            Value::from(1),
        )]),
        locked: vec!["elections[].contests[].max_votes".to_string()],
        ..Default::default()
    });

    let empty = Blueprint {
        version: BLUEPRINT_VERSION,
        ..Default::default()
    };
    assert!(apply_profile(&empty, &profile).is_ok());
}

#[test]
fn a_default_of_the_wrong_shape_is_reported_rather_than_panicking() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[(
            "trustee_threshold",
            Value::from("not a number"),
        )]),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });

    let report = apply_profile(&plan(), &profile)
        .expect_err("a string is not a threshold");
    assert!(says(&report, "the wrong shape for where it was put"));
}

// -- required --------------------------------------------------------------

/// The path is the point: the wizard routes a problem to the step that owns its
/// path, so a required field lands on the right screen with no extra table.
#[test]
fn a_required_field_left_empty_is_an_error_on_the_path_that_owns_it() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["logo_url".to_string()],
        ..Default::default()
    });

    let mut report = Report::default();
    check_required(&plan(), &profile, &mut report);

    assert_eq!(report.problems.len(), 1);
    assert_eq!(report.problems[0].path, "logo_url");
    assert_eq!(report.problems[0].severity, Severity::Error);
}

#[test]
fn a_required_field_that_is_filled_in_says_nothing() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["external_id".to_string()],
        ..Default::default()
    });

    let mut report = Report::default();
    check_required(&plan(), &profile, &mut report);
    assert!(report.problems.is_empty());
}

#[test]
fn requiring_a_list_that_is_empty_is_an_error() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["contacts".to_string()],
        ..Default::default()
    });

    let mut report = Report::default();
    check_required(&plan(), &profile, &mut report);
    assert_eq!(report.problems.len(), 1, "no contacts were named");
}

#[test]
fn requiring_something_of_every_contest_checks_every_contest() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["elections[].contests[].description".to_string()],
        ..Default::default()
    });

    let mut report = Report::default();
    check_required(&plan(), &profile, &mut report);
    assert_eq!(report.problems.len(), 1, "neither contest has one");
}

// -- what the front end is told --------------------------------------------

#[test]
fn the_hidden_and_locked_paths_are_handed_over_as_they_were_written() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[
            ("notes", Value::from("x")),
            ("logo_url", Value::from("y")),
        ]),
        hidden: vec!["notes".to_string()],
        locked: vec!["logo_url".to_string()],
        ..Default::default()
    });

    assert_eq!(profile.hidden_paths(), vec!["notes"]);
    assert_eq!(profile.locked_paths(), vec!["logo_url"]);
}

// -- through the whole compile ---------------------------------------------

/// The end-to-end property. A locked value has to reach the *bundle*, not just
/// the plan — anything less and the lock is decoration.
#[test]
fn a_locked_value_reaches_the_built_bundle() {
    use crate::election_config::architect::compile_plan;
    use crate::election_config::{BuildOptions, TemplateSet};

    let profile = profile_of(ClientProfile {
        id: "smart-td".to_string(),
        defaults: defaults(&[(
            "elections[].contests[].description",
            Value::from("Locked by the build"),
        )]),
        locked: vec!["elections[].contests[].description".to_string()],
        ..Default::default()
    });

    let mut disagrees = plan();
    disagrees.elections[0].contests[0].description =
        "typed by hand".to_string();

    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(
        &disagrees,
        &templates,
        &BuildOptions::default(),
        Some(&profile),
    )
    .expect("compiles");

    let descriptions: Vec<&str> = compiled.bundle.export["contests"]
        .as_array()
        .expect("contests")
        .iter()
        .map(|contest| contest["description"].as_str().unwrap_or_default())
        .collect();

    assert!(
        descriptions
            .iter()
            .all(|each| *each == "Locked by the build"),
        "the lock has to survive all the way into the bundle: {descriptions:?}"
    );
}

/// A required field the profile adds stops the build, in the plan's own
/// vocabulary, on the path the wizard routes by.
#[test]
fn a_profiles_required_field_stops_the_build() {
    use crate::election_config::architect::compile_plan;
    use crate::election_config::{BuildOptions, TemplateSet};

    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["logo_url".to_string()],
        ..Default::default()
    });

    let templates = TemplateSet::builtin().unwrap();
    let report = compile_plan(
        &plan(),
        &templates,
        &BuildOptions::default(),
        Some(&profile),
    )
    .expect_err("this build requires a logo");

    assert!(report
        .problems
        .iter()
        .any(|problem| problem.path == "logo_url"));
}

#[test]
fn a_profile_document_round_trips_through_json() {
    let document = ClientProfile {
        id: "smart-td".to_string(),
        display_name: Some("SMART TD Locals".to_string()),
        defaults: defaults(&[("trustee_threshold", Value::from(3))]),
        locked: vec!["trustee_threshold".to_string()],
        hidden: vec!["notes".to_string()],
        required: vec!["logo_url".to_string()],
    };

    let text = serde_json::to_string(&document).unwrap();
    let read: ClientProfile = serde_json::from_str(&text).unwrap();

    assert_eq!(read.id, "smart-td");
    assert_eq!(read.locked, vec!["trustee_threshold".to_string()]);
    assert_eq!(read.defaults["trustee_threshold"], Value::from(3));
}
