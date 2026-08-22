// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::election_config::archive::layout;
use crate::election_config::policy::{
    Behaviour, CandidatesOrder, OverVote, Overrides, PolicyPatch, TallyPatch,
};
use crate::election_config::sheet;
use crate::election_config::sources::{Sources, VecCensus};
use crate::election_config::{
    build, validate, BuildOptions, Bundle, ImportElectionEventSchema,
    TemplateSet,
};
use crate::types::ceremonies::CeremoniesPolicy;

/// The census and files the plan itself is still carrying.
///
/// One function rather than a hundred call sites. `Sources` is a separate argument
/// because the census is on its way out of `Blueprint` — when the field goes, this
/// helper becomes the one place a test says where its voters come from, and every
/// test that does not care about a census never learns that it changed.
fn sources_of(plan: &Blueprint) -> Sources {
    Sources::from_plan(plan)
}

/// `validate_plan` against the plan's own data.
fn checked(plan: &Blueprint) -> Report {
    validate_plan(plan, &sources_of(plan))
}

/// `to_workbook` against the plan's own data.
fn workbook_of(plan: &Blueprint) -> Result<Workbook, Problem> {
    to_workbook(plan, &sources_of(plan))
}

/// A moment in a zone that does not observe daylight saving.
///
/// Phoenix rather than Los Angeles on purpose: a March window in California
/// crosses the clock change, which is a real thing worth warning about and a
/// distraction in a fixture every other test builds on. The crossing has its
/// own test.
fn at(local: &str) -> Timestamp {
    Timestamp::new(local, "America/Phoenix", -420)
}

/// A plan somebody could plausibly have filled in, and which has nothing wrong
/// with it. Every test breaks one thing about it.
fn sound() -> Blueprint {
    Blueprint {
        version: BLUEPRINT_VERSION,
        voters: Vec::new(),
        auth_preset: None,
        external_id: "union-2027".to_string(),
        name: Translated::new("Union Election 2027"),
        description: Translated::default(),
        elections_order: "custom".to_string(),
        show_cast_vote_logs: "show-logs-tab".to_string(),
        // Online only, which is both the platform's default and the only
        // combination whose other half is entirely inside the bundle. Each of the
        // other three has its own test.
        voting_channels: VotingChannelSet::default(),
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
            milestones: vec![Milestone {
                event: "Candidate nominations close".to_string(),
                date: "2027-01-15".to_string(),
            }],
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
                areas: vec![],
                overrides: Overrides::default(),
            }],
            ..Default::default()
        }],
        defaults: Behaviour::default(),
        notes: String::new(),
        ..Default::default()
    }
}

fn compiled(plan: &Blueprint) -> Bundle {
    let workbook = workbook_of(plan).expect("the plan compiles to rows");
    let templates = TemplateSet::builtin().unwrap();
    // The same options `compile_plan` builds, so a test asserting on this sees the
    // document the wizard ships rather than a nearby one. The trustees are the only
    // part of a plan that does not travel through the workbook, so they are the
    // only thing this has to add.
    let options = BuildOptions {
        keys_ceremony: (!plan.trustees.is_empty()).then(|| {
            crate::election_config::build::KeysCeremonyPlan {
                trustee_names: plan
                    .trustees
                    .iter()
                    .map(|trustee| trustee.name.clone())
                    .collect(),
                threshold: i64::from(plan.trustee_threshold),
            }
        }),
        // The photographs, for the same reason and by the same route: bytes cannot
        // travel in a workbook cell. Omitting them here made three tests fail with
        // "no image member" against a generator that was writing them correctly —
        // which is the second time this helper drifting from `compile_plan` has
        // produced a wrong answer, the first being the key ceremony above.
        images: plan_images(plan, &sources_of(plan)),
        // And who runs the ceremony, which is the third thing this helper has had
        // to be told after `compile_plan` learned it. The comment above is not
        // rhetorical: every field added to `BuildOptions` that `compile_plan`
        // derives from the plan has to be derived here too, or a test asserts on a
        // bundle nobody ships.
        ceremony_policy: plan.ceremony_policy.clone(),
        // The fourth. Two sessions added one each on the same afternoon, which is
        // the strongest argument yet that this helper should call `compile_plan`
        // rather than reproduce it.
        materials: plan_materials(plan, &sources_of(plan)),
        ..BuildOptions::default()
    };
    match build(&workbook, &templates, &options, &Sources::default()) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    }
}

/// What the bundle validator says about what a plan compiles to.
///
/// Distinct from `validate_plan`, and the distinction matters: a rule about the
/// *bundle* also catches a hand-written workbook, so that is where a rule belongs
/// unless it is about something only a plan carries. These go through the whole
/// pipeline — plan, workbook, templates, builder, schema — so a check that passes
/// here passes on what the importer actually receives.
fn validated(plan: &Blueprint) -> Report {
    let schema: ImportElectionEventSchema =
        serde_json::from_value(compiled(plan).export)
            .expect("a built export deserializes into the import schema");
    validate(&schema)
}

fn codes(report: &Report) -> Vec<String> {
    report
        .problems
        .iter()
        .map(|problem| format!("{:?}", problem.code))
        .collect()
}

fn says(report: &Report, needle: &str) -> bool {
    report
        .problems
        .iter()
        .any(|problem| problem.message.contains(needle))
}

// -- compiling a plan ------------------------------------------------------

/// Until `compile_plan` existed, `to_workbook` and `side_files` had no callers
/// outside this file: the plan could be validated and mapped, but nothing joined
/// those steps to the builder, so no wizard could actually produce anything.
#[test]
fn a_plan_compiles_to_an_archive_the_importer_dispatches_on() {
    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let importable: Vec<&str> = compiled
        .layout
        .importable
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect();

    assert!(importable
        .iter()
        .any(|name| name.starts_with("export_election_event")));
    assert!(!compiled.report.has_errors());
}

/// The ceremony schedule, the contacts, the trustees and the plan itself are not
/// part of an import. Inside the archive they would suggest otherwise.
#[test]
fn the_plans_own_files_travel_beside_the_archive_not_inside_it() {
    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let auxiliary: Vec<&str> = compiled
        .layout
        .auxiliary
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect();
    let importable: Vec<&str> = compiled
        .layout
        .importable
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect();

    for expected in [
        "blueprint.json",
        "ceremony_schedule.json",
        "points_of_contact.json",
        "trustees_list.json",
    ] {
        assert!(
            auxiliary.contains(&expected),
            "{expected} should sit beside the archive; found {auxiliary:?}"
        );
        assert!(
            !importable.contains(&expected),
            "{expected} must not be inside the importable archive"
        );
    }
}

#[test]
fn a_plan_with_errors_produces_no_files_and_says_why() {
    let mut plan = sound();
    plan.trustee_threshold = 9; // more than there are trustees

    let templates = TemplateSet::builtin().unwrap();
    let report = compile_plan(Compile {
        plan: &plan,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect_err("a plan nobody could decrypt must not compile");

    assert!(report.has_errors());
    assert!(says(&report, "could never be decrypted"));
}

// -- the property the whole design rests on --------------------------------

#[test]
fn a_plan_becomes_a_bundle_the_platform_accepts() {
    // The wizard is a different way of filling in the same rows, so it inherits
    // the builder, the templates, the ids, the CSV shapes and the validator. If
    // this passes, none of those had to be written a second time.
    let bundle = compiled(&sound());

    let schema: ImportElectionEventSchema = serde_json::from_value(
        bundle.export.clone(),
    )
    .expect("the compiled export must deserialize into the import schema");

    let report = validate(&schema);
    assert!(
        !report.has_errors(),
        "a sound plan must compile to a valid bundle:\n{report}"
    );
}

#[test]
fn the_same_plan_compiles_to_the_same_bytes_twice() {
    // The TypeScript stamped new Date() into every entity, so no two runs of the
    // same answers agreed. Ids are derived and timestamps fixed, so these do.
    let first = serde_json::to_string(&compiled(&sound()).export).unwrap();
    let second = serde_json::to_string(&compiled(&sound()).export).unwrap();
    assert_eq!(first, second);
}

#[test]
fn it_produces_the_members_the_importer_dispatches_on() {
    // Not `election_config.json` inside a nested `official_election_setup.zip`,
    // which is what the TypeScript wrote and what no importer reads.
    let bundle = compiled(&sound());
    let layout = crate::election_config::archive::layout(&bundle);
    let names: Vec<&str> =
        layout.importable.iter().map(|a| a.name.as_str()).collect();

    assert!(names
        .iter()
        .any(|name| name.starts_with("export_election_event")
            && name.ends_with(".json")));
    assert!(names
        .iter()
        .any(|name| name.starts_with("export_scheduled_events")));
    assert!(!names
        .iter()
        .any(|name| name.contains("election_config.json")));
    assert!(!names
        .iter()
        .any(|name| name.contains("official_election_setup")));
}

#[test]
fn no_realm_is_invented() {
    // The TypeScript embedded a realm copied from one environment. The importer
    // takes keycloak_event_realm wholesale, so that would replace whatever the
    // target environment had provisioned.
    let bundle = compiled(&sound());
    assert_eq!(
        bundle.export["keycloak_event_realm"],
        serde_json::Value::Null
    );
}

// -- what the plan turns into ----------------------------------------------

#[test]
fn every_election_contest_and_candidate_arrives() {
    let bundle = compiled(&sound());
    assert_eq!(bundle.export["elections"].as_array().unwrap().len(), 1);
    assert_eq!(bundle.export["contests"].as_array().unwrap().len(), 1);
    assert_eq!(bundle.export["candidates"].as_array().unwrap().len(), 2);
}

#[test]
fn names_arrive_in_every_language_the_plan_enables() {
    let mut plan = sound();
    plan.name
        .by_language
        .insert("es".to_string(), "Elección Sindical 2027".to_string());
    let bundle = compiled(&plan);

    let i18n = &bundle.export["election_event"]["presentation"]["i18n"];
    assert_eq!(i18n["en"]["name"], serde_json::json!("Union Election 2027"));
    assert_eq!(
        i18n["es"]["name"],
        serde_json::json!("Elección Sindical 2027")
    );
}

#[test]
fn a_missing_translation_falls_back_rather_than_leaving_a_blank_ballot_line() {
    // Spanish is enabled and the contest has no Spanish name. Showing nothing
    // would be worse than showing the English.
    let bundle = compiled(&sound());
    let i18n = &bundle.export["contests"][0]["presentation"]["i18n"];
    assert_eq!(i18n["en"]["name"], serde_json::json!("President"));
    assert_eq!(i18n["es"]["name"], serde_json::json!("President"));
}

#[test]
fn the_enabled_languages_reach_the_login_page() {
    // Nothing else in the platform sets supportedLocales, so this is the only
    // thing that puts a language in Keycloak's picker.
    let bundle = compiled(&sound());
    assert_eq!(
        bundle.realm_patch.patch["supportedLocales"],
        serde_json::json!(["en", "es"])
    );
    assert_eq!(
        bundle.realm_patch.patch["displayName"],
        serde_json::json!("Union Election 2027")
    );
}

#[test]
fn a_plan_with_no_language_still_produces_a_readable_ballot() {
    let mut plan = sound();
    plan.languages = vec![];
    let bundle = compiled(&plan);
    assert_eq!(
        bundle.export["election_event"]["presentation"]["i18n"]["en"]["name"],
        serde_json::json!("Union Election 2027")
    );
}

#[test]
fn the_voting_window_becomes_the_scheduled_events_the_importer_reads() {
    let bundle = compiled(&sound());
    assert_eq!(bundle.scheduled_events.len(), 2);

    // Event-wide: the wizard asks once, and that covers every election.
    let payload = &bundle.scheduled_events.rows[0][10];
    assert_eq!(
        *payload,
        crate::election_config::JsonField::Value(
            serde_json::json!({"election_id": null})
        )
    );
}

#[test]
fn a_plan_with_no_dates_still_builds_and_says_it_needs_hands() {
    let mut plan = sound();
    plan.schedule = Schedule::default();
    let bundle = compiled(&plan);
    assert!(bundle.scheduled_events.is_empty());
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("by hand")));
}

#[test]
fn every_contest_lands_on_the_one_area() {
    // The wizard does not do districting, so one area covers everybody. Without
    // it there is no ballot and no voter sees anything.
    let bundle = compiled(&sound());
    assert_eq!(bundle.export["areas"].as_array().unwrap().len(), 1);
    assert_eq!(
        bundle.export["areas"][0]["name"],
        serde_json::json!(DEFAULT_AREA_NAME)
    );
    assert_eq!(bundle.export["area_contests"].as_array().unwrap().len(), 1);
}

/// The event's defaults reach every contest, in the platform's own words —
/// no mapping step, so nothing to be lossy about.
#[test]
fn the_event_defaults_reach_every_contest() {
    let mut plan = sound();
    plan.defaults.policies.over_vote = OverVote::Allowed;
    plan.defaults.policies.candidates_order = CandidatesOrder::Random;

    let presentation =
        compiled(&plan).export["contests"][0]["presentation"].clone();

    assert_eq!(
        presentation["over_vote_policy"],
        serde_json::json!("allowed")
    );
    assert_eq!(
        presentation["candidates_order"],
        serde_json::json!("random")
    );
}

/// The gap that made this worth doing: until contests carried a tally, every
/// one of them took the template's plurality-at-large, so the wizard could not
/// produce a ranked election at all.
#[test]
fn a_contest_can_be_preferential() {
    let mut plan = sound();
    plan.elections[0].contests[0].overrides.tally = TallyPatch {
        voting_type: Some("preferential".to_string()),
        counting_algorithm: Some("instant-runoff".to_string()),
        ..Default::default()
    };

    let contest = compiled(&plan).export["contests"][0].clone();

    assert_eq!(contest["voting_type"], serde_json::json!("preferential"));
    assert_eq!(
        contest["counting_algorithm"],
        serde_json::json!("instant-runoff")
    );
}

#[test]
fn a_contest_overrides_the_event_default() {
    let mut plan = sound();
    plan.defaults.policies.over_vote = OverVote::Allowed;
    plan.elections[0].contests[0].overrides.policies = PolicyPatch {
        over_vote: Some(OverVote::NotAllowedWithMsgAndDisable),
        ..Default::default()
    };

    let presentation =
        compiled(&plan).export["contests"][0]["presentation"].clone();
    assert_eq!(
        presentation["over_vote_policy"],
        serde_json::json!("not-allowed-with-msg-and-disable")
    );
}

/// An election that has claimed the decision does not consult its contests.
/// The alternative — copying the shared value onto each contest — is how
/// "shared" goes stale the first time somebody edits one of them.
#[test]
fn an_election_that_shares_one_set_ignores_its_contests() {
    let mut plan = sound();
    plan.elections[0].shared = Some(Overrides {
        policies: PolicyPatch {
            over_vote: Some(OverVote::Allowed),
            ..Default::default()
        },
        ..Default::default()
    });
    plan.elections[0].contests[0].overrides.policies = PolicyPatch {
        over_vote: Some(OverVote::NotAllowedWithMsgAndDisable),
        ..Default::default()
    };

    let presentation =
        compiled(&plan).export["contests"][0]["presentation"].clone();
    assert_eq!(
        presentation["over_vote_policy"],
        serde_json::json!("allowed"),
        "the election claimed it, so the contest is not consulted"
    );
}

/// Was hard-coded to zero with a comment saying the wizard does not ask, so
/// "rank at least three" was unexpressible.
#[test]
fn a_contest_can_require_a_minimum_number_of_choices() {
    let mut plan = sound();
    plan.elections[0].contests[0].overrides.tally.min_votes = Some(3);

    assert_eq!(
        compiled(&plan).export["contests"][0]["min_votes"],
        serde_json::json!(3)
    );
}

/// A plan saved under version 1 has already been reviewed by somebody. It has
/// to compile to the bytes it always did — including where version 1's mapping
/// was wrong, because fixing it here would change an approved election.
#[test]
fn a_version_one_plan_compiles_to_the_bytes_it_used_to() {
    let document = r#"{
        "version": 1,
        "external_id": "old",
        "name": {"en": "Old"},
        "elections": [{
            "external_id": "e",
            "name": {"en": "E"},
            "contests": [{
                "external_id": "c",
                "name": {"en": "C"},
                "max_votes": 1,
                "winners": 1,
                "candidates": [{"external_id": "a", "name": {"en": "A"}}]
            }]
        }],
        "policies": {
            "over_vote": "restricted",
            "blank_vote": "allowed",
            "under_vote": "restricted",
            "invalid_vote": "restricted"
        }
    }"#;

    let plan = read_plan(document).expect("an older plan still opens").plan;
    assert_eq!(plan.version, BLUEPRINT_VERSION);

    let presentation =
        compiled(&plan).export["contests"][0]["presentation"].clone();
    assert_eq!(
        presentation["over_vote_policy"],
        serde_json::json!("not-allowed-with-msg-and-disable")
    );
    assert_eq!(
        presentation["blank_vote_policy"],
        serde_json::json!("allowed")
    );
    assert_eq!(
        presentation["under_vote_policy"],
        serde_json::json!("warn-only-in-review"),
        "version 1 mapped 'restricted' here to a warning; reproduced, not fixed"
    );
    assert_eq!(
        presentation["invalid_vote_policy"],
        serde_json::json!("not-allowed")
    );
}

#[test]
fn a_multi_winner_contest_elects_what_the_plan_says() {
    // The TypeScript hard-coded winning_candidates_num to 1 while letting
    // max_votes be anything, so "choose 3" silently elected one person.
    let mut plan = sound();
    let contest = &mut plan.elections[0].contests[0];
    contest.max_votes = 3;
    contest.winners = 3;
    contest.candidates.push(PlannedCandidate {
        external_id: "carol".to_string(),
        name: Translated::new("Carol"),
        description: Translated::default(),
        ivr_prompt: Translated::default(),
        explicit_blank: false,
        explicit_invalid: false,
        disabled: false,

        image: None,
    });

    let bundle = compiled(&plan);
    assert_eq!(
        bundle.export["contests"][0]["max_votes"],
        serde_json::json!(3)
    );
    assert_eq!(
        bundle.export["contests"][0]["winning_candidates_num"],
        serde_json::json!(3)
    );
}

#[test]
fn a_blank_option_is_marked_as_one_rather_than_becoming_a_candidate() {
    let mut plan = sound();
    plan.elections[0].contests[0]
        .candidates
        .push(PlannedCandidate {
            external_id: "none-of-the-above".to_string(),
            name: Translated::new("None of the above"),
            description: Translated::default(),
            ivr_prompt: Translated::default(),
            explicit_blank: true,
            explicit_invalid: false,
            disabled: false,

            image: None,
        });

    let bundle = compiled(&plan);
    let candidates = bundle.export["candidates"].as_array().unwrap();
    let blank = candidates
        .iter()
        .find(|c| c["external_id"] == serde_json::json!("none-of-the-above"))
        .expect("the blank option");
    assert_eq!(
        blank["presentation"]["is_explicit_blank"],
        serde_json::json!(true)
    );
}

