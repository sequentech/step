// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::election_config::sources::Sources;
use crate::election_config::Workbook;
use crate::types::ceremonies::CeremoniesPolicy;

use crate::election_config::architect::{
    plan_images, to_workbook, Blueprint, CandidateImage, Contact, PlannedArea,
    PlannedCandidate, PlannedContest, PlannedElection, Schedule, Translated,
    Trustee, VotingChannelSet, BLUEPRINT_VERSION,
};
use crate::election_config::policy::{
    Behaviour, Overrides, PolicyPatch, TallyPatch,
};
use crate::election_config::time::Timestamp;

use crate::election_config::{
    build, BuildOptions, ImportElectionEventSchema, TemplateSet,
};

/// `to_workbook` against the census and files the plan is still carrying.
///
/// One function rather than every call site: when the census leaves `Blueprint`,
/// this is where a test says where its voters come from.
fn workbook_of(plan: &Blueprint) -> Result<Workbook, Problem> {
    to_workbook(plan, &Sources::from_plan(plan))
}

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
        description: Translated::default(),
        voting_channels: VotingChannelSet::default(),
        elections_order: "custom".to_string(),
        show_cast_vote_logs: "show-logs-tab".to_string(),
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
        ceremony_policy: CeremoniesPolicy::MANUAL_CEREMONIES,
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
            description: Translated::default(),
            contests: vec![PlannedContest {
                external_id: "president".to_string(),
                name: Translated::new("President"),
                description: Translated::new("Elects the president"),
                ivr_prompt: Translated::default(),
                max_votes: 1,
                winners: 1,
                allow_writeins: false,
                write_in_slots: 0,
                candidates: vec![
                    PlannedCandidate {
                        external_id: "alice".to_string(),
                        name: Translated::new("Alice"),
                        description: Translated::default(),
                        ivr_prompt: Translated::default(),
                        explicit_blank: false,
                        explicit_invalid: false,
                        disabled: false,

                        image: None,
                    },
                    PlannedCandidate {
                        external_id: "bob".to_string(),
                        name: Translated::new("Bob"),
                        description: Translated::default(),
                        ivr_prompt: Translated::default(),
                        explicit_blank: false,
                        explicit_invalid: false,
                        disabled: false,

                        image: None,
                    },
                ],
                areas: Vec::new(),
                overrides: Overrides::default(),
            }],
            ..Default::default()
        }],
        defaults: Behaviour::default(),
        notes: String::new(),
        ..Default::default()
    }
}

