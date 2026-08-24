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
        description: Translated::default(),
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
            description: Translated::default(),
            contests: vec![
                PlannedContest {
                    external_id: "president".to_string(),
                    name: Translated::new("President"),
                    description: Translated::default(),
                    max_votes: 1,
                    winners: 1,
                    candidates: vec![PlannedCandidate {
                        external_id: "alice".to_string(),
                        name: Translated::new("Alice"),
                        description: Translated::default(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                PlannedContest {
                    external_id: "board".to_string(),
                    name: Translated::new("Board"),
                    description: Translated::default(),
                    max_votes: 3,
                    winners: 3,
                    ..Default::default()
                },
            ],
            ..Default::default()
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

/// The paths the module docs and the delivery docs both print as the reason
/// this design exists. They were **refused**: `Overrides` and `shared` carry
/// `skip_serializing_if`, so the shape a path is checked against had no such
/// keys, and the motivating example was unimplementable.
#[test]
fn the_paths_the_docs_advertise_are_accepted() {
    for path in [
        "elections[].contests[].overrides.tally.counting_algorithm",
        "elections[].contests[].overrides.policies.over_vote",
        "elections[].shared.tally.counting_algorithm",
        "elections[].shared.policies.over_vote",
        "schedule.voting_opens",
        "schedule.voting_opens.zone",
        "trustee_threshold",
    ] {
        let document = ClientProfile {
            id: "acme".to_string(),
            defaults: defaults(&[(path, Value::from("x"))]),
            locked: vec![path.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_ok(),
            "'{path}' is printed in the docs and must be usable"
        );
    }
}

/// Every scalar the wizard lets a profile fix, whether or not a default plan
/// serialises it.
///
/// `auth_preset` is an `Option` that skips serialising while empty, so the shape
/// a path is checked against had no such key and *How voters sign in* — a setting
/// the client profile builder offers, and one somebody would obviously hide —
/// was refused with `'auth_preset' names nothing a plan has`. A profile
/// downloaded from the builder with that row touched could not be loaded at all,
/// and the message sent whoever met it looking for a typo they had not made.
///
/// The others are here because they are the same shape of field and the mistake
/// is not specific to one of them: any `Option` or `skip_serializing_if` added to
/// `Blueprint` is invisible to `shape_of_a_plan` until somebody fills it in.
///
/// **This list is not the guard.** It is six fields somebody remembered, and it grew
/// by two only because a client met the sixth. The guard is in `beyond`:
/// `check-core-contract.mjs` builds a profile naming *every* path the client profile
/// builder's catalogue offers and asks this crate to accept it, so the catalogue and
/// the shape cannot drift without a red job. That check runs where the catalogue
/// lives, which is why it is not a list retyped here.
#[test]
fn a_profile_may_fix_a_field_an_empty_plan_leaves_out() {
    for path in [
        "auth_preset",
        "logo_url",
        "schedule.key_ceremony",
        "schedule.tally_ceremony",
        // The fifth and sixth, reported from the wizard: both are maps with
        // `skip_serializing_if`, so a profile hiding *Wording overrides* or *Sign-in
        // page wording* — two rows the builder offers side by side — was refused
        // with `'i18n' names nothing a plan has`, and reloading did not help because
        // the document was the thing being refused.
        "i18n",
        "keycloak_messages",
    ] {
        let document = ClientProfile {
            id: "acme".to_string(),
            defaults: defaults(&[(path, Value::from("x"))]),
            hidden: vec![path.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_ok(),
            "'{path}' is a setting the wizard offers and must be usable"
        );
    }
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

/// Hiding a whole screen is the commonest thing a delivery profile does, and it
/// was refused.
///
/// The builder's "All hidden" button writes the step's *prefixes* — `schedule`,
/// `messages`, `voting_channels` — none of which is a settings row, so none of
/// them gets a default seeded. Every such profile was then refused on load, and
/// the only symptom was a wizard that would not start.
#[test]
fn hiding_a_screen_needs_no_default() {
    for path in ["schedule", "messages", "voters", "voting_channels"] {
        let document = ClientProfile {
            id: "acme".to_string(),
            hidden: vec![path.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_ok(),
            "hiding '{path}' is what a delivery profile does and must load"
        );
    }
}

/// The other half, and the reason the rule existed: a *locked* path with nothing
/// to lock to shows the client a value it does not fix.
#[test]
fn locking_without_a_value_is_still_refused() {
    let report = refused(ClientProfile {
        id: "acme".to_string(),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });
    assert!(says(&report, "locked but has no default"));
}

/// An area's identifier is worked out from its name, so locking it needs no value.
///
/// The exception to the rule above, and the reason it is an exception rather than a
/// relaxation: `defaults` holds **one** value per path and applies it to every element
/// the path resolves to. One identifier shared by every area is a duplicate by
/// construction. There is no value to give, so requiring one made *fixed* unreachable
/// for the only two settings on that screen a delivery profile wants to take away.
#[test]
fn a_derived_path_may_be_locked_without_a_value() {
    for path in ["areas[].external_id", "areas[].parent_external_id"] {
        let document = ClientProfile {
            id: "acme".to_string(),
            locked: vec![path.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_ok(),
            "locking '{path}' says the client does not type one, and needs no value"
        );
    }
}

/// And a value for one is refused rather than merely unnecessary.
///
/// `is_fixed` writes a default **unconditionally**, so this would blank every area's
/// identifier on every compile and report "an area needs an identifier" about a box
/// the client cannot see. `EA-F4-052` one level deeper, and this is the door.
#[test]
fn a_derived_path_is_refused_a_default() {
    let report = refused(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("areas[].external_id", Value::from(""))]),
        ..Default::default()
    });
    assert!(says(&report, "is derived per row"));

    // Including the shape that looks harmless: a *name* for every area at once is
    // just as wrong as an identifier for every area at once, and this path is not
    // exempt, so the ordinary rule still refuses nothing here. Checked so the
    // exemption is known to be narrow.
    let allowed = ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("areas[].name", Value::from("North"))]),
        ..Default::default()
    };
    assert!(
        Profile::read(&allowed).is_ok(),
        "only the two derived paths are exempt; the rest of the row is ordinary"
    );
}

/// Hiding one still needs no value, which it never did — checked because the two
/// halves are now decided by different code and could drift apart.
#[test]
fn a_derived_path_may_be_hidden_without_a_value() {
    let document = ClientProfile {
        id: "acme".to_string(),
        hidden: vec![
            "areas[].external_id".to_string(),
            "areas[].parent_external_id".to_string(),
        ],
        ..Default::default()
    };
    assert!(Profile::read(&document).is_ok());
}

/// Hiding does not stop enforcing where a value *is* given — the split is about
/// what a profile must say, not about what it may.
#[test]
fn a_hidden_path_with_a_value_still_fixes_it() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("trustee_threshold", Value::from(5))]),
        hidden: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });
    let plan = Blueprint {
        trustee_threshold: 2,
        ..Default::default()
    };
    let applied = apply_profile(&plan, &profile).expect("applies");
    assert_eq!(applied.trustee_threshold, 5);
}