/// A candidate who stood down is delivered as one, rather than as a candidate.
///
/// The wizard has drawn this checkbox for a while. `PlannedCandidate` had no field
/// for it and no `#[serde(flatten)]` catch-all, so every tick was accepted on
/// screen, dropped here, absent from the delivery, and gone on reopen — the worst
/// shape a setting can have, because nothing anywhere said it had not worked.
#[test]
fn a_candidate_who_stood_down_is_delivered_as_disabled() {
    let mut plan = sound();
    plan.elections[0].contests[0]
        .candidates
        .push(PlannedCandidate {
            external_id: "stood-down".to_string(),
            name: Translated::new("Dana"),
            description: Translated::default(),
            ivr_prompt: Translated::default(),
            explicit_blank: false,
            explicit_invalid: false,
            disabled: true,
            image: None,
        });

    let bundle = compiled(&plan);
    let candidates = bundle.export["candidates"].as_array().unwrap();
    let dana = candidates
        .iter()
        .find(|c| c["external_id"] == serde_json::json!("stood-down"))
        .expect("the candidate who stood down");
    assert_eq!(dana["presentation"]["is_disabled"], serde_json::json!(true));
    // And the ones still standing are not, which is what makes the assertion above
    // about this candidate rather than about the column's default.
    let alice = candidates
        .iter()
        .find(|c| c["external_id"] == serde_json::json!("alice"))
        .expect("a candidate still standing");
    assert_eq!(
        alice["presentation"]["is_disabled"],
        serde_json::json!(false)
    );
}

#[test]
fn the_order_things_were_arranged_in_survives() {
    // The wizard lets somebody drag candidates into an order. Losing it means a
    // ballot in a different order than the one they approved.
    let mut plan = sound();
    plan.elections[0].contests[0].candidates.reverse();
    let bundle = compiled(&plan);

    let candidates = bundle.export["candidates"].as_array().unwrap();
    let bob = candidates
        .iter()
        .find(|c| c["external_id"] == serde_json::json!("bob"))
        .unwrap();
    assert_eq!(bob["presentation"]["sort_order"], serde_json::json!(0));
}

// -- candidate photographs ---------------------------------------------------

/// The sound plan with a photograph on Alice.
fn with_photograph() -> Blueprint {
    let mut plan = sound();
    plan.elections[0].contests[0].candidates[0].image = Some(CandidateImage {
        file_name: "alice.png".to_string(),
        bytes: vec![0x89, 0x50, 0x4e, 0x47],
    });
    plan
}

/// The identifier the plan derives for Alice's photograph.
fn alice_document() -> String {
    let ids = IdFactory::new("union-2027").expect("a namespace");
    image_document_id(&ids, "alice")
}

#[test]
fn a_photograph_reaches_all_three_places_it_has_to() {
    // The whole contract in one test, because the parts are only correct together:
    // the identifier on the candidate, the url a voter's ballot reads, and the
    // archive member the importer uploads. Two of three is a broken ballot.
    let plan = with_photograph();
    let bundle = compiled(&plan);
    let document = alice_document();

    let alice = bundle.export["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .find(|each| each["external_id"] == serde_json::json!("alice"))
        .expect("alice is in the bundle")
        .clone();

    assert_eq!(alice["image_document_id"], serde_json::json!(document));

    let urls = alice["presentation"]["urls"]
        .as_array()
        .expect("an urls array");
    let image = urls
        .iter()
        .find(|url| url["is_image"] == serde_json::json!(true))
        .expect("an image url");
    // Bucket-relative and naming the same document: the portal concatenates this
    // onto `PUBLIC_BUCKET_URL`, and import rewrites the identifier here and on the
    // field above through one map — which only keeps them together because they
    // are the same string now.
    assert_eq!(
        image["url"],
        serde_json::json!(format!(
            "tenant-{}/document-{document}/alice.png",
            bundle.tenant_id
        ))
    );

    let entry = format!("images/document_{document}_alice.png");
    let member = layout(&bundle)
        .importable
        .into_iter()
        .find(|artifact| artifact.name == entry)
        .expect("the picture is inside the zip");
    assert_eq!(member.bytes, vec![0x89, 0x50, 0x4e, 0x47]);
}

#[test]
fn a_photograph_survives_the_bundle_validator() {
    // `check_images` is the rule that would refuse a mismatch, so a plan the wizard
    // built has to satisfy it. If this fails, the generator and the checker
    // disagree about the contract and one of them is wrong.
    let report = validated(&with_photograph());
    assert!(!report.has_errors(), "{report}");
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.path.starts_with("candidates")),
        "{report}"
    );
}

#[test]
fn two_builds_of_one_plan_produce_the_same_picture_entry() {
    // The identifier is derived from the candidate's `external_id`, not minted, so
    // regenerating a plan reuses it — which is what makes a rebuild a diff somebody
    // can read rather than a new document.
    let plan = with_photograph();
    let names = |bundle: &Bundle| -> Vec<String> {
        layout(bundle)
            .importable
            .into_iter()
            .map(|artifact| artifact.name)
            .filter(|name| name.starts_with("images/"))
            .collect()
    };
    assert_eq!(names(&compiled(&plan)), names(&compiled(&plan)));
    assert_eq!(names(&compiled(&plan)).len(), 1);
}

#[test]
fn a_candidate_with_no_photograph_carries_no_url_and_no_member() {
    // The ordinary case, and the one that must not grow an empty `urls` array: the
    // portal's `getImageUrl` takes the first `is_image` entry, so an entry with a
    // path to nothing would be a broken image rather than no image.
    let bundle = compiled(&sound());
    let alice = bundle.export["candidates"][0].clone();
    assert!(alice["image_document_id"].is_null());
    assert!(alice["presentation"]["urls"].as_array().is_none_or(|urls| {
        !urls
            .iter()
            .any(|url| url["is_image"] == serde_json::json!(true))
    }));
    assert!(!layout(&bundle)
        .importable
        .iter()
        .any(|artifact| artifact.name.starts_with("images/")));
}

#[test]
fn a_write_in_slot_gets_no_photograph_column_of_its_own() {
    // A slot is a blank line rather than a person. The cell still has to be there,
    // because `sheet_of` refuses a ragged row — and that guard exists because a
    // missing cell shifts every later column left and surfaces three sheets away.
    let mut plan = sound();
    plan.elections[0].contests[0].allow_writeins = true;
    plan.elections[0].contests[0].write_in_slots = 2;

    let bundle = compiled(&plan);
    for candidate in bundle.export["candidates"].as_array().expect("candidates")
    {
        if candidate["presentation"]["is_write_in"] == serde_json::json!(true) {
            assert!(candidate["image_document_id"].is_null());
        }
    }
}

#[test]
fn a_plan_saved_before_photographs_existed_still_opens() {
    let plan: Blueprint = serde_json::from_str(
        r#"{"version": 2, "external_id": "old", "name": {"en": "Old"},
            "elections": [{"external_id": "e", "contests": [{"external_id": "c",
              "candidates": [{"external_id": "a"}]}]}]}"#,
    )
    .expect("a plan from before this field reads");
    assert!(plan.elections[0].contests[0].candidates[0].image.is_none());
}

#[test]
fn a_photograph_round_trips_through_a_saved_plan() {
    // Base64 in the saved document, raw bytes in memory. The bytes are in the plan
    // on purpose: a plan that lost its photographs on reopening would mean
    // uploading them all again, which is the tedium this removes.
    let plan = with_photograph();
    let text = serde_json::to_string(&plan).expect("a plan serializes");
    assert!(
        text.contains("iVBORw=="),
        "the bytes are base64, not an array"
    );

    let read: Blueprint = serde_json::from_str(&text).expect("and reads back");
    assert_eq!(read, plan);
}

// -- the ways of voting -----------------------------------------------------

#[test]
fn the_channels_reach_the_event_and_every_election() {
    // Both, from one field on the plan. The Publish screen reads
    // `voting_channels` off whichever record it is showing — an event or an
    // election — so writing it in one place leaves the start button present at one
    // level and missing at the other.
    let mut plan = sound();
    plan.voting_channels = VotingChannelSet {
        online: true,
        kiosk: true,
        telephone: false,
        early_voting: false,
    };
    let bundle = compiled(&plan);

    for at in [
        &bundle.export["election_event"]["voting_channels"],
        &bundle.export["elections"][0]["voting_channels"],
    ] {
        assert_eq!(at["online"], serde_json::json!(true));
        assert_eq!(at["kiosk"], serde_json::json!(true));
        assert_eq!(at["telephone"], serde_json::json!(false));
        assert_eq!(at["early_voting"], serde_json::json!(false));
    }
}

#[test]
fn no_way_of_voting_open_is_refused() {
    let mut plan = sound();
    plan.voting_channels = VotingChannelSet {
        online: false,
        kiosk: false,
        telephone: false,
        early_voting: false,
    };
    let report = validated(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "nobody can vote"));
}

#[test]
fn an_area_that_allows_early_voting_says_so_and_one_that_does_not_says_that() {
    // Both spellings written, never a blank. `area.hbs` defaults to
    // `no_early_voting`, so leaving the negative case out would build the same
    // bytes — and would leave a workbook that answers the question for one area
    // and is silent for the next, where nobody can tell "no" from "unanswered".
    let mut plan = districted();
    plan.voting_channels.early_voting = true;
    plan.areas[1].allow_early_voting = true;

    let bundle = compiled(&plan);
    let allows = |index: usize| {
        bundle.export["areas"][index]["presentation"]["allow_early_voting"]
            .as_str()
            .expect("every area states its early voting policy")
            .to_string()
    };
    assert_eq!(allows(0), "no_early_voting");
    assert_eq!(allows(1), "allow_early_voting");
    assert_eq!(allows(2), "no_early_voting");
}

#[test]
fn early_voting_with_no_area_allowing_it_is_refused() {
    // The failure the plan predicted: a channel on with nothing behind it. The
    // Admin Portal grows an Early voting start button, somebody presses it, and
    // the period opens with no voter entitled to it.
    let mut plan = districted();
    plan.voting_channels.early_voting = true;

    let report = validated(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "no area allows it"));
}

#[test]
fn an_area_allowing_early_voting_with_the_channel_off_is_refused() {
    // The reverse, and not a hypothetical shape: the Admin Portal hides an area's
    // early-voting field while the event's channel is off, so a bundle in this
    // state cannot have come from there, and the area's setting does nothing.
    let mut plan = districted();
    plan.areas[1].allow_early_voting = true;

    let report = validated(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "the setting does nothing"));
    // Named, because "some area" is not something anybody can act on.
    assert!(says(&report, "North Local 1"));
}

#[test]
fn kiosk_says_the_bundle_does_not_create_its_auth_client() {
    // Kiosk voters arrive on a `?kiosk` URL, which the portal answers with a
    // separate auth client named `<client>-kiosk` — see `AuthContextProvider`. The
    // bundle provisions no Keycloak client, deliberately, so this is a warning
    // rather than an error: the missing half is in the environment, which this
    // pass cannot see.
    let mut plan = sound();
    plan.voting_channels.kiosk = true;

    let report = validated(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "-kiosk"));
}

#[test]
fn telephone_says_the_ivr_is_configured_after_import() {
    // Turning it on reveals the event's IVR tab in the Admin Portal, and that tab
    // is where telephone voting is actually described. None of it is in a bundle,
    // so the useful thing to say is where the rest of the work is.
    let mut plan = sound();
    plan.voting_channels.telephone = true;

    let report = validated(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "IVR tab"));
}

#[test]
fn a_plan_written_before_the_channels_existed_is_online() {
    // `bool`'s own default is false, so a derived `Default` would have opened an
    // event nobody can vote in — and `#[serde(default = "yes")]` would not have
    // caught it, because that only runs when deserialising.
    assert_eq!(
        VotingChannelSet::default(),
        VotingChannelSet {
            online: true,
            kiosk: false,
            telephone: false,
            early_voting: false,
        }
    );

    // And through serde, which is the path a saved plan takes.
    let plan: Blueprint = serde_json::from_str(
        r#"{"version": 2, "external_id": "old", "name": {"en": "Old"}}"#,
    )
    .expect("a plan from before this field reads");
    assert!(plan.voting_channels.online);
    assert!(!plan.voting_channels.kiosk);
}

// -- the ballot's languages -------------------------------------------------

/// What the event sheet says a voter reads before they choose.
fn default_language_of(plan: &Blueprint) -> String {
    let workbook = workbook_of(plan).expect("the plan compiles to rows");
    let row = workbook
        .rows(crate::election_config::sheet::SHEET_ELECTION_EVENT)
        .first()
        .cloned()
        .expect("one event row");
    row.get("presentation.language_conf.default_language_code")
        .map(|cell| format!("{cell}"))
        .unwrap_or_default()
}

/// The behaviour before `default_language` existed, kept: a plan saved without
/// one still opens in the first language it lists.
#[test]
fn a_plan_that_names_no_default_language_uses_the_first() {
    let plan = sound();
    assert_eq!(plan.default_language, None);
    assert!(default_language_of(&plan).contains("en"));
}

/// The gap this closes. The languages were configurable and the default was
/// not — `event_sheet` wrote `languages.first()` and nothing could say
/// otherwise, so a client wanting Spanish first had to reorder the list without
/// being told that was what the order meant.
#[test]
fn a_plan_can_say_which_language_voters_get_first() {
    let mut plan = sound();
    plan.default_language = Some("es".to_string());
    assert!(default_language_of(&plan).contains("es"));
}

/// An error rather than a warning. The builder falls back to the first
/// language, so this imports cleanly and opens in a language nobody chose —
/// which nobody notices until a voter says the ballot came up wrong.
#[test]
fn a_default_language_the_ballot_does_not_offer_is_refused() {
    let mut plan = sound();
    plan.default_language = Some("fr".to_string());

    let report = checked(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "not one of the languages"));
    assert!(codes(&report).contains(&"InvalidValue".to_string()));
}

/// And the fallback still holds even if validation is somehow skipped: the
/// sheet writer filters the chosen language against the list rather than
/// trusting it, so a bad plan cannot emit a language the event does not have.
#[test]
fn the_sheet_never_writes_a_language_the_event_does_not_have() {
    let mut plan = sound();
    plan.default_language = Some("fr".to_string());
    let written = default_language_of(&plan);
    assert!(
        written.contains("en"),
        "expected the fallback, got {written}"
    );
}

// -- the plan's own checks -------------------------------------------------

#[test]
fn a_sound_plan_has_nothing_to_report() {
    let report = checked(&sound());
    assert!(report.is_empty(), "{report}");
}

#[test]
fn a_threshold_no_number_of_trustees_can_meet_is_an_error() {
    // The worst failure mode there is: everything works until the tally, and then
    // the result cannot be decrypted by anybody.
    let mut plan = sound();
    plan.trustee_threshold = 5;
    let report = checked(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "could never be decrypted"));
}

#[test]
fn a_threshold_of_one_is_refused() {
    // This test used to assert the opposite — allowed, with a warning — and `EA-70`
    // reverses it on purpose. A threshold of one means any single trustee decrypts
    // every ballot alone, which is exactly the guarantee threshold encryption is
    // there to provide; and it is not recoverable, because by the time anybody looks
    // the votes have been cast under that key. A warning on the last screen somebody
    // reads, about a property they cannot check afterwards, is what this pass exists
    // to refuse.
    let mut plan = sound();
    plan.trustee_threshold = 1;
    let report = checked(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "any single trustee can open the tally alone"));
}

#[test]
fn a_single_trustee_is_refused() {
    let mut plan = sound();
    plan.trustees.truncate(1);
    plan.trustee_threshold = 1;
    let report = checked(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "one trustee"));
    assert!(says(&report, "at least two people"));
}

#[test]
fn no_trustees_is_now_an_error_rather_than_a_warning() {
    // Also a reversal. An empty list built a bundle with an empty `keys_ceremonies`,
    // which imports and then has nobody to generate a key.
    let mut plan = sound();
    plan.trustees.clear();
    let report = checked(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "no trustees"));
}

#[test]
fn a_trustee_needs_an_email_that_could_receive_an_invitation() {
    // The address the ceremony invitation goes to. A trustee who never gets it does
    // not attend, and the threshold failing is discovered at the tally — the same
    // failure as a name resolving to nobody, by a different route.
    let cases = [
        ("", "needs an email address"),
        ("Ada Lovelace", "is not an email address"),
        ("ada@", "is not an email address"),
        ("@example.org", "is not an email address"),
        ("ada@example", "is not an email address"),
        ("ada @example.org", "is not an email address"),
        ("ada@@example.org", "is not an email address"),
    ];
    for (email, expected) in cases {
        let mut plan = sound();
        plan.trustees[0].email = email.to_string();
        let report = checked(&plan);
        assert!(
            report.has_errors(),
            "'{email}' should be refused:\n{report}"
        );
        assert!(
            says(&report, expected),
            "'{email}' should say '{expected}':\n{report}"
        );
    }
}

#[test]
fn an_unusual_but_valid_address_is_accepted() {
    // Stricter than the specification would refuse a working address, which is a
    // worse defect than accepting one that bounces — this cannot know either way.
    for email in [
        "ada+trustee@example.co.uk",
        "a.b-c_d@sub.example.org",
        "\"quoted\"@example.org",
    ] {
        let mut plan = sound();
        plan.trustees[0].email = email.to_string();
        let report = checked(&plan);
        assert!(!report.has_errors(), "'{email}' should pass:\n{report}");
    }
}

#[test]
fn a_threshold_of_zero_is_refused() {
    let mut plan = sound();
    plan.trustee_threshold = 0;
    assert!(checked(&plan).has_errors());
}

#[test]
fn voting_that_closes_before_it_opens_is_refused() {
    let mut plan = sound();
    plan.schedule.voting_closes = Some(at("2027-01-01T00:00"));
    let report = checked(&plan);
    assert!(says(&report, "closes before it opens"));
}

#[test]
fn a_key_ceremony_after_voting_opens_is_refused() {
    // The key has to exist before a vote can be encrypted with it.
    let mut plan = sound();
    plan.schedule.key_ceremony = Some(at("2027-03-02T10:00"));
    let report = checked(&plan);
    assert!(says(&report, "before voting opens"));
}

#[test]
fn a_tally_before_voting_closes_is_refused() {
    let mut plan = sound();
    plan.schedule.tally_ceremony = Some(at("2027-03-01T10:00"));
    let report = checked(&plan);
    assert!(says(&report, "votes that had not been cast"));
}

/// The whole reason this plan carries offsets. The scheduler reads the emitted
/// date with `DateTime::parse_from_rfc3339`, which requires one — a wall clock
/// yields no date, the poller drops the event, and voting never opens with
/// nothing anywhere saying why.
#[test]
fn the_voting_window_is_emitted_as_an_instant_the_scheduler_can_read() {
    let workbook = workbook_of(&sound()).expect("a sound plan should compile");
    let rows = workbook.rows("scheduledevents");
    assert_eq!(rows.len(), 2, "an opening and a closing");

    for row in rows {
        let written = row.text("scheduled_datetime").expect("a date");
        assert!(
            chrono::DateTime::parse_from_rfc3339(&written).is_ok(),
            "the platform's own parser rejects {written:?}"
        );
    }
}

/// Real, common, and invisible in the clock times: a March window in California
/// is an hour shorter than it looks. Said out loud rather than refused.
#[test]
fn a_voting_window_crossing_a_clock_change_is_said_out_loud() {
    let mut plan = sound();
    plan.schedule.voting_opens = Some(Timestamp::new(
        "2027-03-01T09:00",
        "America/Los_Angeles",
        -480,
    ));
    plan.schedule.voting_closes = Some(Timestamp::new(
        "2027-03-15T17:00",
        "America/Los_Angeles",
        -420,
    ));

    let report = checked(&plan);

    assert!(says(&report, "daylight-saving change"));
    assert!(!report.has_errors(), "legitimate, so not an error");
}

