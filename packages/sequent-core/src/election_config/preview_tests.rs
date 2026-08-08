// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;

use crate::election_config::architect::{
    to_workbook, Blueprint, Contact, PlannedArea, PlannedCandidate,
    PlannedContest, PlannedElection, Schedule, Translated, Trustee,
    BLUEPRINT_VERSION,
};
use crate::election_config::policy::{
    Behaviour, Overrides, PolicyPatch, TallyPatch,
};
use crate::election_config::time::Timestamp;
use crate::election_config::{
    build, BuildOptions, ImportElectionEventSchema, TemplateSet,
};

fn at(local: &str) -> Timestamp {
    Timestamp::new(local, "America/Phoenix", -420)
}

/// One election, one contest, two candidates, one area. The smallest thing that
/// is still a ballot.
fn sound() -> Blueprint {
    Blueprint {
        version: BLUEPRINT_VERSION,
        voters: Vec::new(),
        auth_preset: None,
        external_id: "union-2027".to_string(),
        name: Translated::new("Union Election 2027"),
        languages: vec!["en".to_string(), "es".to_string()],
        default_language: None,
        logo_url: None,
        contacts: vec![Contact {
            name: "Dana Reed".to_string(),
            role: "Returning officer".to_string(),
            email: "dana@example.org".to_string(),
        }],
        trustees: vec![
            Trustee {
                name: "A".to_string(),
                email: "a@example.org".to_string(),
            },
            Trustee {
                name: "B".to_string(),
                email: "b@example.org".to_string(),
            },
            Trustee {
                name: "C".to_string(),
                email: "c@example.org".to_string(),
            },
        ],
        trustee_threshold: 2,
        areas: vec![],
        schedule: Schedule {
            key_ceremony: Some(at("2027-02-01T10:00")),
            voting_opens: Some(at("2027-03-01T09:00")),
            voting_closes: Some(at("2027-03-15T17:00")),
            tally_ceremony: Some(at("2027-03-16T10:00")),
            milestones: Vec::new(),
        },
        elections: vec![PlannedElection {
            shared: None,
            external_id: "officers".to_string(),
            name: Translated::new("Officers"),
            contests: vec![PlannedContest {
                external_id: "president".to_string(),
                name: Translated::new("President"),
                description: "Elects the president".to_string(),
                max_votes: 1,
                winners: 1,
                candidates: vec![
                    PlannedCandidate {
                        external_id: "alice".to_string(),
                        name: Translated::new("Alice"),
                        explicit_blank: false,
                        explicit_invalid: false,
                    },
                    PlannedCandidate {
                        external_id: "bob".to_string(),
                        name: Translated::new("Bob"),
                        explicit_blank: false,
                        explicit_invalid: false,
                    },
                ],
                areas: Vec::new(),
                overrides: Overrides::default(),
            }],
        }],
        defaults: Behaviour::default(),
        notes: String::new(),
    }
}

fn built(plan: &Blueprint) -> Bundle {
    let workbook = to_workbook(plan).expect("the plan compiles to rows");
    let templates = TemplateSet::builtin().unwrap();
    match build(&workbook, &templates, &BuildOptions::default()) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    }
}

fn preview(plan: &Blueprint) -> PublicationPreview {
    preview_publication(&built(plan), &PreviewOptions::default())
        .expect("a sound plan previews")
}

// -- what the preview is ---------------------------------------------------

/// The property everything else rests on.
///
/// If this passes, the preview is not a second reading of the plan: it is what
/// the platform's own ballot-style builder makes of the entities the plan
/// compiled into. Nothing in `preview.rs` decides what a ballot contains.
#[test]
fn a_plan_previews_as_the_ballot_the_platform_would_generate() {
    let preview = preview(&sound());

    assert_eq!(preview.ballot_styles.len(), 1, "one area, one election");
    let style = &preview.ballot_styles[0];

    assert_eq!(style.contests.len(), 1);
    let contest = &style.contests[0];
    assert_eq!(contest.candidates.len(), 2);

    // Order is asserted on `sort_order`, not on the array. The Voting Portal
    // sorts a contest's candidates itself at render time, through the platform's
    // own `sortCandidatesInContest` under the contest's `candidates_order` — so
    // the array order carries no promise, and a test that asserted it would be
    // asserting an accident.
    let order = |name: &str| -> Option<i64> {
        contest
            .candidates
            .iter()
            .find(|candidate| candidate.name.as_deref() == Some(name))
            .and_then(|candidate| candidate.presentation.as_ref())
            .and_then(|presentation| presentation.sort_order)
    };
    assert_eq!(order("Alice"), Some(0));
    assert_eq!(order("Bob"), Some(1));

    assert_eq!(contest.max_votes, 1);
    assert_eq!(contest.winning_candidates_num, 1);
}