fn built(plan: &Blueprint) -> Bundle {
    let workbook = workbook_of(plan).expect("the plan compiles to rows");
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

/// Byte equality can pass by luck; this cannot.
///
/// Two `HashMap`s with the same keys can happen to iterate the same way, so the
/// test above proves reproducibility only probabilistically. This states the
/// guarantee directly: every object in the document has its keys in name order,
/// at every depth. It is also the test that fails if `to_document` goes back to
/// trusting `serde_json::Value` to sort for it — which it does not do when
/// `preserve_order` is unified into the build by something else.
#[test]
fn a_document_has_its_keys_in_name_order_at_every_depth() {
    fn ordered(value: &serde_json::Value, where_at: &str) {
        match value {
            serde_json::Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                let mut sorted = keys.clone();
                sorted.sort();
                assert_eq!(keys, sorted, "keys out of order at {where_at}");
                for (key, nested) in map {
                    ordered(nested, &format!("{where_at}.{key}"));
                }
            }
            serde_json::Value::Array(items) => {
                for (at, nested) in items.iter().enumerate() {
                    ordered(nested, &format!("{where_at}[{at}]"));
                }
            }
            _ => {}
        }
    }

    let plan = sound();
    let document = preview(&plan).to_document();
    // The i18n maps are the ones that move, so the plan must have two languages
    // for this to be able to fail at all.
    assert!(
        plan.languages.len() > 1,
        "needs a bilingual plan to be a test"
    );
    ordered(&document, "document");
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
/// the branch that needs it: a preview passes the key it wants shown instead.
///
/// Proved by setting the variable to something a preview must not use, rather than
/// by requiring it unset. The devenv shell exports `DEMO_PUBLIC_KEY`, so the old
/// assertion could not hold inside the devcontainer — and a preview ignoring a
/// variable that happens to be absent proves nothing about whether it reads one.
#[test]
fn a_preview_needs_no_environment() {
    std::env::set_var("DEMO_PUBLIC_KEY", "a-key-a-preview-must-never-show");

    let preview = preview(&sound());
    let key = preview.ballot_styles[0]
        .public_key
        .as_ref()
        .expect("a ballot style carries a key field");

    assert_eq!(key.public_key, NOT_A_KEY);
    assert!(key.is_demo);
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
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "north".to_string(),
            name: "North".to_string(),
            parent_external_id: Some("national".to_string()),
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "south".to_string(),
            name: "South".to_string(),
            parent_external_id: Some("national".to_string()),
            allow_early_voting: false,
        },
    ];

    // President is national — everybody votes on it. The regional seat is
    // North's alone.
    plan.elections[0].contests[0].areas = vec!["national".to_string()];
    plan.elections[0].contests.push(PlannedContest {
        external_id: "north-seat".to_string(),
        name: Translated::new("North seat"),
        description: Translated::default(),
        ivr_prompt: Translated::default(),
        max_votes: 1,
        winners: 1,
        allow_writeins: false,
        write_in_slots: 0,
        candidates: vec![PlannedCandidate {
            external_id: "cleo".to_string(),
            name: Translated::new("Cleo"),
            description: Translated::default(),
            ivr_prompt: Translated::default(),
            explicit_blank: false,
            explicit_invalid: false,
            disabled: false,

            image: None,
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
        allow_early_voting: false,
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
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "south".to_string(),
            name: "South".to_string(),
            parent_external_id: None,
            allow_early_voting: false,
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

/// A picker over elections needs their names, for the same reason areas do.
///
/// A ballot style names its election by the id windmill will write. The wizard's
/// preview shows one area × one election, because that is what a voter is handed,
/// so it needs to label the choice — and an election carries no `name` column: the
/// platform keeps it under `presentation.i18n.<lang>.name`.
#[test]
fn the_elections_come_back_named_from_their_presentation() {
    let mut plan = sound();
    plan.elections[0].name = Translated::new("Officers");
    // Cloned and renamed rather than built field by field: a field added to
    // `PlannedElection` later should not break a test about names.
    let mut second = plan.elections[0].clone();
    second.external_id = "bylaws".to_string();
    second.name = Translated::new("Bylaw amendments");
    // Identifiers are unique across the whole event, not per election, so the
    // clone's contests and candidates need their own.
    for contest in &mut second.contests {
        contest.external_id = format!("bylaws-{}", contest.external_id);
        for candidate in &mut contest.candidates {
            candidate.external_id = format!("bylaws-{}", candidate.external_id);
        }
    }
    plan.elections.push(second);

    let bundle = built(&plan);
    let schema: ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone()).unwrap();
    let preview =
        preview_publication(&bundle, &PreviewOptions::default()).unwrap();

    let mut names: Vec<String> = preview
        .elections(&schema)
        .into_iter()
        .map(|election| election.name)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["Bylaw amendments".to_string(), "Officers".to_string()]
    );

    // One entry per election, not one per ballot style: an election with ballots
    // in four areas is still one choice in the picker.
    assert_eq!(preview.elections(&schema).len(), 2);
}

#[test]
fn a_candidates_photograph_previews_from_the_plans_own_bytes() {
    // The bundle names a photograph at `tenant-…/document-…/face.png`, which is
    // relative to `PUBLIC_BUCKET_URL` and correct for a ballot a voter opens after
    // import. A preview has imported nothing and has no bucket, so that path
    // resolves against whatever serves the wizard and 404s — the reviewer sees a
    // broken picture where a photograph is configured, which is the one thing a
    // preview must not misreport. The bytes are in the plan, so the preview uses
    // them.
    let mut plan = sound();
    plan.elections[0].contests[0].candidates[0].image = Some(CandidateImage {
        file_name: "face.png".to_string(),
        // A one-pixel PNG: enough to be encoded and compared, and no byte of it is
        // load-bearing.
        bytes: vec![0x89, b'P', b'N', b'G', 1, 2, 3],
    });

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let bundle = build(
        &workbook,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions {
            images: plan_images(&plan),
            ..BuildOptions::default()
        },
    )
    .expect("a sound plan builds");

    // The bundle that ships is untouched: the file is uploaded and the url points
    // into the bucket, exactly as before.
    let shipped = bundle.export["candidates"][0]["presentation"]["urls"][0]
        ["url"]
        .as_str()
        .expect("the built candidate keeps a bucket path");
    assert!(
        shipped.starts_with("tenant-"),
        "the shipped bundle should still carry a bucket path, got {shipped}"
    );

    let preview = preview_publication(&bundle, &PreviewOptions::default())
        .expect("a sound plan previews");

    let styles = serde_json::to_string(&preview.ballot_styles).unwrap();
    assert!(
        styles.contains("data:image/png;base64,iVBORwECAw=="),
        "the preview should carry the photograph inline, got:\n{styles}"
    );
    assert!(
        !styles.contains("face.png"),
        "no bucket path should survive in the preview:\n{styles}"
    );
}