/// A plan written before the wizard knew about zones still opens, and still
/// means what it meant — UTC — rather than failing or silently shifting.
#[test]
fn a_plan_saved_before_timezones_existed_still_opens() {
    let text = r#"{
        "version": 1,
        "external_id": "old",
        "schedule": {
            "voting_opens": "2027-03-01T09:00",
            "voting_closes": "2027-03-15T17:00"
        }
    }"#;

    let plan: Blueprint = serde_json::from_str(text).expect("an older plan");
    let opens = plan.schedule.voting_opens.as_ref().expect("a time");

    assert_eq!(opens.local, "2027-03-01T09:00");
    assert_eq!(opens.offset_minutes, 0);
    assert_eq!(opens.to_rfc3339().unwrap(), "2027-03-01T09:00:00+00:00");
}

#[test]
fn an_incomplete_voting_window_is_a_warning_not_an_error() {
    // A plan being filled in is not a broken plan, and it still has to be
    // saveable.
    let mut plan = sound();
    plan.schedule.voting_closes = None;
    let report = checked(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "opened"));
}

#[test]
fn a_contest_electing_more_than_a_voter_may_choose_is_refused() {
    let mut plan = sound();
    plan.elections[0].contests[0].winners = 3;
    let report = checked(&plan);
    assert!(says(&report, "a voter may only choose"));
    assert!(codes(&report).contains(&"ContestArithmetic".to_string()));
}

#[test]
fn a_contest_electing_more_than_it_has_candidates_is_refused() {
    let mut plan = sound();
    plan.elections[0].contests[0].max_votes = 5;
    plan.elections[0].contests[0].winners = 5;
    let report = checked(&plan);
    assert!(says(&report, "from a field of 2"));
}

#[test]
fn blank_and_invalid_options_do_not_count_as_candidates() {
    // Filling a two-winner contest with "none of the above" is not a field of two.
    let mut plan = sound();
    let contest = &mut plan.elections[0].contests[0];
    contest.candidates.truncate(1);
    contest.candidates.push(PlannedCandidate {
        external_id: "blank".to_string(),
        name: Translated::new("None of the above"),
        description: Translated::default(),
        ivr_prompt: Translated::default(),
        explicit_blank: true,
        explicit_invalid: false,
        disabled: false,

        image: None,
    });
    contest.max_votes = 2;
    contest.winners = 2;

    let report = checked(&plan);
    assert!(says(&report, "from a field of 1"));
}

#[test]
fn a_contest_with_no_candidates_yet_is_a_warning() {
    let mut plan = sound();
    plan.elections[0].contests[0].candidates.clear();
    let report = checked(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "no candidates yet"));
}

#[test]
fn areas_inside_each_other_are_refused_by_the_plan_validator() {
    // Self-parenting had its own message; a two-hop loop passed the plan validator
    // and was only caught later, in bundle vocabulary the author never wrote.
    let mut plan = sound();
    plan.areas = vec![
        PlannedArea {
            external_id: "north".to_string(),
            name: "North".to_string(),
            parent_external_id: Some("south".to_string()),
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "south".to_string(),
            name: "South".to_string(),
            parent_external_id: Some("north".to_string()),
            allow_early_voting: false,
        },
    ];

    let report = checked(&plan);
    assert!(
        report
            .errors()
            .any(|problem| problem.code == Code::AreaCycle),
        "expected an area cycle, got:\n{report}"
    );
}

#[test]
fn a_plan_with_no_elections_is_refused() {
    let mut plan = sound();
    plan.elections.clear();
    assert!(checked(&plan).has_errors());
}

#[test]
fn a_plan_with_no_name_or_identifier_is_refused() {
    let mut plan = sound();
    plan.name = Translated::default();
    plan.external_id = "  ".to_string();
    let report = checked(&plan);
    assert_eq!(report.errors().count(), 2, "{report}");
}

#[test]
fn no_points_of_contact_is_worth_saying() {
    let mut plan = sound();
    plan.contacts.clear();
    assert!(says(&checked(&plan), "who gets called"));
}

#[test]
fn a_plan_from_a_newer_version_is_refused_rather_than_half_read() {
    // Opening it would silently drop whatever that version added, and the author
    // would not know which parts survived.
    let mut plan = sound();
    plan.version = BLUEPRINT_VERSION + 1;
    let report = checked(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "newer version"));
}

// -- round trip and side files ---------------------------------------------

#[test]
fn a_plan_survives_being_saved_and_opened() {
    // What the TypeScript could not do: it reconstructed its state by parsing the
    // generated bundle, so the threshold and the ceremony dates were lost every
    // time.
    let plan = sound();
    let saved = serde_json::to_string(&plan).unwrap();
    let opened: Blueprint = serde_json::from_str(&saved).unwrap();
    assert_eq!(plan, opened);
}

#[test]
fn an_older_plan_missing_the_newer_fields_still_opens() {
    // Everything optional has a default, so a plan saved before a field existed
    // is still readable. A wizard whose saved documents stop opening is a wizard
    // nobody trusts.
    let minimal = serde_json::json!({
        "version": 1,
        "external_id": "union-2027",
    });
    let plan: Blueprint = serde_json::from_value(minimal).unwrap();
    assert_eq!(plan.external_id, "union-2027");
    assert_eq!(plan.trustee_threshold, 2);
    assert!(plan.elections.is_empty());
}

#[test]
fn the_plan_travels_with_its_output() {
    let files = side_files(&sound());
    let names: Vec<&str> =
        files.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"blueprint.json"));
    assert!(names.contains(&"ceremony_schedule.json"));
    assert!(names.contains(&"points_of_contact.json"));
    assert!(names.contains(&"trustees_list.json"));
}

#[test]
fn the_saved_plan_is_the_plan() {
    // Not a summary of it: opening the archive and reading blueprint.json has to
    // give back something the wizard can resume from.
    let plan = sound();
    let files = side_files(&plan);
    let (_, saved) = files
        .iter()
        .find(|(name, _)| name == "blueprint.json")
        .unwrap();
    let reopened: Blueprint = serde_json::from_str(saved).unwrap();
    assert_eq!(reopened, plan);
}

#[test]
fn the_side_files_are_json_and_end_in_a_newline() {
    for (name, contents) in side_files(&sound()) {
        serde_json::from_str::<serde_json::Value>(&contents)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(contents.ends_with('\n'), "{name}");
    }
}

#[test]
fn a_plan_with_nobody_in_it_writes_no_empty_lists() {
    // An empty points_of_contact.json is a file somebody has to open to discover
    // it says nothing.
    let mut plan = sound();
    plan.contacts.clear();
    plan.trustees.clear();
    let names: Vec<String> = side_files(&plan)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert!(!names.contains(&"points_of_contact.json".to_string()));
    assert!(!names.contains(&"trustees_list.json".to_string()));
    // The plan and the ceremony dates are always worth writing.
    assert!(names.contains(&"blueprint.json".to_string()));
}

// -- districting -----------------------------------------------------------

/// The sound plan, districted: two locals inside a region, and a contest that
/// only one of them votes on.
fn districted() -> Blueprint {
    let mut plan = sound();
    plan.areas = vec![
        PlannedArea {
            external_id: "region-north".to_string(),
            name: "North Region".to_string(),
            parent_external_id: None,
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "local-1".to_string(),
            name: "North Local 1".to_string(),
            parent_external_id: Some("region-north".to_string()),
            allow_early_voting: false,
        },
        PlannedArea {
            external_id: "local-2".to_string(),
            name: "North Local 2".to_string(),
            parent_external_id: Some("region-north".to_string()),
            allow_early_voting: false,
        },
    ];
    // The president is everywhere; the local officer is one local's business.
    plan.elections[0].contests.push(PlannedContest {
        overrides: Overrides::default(),
        external_id: "local-officer".to_string(),
        name: Translated::new("Local Officer"),
        description: Translated::default(),
        ivr_prompt: Translated::default(),
        max_votes: 1,
        winners: 1,
        allow_writeins: false,
        write_in_slots: 0,
        candidates: vec![PlannedCandidate {
            external_id: "carol".to_string(),
            name: Translated::new("Carol"),
            description: Translated::default(),
            ivr_prompt: Translated::default(),
            explicit_blank: false,
            explicit_invalid: false,
            disabled: false,

            image: None,
        }],
        areas: vec!["local-1".to_string()],
    });
    plan
}

#[test]
fn a_plan_with_no_areas_still_puts_every_contest_on_one_ballot() {
    // Districting is optional. Without an area and a link, no voter sees anything.
    let bundle = compiled(&sound());
    assert_eq!(bundle.export["areas"].as_array().unwrap().len(), 1);
    assert_eq!(
        bundle.export["areas"][0]["name"],
        serde_json::json!(DEFAULT_AREA_NAME)
    );
    assert_eq!(bundle.export["area_contests"].as_array().unwrap().len(), 1);
}

#[test]
fn a_districted_plan_becomes_a_bundle_the_platform_accepts() {
    let bundle = compiled(&districted());
    let schema: ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone()).unwrap();
    let report = validate(&schema);
    assert!(!report.has_errors(), "{report}");
}

#[test]
fn the_areas_arrive_with_their_tree_intact() {
    let bundle = compiled(&districted());
    let areas = bundle.export["areas"].as_array().unwrap();
    assert_eq!(areas.len(), 3);

    let region = areas
        .iter()
        .find(|area| area["name"] == serde_json::json!("North Region"))
        .unwrap();
    let local = areas
        .iter()
        .find(|area| area["name"] == serde_json::json!("North Local 1"))
        .unwrap();

    assert_eq!(region["parent_id"], serde_json::Value::Null);
    assert_eq!(local["parent_id"], region["id"]);
}

#[test]
fn a_contest_assigned_to_no_area_is_on_every_ballot() {
    // What a plan that has not thought about it wants, and what dropping the
    // contest instead would silently cost.
    let bundle = compiled(&districted());
    let president = &bundle.export["contests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contest| {
            contest["external_id"] == serde_json::json!("president")
        })
        .unwrap()["id"];

    let on: Vec<&serde_json::Value> = bundle.export["area_contests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| &link["contest_id"] == president)
        .collect();
    assert_eq!(on.len(), 3, "the president should be on all three ballots");
}

#[test]
fn a_local_contest_is_only_on_the_ballots_it_names() {
    let bundle = compiled(&districted());
    let local_officer = &bundle.export["contests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contest| {
            contest["external_id"] == serde_json::json!("local-officer")
        })
        .unwrap()["id"];

    let links: Vec<&serde_json::Value> = bundle.export["area_contests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| &link["contest_id"] == local_officer)
        .collect();
    assert_eq!(links.len(), 1);

    let local_1 = bundle.export["areas"]
        .as_array()
        .unwrap()
        .iter()
        .find(|area| area["name"] == serde_json::json!("North Local 1"))
        .unwrap();
    assert_eq!(links[0]["area_id"], local_1["id"]);
}

#[test]
fn assigning_a_contest_to_the_same_area_twice_produces_one_link() {
    // Both rows would mint the same id and one would overwrite the other.
    let mut plan = districted();
    plan.elections[0].contests[1].areas =
        vec!["local-1".to_string(), "local-1".to_string()];
    let bundle = compiled(&plan);
    let local_officer = &bundle.export["contests"][1]["id"];
    assert_eq!(
        bundle.export["area_contests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| &link["contest_id"] == local_officer)
            .count(),
        1
    );
}

#[test]
fn a_contest_naming_an_area_nobody_defined_is_refused() {
    let mut plan = districted();
    plan.elections[0].contests[1].areas = vec!["nowhere".to_string()];
    let report = checked(&plan);
    assert!(says(&report, "no area has the identifier 'nowhere'"));
    assert!(report.has_errors());
}

#[test]
fn two_areas_may_not_share_a_name() {
    // The voters CSV resolves by name, so voters would land in whichever the
    // importer found first.
    let mut plan = districted();
    plan.areas[1].name = "North Region".to_string();
    let report = checked(&plan);
    assert!(says(&report, "both named 'North Region'"));
}

#[test]
fn an_area_needs_a_name_because_that_is_what_a_voter_is_matched_on() {
    let mut plan = districted();
    plan.areas[0].name = "  ".to_string();
    let report = checked(&plan);
    assert!(says(&report, "identifies a voter's area by name"));
}

#[test]
fn an_area_cannot_be_inside_itself() {
    let mut plan = districted();
    plan.areas[0].parent_external_id = Some("region-north".to_string());
    let report = checked(&plan);
    assert!(says(&report, "cannot be inside itself"));
    assert!(codes(&report).contains(&"AreaCycle".to_string()));
}

#[test]
fn a_parent_that_does_not_exist_is_refused() {
    let mut plan = districted();
    plan.areas[1].parent_external_id = Some("region-south".to_string());
    let report = checked(&plan);
    assert!(says(&report, "no area has the identifier 'region-south'"));
}

#[test]
fn a_districted_plan_survives_being_saved_and_opened() {
    let plan = districted();
    let opened: Blueprint =
        serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    assert_eq!(plan, opened);
}

#[test]
fn a_plan_saved_before_districting_existed_still_opens() {
    // Everything about areas is optional, so an older plan reads as one ballot
    // for everybody — which is what it was.
    let older = serde_json::json!({
        "version": 1,
        "external_id": "union-2027",
        "elections": [{
            "external_id": "officers",
            "contests": [{"external_id": "president"}],
        }],
    });
    let plan: Blueprint = serde_json::from_value(older).unwrap();
    assert!(plan.areas.is_empty());
    assert!(plan.elections[0].contests[0].areas.is_empty());
}

/// The census travels with the plan, in the columns the builder already reads.
#[test]
fn a_plan_may_carry_its_own_census() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];
    plan.voters = vec![
        PlannedVoter {
            username: "ada".into(),
            email: "ada@example.org".into(),
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
            area_external_id: "local-1".into(),
            extra: [("department".to_string(), "engineering".to_string())]
                .into_iter()
                .collect(),
        },
        PlannedVoter {
            username: "grace".into(),
            area_external_id: "local-1".into(),
            ..Default::default()
        },
    ];

    let workbook = workbook_of(&plan).expect("this plan is sound");
    let voters = workbook
        .sheet("voters")
        .expect("the census should be a sheet");

    // The columns the builder derives are absent: `id` comes from `ids::uid` and
    // `authorized-election-ids` from the areas, so a census that carried them
    // would be handing over values the builder is about to overwrite.
    assert!(!voters.headers.contains(&"id".to_string()));
    assert!(!voters
        .headers
        .contains(&"authorized-election-ids".to_string()));

    // And a client's own column survives, which is the point of the passthrough.
    assert!(voters.headers.contains(&"department".to_string()));
    assert_eq!(voters.rows.len(), 2);
}

/// A caller's own census and files reach the bundle, and the plan's are not consulted.
///
/// `Compile::sources` is `None` at every call site today, so nothing would notice if
/// it were ignored. This hands over a plan that carries **nothing** — no voters, no
/// logo bytes — beside sources that carry both, and looks for them in the finished
/// bundle. It is the Rust half of what `compilePlan`'s new `census` and `files`
/// options do, and the only half that can be tested without a browser.
#[test]
fn the_caller_may_bring_the_census_and_the_files() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];
    // A logo the plan names and does not carry, which is what a plan opened from a
    // save file looks like.
    plan.logo = Some(CandidateImage {
        file_name: "union.png".to_string(),
        bytes: Vec::new(),
    });
    assert!(plan.voters.is_empty());

    let sources = Sources {
        census: Some(std::sync::Arc::new(VecCensus::new(vec![PlannedVoter {
            username: "ada".into(),
            area_external_id: "north".into(),
            ..Default::default()
        }]))),
        files: [(
            "union.png".to_string(),
            std::sync::Arc::from(b"PNG".to_vec()),
        )]
        .into_iter()
        .collect(),
    };

    let templates = TemplateSet::builtin().expect("the built-in templates");
    let compiled = compile_plan(Compile {
        plan: &plan,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: Some(&sources),
    })
    .expect("a plan with a census beside it compiles");

    assert_eq!(compiled.bundle.voters.rows.len(), 1);
    assert_eq!(
        compiled
            .bundle
            .images
            .iter()
            .map(|image| (image.file_name.as_str(), image.bytes.as_slice()))
            .collect::<Vec<_>>(),
        vec![("union.png", b"PNG".as_slice())],
        "the caller's bytes should be the ones that travel"
    );
}

/// Two candidates cannot both have `photo.jpg`.
///
/// **The failure this prevents has no symptom.** Bytes travel keyed by name from
/// the moment they leave the plan — in `sources.files`, in a save file's `files/`
/// directory, in `BuildOptions::images` — so two photographs called `photo.jpg` are
/// one photograph, and the wrong face appears on a ballot. Nothing catches it today
/// because the plan carries the bytes inline, where two identical names are two
/// different values. The moment it stops, they are one.
#[test]
fn two_files_may_not_share_a_name() {
    let mut plan = sound();
    let image = |name: &str| CandidateImage {
        file_name: name.to_string(),
        bytes: b"JPEG".to_vec(),
    };
    plan.elections[0].contests[0].candidates[0].image =
        Some(image("photo.jpg"));
    plan.elections[0].contests[0].candidates[1].image =
        Some(image("photo.jpg"));

    let report = checked(&plan);
    let clash = report
        .problems
        .iter()
        .find(|problem| problem.id.as_deref() == Some("file.duplicate-name"))
        .unwrap_or_else(|| panic!("expected a clash, got:\n{report}"));

    // Names the other one, because "these two clash" is only actionable if it says
    // which two.
    assert!(
        clash
            .message
            .contains(&plan.elections[0].contests[0].candidates[0].external_id),
        "{}",
        clash.message
    );

    // And two different names are two files.
    plan.elections[0].contests[0].candidates[1].image =
        Some(image("other.jpg"));
    assert!(!checked(&plan)
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("file.duplicate-name")));
}

/// A file the plan names and nobody holds, and a file nobody names.
#[test]
fn the_plan_and_the_files_are_checked_against_each_other() {
    let mut plan = sound();
    plan.elections[0].contests[0].candidates[0].image = Some(CandidateImage {
        file_name: "ada.jpg".to_string(),
        bytes: Vec::new(),
    });

    // Named, held by nobody: an archive entry with nothing in it fails the whole
    // import rather than losing a picture.
    let named: Vec<String> = validate_plan(&plan, &Sources::default())
        .problems
        .into_iter()
        .filter_map(|problem| problem.id)
        .collect();
    assert!(named.contains(&"file.missing".to_string()), "{named:?}");

    // Held by the caller rather than by the plan, which is what a reopened save
    // file looks like. No complaint.
    let beside = Sources {
        files: [(
            "ada.jpg".to_string(),
            std::sync::Arc::from(b"JPEG".to_vec()),
        )]
        .into_iter()
        .collect(),
        ..Sources::default()
    };
    let named: Vec<String> = validate_plan(&plan, &beside)
        .problems
        .into_iter()
        .filter_map(|problem| problem.id)
        .collect();
    assert!(!named.contains(&"file.missing".to_string()), "{named:?}");

    // And a file nothing names travels in the delivery and is shown to nobody.
    let spare = Sources {
        files: [
            (
                "ada.jpg".to_string(),
                std::sync::Arc::from(b"JPEG".to_vec()),
            ),
            (
                "nobody.png".to_string(),
                std::sync::Arc::from(b"PNG".to_vec()),
            ),
        ]
        .into_iter()
        .collect(),
        ..Sources::default()
    };
    let report = validate_plan(&plan, &spare);
    assert!(!report.has_errors(), "{report}");
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("file.unused")));
}