/// A preview is a document somebody reads, forwards and compares against the
/// last one. Two things would otherwise make every preview of one plan a
/// different file: windmill mints a `Uuid::new_v4()` per ballot style, which is
/// right for a table row and wrong for a document; and a ballot style's
/// translations live in a `HashMap`, whose serialized key order is whatever the
/// hasher produced. [`PublicationPreview::to_document`] is the answer to the
/// second, and this test is why it exists rather than a plain `to_string`.
#[test]
fn two_previews_of_one_plan_are_the_same_bytes() {
    let plan = sound();
    let first = preview(&plan).to_document().to_string();
    let second = preview(&plan).to_document().to_string();
    assert_eq!(first, second);
}

/// The reason `to_document` is not `to_string`. If this ever stops failing —
/// because `I18nContent` became ordered — `to_document` can go.
#[test]
fn serializing_the_struct_directly_is_not_stable() {
    let plan = sound();
    let ordered: Vec<String> = (0..12)
        .map(|_| serde_json::to_string(&preview(&plan)).unwrap())
        .collect();

    assert!(
        ordered.iter().any(|each| each != &ordered[0]),
        "a HashMap serialized twelve times gave one order every time; if \
         translations are now ordered, delete `to_document`"
    );
}

/// Before the key ceremony there is no key. The platform's own flag says so, and
/// the stand-in is deliberately not a usable one.
#[test]
fn the_preview_key_is_marked_as_not_a_real_key() {
    let preview = preview(&sound());
    let key = preview.ballot_styles[0]
        .public_key
        .as_ref()
        .expect("a ballot style carries a key field");

    assert!(key.is_demo, "a preview must not look like a live ballot");
    assert_eq!(key.public_key, NOT_A_KEY);
}

/// `create_ballot_style` used to read `DEMO_PUBLIC_KEY` from the process
/// environment on its happy path, with a `?`. `std::env::var` always fails on
/// `wasm32-unknown-unknown`, so this call is the whole reason the read moved into
/// the branch that needs it — and a test process has no such variable either,
/// which is why every test in this file would fail without that change.
#[test]
fn a_preview_needs_no_environment() {
    assert!(std::env::var("DEMO_PUBLIC_KEY").is_err());
    preview(&sound());
}

// -- what the file is ------------------------------------------------------

/// The keys `voting-portal/src/routes/PreviewPublicationEvent.tsx` destructures
/// off the document it fetches. A preview the portal cannot open is not a
/// preview; renaming one of these would break it silently, at the point where
/// somebody is looking at a blank screen instead of a ballot.
#[test]
fn the_document_has_the_five_keys_the_voting_portal_reads() {
    let document = preview(&sound()).to_document();
    let object = document.as_object().expect("an object");

    let mut keys: Vec<&String> = object.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "ballot_styles",
            "documents",
            "election_event",
            "elections",
            "support_materials"
        ]
    );
}

// -- districting -----------------------------------------------------------

/// One ballot style per area × election, and an area inherits its ancestors'
/// contests. This is the resolution `elections_contests_for_area` performs for
/// windmill, called here so a preview cannot show a voter a contest they will not
/// be given.
#[test]
fn each_area_gets_the_contests_it_and_its_parents_vote_on() {
    let mut plan = sound();
    plan.areas = vec![
        PlannedArea {
            external_id: "national".to_string(),
            name: "National".to_string(),
            parent_external_id: None,
        },
        PlannedArea {
            external_id: "north".to_string(),
            name: "North".to_string(),
            parent_external_id: Some("national".to_string()),
        },
        PlannedArea {
            external_id: "south".to_string(),
            name: "South".to_string(),
            parent_external_id: Some("national".to_string()),
        },
    ];

    // President is national — everybody votes on it. The regional seat is
    // North's alone.
    plan.elections[0].contests[0].areas = vec!["national".to_string()];
    plan.elections[0].contests.push(PlannedContest {
        external_id: "north-seat".to_string(),
        name: Translated::new("North seat"),
        description: String::new(),
        max_votes: 1,
        winners: 1,
        candidates: vec![PlannedCandidate {
            external_id: "cleo".to_string(),
            name: Translated::new("Cleo"),
            explicit_blank: false,
            explicit_invalid: false,
        }],
        areas: vec!["north".to_string()],
        overrides: Overrides::default(),
    });

    let bundle = built(&plan);
    let schema: ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone()).unwrap();
    let preview =
        preview_publication(&bundle, &PreviewOptions::default()).unwrap();
    assert_eq!(preview.ballot_styles.len(), 3, "three areas, one election");

    // A ballot style names its area by id, so the name has to come back through
    // the entities it was built from.
    let count = |area_name: &str| -> usize {
        let area = schema
            .areas
            .iter()
            .find(|area| area.name.as_deref() == Some(area_name))
            .unwrap_or_else(|| panic!("no area called {area_name}"));
        preview
            .ballot_styles
            .iter()
            .find(|style| style.area_id == area.id)
            .map(|style| style.contests.len())
            .unwrap_or_else(|| panic!("no ballot style for {area_name}"))
    };

    assert_eq!(
        count("North"),
        2,
        "North inherits President and has its own"
    );
    assert_eq!(count("South"), 1, "South only votes on President");
    assert_eq!(count("National"), 1, "the parent has no regional seat");
}