#[test]
fn a_profile_needs_an_id() {
    let report = refused(ClientProfile::default());
    assert!(says(&report, "a profile needs an id"));
}

/// A lock with nothing to lock *to* enforces nothing: `apply_profile` writes
/// only what `defaults` names. It used to be a warning, which meant a profile
/// could load, claim to fix a field, and fix nothing.
#[test]
fn locking_something_with_no_default_is_refused() {
    let report = refused(ClientProfile {
        id: "acme".to_string(),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });

    assert!(report.has_errors());
    assert!(says(&report, "nothing would be enforced"));
}

/// Proof of the above, at the level that matters: with the lock unenforced, a
/// hand-edited plan simply keeps its own value.
#[test]
fn a_lock_without_a_default_would_have_enforced_nothing() {
    // Constructed directly, since `Profile::read` now refuses this shape.
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[("trustee_threshold", Value::from(3))]),
        locked: vec!["trustee_threshold".to_string()],
        ..Default::default()
    });

    let mut hand_edited = plan();
    hand_edited.trustee_threshold = 999;

    let applied = apply_profile(&hand_edited, &profile).expect("applies");
    assert_eq!(
        applied.trustee_threshold, 3,
        "the default is what enforces it"
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
        .map(|contest| contest.description.get("en").unwrap_or(""))
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
    use crate::election_config::architect::{compile_plan, Compile};
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
        Translated::new("typed by hand");

    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &disagrees,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: Some(&profile),
        sources: None,
    })
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
    use crate::election_config::architect::{compile_plan, Compile};
    use crate::election_config::{BuildOptions, TemplateSet};

    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        required: vec!["logo_url".to_string()],
        ..Default::default()
    });

    let templates = TemplateSet::builtin().unwrap();
    let report = compile_plan(Compile {
        plan: &plan(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: Some(&profile),
        sources: None,
    })
    .expect_err("this build requires a logo");

    assert!(report
        .problems
        .iter()
        .any(|problem| problem.path == "logo_url"));
}