/// The census is read from the source, not from the plan.
///
/// **The test that makes this change more than a rename.** `Sources::from_plan`
/// derives the source from the fields the plan still has, which is what keeps every
/// other test in this file green — and is also exactly what would hide a check that
/// went on reading `plan.voters`. So this hands over a plan whose census is empty
/// and a source that has one, and asks for the two answers only a census can give:
/// a Voters sheet, and a problem about a voter's area.
///
/// When the field goes, this test does not change. That is the point of writing it
/// now rather than after.
#[test]
fn the_census_comes_from_the_source() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];
    assert!(
        plan.voters.is_empty(),
        "the plan must not be the source here"
    );

    let census = Sources {
        census: Some(std::sync::Arc::new(VecCensus::new(vec![
            PlannedVoter {
                username: "ada".into(),
                area_external_id: "north".into(),
                extra: [("department".to_string(), "engineering".to_string())]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
            PlannedVoter {
                username: "grace".into(),
                area_external_id: "nowhere".into(),
                ..Default::default()
            },
        ]))),
        ..Sources::default()
    };

    let sheet = to_workbook(&plan, &census)
        .expect("this plan is sound")
        .sheet("voters")
        .cloned()
        .expect("the source's census should be a sheet");
    assert_eq!(sheet.rows.len(), 2);
    assert!(sheet.headers.contains(&"department".to_string()));

    assert!(
        validate_plan(&plan, &census)
            .problems
            .iter()
            .any(|problem| problem.id.as_deref() == Some("voter.area-unknown")),
        "the check should have read the source's second voter"
    );
}

/// What the census check says, now that something reads it.
///
/// Each of these was written and none was covered: the whole of `check_census`
/// could have been deleted and this file would have passed. They are here because
/// the checks just moved onto a different reader, and a silent one would look
/// exactly like a census with nothing wrong in it.
#[test]
fn the_census_check_names_what_is_wrong_with_a_row() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];

    let named = |voters: Vec<PlannedVoter>| -> Vec<String> {
        let sources = Sources {
            census: Some(std::sync::Arc::new(VecCensus::new(voters))),
            ..Sources::default()
        };
        validate_plan(&plan, &sources)
            .problems
            .into_iter()
            .filter_map(|problem| problem.id)
            .collect()
    };

    let at = |area: &str| PlannedVoter {
        username: "ada".into(),
        area_external_id: area.into(),
        ..Default::default()
    };

    assert!(named(vec![at("north")]).is_empty(), "this census is fine");
    assert!(named(vec![at("elsewhere")])
        .contains(&"voter.area-unknown".to_string()));
    assert!(named(vec![at("")]).contains(&"voter.no-area".to_string()));
    assert!(named(vec![PlannedVoter {
        username: "   ".into(),
        area_external_id: "north".into(),
        ..Default::default()
    }])
    .contains(&"voter.no-username".to_string()));
    assert!(named(vec![at("north"), at("north")])
        .contains(&"voter.duplicate-username".to_string()));

    // Per row for the error, once for the aggregate. The existing
    // `a_census_that_ignores_the_districting_says_so_once` counts the second half;
    // this counts both, because the rewrite that made this one pass instead of two
    // could have lost either.
    let none = named((0..12).map(|_| at("")).collect());
    assert_eq!(none.iter().filter(|id| *id == "voter.no-area").count(), 12);
    assert_eq!(
        none.iter()
            .filter(|id| *id == "census.no-area-column")
            .count(),
        1
    );
}

/// A plan with no census has no sheet, rather than an empty one.
#[test]
fn no_census_is_not_an_empty_census() {
    // `build_tables` reads a present-but-empty Voters sheet as "this election
    // has no voters", which is a different claim from "this plan does not carry
    // the census" — and produces a bundle importing an election nobody can vote
    // in.
    let workbook = workbook_of(&sound()).expect("this plan is sound");

    assert!(
        workbook.sheet("voters").is_none(),
        "a plan with no voters should not emit the sheet at all"
    );
}

/// The sheet's own column names have to be ones the builder reads.
#[test]
fn the_voters_sheet_matches_what_the_builder_reads() {
    // `voters_sheet` spells its columns rather than importing
    // `VOTER_LEADING_COLUMNS`, because that constant lives behind a feature gate
    // this module does not have. Two lists means they can drift, and the way
    // they would drift is a census column the builder silently ignores.
    let mut plan = sound();
    plan.voters = vec![PlannedVoter {
        username: "ada".into(),
        ..Default::default()
    }];

    let workbook = workbook_of(&plan).expect("this plan is sound");
    let voters = workbook
        .sheet("voters")
        .expect("the census should be a sheet");

    // Two directions, because the one-way version missed a real defect for weeks.
    //
    // It asserted only that every column emitted is one the builder knows, which
    // says nothing about a column the builder cannot do without. `area.external_id`
    // is exactly that: `build_tables::voter_area_name` reads it off every row and
    // reports "a voter needs an area" when it is absent, so the whole plan compiled
    // to nothing while this test stayed green.
    //
    // It is not in `VOTER_LEADING_COLUMNS` and should not be. That list is the
    // columns the builder *derives or reorders* on the way out; this is one it
    // *consumes* on the way in, turning the id into the `area_name` the finished
    // CSV carries.
    const CONSUMED: &[&str] = &["area.external_id"];

    for column in &voters.headers {
        assert!(
            crate::election_config::build::VOTER_LEADING_COLUMNS
                .contains(&column.as_str())
                || CONSUMED.contains(&column.as_str()),
            "'{column}' is not a column the builder reads"
        );
    }

    for required in CONSUMED {
        assert!(
            voters.headers.iter().any(|column| column == required),
            "the builder needs '{required}' on every voter row and the sheet has no \
             such column, so every plan with a census compiles to nothing"
        );
    }
}

/// Two voters cannot share a username: the second would replace the first.
#[test]
fn a_duplicate_username_is_refused() {
    let mut plan = sound();
    plan.voters = vec![
        PlannedVoter {
            username: "ada".into(),
            ..Default::default()
        },
        PlannedVoter {
            username: "ada".into(),
            ..Default::default()
        },
    ];

    let report = checked(&plan);

    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path == "voters[1].username"
                && problem.severity == Severity::Error),
        "{:?}",
        report.problems
    );
}

/// An area name no area has means a voter with no ballot.
#[test]
fn a_voter_in_an_area_that_does_not_exist_is_refused() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];
    plan.voters = vec![PlannedVoter {
        username: "ada".into(),
        // The census says one thing and the areas another, which is what
        // retyping an identifier rather than copying it produces.
        area_external_id: "local-one".into(),
        ..Default::default()
    }];

    let report = checked(&plan);

    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path == "voters[0].area.external_id"),
        "{:?}",
        report.problems
    );
}

/// Districting nobody's census mentions is worth saying once.
#[test]
fn a_census_that_ignores_the_districting_says_so_once() {
    let mut plan = sound();
    plan.areas = vec![PlannedArea {
        external_id: "north".into(),
        name: "North Local 1".into(),
        parent_external_id: None,
        allow_early_voting: false,
    }];
    plan.voters = (0..5)
        .map(|index| PlannedVoter {
            username: format!("voter-{index}"),
            ..Default::default()
        })
        .collect();

    let report = checked(&plan);

    // Once, not once per voter: ten thousand copies of one sentence is a report
    // nobody reads.
    assert_eq!(
        report
            .problems
            .iter()
            .filter(|problem| problem.path == "voters")
            .count(),
        1,
        "{:?}",
        report.problems
    );
}

/// A write-in slot becomes a candidate row the codec can use.
///
/// The two halves the codec needs — `presentation.allow_writeins` on the contest
/// and one `presentation.is_write_in` candidate per slot — and they are the two
/// `validate::check_write_ins` refuses independently, so a plan that produces one
/// without the other is caught rather than imported.
#[test]
fn a_write_in_slot_reaches_the_ballot_as_its_own_row() {
    let mut plan = sound();
    plan.elections[0].contests[0].allow_writeins = true;
    plan.elections[0].contests[0].write_in_slots = 2;
    let real = plan.elections[0].contests[0].candidates.len();

    // The normalised key, not the tab's label: `Workbook::rows` folds case and
    // spaces so `Admin Users` and `AdminUsers` cannot be two sheets.
    let workbook = workbook_of(&plan).expect("a workbook");
    let contests = workbook.rows(sheet::SHEET_CONTESTS);
    let candidates = workbook.rows(sheet::SHEET_CANDIDATES);

    assert_eq!(
        contests[0]
            .get("presentation.allow_writeins")
            .map(|cell| format!("{cell:?}")),
        Some("Bool(true)".to_string())
    );

    let slots: Vec<&crate::election_config::sheet::Row> = candidates
        .iter()
        .filter(|row| {
            row.get("presentation.is_write_in")
                .map(|cell| format!("{cell:?}"))
                == Some("Bool(true)".to_string())
        })
        .collect();
    assert_eq!(slots.len(), 2, "one row per slot");

    // Named after the contest, so two contests with write-ins do not collide —
    // `ids::uid` would hash the same string to the same uuid.
    let first = format!("{:?}", slots[0].get("external_id").expect("an id"));
    assert!(first.contains(&plan.elections[0].contests[0].external_id));

    // And they sort after the people actually standing, so a ballot does not
    // open with two blank lines.
    for (offset, slot) in slots.iter().enumerate() {
        assert_eq!(
            slot.get("presentation.sort_order")
                .map(|cell| format!("{cell:?}")),
            Some(format!("Number({})", real + offset))
        );
    }
}

/// A contest that allows none writes none.
#[test]
fn a_contest_without_write_ins_gets_no_extra_rows() {
    let workbook = workbook_of(&sound()).expect("a workbook");
    assert!(!workbook.rows(sheet::SHEET_CANDIDATES).iter().any(|row| {
        row.get("presentation.is_write_in")
            .map(|cell| format!("{cell:?}"))
            == Some("Bool(true)".to_string())
    }));
}

/// A description is translated, like every other text a voter reads.
///
/// It was a `String`, and that was wrong rather than merely limited: the Admin
/// Portal edits `presentation.i18n.<lang>.description` per language for the
/// event, the election, the contest and the candidate, and keeps the flat
/// `description` column as a mirror of the English one. A plan carrying one text
/// put the same words in front of every voter whatever language they read.
#[test]
fn every_description_is_written_once_per_language() {
    let mut plan = sound();
    plan.description = Translated::new("The union's 2027 elections");
    plan.description
        .by_language
        .insert("es".to_string(), "Elecciones 2027".to_string());
    plan.elections[0].description = Translated::new("All officer positions");
    plan.elections[0].contests[0].description = Translated::new("One seat");
    plan.elections[0].contests[0].candidates[0].description =
        Translated::new("Incumbent");

    let workbook = workbook_of(&plan).expect("a workbook");

    for (sheet, text) in [
        (sheet::SHEET_ELECTION_EVENT, "The union's 2027 elections"),
        (sheet::SHEET_ELECTIONS, "All officer positions"),
        (sheet::SHEET_CONTESTS, "One seat"),
        (sheet::SHEET_CANDIDATES, "Incumbent"),
    ] {
        let row = &workbook.rows(sheet)[0];
        assert_eq!(
            row.get("presentation.i18n.en.description")
                .map(|cell| format!("{cell:?}")),
            Some(format!("String({text:?})")),
            "{sheet} lost its English description"
        );
        // And the flat column, which is what the Admin Portal's list views read.
        assert_eq!(
            row.get("description").map(|cell| format!("{cell:?}")),
            Some(format!("String({text:?})")),
            "{sheet} did not mirror English into the flat column"
        );
    }

    // The Spanish one is its own column, not a second copy of the English.
    assert_eq!(
        workbook.rows(sheet::SHEET_ELECTION_EVENT)[0]
            .get("presentation.i18n.es.description")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"Elecciones 2027\")".to_string())
    );
}

/// A plan saved when a description was a plain string still opens.
///
/// One release carried `"description": "One seat"` where a map now belongs. Serde
/// would refuse it, and the client who saved that plan would be told their own
/// file is not a plan — which is the worst possible way to learn about a schema
/// change.
#[test]
fn a_plan_saved_before_descriptions_were_translated_still_opens() {
    let plan = sound();
    let mut document = serde_json::to_value(&plan).expect("a plan serialises");
    document["elections"][0]["contests"][0]["description"] =
        serde_json::json!("One seat");

    let reopened: Blueprint =
        serde_json::from_value(document).expect("an older plan still opens");
    assert_eq!(
        reopened.elections[0].contests[0].description.get("en"),
        Some("One seat"),
        "the plain string should have been read as English"
    );
}

/// And an empty one does not become an empty English entry.
///
/// `"description": ""` is what the old shape wrote for a contest nobody
/// described. Read literally it produces `{"en": ""}`, which is a description
/// that exists and is blank — and the sheet would then write an empty string
/// where a blank cell belongs.
#[test]
fn an_empty_old_description_stays_absent() {
    let plan = sound();
    let mut document = serde_json::to_value(&plan).expect("a plan serialises");
    document["elections"][0]["contests"][0]["description"] =
        serde_json::json!("");

    let reopened: Blueprint =
        serde_json::from_value(document).expect("it opens");
    assert_eq!(
        reopened.elections[0].contests[0].description,
        Translated::default()
    );
}

/// Which order the ballot is arranged in, said rather than implied.
///
/// The wizard already wrote each contest's `presentation.sort_order` from its
/// place in the list, and nothing said whether the portal should honour it — so an
/// election could be arranged carefully and shuffled anyway, or left alone and
/// expected to shuffle.
#[test]
fn the_order_the_ballot_is_arranged_in_reaches_the_bundle() {
    let mut plan = sound();
    plan.elections_order = "alphabetical".to_string();
    plan.elections[0].contests_order = "random".to_string();

    let workbook = workbook_of(&plan).expect("a workbook");
    assert_eq!(
        workbook.rows(sheet::SHEET_ELECTION_EVENT)[0]
            .get("presentation.elections_order")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"alphabetical\")".to_string())
    );
    assert_eq!(
        workbook.rows(sheet::SHEET_ELECTIONS)[0]
            .get("presentation.contests_order")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"random\")".to_string())
    );
}

/// The arrangement somebody made is the default, since that is what a wizard is.
#[test]
fn a_plan_that_says_nothing_keeps_the_order_it_was_given() {
    let plan = sound();
    assert_eq!(plan.elections_order, "custom");
    assert_eq!(plan.elections[0].contests_order, "custom");
}

/// A sheet whose columns and values disagree is refused, not written.
///
/// Every sheet here builds its column names in one place and its cells in
/// another, and nothing tied them together: adding `elections_order`'s value
/// without its column shifted every later cell one place left, so
/// `presentation.sort_order` was read as a language code and the failure surfaced
/// as "no entry found for key" in a test about languages.
///
/// Asserted through `sheet_of` directly, because producing a ragged sheet from a
/// `Blueprint` now requires editing the builder — which is the point.
#[test]
fn a_ragged_sheet_is_refused_rather_than_written() {
    let refused = sheet_of(
        "Contests",
        vec!["external_id".to_string(), "max_votes".to_string()],
        vec![vec![Cell::text("president")]],
    );
    let problem = refused.expect_err("a short row should be refused");
    assert!(
        problem.message.contains("1 cells under 2 columns"),
        "{}",
        problem.message
    );

    // And the well-formed case still builds.
    assert!(sheet_of(
        "Contests",
        vec!["external_id".to_string(), "max_votes".to_string()],
        vec![vec![Cell::text("president"), Cell::Int(1)]],
    )
    .is_ok());
}

/// The trustees and their threshold travel, and *not* as a key ceremony.
///
/// They used to become one, and it was the wrong shape. A ceremony's `trustee_ids`
/// carries trustee *names*, which the importer resolves against trustees the target
/// tenant already has — one it cannot find becomes `""` and then fails to parse as a
/// `Uuid`, refusing the whole import with a message that never mentions trustees. Neither
/// a spreadsheet nor a browser knows which trustees a tenant has, and no derivation fixes
/// it, because a valid value is a row in that database.
///
/// So this is delivery information. It leaves the wizard in the outer zip, where a person
/// reads it and then makes the ceremony in the Admin Portal, picking trustees from the
/// ones that exist rather than spelling them.
#[test]
fn the_trustees_and_threshold_travel_as_delivery_information() {
    let plan = sound();
    let bundle = compiled(&plan);

    assert!(
        bundle.export["keys_ceremonies"]
            .as_array()
            .expect("an array")
            .is_empty(),
        "the import must not be handed a ceremony naming trustees it cannot resolve"
    );

    // The names and the threshold are not lost — `side_files` carries them beside the
    // archive, which is what the delivery zip nests them into.
    let beside = side_files(&plan);
    let names: Vec<&str> =
        beside.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        names.contains(&"trustees_list.json"),
        "the trustee list should travel beside the archive: {names:?}"
    );

    let trustees = beside
        .iter()
        .find(|(name, _)| name == "trustees_list.json")
        .map(|(_, contents)| contents.as_str())
        .expect("the trustee list");
    assert!(
        trustees.contains(&plan.trustee_threshold.to_string()),
        "the threshold should travel with the names: {trustees}"
    );
}

/// The same plan produces the same ceremony id twice.
#[test]
fn a_key_ceremony_has_a_stable_id() {
    let plan = sound();
    assert_eq!(
        compiled(&plan).export["keys_ceremonies"][0]["id"],
        compiled(&plan).export["keys_ceremonies"][0]["id"]
    );
}

/// No trustees, no ceremony — rather than one with no members.
#[test]
fn a_plan_with_no_trustees_emits_no_ceremony() {
    let mut plan = sound();
    plan.trustees.clear();
    plan.trustee_threshold = 0;
    // No trustees is a warning rather than an error, so this does compile — which
    // is why the early return matters. A ceremony with no members and a threshold
    // of zero would import, and the key would be generated with nobody holding a
    // share of it.
    assert!(says(&checked(&plan), "no trustees"));
    assert!(compiled(&plan).export["keys_ceremonies"]
        .as_array()
        .expect("an array")
        .is_empty());
}

/// A nameless trustee is refused, because the importer resolves by name.
#[test]
fn a_trustee_with_no_name_is_refused() {
    // `.unwrap_or_default()` in the importer turns an unmatched name into an empty
    // string, so a blank one here is a ceremony member who does not exist — and
    // nothing downstream reports it.
    let mut plan = sound();
    plan.trustees[0].name = "  ".to_string();

    let report = checked(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "resolves the key ceremony's"));
}

/// Shown by default, because verifiability should be argued out of, not into.
#[test]
fn a_voter_can_look_up_their_own_ballot_unless_the_plan_says_otherwise() {
    let plan = sound();
    assert_eq!(plan.show_cast_vote_logs, "show-logs-tab");
    assert_eq!(
        workbook_of(&plan)
            .expect("a workbook")
            .rows(sheet::SHEET_ELECTION_EVENT)[0]
            .get("presentation.show_cast_vote_logs")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"show-logs-tab\")".to_string())
    );
}