// -- the ballot rules actually reach the ballot ----------------------------

/// The rules screen is the wizard's biggest surface, and until this test the only
/// proof they landed anywhere was a CSV column. Here they are on the contest a
/// voter is handed.
#[test]
fn a_contests_own_rules_reach_the_ballot_a_voter_is_given() {
    let mut plan = sound();
    plan.elections[0].contests[0].overrides = Overrides {
        policies: PolicyPatch {
            over_vote: Some(
                "not-allowed-with-msg-and-disable".parse().unwrap(),
            ),
            ..PolicyPatch::default()
        },
        tally: TallyPatch {
            min_votes: Some(1),
            ..TallyPatch::default()
        },
        ..Overrides::default()
    };

    let preview = preview(&plan);
    let contest = &preview.ballot_styles[0].contests[0];

    assert_eq!(contest.min_votes, 1);
    let presentation = contest
        .presentation
        .as_ref()
        .expect("a contest carries its presentation");
    assert_eq!(
        presentation.over_vote_policy,
        Some(crate::ballot::EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE)
    );
}

// -- the schedule ----------------------------------------------------------

/// The window shown on a preview is the one in
/// `export_scheduled_events-<id>.csv`, which is where the platform reads it from
/// — not the plan's own schedule, which could in principle have failed to reach
/// the bundle.
#[test]
fn the_window_comes_from_the_rows_the_importer_reads() {
    let preview = preview(&sound());
    let dates = preview.ballot_styles[0]
        .election_dates
        .as_ref()
        .expect("a ballot style carries its dates");
    let scheduled = dates
        .scheduled_event_dates
        .as_ref()
        .expect("the plan schedules a voting period");

    assert!(
        scheduled.contains_key("START_VOTING_PERIOD"),
        "expected a start; got {:?}",
        scheduled.keys().collect::<Vec<_>>()
    );
    assert!(scheduled.contains_key("END_VOTING_PERIOD"));
}

// -- refusals --------------------------------------------------------------

/// An event whose areas vote on nothing produces no ballot styles. Showing an
/// empty preview would read as "the ballot is fine and empty" rather than "no
/// voter would be given anything".
#[test]
fn a_plan_nobody_would_get_a_ballot_for_says_so() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".to_string(),
        name: "North".to_string(),
        parent_external_id: None,
    }];
    // A contest assigned to an area that is not North: no area votes on it.
    plan.elections[0].contests[0].areas = vec!["north".to_string()];

    let bundle = built(&plan);
    let mut broken = bundle.clone();
    // Drop the area_contests, which is what "nobody votes on anything" looks
    // like once it has been built.
    if let Some(object) = broken.export.as_object_mut() {
        object.insert(
            "area_contests".to_string(),
            serde_json::Value::Array(Vec::new()),
        );
    }

    let report = preview_publication(&broken, &PreviewOptions::default())
        .expect_err("no ballot means no preview");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.message.contains("no voter would be given")));
}

// -- naming the areas for a picker -----------------------------------------

/// A ballot style names its area by id, because that is what the platform
/// writes. A wizard offering four uuids as a choice is not offering a choice, so
/// the names travel beside the document rather than inside it.
#[test]
fn the_areas_come_back_named_without_changing_the_document() {
    let mut plan = sound();
    plan.areas = vec![
        PlannedArea {
            external_id: "north".to_string(),
            name: "North".to_string(),
            parent_external_id: None,
        },
        PlannedArea {
            external_id: "south".to_string(),
            name: "South".to_string(),
            parent_external_id: None,
        },
    ];
    plan.elections[0].contests[0].areas =
        vec!["north".to_string(), "south".to_string()];

    let bundle = built(&plan);
    let schema: ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone()).unwrap();
    let preview =
        preview_publication(&bundle, &PreviewOptions::default()).unwrap();

    let mut names: Vec<String> = preview
        .areas(&schema)
        .into_iter()
        .map(|area| area.name)
        .collect();
    names.sort();
    assert_eq!(names, vec!["North".to_string(), "South".to_string()]);

    // And the document itself is untouched — still the five keys the platform
    // writes, with no sixth one for the names.
    let document = preview.to_document();
    let mut keys: Vec<&String> = document.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "ballot_styles",
            "documents",
            "election_event",
            "elections",
            "support_materials"
        ]
    );
}