#[test]
fn a_profile_document_round_trips_through_json() {
    let document = ClientProfile {
        presets: Vec::new(),
        only_our_presets: false,
        // `true`, not `false`: the field is `skip_serializing_if` off, so a `false`
        // here would be omitted from the JSON and the round trip would prove nothing
        // about it.
        preview_slim: true,
        auth_presets: Vec::new(),
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

/// A profile may name its own ballot-rule sets, in the client's own words.
#[test]
fn a_profile_may_offer_its_own_presets() {
    let document = ClientProfile {
        id: "smart-td".into(),
        presets: vec![NamedPreset {
            name: "Local officer election".into(),
            about: "How Article 21 says a local ballot runs.".into(),
            values: [
                (
                    "over_vote".to_string(),
                    "not-allowed-with-msg-and-disable".to_string(),
                ),
                ("blank_vote".to_string(), "allowed".to_string()),
            ]
            .into_iter()
            .collect(),
        }],
        ..Default::default()
    };

    let read = Profile::read(&document).expect("this profile is sound");

    assert_eq!(read.presets.len(), 1);
    assert_eq!(read.presets[0].name, "Local officer election");
    // Ours are offered alongside unless the profile says otherwise: an author
    // naming one of a client's ballots should not have to re-describe the
    // general case to keep it.
    assert!(!read.only_our_presets);
}

/// The gate that makes presets safe to put in front of a client.
#[test]
fn a_preset_cannot_invent_a_behaviour_the_platform_lacks() {
    // `not-allowed` is a real value — for blank votes. An under-vote can only
    // ever be warned about, and two earlier versions of this tool emitted
    // exactly this: a contest that imports cleanly and then behaves in a way
    // nobody chose.
    let document = ClientProfile {
        id: "acme".into(),
        presets: vec![NamedPreset {
            name: "Strict".into(),
            values: [("under_vote".to_string(), "not-allowed".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let report = Profile::read(&document).expect_err("this must be refused");

    assert!(report.has_errors());
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path == "presets[0].values.under_vote"),
        "the problem should name the value that is wrong, not the profile: {:?}",
        report.problems
    );
}

/// And a rule that is not a rule at all.
#[test]
fn a_preset_cannot_name_a_rule_that_does_not_exist() {
    let document = ClientProfile {
        id: "acme".into(),
        presets: vec![NamedPreset {
            name: "Strict".into(),
            values: [("over_vote_polciy".to_string(), "allowed".to_string())]
                .into_iter()
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let report = Profile::read(&document).expect_err("a typo must be refused");

    // Ignoring it would give a preset that quietly does less than it says,
    // which is the failure nobody notices until a client asks why their
    // button did nothing.
    assert!(report.has_errors());
}

/// Hiding ours and offering none leaves nothing to choose.
#[test]
fn a_profile_cannot_leave_the_client_with_no_preset_at_all() {
    let document = ClientProfile {
        id: "acme".into(),
        only_our_presets: true,
        ..Default::default()
    };

    let report = Profile::read(&document).expect_err("this must be refused");

    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path == "only_our_presets"),
        "{:?}",
        report.problems
    );
}

/// A preset needs a name; it is what the button says.
#[test]
fn a_preset_needs_a_name() {
    let document = ClientProfile {
        id: "acme".into(),
        presets: vec![NamedPreset {
            name: "   ".into(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert!(Profile::read(&document).is_err());
}

/// A profile may name any field the plan has, including the skipped-when-empty
/// ones.
///
/// Third time this trap has been hit — `Overrides`, then `messages`, now `css`
/// and `i18n`. `shape_of_a_plan` is built from `Blueprint::default()`, and a
/// field with `skip_serializing_if` is simply absent from it, so a profile
/// naming one is refused as a typo. From outside it looks like a wizard that
/// will not start for one client, over a field that plainly exists.
#[test]
fn a_profile_may_name_a_field_that_is_skipped_when_empty() {
    for path in ["css", "i18n", "messages"] {
        let document = ClientProfile {
            id: "acme".to_string(),
            hidden: vec![path.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_ok(),
            "'{path}' is a real field and a profile must be able to name it"
        );
    }
}

/// A starting value reaches a translated field whose map is drawn but blank.
///
/// The reported bug. A new plan's `name` is `{"en": ""}` — the wizard draws a
/// box per language, so the key exists before anybody types — and a shallow
/// emptiness check read that as an answer already given, so the profile's value
/// was skipped and the client saw an empty box above an error asking for a name.
#[test]
fn a_starting_value_fills_a_translated_field_that_is_only_apparently_set() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[(
            "name",
            serde_json::json!({"en": "SMART Elections"}),
        )]),
        ..Default::default()
    });
    let plan = Blueprint {
        name: Translated::new(""),
        ..Default::default()
    };

    let seeded = apply_profile(&plan, &profile).expect("applies");
    assert_eq!(
        seeded.name.by_language.get("en").map(String::as_str),
        Some("SMART Elections")
    );
}

/// And still does not overwrite a real answer.
#[test]
fn a_starting_value_leaves_a_name_somebody_typed_alone() {
    let profile = profile_of(ClientProfile {
        id: "acme".to_string(),
        defaults: defaults(&[(
            "name",
            serde_json::json!({"en": "SMART Elections"}),
        )]),
        ..Default::default()
    });
    let plan = Blueprint {
        name: Translated::new("Their own name"),
        ..Default::default()
    };

    let seeded = apply_profile(&plan, &profile).expect("applies");
    assert_eq!(
        seeded.name.by_language.get("en").map(String::as_str),
        Some("Their own name")
    );
}

/// A card that is a group of controls rather than a group of fields.
///
/// **This was a live bug, not a new feature.** The builder has written
/// `messages.invitation-to-vote` into `hidden` since those two cards existed — it is how
/// somebody switches off *The message telling a voter their ballot is ready* — and
/// `read` refused it, because a message is one of a list and that is not a path any plan
/// has. So switching a messaging card off produced a profile the wizard would not start
/// on. Found by asking the vendored core what it accepts, one path at a time.
#[test]
fn a_card_of_controls_can_be_switched_off() {
    for path in [
        "messages.invitation-to-vote",
        "messages.get-out-the-vote",
        "elections.exchange",
        // Two ballot options a contest offers by *carrying a candidate*: the
        // plan says `explicit_blank` on one of them, and the switch on the
        // screen is one decision per contest rather than one per candidate.
        "elections.contests.blank_vote",
        "elections.contests.decline_to_vote",
    ] {
        let document = ClientProfile {
            id: "acme".to_string(),
            hidden: vec![path.to_string()],
            ..Default::default()
        };
        let read = Profile::read(&document);
        assert!(
            read.is_ok(),
            "a profile hiding the card '{path}' should be readable: {:?}",
            read.err()
        );
    }
}

/// And a typo is still a typo, which is the whole reason the refusal exists.
#[test]
fn a_path_naming_nothing_is_still_refused() {
    for typo in [
        "messages.invitation-to-vot",
        // The two ballot options are the likeliest to be mistyped, because the
        // screen calls them *Blank Vote* and *Decline to Vote* and the plan
        // calls the flags `explicit_blank` and `explicit_invalid`. Neither
        // spelling is the path, and guessing either must fail loudly.
        "elections.contests.explicit_blank",
        "elections.contests.decline",
    ] {
        let document = ClientProfile {
            id: "acme".to_string(),
            hidden: vec![typo.to_string()],
            ..Default::default()
        };
        assert!(
            Profile::read(&document).is_err(),
            "'{typo}' names no control and no field, so it is a typo"
        );
    }
}

/// Paths a profile plainly ought to be able to name, asked of the real shape.
///
/// **The trap this file keeps meeting**, written down as a test rather than as a
/// fourth comment. `shape_of_a_plan` is a hand-filled `Blueprint`, and any field
/// that is `Option` or `skip_serializing_if` is absent from it unless somebody
/// remembered to fill it in — at which point a profile naming that field is
/// refused as a typo, and the report says the path "names nothing a plan has"
/// about a path that plainly exists. `Overrides`, `messages`, `css`/`i18n` and now
/// `candidates[].image` have all been that bug, and each one looked from outside
/// like a wizard that will not start for one client.
#[test]
fn a_profile_can_name_every_optional_corner_of_a_plan() {
    for path in [
        "elections[].contests[].candidates[].image",
        "elections[].contests[].candidates[].disabled",
        "logo",
        "css",
        "i18n",
        "messages",
        "elections[].contests[].overrides.tally.min_votes",
    ] {
        let document = ClientProfile {
            id: "a-client".to_string(),
            hidden: vec![path.to_string()],
            ..Default::default()
        };
        let read = Profile::read(&document);
        assert!(
            read.is_ok(),
            "a profile hiding {path} was refused: {:?}",
            read.err()
        );
    }
}

/// The time zone is a control, and what a profile may say about one.
///
/// It is written onto all four moments rather than stored once — `Timestamp`
/// carries its own zone — so there is no single `schedule.zone` in a plan and
/// there never will be. Hiding *the time zone* is one decision, not four, and the
/// Election Schedule screen has always asked whether it is hidden; what was
/// missing was any way for a profile to say so.
///
/// **Hidden yes, fixed no**, and the asymmetry is the interesting half. `locked`
/// with no default is an error on purpose — a client shown a value they cannot
/// change has to be shown *some* value — and a control cannot carry a default,
/// because there is no path for one to be written to. So this mechanism can take
/// the zone off a client's screen and cannot pin it to Madrid. Pinning it needs a
/// field on `ClientProfile` that the wizard reads, the way `preview_slim` is read,
/// and that is not this.
#[test]
fn the_time_zone_is_a_control_a_profile_may_hide() {
    let hidden = ClientProfile {
        id: "a-client".to_string(),
        hidden: vec!["schedule.zone".to_string()],
        ..Default::default()
    };
    assert!(
        Profile::read(&hidden).is_ok(),
        "a profile hiding the time zone was refused"
    );

    // Refused in `defaults`, like every other control: a value would have nowhere
    // to be written.
    let valued = ClientProfile {
        id: "a-client".to_string(),
        defaults: [(
            "schedule.zone".to_string(),
            serde_json::json!("Europe/Madrid"),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    assert!(
        Profile::read(&valued).is_err(),
        "a default for a control was accepted"
    );

    // And therefore refused when locked, which needs one.
    let locked = ClientProfile {
        id: "a-client".to_string(),
        locked: vec!["schedule.zone".to_string()],
        ..Default::default()
    };
    assert!(
        Profile::read(&locked).is_err(),
        "a control was locked with no value to lock it to"
    );
}