/// Two messages sharing a name would translate to whichever the catalogue held.
///
/// Read off the source rather than off a report, because no fixture triggers every
/// check at once and the failure this guards against is a *new* message copying its
/// neighbour's id — which is exactly the moment nobody is looking at the whole list.
#[test]
fn every_named_problem_has_its_own_name() {
    let source = include_str!("architect.rs");
    let mut names: Vec<&str> = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find(".id(\"") {
        rest = &rest[at + 5..];
        let end = rest.find('"').expect("an id is a string literal");
        names.push(&rest[..end]);
        rest = &rest[end..];
    }

    assert!(
        names.len() >= 35,
        "expected the plan checks to be named; found {}",
        names.len()
    );

    let mut sorted = names.clone();
    sorted.sort_unstable();
    let mut duplicated: Vec<&str> = sorted
        .windows(2)
        .filter(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
        .collect();
    duplicated.dedup();
    assert_eq!(
        duplicated,
        Vec::<&str>::new(),
        "two messages share one name"
    );

    // `<area>.<complaint>`, because the area alone is the path — and the path is
    // precisely what cannot tell two complaints about one contest apart.
    let shapeless: Vec<&&str> = names
        .iter()
        .filter(|name| !name.contains('.') || name.ends_with('.'))
        .collect();
    assert_eq!(
        shapeless,
        Vec::<&&str>::new(),
        "an id names an area and a complaint"
    );
}

/// A complaint with no name shows English inside a Spanish wizard.
#[test]
fn every_complaint_about_a_plan_carries_its_name_and_its_specifics() {
    let mut plan = sound();
    // One plan, many different mistakes, so this covers more than one check each
    // time it runs rather than asserting one message at a time.
    plan.name = Default::default();
    plan.external_id = String::new();
    plan.default_language = Some("de".to_string());
    plan.trustees[0].email = "not an address".to_string();
    plan.trustee_threshold = 1;
    plan.elections[0].contests[0].winners = 9;
    plan.contacts.clear();

    let report = checked(&plan);
    assert!(report.problems.len() >= 6, "expected several complaints");

    let unnamed: Vec<&str> = report
        .problems
        .iter()
        .filter(|problem| problem.id.is_none())
        .map(|problem| problem.message.as_str())
        .collect();
    assert_eq!(unnamed, Vec::<&str>::new(), "every complaint is named");

    // The specifics travel as data, or a translation can only say something vaguer
    // than the English it replaces.
    let email = report
        .problems
        .iter()
        .find(|problem| {
            problem.id.as_deref() == Some("trustee.email-malformed")
        })
        .expect("the malformed address");
    assert_eq!(
        email.details.get("email").map(String::as_str),
        Some("not an address")
    );

    let elects = report
        .problems
        .iter()
        .find(|problem| {
            problem.id.as_deref() == Some("contest.elects-more-than-chosen")
        })
        .expect("the contest arithmetic");
    assert_eq!(elects.details.get("winners").map(String::as_str), Some("9"));
    assert!(elects.details.contains_key("chosen"));
}

/// Two empty boxes are not two people.
#[test]
fn a_trustee_row_with_nothing_in_it_is_nobody() {
    // The wizard keeps the list at least as long as the threshold, so a new plan
    // arrives holding two blank rows. Counting those would report the commonest
    // state of a new plan as complete — and would replace "nobody is holding the
    // key", which is the thing that matters, with two complaints about empty fields.
    let mut plan = sound();
    plan.trustees = vec![
        Trustee {
            name: String::new(),
            email: String::new(),
        },
        Trustee {
            name: "  ".to_string(),
            email: " ".to_string(),
        },
    ];

    let report = checked(&plan);
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref() == Some("trustees.none")),
        "two blank rows should read as no trustees, not as two"
    );

    // And one real person among blanks is one person.
    plan.trustees[0] = Trustee {
        name: "Ada Lovelace".to_string(),
        email: "ada@example.org".to_string(),
    };
    assert!(checked(&plan)
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("trustees.only-one")));
}

/// The half of the language question that had no control.
#[test]
fn a_plan_can_say_which_language_a_voter_starts_in() {
    let mut plan = sound();
    plan.language_detection_policy = Some("force-default".to_string());

    let workbook = workbook_of(&plan).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_ELECTION_EVENT);
    assert_eq!(
        rows[0]
            .get("presentation.language_conf.language_detection_policy")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"force-default\")".to_string())
    );
}

#[test]
fn a_language_detection_policy_the_platform_never_heard_of_is_refused() {
    // The one language setting whose values are not the plan's own languages, so
    // the only one that can carry a word from nowhere.
    let mut plan = sound();
    plan.language_detection_policy = Some("guess".to_string());

    let report = checked(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "is not a language detection policy"));
}

#[test]
fn the_branding_switches_reach_the_event_row() {
    // Both are already written by `election_event.hbs`, so before this the wizard
    // shipped a value for each and offered no way to choose it.
    let mut plan = sound();
    plan.skip_election_list = Some(true);
    plan.show_user_profile = Some(true);
    plan.materials_activated = Some(true);

    let workbook = workbook_of(&plan).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_ELECTION_EVENT);
    for column in [
        "presentation.skip_election_list",
        "presentation.show_user_profile",
        "presentation.materials.activated",
    ] {
        assert_eq!(
            rows[0].get(column).map(|cell| format!("{cell:?}")),
            Some("Bool(true)".to_string()),
            "{column}"
        );
    }
}

#[test]
fn support_material_headings_travel_per_language() {
    let mut plan = sound();
    plan.languages = vec!["en".to_string(), "es".to_string()];
    plan.materials_title = Translated::new("Voter guides");
    plan.materials_title
        .by_language
        .insert("es".to_string(), "Guías del votante".to_string());
    plan.materials_subtitle = Translated::new("How to vote");

    let workbook = workbook_of(&plan).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_ELECTION_EVENT);
    assert_eq!(
        rows[0]
            .get("presentation.i18n.es.materialsTitle")
            .map(|cell| format!("{cell:?}")),
        Some("String(\"Guías del votante\")".to_string())
    );
    assert!(rows[0]
        .get("presentation.i18n.en.materialsSubtitle")
        .is_some());
}

/// A plan written before any of these existed compiles to the bytes it used to.
#[test]
fn a_plan_that_says_nothing_new_writes_no_new_columns() {
    // Each field is emitted only when set, because `election_event.hbs` already
    // carries a value for every one of them — so an absent column leaves the
    // template's own, and a plan saved last week is unaffected.
    let workbook = workbook_of(&sound()).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_ELECTION_EVENT);
    for column in [
        "presentation.language_conf.language_detection_policy",
        "presentation.skip_election_list",
        "presentation.show_user_profile",
        "presentation.materials.activated",
        "presentation.i18n.en.materialsTitle",
    ] {
        assert!(rows[0].get(column).is_none(), "{column} should be absent");
    }
}

/// A plan written before the field existed still opens, and still means manual.
#[test]
fn a_plan_saved_before_ceremonies_had_a_policy_still_means_manual() {
    let mut value = serde_json::to_value(sound()).expect("a plan");
    value
        .as_object_mut()
        .expect("an object")
        .remove("ceremony_policy");

    let reopened: Blueprint =
        serde_json::from_value(value).expect("a plan with no policy");
    assert_eq!(
        reopened.ceremony_policy,
        CeremoniesPolicy::MANUAL_CEREMONIES
    );
}

/// The end of the claim that a support material could not travel in a bundle.
#[test]
fn a_support_material_reaches_both_the_json_and_the_archive() {
    let mut plan = sound();
    plan.materials_activated = Some(true);
    plan.materials = vec![PlannedMaterial {
        external_id: "rules".to_string(),
        title: Translated::new("Rules of the election"),
        kind: "document".to_string(),
        file_name: "rules.pdf".to_string(),
        bytes: b"%PDF-1.4 rules".to_vec(),
        is_hidden: false,
    }];

    let compiled = compiled(&plan);

    // The row, carrying the identifier that puts the archive entry into the
    // importer's replacement map.
    let rows = compiled.export["support_materials"]
        .as_array()
        .expect("support_materials is an array");
    assert_eq!(rows.len(), 1);
    let document_id = rows[0]["document_id"].as_str().expect("a document id");
    assert_eq!(
        rows[0]["election_event_id"],
        compiled.export["election_event"]["id"]
    );

    // And the file, under the private folder rather than the public one.
    //
    // Asserted against the **archive layout** rather than `bundle.materials`,
    // because they are different claims: the second is a list the bundle carries,
    // the first is a file that will be in the zip. Only the second is what an
    // importer sees.
    let entry = format!("export_S3_files/document_{document_id}_rules.pdf");
    let layout = crate::election_config::archive::layout(&compiled);
    assert!(
        layout.importable.iter().any(|file| file.name == entry),
        "expected {entry} in the archive, got {:?}",
        layout
            .importable
            .iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>()
    );
}

/// The failure mode the folder choice exists to avoid.
#[test]
fn a_support_material_is_private_and_a_photograph_is_public() {
    // Swapped, a material is published to anybody holding the URL and a
    // photograph 404s on every ballot — neither of which fails a build.
    let file = MaterialFile {
        document_id: "d".to_string(),
        file_name: "rules.pdf".to_string(),
        bytes: Vec::new(),
    };
    assert!(file.entry_name().starts_with("export_S3_files/"));

    let image = ImageFile {
        document_id: "d".to_string(),
        file_name: "face.png".to_string(),
        bytes: Vec::new(),
    };
    assert!(image.entry_name().starts_with("images/"));
}

#[test]
fn materials_shipped_with_the_tab_switched_off_are_said_out_loud() {
    let mut plan = sound();
    plan.materials_activated = Some(false);
    plan.materials = vec![PlannedMaterial {
        external_id: "rules".to_string(),
        file_name: "rules.pdf".to_string(),
        bytes: b"x".to_vec(),
        ..Default::default()
    }];

    let compiled = compiled(&plan);
    let report = crate::election_config::validate::validate(
        &serde_json::from_value(compiled.export.clone()).expect("the schema"),
    );
    assert!(report
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("material.tab-off")));
}

/// The workbook carries them too, which is what makes the janitor's route work.
#[test]
fn a_workbook_can_carry_support_materials() {
    // The plan writes a Materials sheet, so a spreadsheet describing the same
    // thing produces the same rows: the sheet names the file, the bytes travel
    // beside it. That is the whole answer to "a cell cannot hold a document".
    let mut plan = sound();
    plan.materials_activated = Some(true);
    plan.materials = vec![PlannedMaterial {
        external_id: "rules".to_string(),
        title: Translated::new("Rules of the election"),
        kind: "document".to_string(),
        file_name: "rules.pdf".to_string(),
        bytes: b"%PDF".to_vec(),
        is_hidden: false,
    }];

    let workbook = workbook_of(&plan).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_MATERIALS);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].text("file"), Some("rules.pdf"));
    assert_eq!(rows[0].text("external_id"), Some("rules"));
    assert_eq!(
        rows[0]
            .get("presentation.i18n.en.title")
            .and_then(serde_json::Value::as_str),
        Some("Rules of the election")
    );
}

#[test]
fn a_row_naming_a_file_nobody_supplied_is_refused() {
    // The failure this exists to stop is silent: the row imports, the document it
    // names was never created, and the tab is a list of broken links.
    let mut plan = sound();
    plan.materials = vec![PlannedMaterial {
        external_id: "rules".to_string(),
        file_name: "rules.pdf".to_string(),
        bytes: b"%PDF".to_vec(),
        ..Default::default()
    }];

    let workbook = workbook_of(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    // The sheet, with the bytes deliberately withheld — which is exactly what a
    // workbook arriving without its folder of files looks like.
    let outcome = build(
        &workbook,
        &templates,
        &BuildOptions::default(),
        &Sources::default(),
    );

    let report = match outcome {
        Ok(bundle) => bundle.warnings,
        Err(report) => report,
    };
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref()
                == Some("material.file-missing")),
        "expected material.file-missing, got {report}"
    );
}

#[test]
fn a_file_nobody_names_is_said_out_loud() {
    // The mirror, and the more dangerous of the two: it reaches the archive, the
    // importer creates a document for it, and nothing points at it ever again.
    let plan = sound();
    let workbook = workbook_of(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    let options = BuildOptions {
        materials: vec![MaterialFile {
            document_id: String::new(),
            file_name: "orphan.pdf".to_string(),
            bytes: b"%PDF".to_vec(),
        }],
        ..BuildOptions::default()
    };

    let bundle = build(&workbook, &templates, &options, &Sources::default())
        .expect("a clean build");
    assert!(bundle
        .warnings
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("material.file-unused")));
    // And it does not reach the archive.
    assert!(bundle.materials.is_empty());
}

/// The logo, all the way from a file somebody dropped to a url on a ballot.
///
/// Three things have to agree or the bundle does not import: the archive entry, the
/// identifier inside it, and the url in the JSON. `process_s3_file` fails the whole
/// import when the entry names an identifier the document never mentions, so this
/// asserts the *pair* rather than either half.
#[test]
fn an_uploaded_logo_reaches_both_the_json_and_the_archive() {
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "local-1000.png".to_string(),
        bytes: b"\x89PNG logo".to_vec(),
    });

    let compiled = compiled(&plan);

    let url = compiled.export["election_event"]["presentation"]["logo_url"]
        .as_str()
        .expect("a composed logo url");
    // Bucket-relative. An absolute one would render as
    // `https://bucket/https://…` — `ui-core`'s `getImageUrl` concatenates.
    assert!(
        !url.starts_with("http"),
        "expected a relative url, got {url}"
    );
    assert!(url.ends_with("/local-1000.png"), "got {url}");

    let document_id = url
        .split("/document-")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .expect("an identifier inside the url");

    let entry = format!("images/document_{document_id}_local-1000.png");
    let layout = crate::election_config::archive::layout(&compiled);
    assert!(
        layout.importable.iter().any(|file| file.name == entry),
        "expected {entry} in the archive, got {:?}",
        layout
            .importable
            .iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>()
    );
}

/// Public, not private — the one thing that cannot be seen by reading the JSON.
#[test]
fn a_logo_travels_in_the_public_branch() {
    // `images/` is uploaded with `is_public: true` and `export_S3_files/` is not, so
    // a logo in the private branch 404s on every ballot it is drawn above, and
    // nothing fails.
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
    });

    let images = plan_images(&plan, &sources_of(&plan));
    assert_eq!(images.len(), 1);
    assert!(images[0].entry_name().starts_with("images/"));
}

/// A workbook carries the logo the same way it carries a support material.
#[test]
fn a_workbook_can_carry_the_logo() {
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
    });

    let workbook = workbook_of(&plan).expect("a workbook");
    let rows = workbook.rows(sheet::SHEET_ELECTION_EVENT);
    // The sheet names the file; the bytes travel beside it, because a cell cannot
    // hold one. Exactly the Materials answer, and for the same reason.
    assert_eq!(rows[0].text("presentation.logo_file"), Some("logo.png"));
    // And it does not also write a link, which would be two answers to one
    // question with nothing saying which wins.
    assert_eq!(rows[0].text("presentation.logo_url"), None);
}

/// The control column is consumed, not carried.
#[test]
fn the_logo_file_column_stays_out_of_the_json() {
    // `presentation.logo_file` is not a field of `ElectionEventPresentation`. The
    // deep merge would carry it into the document as one, and the importer now
    // validates against that schema — so the column has to be excluded rather than
    // merely ignored.
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
    });

    let compiled = compiled(&plan);
    assert!(
        compiled.export["election_event"]["presentation"]
            .get("logo_file")
            .is_none(),
        "logo_file leaked into the document"
    );
}

/// A plan that already carries a link still builds to what it used to.
#[test]
fn a_typed_link_still_works_when_there_is_no_file() {
    let mut plan = sound();
    plan.logo_url = Some("tenant-x/document-y/old.png".to_string());

    let compiled = compiled(&plan);
    assert_eq!(
        compiled.export["election_event"]["presentation"]["logo_url"],
        serde_json::json!("tenant-x/document-y/old.png")
    );
}

/// Two answers to one question, said out loud rather than resolved in silence.
#[test]
fn a_plan_carrying_both_a_logo_file_and_a_link_is_told_which_wins() {
    let mut plan = sound();
    plan.logo_url = Some("https://example.org/old.png".to_string());
    plan.logo = Some(CandidateImage {
        file_name: "new.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
    });

    let report = checked(&plan);
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref() == Some("logo.file-and-link")),
        "expected logo.file-and-link, got {report}"
    );
}

/// A name with nothing behind it fails the import rather than losing a picture.
#[test]
fn a_logo_named_with_no_bytes_is_refused() {
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: Vec::new(),
    });

    let report = checked(&plan);
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref() == Some("logo.no-bytes")),
        "expected logo.no-bytes, got {report}"
    );
}

/// The workbook half of the same failure: a sheet naming a file nobody supplied.
#[test]
fn a_sheet_naming_a_logo_nobody_supplied_is_refused() {
    let mut plan = sound();
    plan.logo = Some(CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: b"\x89PNG".to_vec(),
    });

    let workbook = workbook_of(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    // The bytes deliberately withheld — a workbook arriving without its folder.
    let outcome = build(
        &workbook,
        &templates,
        &BuildOptions::default(),
        &Sources::default(),
    );
    let report = match outcome {
        Ok(bundle) => bundle.warnings,
        Err(report) => report,
    };
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref() == Some("logo.file-missing")),
        "expected logo.file-missing, got {report}"
    );
}

/// A message is not a defect, and nothing about it is said unprompted.
///
/// This asserted the opposite until the `messages.not-automatic` warning was
/// removed: it fired on every plan carrying a message, so the one screen where a
/// client does creative work always ended in an amber panel about a gap in the
/// platform rather than anything in their plan. Kept as the inverse, because
/// "adding a message produces no complaint" is worth holding still.
#[test]
fn a_planned_message_is_not_itself_a_problem() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::InvitationToVote,
        subject: Translated::new("Your ballot is ready"),
        body: Translated::new("Vote here."),
        html: Translated::default(),
        schedule: MessageSchedule::default(),
    });

    let report = checked(&plan);
    assert!(!report.has_errors(), "a message is not a defect");
    assert!(
        !says(&report, "nothing sends these"),
        "the platform's own gap is not this plan's problem: {report}"
    );
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.path == "messages"),
        "a message with nothing wrong with it draws no comment: {report}"
    );
}

/// "Every Monday" is not a time, and whoever sends it should not have to guess.
#[test]
fn a_weekly_repeat_with_no_time_says_so() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::GetOutTheVote,
        subject: Translated::new("You have not voted yet"),
        body: Translated::new("There is still time."),
        html: Translated::default(),
        schedule: MessageSchedule {
            on: vec![],
            weekly: vec![1, 4],
            // Deliberately absent, which is what every plan written before this
            // field existed looks like.
            weekly_at: String::new(),
        },
    });

    let report = checked(&plan);
    assert!(!report.has_errors(), "a missing hour does not stop a build");
    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref()
                == Some("messages.weekly-no-time")),
        "expected messages.weekly-no-time, got {report}"
    );
}

/// And it stops saying so once the hour is there, or the warning is noise.
#[test]
fn a_weekly_repeat_with_a_time_is_quiet_about_it() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::GetOutTheVote,
        subject: Translated::new("You have not voted yet"),
        body: Translated::new("There is still time."),
        html: Translated::default(),
        schedule: MessageSchedule {
            on: vec![],
            weekly: vec![1, 4],
            weekly_at: "09:30".to_string(),
        },
    });

    let report = checked(&plan);
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref()
                == Some("messages.weekly-no-time")),
        "the hour is there, so nothing to say: {report}"
    );
}

/// A send-once message has no weekly hour to be missing.
#[test]
fn a_message_that_does_not_repeat_is_not_asked_for_an_hour() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::InvitationToVote,
        subject: Translated::new("Your ballot is ready"),
        body: Translated::new("Vote here."),
        html: Translated::default(),
        // No `weekly`, so the question does not arise. Without this the check
        // would nag every single-send message on the screen, which is how a
        // warning list stops being read.
        schedule: MessageSchedule::default(),
    });

    let report = checked(&plan);
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.id.as_deref()
                == Some("messages.weekly-no-time")),
        "a message that never repeats has no hour to give: {report}"
    );
}

/// Templates belong to the tenant, so they travel beside the import, not in it.
#[test]
fn messages_leave_as_two_files_outside_the_bundle() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::GetOutTheVote,
        subject: Translated::new("You have not voted yet"),
        body: Translated::new("There is still time."),
        html: Translated::new("<p>There is still time.</p>"),
        schedule: MessageSchedule {
            // Several dates, which is the shape a reminder campaign has.
            on: vec![],
            weekly: vec![1, 4],
            weekly_at: "09:30".to_string(),
        },
    });

    let files = side_files(&plan);
    let named = |want: &str| {
        files
            .iter()
            .find(|(name, _)| name == want)
            .map(|(_, text)| text.clone())
    };

    let templates = named("admin_portal/communication_templates.json")
        .expect("the Admin Portal's own file");
    assert!(templates.contains("get-out-the-vote"));
    // The HTML body travels: a sender that can render it should not have to be
    // handed the plain text and told to guess the markup.
    assert!(templates.contains("<p>There is still time.</p>"));

    let messaging = named("voter_messaging.json").expect("the schedule");
    assert!(messaging.contains("\"sends_automatically\": false"));
    assert!(messaging.contains("\"weekly\""));
    // The hour, in the file somebody actually reads to send these by hand. The
    // `sends` object is built field by field, so a field not named there is
    // dropped without anything failing — this is what notices.
    assert!(
        messaging.contains("09:30"),
        "the send file carries the hour, not only the day: {messaging}"
    );

    // And not inside the import, which is the whole point of it being here.
    //
    // Asserted on the built bundle rather than on the workbook, and the difference
    // matters now: the workbook carries a Messages sheet so that the spreadsheet in
    // a delivery is the whole plan, and `build` never reads it. "Not in the
    // workbook" and "not in the import" were the same sentence until there were
    // sheets the builder does not visit. This is the one that was always meant.
    let bundle = compiled(&plan);
    assert!(
        !bundle.export.to_string().contains("get-out-the-vote"),
        "a tenant's templates must not ride in an election event import"
    );
    assert!(
        !bundle
            .templates
            .iter()
            .any(|template| template.alias.contains("get-out-the-vote")),
        "and the builder minted no template from the plan's messages"
    );

    // The sheet is there, though — that is what puts it in the delivery's
    // spreadsheet rather than only in two JSON files beside it.
    let workbook = workbook_of(&plan).expect("a workbook");
    assert!(workbook.has("messages"), "the plan's messages are a sheet");
}

/// The verdict may not lie: anything the builder refuses, the wizard refuses first.
///
/// This is the invariant behind a bug that reached a user. The Final Review screen
/// said **Ready to build** for a plan that could not be built, because
/// `validate_plan` and `compile_plan` check different things and nothing held them
/// to each other. What a person then does is press the button and get nothing —
/// there is no worse outcome for this screen, because the verdict is the one thing
/// on it they are entitled to trust.
///
/// Written as a family of mutations rather than one case on purpose. The specific
/// defect was a census whose area could not be resolved, and fixing only that would
/// leave the next divergence to be found the same way. Each mutation below breaks a
/// plan in a way somebody plausibly could, and the assertion is not about *which*
/// message comes back — the two layers word things differently and should — but that
/// `validate_plan` refuses whenever `compile_plan` does.
///
/// The converse is deliberately not asserted. `validate_plan` may be stricter: it
/// carries warnings and advice the builder has no opinion about, and a wizard that
/// only ever said what the builder says would be a thinner screen.
#[test]
fn a_plan_the_builder_refuses_is_refused_by_the_wizard_first() {
    /// A plan, and the name of what was done to it.
    fn mutations() -> Vec<(&'static str, Blueprint)> {
        let mut cases: Vec<(&'static str, Blueprint)> = Vec::new();

        // The defect that started this: a voter whose area is blank. It was
        // accepted here as "the default area" and refused by the builder, which
        // needs an `area.external_id` on every row.
        let mut blank_area = sound();
        blank_area.voters = vec![PlannedVoter {
            username: "ada".into(),
            area_external_id: String::new(),
            ..Default::default()
        }];
        cases.push(("a voter with no area", blank_area));

        // The same, spelled wrong rather than left out — the likelier mistake,
        // and the one a copy-paste from a spreadsheet makes.
        let mut wrong_area = sound();
        wrong_area.voters = vec![PlannedVoter {
            username: "ada".into(),
            area_external_id: "nowhere".into(),
            ..Default::default()
        }];
        cases.push(("a voter in an area that does not exist", wrong_area));

        // An area with no name. The voters CSV identifies an area by name, so
        // this is unreachable rather than merely untidy.
        let mut unnamed_area = sound();
        if let Some(area) = unnamed_area.areas.first_mut() {
            area.name = String::new();
        }
        cases.push(("an area with no name", unnamed_area));

        // Nothing to vote on.
        let mut no_contests = sound();
        for election in &mut no_contests.elections {
            election.contests.clear();
        }
        cases.push(("an election with no contests", no_contests));

        // A contest with nothing on it.
        let mut no_candidates = sound();
        for election in &mut no_candidates.elections {
            for contest in &mut election.contests {
                contest.candidates.clear();
            }
        }
        cases.push(("a contest with no candidates", no_candidates));

        // Two things sharing an identifier. Identifiers are event-wide, so the
        // second silently replaces the first.
        let mut duplicate = sound();
        if let Some(election) = duplicate.elections.first_mut() {
            if let Some(contest) = election.contests.first_mut() {
                if contest.candidates.len() >= 2 {
                    let first = contest.candidates[0].external_id.clone();
                    contest.candidates[1].external_id = first;
                }
            }
        }
        cases.push(("two candidates sharing an identifier", duplicate));

        // More choices than there are things to choose.
        let mut too_many = sound();
        if let Some(election) = too_many.elections.first_mut() {
            if let Some(contest) = election.contests.first_mut() {
                contest.max_votes = 99;
            }
        }
        cases.push((
            "a contest allowing more choices than it has options",
            too_many,
        ));

        cases
    }

    let templates = TemplateSet::builtin().unwrap();
    let mut lies: Vec<String> = Vec::new();

    for (what, plan) in mutations() {
        let compiled = compile_plan(Compile {
            plan: &plan,
            templates: &templates,
            options: &BuildOptions::default(),
            profile: None,
            sources: None,
        });

        // Two ways the builder refuses: an `Err` report, or an `Ok` carrying
        // errors. Both are refusals, and only checking the first would miss the
        // case that produced the original bug.
        let builder_refuses = match &compiled {
            Err(report) => report.has_errors(),
            Ok(compiled) => compiled.report.has_errors(),
        };

        if !builder_refuses {
            continue;
        }

        let verdict = checked(&plan);
        if !verdict.has_errors() {
            lies.push(format!(
                "{what}: the builder refuses it and validate_plan says it is fine"
            ));
        }
    }

    assert!(
        lies.is_empty(),
        "the Final Review verdict would say Ready to build for a plan that \
         cannot be built:\n  {}",
        lies.join("\n  ")
    );
}

/// The bundle says which platform version wrote it.
///
/// This is the field that lets an importer refuse a bundle it cannot read *as a version
/// mismatch* rather than as a serde error twelve fields deep, and it is the reason this
/// tool does not need to emit anything for older platforms.
///
/// Worth a test of its own because of how a real report played out. A zip from this tool
/// was refused by a v9.5 environment with `election_event: missing field \`name\` at line
/// 244 column 3` — `name` having become `external_id` in `f016a4b4a7`. The tempting fix
/// was to emit `name` again for the benefit of older platforms. It was the wrong one:
/// the bundle correctly declared `v10.0.0`, and v9.5's importer deserialized *before*
/// checking the version, so the gate that exists precisely to say "upgrade your system"
/// never ran. That ordering is already fixed on `main`, which reads the version off the
/// raw JSON first.
///
/// So the contract this tool owes is exactly this one: say what wrote you, truthfully.
/// Shaping the output for every platform that ever existed is the alternative, and it
/// has no end.
#[test]
fn the_bundle_declares_the_version_that_wrote_it() {
    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let declared = compiled
        .bundle
        .export
        .get("version")
        .and_then(|value| value.as_str())
        .expect("the export should say which version wrote it");

    assert_eq!(
        declared,
        crate::election_config::build::DEFAULT_VERSION,
        "the bundle should declare the version this build writes"
    );

    // And the platform's own checker has to accept it against itself. `extract_semver`
    // is private, so this asks the public function the importer actually calls: a
    // version that cannot be parsed is refused as "Could not parse imported version"
    // rather than as a mismatch, which is the gate failing open in the other direction.
    assert!(
        crate::util::version::check_version_compatibility(declared, declared).is_ok(),
        "the platform's own version check cannot read '{declared}', so an importer \
         would refuse this bundle without ever reaching a mismatch verdict"
    );

    // And the version has to *discriminate*, which is the property the whole
    // arrangement rests on: a platform of a different major must refuse this bundle on
    // the version alone, before it ever tries to read a field that has moved. Asserted
    // in both directions because both happen — an operator importing into an
    // environment behind this build, and into one ahead of it.
    let (major, _, _) = declared
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .fold((0u32, 0u32, 0u32), |(a, b, _), next| {
            (if a == 0 { next } else { a }, b, 0)
        });

    for other in [major.saturating_sub(1), major + 1] {
        if other == major {
            continue;
        }
        let platform = format!("{other}.0.0");
        assert!(
            crate::util::version::check_version_compatibility(declared, &platform)
                .is_err(),
            "a platform on {platform} would accept a {declared} bundle, so a schema \
             change between them surfaces as a serde error deep in the file instead of \
             'Please upgrade your system' — which is exactly how a v9.5 environment \
             reported `missing field \\`name\\`` for a v10 export"
        );
    }
}

/// The archive names no trustees, because it cannot name them correctly.
///
/// A real import failed with `Error parsing trustee_ids as UUIDs`, and the message is
/// worth unpacking because it says nothing about what went wrong. A keys ceremony's
/// `trustee_ids` carries trustee **names**, which the importer resolves against the
/// trustees a tenant already has; a name it cannot find becomes an empty string through
/// `unwrap_or_default`, and the insert then tries to parse `""` as a `Uuid`. So one
/// unrecognised name does not produce a ceremony with a gap — it refuses the entire
/// import, and the event is never created.
///
/// The wizard runs in a browser with no connection to the environment being imported
/// into, so it cannot know which trustees exist and cannot emit this safely. It emits
/// nothing, says so in the report, and the names, threshold and dates travel in
/// `auxiliary` for the person who will make the ceremony in the Admin Portal.
#[test]
fn the_archive_carries_no_keys_ceremony_it_cannot_fill_in_correctly() {
    let templates = TemplateSet::builtin().unwrap();
    let plan = sound();
    assert!(
        !plan.trustees.is_empty(),
        "this test is meaningless unless the plan names trustees"
    );

    let compiled = compile_plan(Compile {
        plan: &plan,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let ceremonies = compiled
        .bundle
        .export
        .get("keys_ceremonies")
        .and_then(|value| value.as_array())
        .expect("the key should still be present, as an empty list");
    assert!(
        ceremonies.is_empty(),
        "a ceremony naming trustees the tenant may not have refuses the whole import: \
         {ceremonies:?}"
    );

    // The names and the threshold still leave the wizard — beside the archive, not
    // inside it, because they are for a person rather than for the importer.
    let beside: Vec<&str> = compiled
        .layout
        .auxiliary
        .iter()
        .map(|artifact| artifact.name.as_str())
        .collect();
    assert!(
        beside.iter().any(|name| name.contains("ceremony")),
        "the ceremony details should travel beside the archive: {beside:?}"
    );
}

/// What a client is handed: a zip that is not importable, holding one that is.
///
/// The shape is `election_architect`'s, and the nesting is the point. Handing over the
/// importable zip alone loses the reopenable plan, the trustee list and the ceremony
/// dates; handing the loose files over beside it leaves somebody to work out which
/// single file goes to the Admin Portal, and one of the others can carry administrator
/// passwords. Nested, the only thing that can be imported is the only thing that looks
/// like an import.
#[test]
fn the_delivery_is_a_zip_that_is_not_importable_holding_one_that_is() {
    use crate::election_config::archive;

    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let delivery =
        archive::delivery(&compiled.layout).expect("a delivery is written");

    // Read it back rather than trusting the writer: a zip of the right length whose
    // members are nonsense is a failure this repository has shipped before.
    let mut outer = zip::ZipArchive::new(std::io::Cursor::new(&delivery.bytes))
        .expect("the delivery should be a readable zip");

    let names: Vec<String> = (0..outer.len())
        .map(|at| outer.by_index(at).unwrap().name().to_string())
        .collect();

    assert!(
        names.contains(&archive::IMPORTABLE_MEMBER.to_string()),
        "the delivery should nest the importable zip: {names:?}"
    );

    // The delivery information travels here, and must not be inside the importable
    // member — `blueprint.json` is the plan somebody reopens, and the trustee list and
    // ceremony dates are what the Admin Portal must never be handed.
    for expected in [
        "blueprint.json",
        "trustees_list.json",
        "ceremony_schedule.json",
    ] {
        assert!(
            names.contains(&expected.to_string()),
            "the delivery should carry {expected}: {names:?}"
        );
    }

    // And the nested member really is the importable bundle.
    let mut nested = Vec::new();
    {
        use std::io::Read;
        outer
            .by_name(archive::IMPORTABLE_MEMBER)
            .expect("the nested member")
            .read_to_end(&mut nested)
            .expect("the nested member reads");
    }
    let inner = zip::ZipArchive::new(std::io::Cursor::new(&nested))
        .expect("the nested member should itself be a zip");
    let inner_names: Vec<String> = (0..inner.len())
        .map(|at| inner.clone().by_index(at).unwrap().name().to_string())
        .collect();
    assert!(
        inner_names.iter().any(|name| name.ends_with(".json")),
        "the importable member should carry the bundle: {inner_names:?}"
    );
    assert!(
        !inner_names.iter().any(|name| name == "blueprint.json"),
        "the plan must not be inside the importable member: {inner_names:?}"
    );
}

/// No keys ceremony is emitted, by either producer.
///
/// `trustee_ids` carries trustee *names*, resolved against trustees the target tenant
/// already has; one it cannot find becomes `""` and then fails to parse as a `Uuid`,
/// refusing the whole import with a message that never mentions trustees. Neither a
/// spreadsheet nor a browser knows which trustees a tenant has, and no derivation helps
/// — a valid value is a row in that database. So it is delivery information, and the
/// export says nothing about it.
#[test]
fn no_keys_ceremony_is_ever_emitted() {
    let templates = TemplateSet::builtin().unwrap();
    let plan = sound();
    assert!(
        !plan.trustees.is_empty(),
        "meaningless unless the plan names trustees"
    );

    let compiled = compile_plan(Compile {
        plan: &plan,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let ceremonies = compiled
        .bundle
        .export
        .get("keys_ceremonies")
        .and_then(|value| value.as_array())
        .expect("the key stays present, as an empty list");
    assert!(
        ceremonies.is_empty(),
        "naming trustees a tenant may not have refuses the whole import: {ceremonies:?}"
    );
}

/// A delivery goes out and the same plan comes back.
///
/// The round trip is the thing worth testing, and it is testable here because both ends
/// are here: `archive::delivery` writes the zip and `archive::plan_in_delivery` reads it.
/// Doing either in TypeScript would have put the layout in two places and let them drift —
/// and drift in this particular pair means a client's own configuration will not reopen,
/// which is the failure they notice a year later when they need last year's answers.
#[test]
fn a_delivery_reopens_as_the_plan_that_made_it() {
    use crate::election_config::archive;

    let templates = TemplateSet::builtin().unwrap();
    let plan = sound();
    let compiled = compile_plan(Compile {
        plan: &plan,
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    let delivery =
        archive::delivery(&compiled.layout).expect("a delivery is written");
    let reopened = archive::plan_in_delivery(&delivery.bytes)
        .expect("the delivery should carry the plan that made it");

    let back: Blueprint =
        serde_json::from_slice(&reopened).expect("the plan should deserialize");

    // The identity a person recognises, rather than a byte comparison: the plan is
    // serialized with `skip_serializing_if` in places, so equality of bytes is a stricter
    // claim than this needs to make and would fail for reasons nobody cares about.
    assert_eq!(back.external_id, plan.external_id);
    assert_eq!(back.name, plan.name);
    assert_eq!(back.elections.len(), plan.elections.len());
    assert_eq!(back.trustees.len(), plan.trustees.len());
    assert_eq!(back.trustee_threshold, plan.trustee_threshold);
}

/// A zip with no plan in it says what it had instead.
///
/// The likely mistake is handing over the *importable* member — it is a zip, it is the one
/// with the official-sounding name, and it deliberately does not contain the plan. A
/// refusal that only said "no plan" would leave somebody opening both files to find out
/// which was which.
#[test]
fn a_zip_without_a_plan_says_what_it_had_instead() {
    use crate::election_config::archive;

    let templates = TemplateSet::builtin().unwrap();
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &templates,
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("a sound plan compiles");

    // The importable member is exactly the wrong file to hand back, and the likeliest.
    let importable = archive::zip(&compiled.layout.importable).expect("a zip");
    let refused = archive::plan_in_delivery(&importable)
        .expect_err("the importable member carries no plan");

    assert!(
        refused.message.contains(archive::PLAN_MEMBER),
        "the refusal should name what it looked for: {}",
        refused.message
    );
    assert!(
        refused.message.contains(".json") || refused.message.contains(".csv"),
        "the refusal should list what the zip did contain: {}",
        refused.message
    );

    // And something that is not a zip at all fails as that, not as a missing member.
    let refused = archive::plan_in_delivery(b"this is not a zip")
        .expect_err("plain text is not a configuration");
    assert!(
        refused.message.contains("not a configuration"),
        "{}",
        refused.message
    );
}

/// A delivery carries the spreadsheet, beside the plan and outside the import.
///
/// Auxiliary on purpose: the platform's importer has never been handed a
/// spreadsheet and must not start now. It is there for the person — the same
/// configuration as `blueprint.json`, in the format they work in.
#[cfg(feature = "election_config_xlsx")]
#[test]
fn a_delivery_carries_the_workbook_beside_the_plan() {
    let compiled = compile_plan(Compile {
        plan: &sound(),
        templates: &TemplateSet::builtin().unwrap(),
        options: &BuildOptions::default(),
        profile: None,
        sources: None,
    })
    .expect("the sample plan compiles");

    let auxiliary: Vec<&str> = compiled
        .layout
        .auxiliary
        .iter()
        .map(|file| file.name.as_str())
        .collect();

    assert!(
        auxiliary.contains(&crate::election_config::archive::WORKBOOK_MEMBER),
        "the delivery carries the spreadsheet: {auxiliary:?}"
    );
    assert!(
        auxiliary.contains(&"blueprint.json"),
        "and the plan, which is still the full-fidelity record"
    );
    assert!(
        !compiled
            .layout
            .importable
            .iter()
            .any(|file| file.name.ends_with(".xlsx")),
        "and no spreadsheet reaches the importer"
    );

    // It is a real file, not an empty placeholder.
    let workbook = compiled
        .layout
        .auxiliary
        .iter()
        .find(|file| {
            file.name == crate::election_config::archive::WORKBOOK_MEMBER
        })
        .unwrap();
    assert!(workbook.bytes.starts_with(b"PK"), "and it is a zip");
}

// -- the sheets only a plan holds -----------------------------------------------

/// The six exist so the delivery's spreadsheet is the whole plan.
///
/// Contacts, trustees, the ceremony and its dates, the milestones, the messages and
/// the notes were reachable only as JSON files beside the archive, which is fine for
/// a machine and useless to somebody working in a spreadsheet.
#[test]
fn the_plans_own_sheets_are_in_the_workbook() {
    let mut plan = sound();
    plan.contacts = vec![Contact {
        name: "Dana Reed".to_string(),
        role: "Returning officer".to_string(),
        email: "dana@example.org".to_string(),
    }];
    plan.notes = "Ask about the hall booking.".to_string();
    plan.schedule.milestones = vec![Milestone {
        event: "Nominations close".to_string(),
        date: "2027-02-01".to_string(),
    }];

    let workbook = workbook_of(&plan).unwrap();

    assert_eq!(workbook.rows("contacts").len(), 1);
    assert_eq!(
        workbook.rows("contacts")[0].get("email"),
        Some(&serde_json::json!("dana@example.org"))
    );
    assert!(
        !workbook.rows("trustees").is_empty(),
        "the sound plan has trustees"
    );
    assert_eq!(workbook.rows("milestones").len(), 1);
    assert_eq!(
        workbook.rows("notes")[0].get("notes"),
        Some(&serde_json::json!("Ask about the hall booking."))
    );
}

/// The threshold and the policy, on a sheet of their own rather than as columns.
///
/// Load-bearing: `build` deep-merges every non-control column on the ElectionEvent
/// row onto the event template as a dotted path, so a `trustee_threshold` column
/// there would end up inside the exported event JSON. A sheet cannot leak.
#[test]
fn the_ceremony_sheet_keeps_the_threshold_out_of_the_event() {
    let mut plan = sound();
    plan.trustee_threshold = 3;

    let workbook = workbook_of(&plan).unwrap();
    let value = |key: &str| {
        workbook
            .rows("ceremony")
            .iter()
            .find(|row| row.get("key") == Some(&serde_json::json!(key)))
            .and_then(|row| row.get("value"))
            .cloned()
    };

    assert_eq!(value("threshold"), Some(serde_json::json!(3)));
    assert_eq!(
        value("policy"),
        Some(serde_json::json!("manual-ceremonies"))
    );

    let bundle = compiled(&plan);
    assert!(
        !bundle.export["election_event"]
            .as_object()
            .unwrap()
            .contains_key("threshold"),
        "the threshold is not a field of an election event"
    );
}

/// A ceremony time keeps its zone and its offset, not just its wall clock.
///
/// Three rows rather than one, because a timestamp is three things: what somebody
/// typed, where they were, and the offset resolved at that moment. Writing only the
/// first would move a ceremony by an hour when the file is reopened elsewhere.
#[test]
fn a_ceremony_time_survives_with_its_zone() {
    let mut plan = sound();
    plan.schedule.key_ceremony = Some(Timestamp::new(
        "2027-03-01T09:00",
        "America/Los_Angeles",
        -480,
    ));

    let workbook = workbook_of(&plan).unwrap();
    let value = |key: &str| {
        workbook
            .rows("ceremony")
            .iter()
            .find(|row| row.get("key") == Some(&serde_json::json!(key)))
            .and_then(|row| row.get("value"))
            .cloned()
    };

    assert_eq!(
        value("key_ceremony"),
        Some(serde_json::json!("2027-03-01T09:00"))
    );
    assert_eq!(
        value("key_ceremony.zone"),
        Some(serde_json::json!("America/Los_Angeles"))
    );
    assert_eq!(
        value("key_ceremony.offset_minutes"),
        Some(serde_json::json!(-480))
    );
}

/// Adding them changes nothing the importer sees.
///
/// The guarantee that makes them safe: `build` reads only the sheets it names, so
/// six more tabs cannot alter a byte of the bundle.
#[test]
fn the_plans_own_sheets_do_not_reach_the_bundle() {
    let mut plain = sound();
    plain.contacts = Vec::new();
    plain.trustees = Vec::new();
    plain.messages = Vec::new();
    plain.notes = String::new();
    plain.schedule.milestones = Vec::new();

    let mut furnished = plain.clone();
    furnished.contacts = vec![Contact {
        name: "Dana Reed".to_string(),
        role: "Returning officer".to_string(),
        email: "dana@example.org".to_string(),
    }];
    furnished.notes = "Anything at all.".to_string();
    furnished.schedule.milestones = vec![Milestone {
        event: "Nominations close".to_string(),
        date: "2027-02-01".to_string(),
    }];

    assert_eq!(
        compiled(&plain).export,
        compiled(&furnished).export,
        "the importer sees the same event either way"
    );
}

/// An empty list is no sheet at all, rather than a tab with only headers.
#[test]
fn a_plan_that_says_nothing_grows_no_empty_tabs() {
    let mut plan = sound();
    plan.contacts = Vec::new();
    plan.messages = Vec::new();
    plan.notes = "   ".to_string();
    plan.schedule.milestones = Vec::new();

    let workbook = workbook_of(&plan).unwrap();
    assert!(!workbook.has("contacts"));
    assert!(!workbook.has("messages"));
    assert!(!workbook.has("milestones"));
    assert!(!workbook.has("notes"), "whitespace is not a note");
    // The ceremony sheet is always there: a threshold and a policy always exist.
    assert!(workbook.has("ceremony"));
}

/// Every sheet this file writes is one the reader knows.
#[test]
fn every_sheet_the_wizard_writes_is_a_known_one() {
    let mut plan = sound();
    plan.notes = "something".to_string();
    plan.schedule.milestones = vec![Milestone {
        event: "Nominations close".to_string(),
        date: "2027-02-01".to_string(),
    }];
    plan.contacts = vec![Contact::default()];

    let workbook = workbook_of(&plan).unwrap();
    assert_eq!(
        workbook.unread_sheets(),
        Vec::<&str>::new(),
        "a sheet the reader does not know is a tab it would report as a typo"
    );
}

// -- the sheets the wizard has no screens for ----------------------------------

/// A `Sheet` built from a grid of text, the way a workbook's own arrives.
fn platform_sheet(name: &str, grid: &[&[&str]]) -> Sheet {
    let cells: Vec<Vec<crate::election_config::paths::Cell>> = grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|text| crate::election_config::paths::Cell::text(*text))
                .collect()
        })
        .collect();
    Sheet::from_grid(name, &cells).unwrap()
}

/// A Parameters sheet reaches the event, because `build` already knows how.
///
/// This is the whole argument for carrying these sheets rather than interpreting
/// them: `election_event.*` is a prefix `build` already patches onto the event
/// template, so a delivery engineer who opens a real janitor workbook in the wizard
/// and rebuilds keeps the setting they had.
#[test]
fn a_parameters_sheet_patches_the_event_it_came_with() {
    let mut plan = sound();
    plan.platform = vec![platform_sheet(
        "Parameters",
        &[
            &["key", "value"],
            &["election_event.presentation.theme", "union-dark"],
        ],
    )];

    let bundle = compiled(&plan);
    assert_eq!(
        bundle.export["election_event"]["presentation"]["theme"],
        serde_json::json!("union-dark"),
        "the patch the workbook carried is on the event"
    );
}

/// Admin users, permissions and templates come out in the delivery.
///
/// Auxiliary, which is where they already were for a janitor build — outside the
/// importable zip, because `admin_users.csv` carries clear-text passwords and
/// importing an election must never be able to create an account.
#[test]
fn the_admin_portal_imports_survive_a_round_trip_through_the_wizard() {
    let mut plan = sound();
    plan.platform = vec![
        platform_sheet(
            "Admin Users",
            &[
                &["username", "password", "permission_labels"],
                &["returning-officer", "secret", "union-2027-admin"],
            ],
        ),
        platform_sheet(
            "Permissions",
            &[
                &["permission", "union-2027-admin"],
                &["election-view", "TRUE"],
            ],
        ),
    ];

    let bundle = compiled(&plan);
    let layout = crate::election_config::archive::layout(&bundle);
    let auxiliary: Vec<&str> = layout
        .auxiliary
        .iter()
        .map(|file| file.name.as_str())
        .collect();

    assert!(
        auxiliary.iter().any(|name| *name == "admin_users.csv"),
        "the delivery carries the admin users: {auxiliary:?}"
    );
    assert!(
        auxiliary
            .iter()
            .any(|name| name.starts_with("export_permissions-")),
        "and the permission matrix: {auxiliary:?}"
    );
    // Not importable, and that is the point rather than an accident.
    assert!(
        !layout
            .importable
            .iter()
            .any(|file| file.name == "admin_users.csv"),
        "clear-text passwords stay out of the importable zip"
    );
}

/// A plan with none of them builds exactly what it built before.
///
/// The guard that this is additive: the field exists for every plan, and a plan the
/// wizard produced from nothing must be byte-identical to one built before the
/// field did.
#[test]
fn a_plan_with_no_platform_sheets_is_unchanged() {
    let plain = sound();
    let mut with_empty = sound();
    with_empty.platform = Vec::new();

    assert_eq!(
        workbook_of(&plain).unwrap(),
        workbook_of(&with_empty).unwrap()
    );
    assert_eq!(
        compiled(&plain).export,
        compiled(&with_empty).export,
        "the same bytes, either way"
    );
}

/// A carried sheet the wizard *does* own is a refusal, not a coin toss.
#[test]
fn a_platform_sheet_that_collides_with_a_real_one_is_refused() {
    let mut plan = sound();
    plan.platform = vec![platform_sheet(
        "Election Event",
        &[&["external_id"], &["somewhere-else"]],
    )];

    let refused = workbook_of(&plan)
        .expect_err("two ElectionEvent sheets cannot both be meant");
    assert_eq!(refused.code, Code::ConflictingColumns);
}

/// It survives being saved and reopened, which is what `blueprint.json` is for.
#[test]
fn a_carried_sheet_survives_the_saved_plan() {
    let mut plan = sound();
    plan.platform = vec![platform_sheet(
        "Parameters",
        &[&["key", "value"], &["tenant_id", "union"]],
    )];

    let saved = serde_json::to_string(&plan).unwrap();
    let reopened: Blueprint = serde_json::from_str(&saved).unwrap();
    assert_eq!(reopened.platform, plan.platform);

    // And a plan saved before the field existed still opens.
    let older = serde_json::json!({
        "version": BLUEPRINT_VERSION,
        "external_id": "union-2027",
        "name": {"en": "Union Election 2027"},
    });
    let opened: Blueprint = serde_json::from_value(older).unwrap();
    assert!(opened.platform.is_empty());
}

/// Write the sample plan out as a real file, for a person to open.
///
/// Ignored: it is not an assertion, it is the only way to satisfy the one
/// acceptance criterion no test can — that Excel and LibreOffice open what this
/// writes. `cargo test -- --ignored emit_a_workbook_to_look_at --nocapture`.
#[test]
#[ignore]
fn emit_a_workbook_to_look_at() {
    let workbook = workbook_of(&sound()).expect("the sample plan writes");
    let bytes = crate::election_config::xlsx_write::write_xlsx(&workbook)
        .expect("and it becomes a file");
    let at = std::env::temp_dir().join("election_workbook.xlsx");
    std::fs::write(&at, &bytes).unwrap();
    println!(
        "wrote {} bytes and {} sheets to {}",
        bytes.len(),
        workbook.sheets().len(),
        at.display()
    );
}

/// An election event exported from a plan reads back as that plan.
///
/// **The strongest test available for this mapping, and the reason it is here
/// rather than beside `plan_from_event`.** The importable document is produced by
/// `Blueprint` → `to_workbook` → `build`, through handlebars templates, across
/// forty-odd fields. A hand-written expectation for the reverse direction would be
/// a second opinion about what the templates emit, and it would share whatever
/// this author misread. Compiling a plan and reading it back compares the mapping
/// against the *emitter*, so a field either survives the round trip or the test
/// names it.
///
/// What cannot survive is asserted separately, in `plan_from_event_tests`: an
/// export carries no trustees, contacts, messages or census, because those are
/// the architect's own and travel in a delivery's auxiliary files. This asserts
/// what *should* come back, so a regression in the mapping shows up as a
/// difference rather than as a missing feature nobody notices.
#[test]
fn an_exported_event_reads_back_as_the_plan_that_made_it() {
    for plan in [sound(), districted(), opinionated()] {
        reads_back(&plan);
    }
}

/// A districted plan with a contest that disagrees with the event about its rules.
///
/// `sound()` names no areas and overrides nothing, so a round trip over it alone
/// would prove the easy half: an export whose every contest inherits everything
/// cannot show a difference between inherited and chosen. This one can — which is
/// what makes the behaviour assertion below worth having.
fn opinionated() -> Blueprint {
    let mut plan = districted();
    if let Some(contest) = plan
        .elections
        .first_mut()
        .and_then(|election| election.contests.first_mut())
    {
        contest.max_votes = 3;
        contest.winners = 2;
        contest.overrides.tally.counting_algorithm =
            Some("cumulative".to_string());
        contest.overrides.tally.min_votes = Some(1);
        contest.overrides.layout.columns = Some(2);
        // Somebody who stood down after the ballot was approved. Here rather than in
        // a fixture of its own because this is the plan the round trip already walks
        // for everything unusual, and a flag is exactly the kind of field an import
        // loses without anybody noticing.
        if let Some(candidate) = contest.candidates.first_mut() {
            candidate.disabled = true;
        }
    }
    plan
}

fn reads_back(plan: &Blueprint) {
    let plan = plan.clone();
    let bundle = compiled(&plan);

    let read = crate::election_config::plan_from_event::plan_from_event(
        &bundle.export,
    )
    .expect("the document this crate just wrote is readable");
    let back = read.plan;

    assert!(
        !read.report.has_errors(),
        "reading back what we wrote should not be an error: {}",
        read.report
    );

    // Compared as a *reader* sees it, per language, not as a map.
    //
    // The emitter fills every enabled language with the English fallback — that
    // is what `an_untranslated_string_comes_back_as_a_real_translation` records
    // for the workbook door, and it is the same decision here: `Translated::get`
    // falls back deliberately, because a blank is never the right answer for a
    // name a voter reads. So a plan that named only English comes back naming
    // both, and asserting on the map would be asserting that the fallback does
    // not happen.
    let reads = |was: &Translated, now: &Translated| {
        for language in ["en", "es"] {
            assert_eq!(
                now.get(language),
                was.get(language),
                "the text a {language} reader sees"
            );
        }
    };

    assert_eq!(back.external_id, plan.external_id);
    reads(&plan.name, &back.name);
    assert_eq!(back.languages, plan.languages);
    assert_eq!(back.elections_order, plan.elections_order);
    assert_eq!(back.show_cast_vote_logs, plan.show_cast_vote_logs);
    assert_eq!(back.voting_channels, plan.voting_channels);

    // The ballot, which is the part with the most to lose.
    assert_eq!(back.elections.len(), plan.elections.len());
    for (was, now) in plan.elections.iter().zip(back.elections.iter()) {
        assert_eq!(now.external_id, was.external_id);
        reads(&was.name, &now.name);
        assert_eq!(now.num_allowed_revotes, was.num_allowed_revotes);
        assert_eq!(now.contests.len(), was.contests.len());

        for (before, after) in was.contests.iter().zip(now.contests.iter()) {
            assert_eq!(after.external_id, before.external_id);
            reads(&before.name, &after.name);
            reads(&before.description, &after.description);
            assert_eq!(after.max_votes, before.max_votes);
            assert_eq!(after.winners, before.winners);
            assert_eq!(
                after.candidates.len(),
                before.candidates.len(),
                "the candidates of {}",
                before.external_id
            );
            for (was_who, now_who) in
                before.candidates.iter().zip(after.candidates.iter())
            {
                assert_eq!(now_who.external_id, was_who.external_id);
                reads(&was_who.name, &now_who.name);
                // The flags, which nothing compared until a candidate's `disabled`
                // needed proving. All three, not just the new one: they travel the
                // same way, and a round trip that checks one of three is a round
                // trip that will lose the other two quietly.
                assert_eq!(
                    (
                        now_who.explicit_blank,
                        now_who.explicit_invalid,
                        now_who.disabled,
                    ),
                    (
                        was_who.explicit_blank,
                        was_who.explicit_invalid,
                        was_who.disabled,
                    ),
                    "the flags of candidate {}",
                    was_who.external_id
                );
            }
            // The rules a contest ends up with, which is the field an import
            // getting this wrong would change silently.
            assert_eq!(
                plan.defaults.apply(&before.overrides),
                back.defaults.apply(&after.overrides),
                "the behaviour of contest {}",
                before.external_id
            );
        }
    }

    // Every area the plan named comes back — but not necessarily *only* those.
    //
    // A plan that does no districting still compiles to an event with one area,
    // because the platform requires a contest to serve somewhere; `sound()` names
    // none, so the export carries the minted one and reading it back finds it.
    // That is the export being honest about its contents rather than a fault in
    // the mapping, and asserting equal lengths here would be asserting that an
    // election event can have no areas at all.
    for was in &plan.areas {
        let found = back
            .areas
            .iter()
            .find(|now| now.external_id == was.external_id)
            .unwrap_or_else(|| {
                panic!("the area {} came back", was.external_id)
            });
        assert_eq!(found.name, was.name);
    }
    assert!(
        !back.areas.is_empty(),
        "an export always names at least the area its contests serve"
    );

    // And every contest still serves the areas it served, by the names a plan
    // uses rather than by the identifiers the export uses.
    for (was, now) in plan.elections.iter().zip(back.elections.iter()) {
        for (before, after) in was.contests.iter().zip(now.contests.iter()) {
            if !before.areas.is_empty() {
                assert_eq!(
                    after.areas, before.areas,
                    "the areas contest {} serves",
                    before.external_id
                );
            } else {
                // **"No areas" means "every area", and an export says so out
                // loud.** A plan leaves the list empty to mean the contest is on
                // every ballot; `build` then writes an `area_contests` row per
                // area, because the platform has no notion of "all". So the
                // imported plan states explicitly what the original left
                // implicit — behaviourally the same election, and a difference
                // worth asserting rather than smoothing over, because somebody
                // reading the imported plan will notice the list is now full.
                assert_eq!(
                    after.areas.len(),
                    back.areas.len(),
                    "a contest that named no area serves all of them"
                );
            }
        }
    }
}

/// A plan saved under version 2 keeps every voter's area.
///
/// Version 2 keyed a voter's area by the area's **name**; version 3 keys it by
/// `external_id`. Every plan anybody has saved carries names, so without the
/// migration each of them would open with an area nothing recognises — the census
/// would look intact on screen and the build would refuse every row of it.
#[test]
fn a_version_two_plan_keeps_its_voters_in_their_areas() {
    let document = r#"{
        "version": 2,
        "external_id": "old",
        "name": {"en": "Old"},
        "areas": [
            {"external_id": "local-1", "name": "North Local 1"},
            {"external_id": "local-2", "name": "South Local 2"}
        ],
        "voters": [
            {"username": "ada", "area_name": "North Local 1"},
            {"username": "grace", "area_name": "South Local 2"},
            {"username": "alan", "area_name": "Nowhere At All"},
            {"username": "kay"}
        ]
    }"#;

    let plan = read_plan(document)
        .expect("a version 2 plan still opens")
        .plan;
    assert_eq!(plan.version, BLUEPRINT_VERSION);

    assert_eq!(plan.voters[0].area_external_id, "local-1");
    assert_eq!(plan.voters[1].area_external_id, "local-2");

    // A name no area answers to is kept as written rather than dropped, so
    // validation reports it against the row that has it. Dropping it would turn a
    // loud problem into a voter who quietly gets no ballot.
    assert_eq!(plan.voters[2].area_external_id, "Nowhere At All");
    assert_eq!(plan.voters[3].area_external_id, "");

    // And the old key does not survive as a passthrough attribute, which would
    // collide with the `area_name` column the finished bundle emits.
    for voter in &plan.voters {
        assert!(
            !voter.extra.contains_key("area_name"),
            "{:?} kept the old key",
            voter.username
        );
    }
}

// -- the client's own wording and stylesheet ---------------------------------

/// The whole-object i18n column must come **before** the named ones.
///
/// `set_path` inserts rather than merges. Written after
/// `presentation.i18n.en.name`, this column replaces the object that column had
/// just filled and takes the event's own name and description with it — a build
/// whose every list view is blank, from a column order nobody would think to
/// look at. Named for the ordering so a later tidy-up has to read this.
#[test]
fn the_wording_blob_is_written_before_the_named_columns() {
    let mut plan = sound();
    plan.i18n.insert(
        "en".to_string(),
        [(
            "candidate.blankVote".to_string(),
            "None of these".to_string(),
        )]
        .into_iter()
        .collect(),
    );

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let row = workbook.rows(sheet::SHEET_ELECTION_EVENT)[0].clone();
    let order: Vec<String> =
        row.without(&[]).into_iter().map(|(name, _)| name).collect();
    let blob = order.iter().position(|each| each == "presentation.i18n.en");
    let named = order
        .iter()
        .position(|each| each == "presentation.i18n.en.name");

    assert!(
        blob.is_some() && named.is_some() && blob < named,
        "the whole-object column must precede the named ones, or it clobbers \
         them: {order:?}"
    );
}

/// And the event's own name survives being written into that object.
#[test]
fn the_event_keeps_its_name_when_a_client_overrides_wording() {
    let mut plan = sound();
    let was = plan.name.clone();
    plan.i18n.insert(
        "en".to_string(),
        [(
            "candidate.blankVote".to_string(),
            "None of these".to_string(),
        )]
        .into_iter()
        .collect(),
    );

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let row = workbook.rows(sheet::SHEET_ELECTION_EVENT)[0].clone();
    assert_eq!(
        row.text("presentation.i18n.en.name").unwrap_or_default(),
        was.get("en").unwrap_or_default().to_string()
    );
}

/// A stylesheet reaches the column the portal reads.
#[test]
fn a_stylesheet_reaches_the_event_sheet() {
    let mut plan = sound();
    plan.css = ".candidate { border-color: rebeccapurple; }".to_string();

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let row = workbook.rows(sheet::SHEET_ELECTION_EVENT)[0].clone();
    assert_eq!(
        row.text("presentation.css"),
        Some(".candidate { border-color: rebeccapurple; }")
    );
}

/// Nothing written means nothing said, not an empty object.
///
/// `{}` in the cell would *replace* whatever a base export already carried,
/// which for an event being rebuilt is the wording somebody set in the Admin
/// Portal.
#[test]
fn saying_nothing_about_wording_writes_an_empty_cell() {
    let plan = sound();

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let row = workbook.rows(sheet::SHEET_ELECTION_EVENT)[0].clone();
    let written = row.text("presentation.i18n.en");
    assert!(
        written.is_none() || written == Some(""),
        "expected an empty cell, got {written:?}"
    );
}

/// The sign-in page's wording reaches the realm, by the road the CSS already took.
///
/// Keycloak is a Java application that never sees the event's presentation, so
/// `Blueprint::i18n` — which the Voting Portal and the ballot verifier read —
/// cannot reach it. These become
/// `keycloak_event_realm.localizationTexts.<locale>.<key>` parameters, a prefix
/// `PARAMETER_PREFIXES` already carries into the realm patch, which is why the
/// realm builder did not have to change.
#[test]
fn the_sign_in_pages_wording_is_written_as_realm_parameters() {
    let mut plan = sound();
    plan.keycloak_messages = BTreeMap::from([
        (
            "en".to_string(),
            BTreeMap::from([(
                "doLogIn".to_string(),
                "Sign in to vote".to_string(),
            )]),
        ),
        (
            "es".to_string(),
            BTreeMap::from([(
                "doLogIn".to_string(),
                "Entra para votar".to_string(),
            )]),
        ),
    ]);

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let said: Vec<(String, String)> = workbook
        .rows(sheet::SHEET_PARAMETERS)
        .iter()
        .filter_map(|row| {
            Some((row.text("key")?.to_string(), row.text("value")?.to_string()))
        })
        .collect();

    assert!(
        said.contains(&(
            "keycloak_event_realm.localizationTexts.en.doLogIn".to_string(),
            "Sign in to vote".to_string()
        )),
        "{said:?}"
    );
    assert!(
        said.contains(&(
            "keycloak_event_realm.localizationTexts.es.doLogIn".to_string(),
            "Entra para votar".to_string()
        )),
        "{said:?}"
    );
}

/// A translation keeps its placeholders, unlike the stylesheet beside it.
///
/// `login_css_patch` escapes MessageFormat's braces because a stylesheet is full of
/// them. A *translation* may legitimately carry `{0}`, and escaping it would put the
/// literal characters on the sign-in page where the voter's name belongs.
#[test]
fn a_translation_is_not_message_format_escaped() {
    let mut plan = sound();
    plan.keycloak_messages = BTreeMap::from([(
        "en".to_string(),
        BTreeMap::from([("hello".to_string(), "Welcome, {0}".to_string())]),
    )]);

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let value = workbook.rows(sheet::SHEET_PARAMETERS)[0]
        .text("value")
        .map(str::to_string);
    assert_eq!(value, Some("Welcome, {0}".to_string()));
}

/// A plan that says nothing about the sign-in page emits no sheet at all.
///
/// `parameters` is one of the sheets a plan carries through from the workbook it was
/// opened from, and `Workbook::new` refuses a duplicate key. Emitting an empty one
/// would make every plan that came from a janitor's workbook a refusal, and would
/// change what a rebuild of an untouched workbook produces.
#[test]
fn no_sign_in_wording_means_no_parameters_sheet() {
    let workbook = workbook_of(&sound()).expect("the plan compiles to rows");
    assert!(
        workbook.sheet(sheet::SHEET_PARAMETERS).is_none(),
        "a plan with no sign-in wording should not invent a Parameters sheet"
    );
}

/// Wording is appended to the workbook's own parameters, not put beside them.
///
/// The case that would otherwise be a refusal: a plan opened from a real workbook
/// carries that workbook's `parameters` sheet in `platform`, and somebody then adds
/// a translation. One sheet comes out, holding both.
#[test]
fn wording_is_appended_to_a_carried_parameters_sheet() {
    let mut plan = sound();
    plan.platform = vec![sheet::Sheet::from_grid(
        "Parameters",
        &[
            vec![
                Cell::text("key".to_string()),
                Cell::text("value".to_string()),
            ],
            vec![
                Cell::text("tenant_id".to_string()),
                Cell::text("acme".to_string()),
            ],
        ],
    )
    .expect("a two-column sheet")];
    plan.keycloak_messages = BTreeMap::from([(
        "en".to_string(),
        BTreeMap::from([("doLogIn".to_string(), "Sign in".to_string())]),
    )]);

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let keys: Vec<String> = workbook
        .rows(sheet::SHEET_PARAMETERS)
        .iter()
        .filter_map(|row| row.text("key").map(str::to_string))
        .collect();

    assert_eq!(
        keys,
        vec![
            "tenant_id".to_string(),
            "keycloak_event_realm.localizationTexts.en.doLogIn".to_string(),
        ],
        "the workbook's own parameter comes first, and the wording after it"
    );
}

// -- the login page's stylesheet -------------------------------------------

/// A stylesheet reaching Keycloak is MessageFormat-escaped; wording is not.
///
/// Keycloak resolves every `localizationTexts` value through
/// `java.text.MessageFormat`, where `{` opens a placeholder. Raw CSS is mostly
/// braces, so an unescaped stylesheet arrives mangled — and the failure is only
/// visible on a real login page, which is the worst place to find it.
///
/// Per-key rather than per-channel, because escaping every message would break
/// a translation legitimately carrying `{0}`.
#[test]
fn the_login_stylesheet_is_escaped_and_wording_is_not() {
    let mut plan = sound();
    plan.keycloak_messages.insert(
        "en".to_string(),
        [
            (
                "loginCustomCss".to_string(),
                ".login { color: red; }".to_string(),
            ),
            ("doLogIn".to_string(), "Sign in, {0} of {1}".to_string()),
        ]
        .into_iter()
        .collect(),
    );

    let workbook = workbook_of(&plan).expect("the plan compiles to rows");
    let row = workbook.rows(sheet::SHEET_PARAMETERS);
    let value = |name: &str| {
        row.iter()
            .find(|each| each.text("key") == Some(name))
            .and_then(|each| each.text("value"))
            .unwrap_or_default()
            .to_string()
    };

    assert_eq!(
        value("keycloak_event_realm.localizationTexts.en.loginCustomCss"),
        ".login '{' color: red; '}'"
    );
    // Untouched: a placeholder in a sentence is a placeholder.
    assert_eq!(
        value("keycloak_event_realm.localizationTexts.en.doLogIn"),
        "Sign in, {0} of {1}"
    );
}

#[test]
fn the_telephone_channel_carries_its_configuration_into_the_event() {
    // The IVR tab used to be the whole of it: `voting_channels.telephone`
    // revealed a screen and everything a telephone election needs was typed in
    // there afterwards, by hand, once per delivery, reproduced by nothing.
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        flow: vec![
            IvrPhase {
                phase: "language_select".to_string(),
                ..Default::default()
            },
            IvrPhase {
                phase: "announcement".to_string(),
                name: "welcome".to_string(),
                prompt_key: "greeting".to_string(),
                ..Default::default()
            },
            IvrPhase {
                phase: "ballot_loop".to_string(),
                receipt_format: "phonetic_hex_4".to_string(),
                ..Default::default()
            },
        ],
        prompts: [(
            "en".to_string(),
            [(
                "greeting".to_string(),
                "Welcome to the election".to_string(),
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect(),
        retry_limits: BTreeMap::new(),
        assistance_phone: String::new(),
    });

    let bundle = compiled(&plan);
    let annotations = &bundle.export["election_event"]["annotations"];

    // **Strings, not objects.** The platform's annotations are `string: string`
    // and the Admin Portal calls `JSON.parse` on what it finds; an object would
    // be silently ignored — `IvrConfig.tsx` checks `typeof === "string"` and
    // falls back to `{}` — so the tab would come up empty with nothing saying
    // why. This is the assertion that would have caught writing them the
    // obvious way.
    assert!(
        annotations["ivr:phone-number"].is_string(),
        "the phone number reached the event as {:?}",
        annotations["ivr:phone-number"]
    );
    assert!(
        annotations["ivr:config"].is_string(),
        "the flow reached the event as {:?}",
        annotations["ivr:config"]
    );
    assert!(
        annotations["ivr:prompts"].is_string(),
        "the prompts reached the event as {:?}",
        annotations["ivr:prompts"]
    );

    assert_eq!(
        annotations["ivr:phone-number"],
        serde_json::json!("+18005550100")
    );

    // And the string parses back to what was authored, in the shape
    // `collectRequiredPromptKeys` walks: `config.flow[].prompt_key`.
    let config: serde_json::Value =
        serde_json::from_str(annotations["ivr:config"].as_str().unwrap())
            .expect("the flow annotation is JSON");
    let flow = config["flow"].as_array().expect("a flow array");
    assert_eq!(flow.len(), 3);
    assert_eq!(flow[1]["phase"], serde_json::json!("announcement"));
    assert_eq!(flow[1]["prompt_key"], serde_json::json!("greeting"));
    assert_eq!(
        flow[2]["receipt_format"],
        serde_json::json!("phonetic_hex_4")
    );
    // Absent extras are left out rather than written empty: a phase carrying
    // `"accept_key": ""` would make the engine wait for a keypress that never
    // comes.
    assert!(flow[0].get("prompt_key").is_none());

    let prompts: serde_json::Value =
        serde_json::from_str(annotations["ivr:prompts"].as_str().unwrap())
            .expect("the prompts annotation is JSON");
    assert_eq!(
        prompts["en"]["greeting"],
        serde_json::json!("Welcome to the election")
    );
}

/// A real event carries more in `ivr:config` than the flow.
///
/// The sample this was modelled on has `retry_limits` and `assistance_phone`
/// beside it. Writing only the flow would drop both on every trip through a
/// plan — the bundle would come back missing settings the event had, which is
/// the quietest kind of wrong.
#[test]
fn the_config_annotation_carries_the_retries_and_the_help_line() {
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        flow: vec![IvrPhase {
            phase: "goodbye".to_string(),
            ..Default::default()
        }],
        prompts: BTreeMap::new(),
        retry_limits: [
            ("auth".to_string(), 3u32),
            ("timeout".to_string(), 7u32),
            ("invalid_input".to_string(), 5u32),
        ]
        .into_iter()
        .collect(),
        assistance_phone: "1-800-555-0199".to_string(),
    });

    let bundle = compiled(&plan);
    let annotations = &bundle.export["election_event"]["annotations"];
    let config: serde_json::Value =
        serde_json::from_str(annotations["ivr:config"].as_str().unwrap())
            .expect("the config annotation is JSON");

    assert_eq!(config["retry_limits"]["auth"], serde_json::json!(3));
    assert_eq!(config["retry_limits"]["timeout"], serde_json::json!(7));
    assert_eq!(
        config["retry_limits"]["invalid_input"],
        serde_json::json!(5)
    );
    assert_eq!(
        config["assistance_phone"],
        serde_json::json!("1-800-555-0199")
    );

    // And back, because a field that only travels one way is a field that goes
    // missing the first time somebody reopens a bundle.
    let read = crate::election_config::plan_from_event::plan_from_event(
        &bundle.export,
    )
    .expect("the export reads back")
    .plan
    .ivr
    .expect("the event configures an IVR");
    assert_eq!(read.retry_limits.get("timeout"), Some(&7));
    assert_eq!(read.assistance_phone, "1-800-555-0199");
}

#[test]
fn an_event_with_no_telephone_gets_no_ivr_annotations() {
    // A web-only event should compile to the bytes it always did. An empty
    // `ivr:config` on every bundle would be a new key for the Admin Portal to
    // parse and a new thing for a diff to show on every rebuild.
    let bundle = compiled(&sound());

    let annotations = &bundle.export["election_event"]["annotations"];
    assert!(
        annotations.get("ivr:config").is_none(),
        "a web-only event carried {annotations:?}"
    );
    assert!(annotations.get("ivr:prompts").is_none());
    assert!(annotations.get("ivr:phone-number").is_none());
}

#[test]
fn the_telephone_configuration_survives_a_round_trip() {
    // Plan → event → plan. The annotations are JSON strings, so reading them
    // back is parsing rather than deserialising, and a round trip is the only
    // thing that proves the two halves agree about the shape.
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        flow: vec![
            IvrPhase {
                phase: "auth".to_string(),
                ..Default::default()
            },
            IvrPhase {
                phase: "announcement".to_string(),
                name: "declaration".to_string(),
                prompt_key: "declaration_text".to_string(),
                accept_key: "2".to_string(),
                ..Default::default()
            },
        ],
        prompts: [
            (
                "en".to_string(),
                [(
                    "declaration_text".to_string(),
                    "Press two to accept".to_string(),
                )]
                .into_iter()
                .collect(),
            ),
            (
                "es".to_string(),
                [(
                    "declaration_text".to_string(),
                    "Pulse dos para aceptar".to_string(),
                )]
                .into_iter()
                .collect(),
            ),
        ]
        .into_iter()
        .collect(),
        retry_limits: BTreeMap::new(),
        assistance_phone: String::new(),
    });

    let bundle = compiled(&plan);
    let read = crate::election_config::plan_from_event::plan_from_event(
        &bundle.export,
    )
    .expect("the document this crate just wrote is readable");

    assert_eq!(read.plan.ivr, plan.ivr);
}

#[test]
fn an_ivr_config_somebody_broke_by_hand_does_not_refuse_the_whole_event() {
    // Somebody edits the annotation in the Admin Portal and leaves it invalid.
    // Refusing the import would tell them nothing and take away the one screen
    // that could show them what is wrong; the part that did not parse is simply
    // missing, and the rest of the event opens.
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        ..Default::default()
    });

    let mut bundle = compiled(&plan);
    bundle.export["election_event"]["annotations"]["ivr:config"] =
        serde_json::json!("{not json at all");

    let read = crate::election_config::plan_from_event::plan_from_event(
        &bundle.export,
    )
    .expect("an event with one broken annotation still reads");

    let ivr = read.plan.ivr.expect("the rest of the IVR section survives");
    assert_eq!(ivr.phone_number, "+18005550100");
    assert!(ivr.flow.is_empty(), "the unparseable flow is dropped");
}

#[test]
fn a_plan_that_configures_the_ivr_is_not_told_to_go_and_configure_it() {
    // The warning was unconditional and is now sometimes false. Telling somebody
    // whose plan carries the flow, the prompts and the number to go and set all
    // three up by hand after import would send them to undo what they had just
    // described — and, worse, would read as the bundle having dropped it.
    let mut plan = sound();
    plan.voting_channels.telephone = true;
    plan.ivr = Some(PlannedIvr {
        phone_number: "+18005550100".to_string(),
        flow: vec![IvrPhase {
            phase: "goodbye".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    });

    let report = validated(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(
        !says(&report, "IVR tab"),
        "a configured IVR was still told to configure itself: {report}"
    );
}

/// `read_plan_value` is the only place a `Blueprint` is deserialized.
///
/// **A rule that was written down and not kept.** `read_plan` said it in its own
/// documentation while five production call sites reached for `serde_json` or
/// `serde_wasm_bindgen` directly — so the migrations ran nowhere, and a version 2
/// plan opened in the wizard had its area names sitting in the identifier field.
///
/// Read off the sources rather than off behaviour, because the failure is a *new*
/// call site rather than a wrong one, and nothing at run time can notice a
/// migration that was skipped. The next such line to be written is the one this
/// catches.
#[test]
fn nothing_else_deserializes_a_plan() {
    let files: &[(&str, &str)] = &[
        ("architect.rs", include_str!("architect.rs")),
        ("open.rs", include_str!("open.rs")),
        ("wasm.rs", include_str!("wasm.rs")),
        ("plan_from_event.rs", include_str!("plan_from_event.rs")),
        (
            "plan_from_workbook.rs",
            include_str!("plan_from_workbook.rs"),
        ),
        ("profile.rs", include_str!("profile.rs")),
    ];

    let mut found: Vec<String> = Vec::new();
    for (name, source) in files {
        for (number, line) in source.lines().enumerate() {
            let line = line.trim();
            let deserializes = line.contains("serde_json::from_value")
                || line.contains("serde_json::from_str")
                || line.contains("serde_json::from_slice")
                || line.contains("serde_wasm_bindgen::from_value");
            if deserializes && line.contains("Blueprint") {
                found.push(format!("{name}:{}", number + 1));
            }
        }
    }

    // Exactly one: `read_plan_value`. A second is either a bypass or a rename, and
    // both want a person to look.
    assert_eq!(
        found.len(),
        1,
        "a plan should only be deserialized in `read_plan_value`; found {found:?}"
    );
    assert!(found[0].starts_with("architect.rs:"), "{found:?}");
}
