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
use crate::election_config::{
    build, validate, BuildOptions, Bundle, ImportElectionEventSchema,
    TemplateSet,
};
use crate::types::ceremonies::CeremoniesPolicy;

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
                max_votes: 1,
                winners: 1,
                allow_writeins: false,
                write_in_slots: 0,
                candidates: vec![
                    PlannedCandidate {
                        external_id: "alice".to_string(),
                        name: Translated::new("Alice"),
                        description: Translated::default(),
                        explicit_blank: false,
                        explicit_invalid: false,

                        image: None,
                    },
                    PlannedCandidate {
                        external_id: "bob".to_string(),
                        name: Translated::new("Bob"),
                        description: Translated::default(),
                        explicit_blank: false,
                        explicit_invalid: false,

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
    let workbook = to_workbook(plan).expect("the plan compiles to rows");
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
        images: plan_images(plan),
        // And who runs the ceremony, which is the third thing this helper has had
        // to be told after `compile_plan` learned it. The comment above is not
        // rhetorical: every field added to `BuildOptions` that `compile_plan`
        // derives from the plan has to be derived here too, or a test asserts on a
        // bundle nobody ships.
        ceremony_policy: plan.ceremony_policy.clone(),
        // The fourth. Two sessions added one each on the same afternoon, which is
        // the strongest argument yet that this helper should call `compile_plan`
        // rather than reproduce it.
        materials: plan_materials(plan),
        ..BuildOptions::default()
    };
    match build(&workbook, &templates, &options) {
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
    let compiled =
        compile_plan(&sound(), &templates, &BuildOptions::default(), None)
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
    let compiled =
        compile_plan(&sound(), &templates, &BuildOptions::default(), None)
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
    let report =
        compile_plan(&plan, &templates, &BuildOptions::default(), None)
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

    let plan = read_plan(document).expect("an older plan still opens");
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
        explicit_blank: false,
        explicit_invalid: false,

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
            explicit_blank: true,
            explicit_invalid: false,

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
    let workbook = to_workbook(plan).expect("the plan compiles to rows");
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

    let report = validate_plan(&plan);
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
    let report = validate_plan(&sound());
    assert!(report.is_empty(), "{report}");
}

#[test]
fn a_threshold_no_number_of_trustees_can_meet_is_an_error() {
    // The worst failure mode there is: everything works until the tally, and then
    // the result cannot be decrypted by anybody.
    let mut plan = sound();
    plan.trustee_threshold = 5;
    let report = validate_plan(&plan);
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
    let report = validate_plan(&plan);
    assert!(report.has_errors(), "{report}");
    assert!(says(&report, "any single trustee can open the tally alone"));
}

#[test]
fn a_single_trustee_is_refused() {
    let mut plan = sound();
    plan.trustees.truncate(1);
    plan.trustee_threshold = 1;
    let report = validate_plan(&plan);
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
    let report = validate_plan(&plan);
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
        let report = validate_plan(&plan);
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
        let report = validate_plan(&plan);
        assert!(!report.has_errors(), "'{email}' should pass:\n{report}");
    }
}

#[test]
fn a_threshold_of_zero_is_refused() {
    let mut plan = sound();
    plan.trustee_threshold = 0;
    assert!(validate_plan(&plan).has_errors());
}

#[test]
fn voting_that_closes_before_it_opens_is_refused() {
    let mut plan = sound();
    plan.schedule.voting_closes = Some(at("2027-01-01T00:00"));
    let report = validate_plan(&plan);
    assert!(says(&report, "closes before it opens"));
}

#[test]
fn a_key_ceremony_after_voting_opens_is_refused() {
    // The key has to exist before a vote can be encrypted with it.
    let mut plan = sound();
    plan.schedule.key_ceremony = Some(at("2027-03-02T10:00"));
    let report = validate_plan(&plan);
    assert!(says(&report, "before voting opens"));
}

#[test]
fn a_tally_before_voting_closes_is_refused() {
    let mut plan = sound();
    plan.schedule.tally_ceremony = Some(at("2027-03-01T10:00"));
    let report = validate_plan(&plan);
    assert!(says(&report, "votes that had not been cast"));
}

/// The whole reason this plan carries offsets. The scheduler reads the emitted
/// date with `DateTime::parse_from_rfc3339`, which requires one — a wall clock
/// yields no date, the poller drops the event, and voting never opens with
/// nothing anywhere saying why.
#[test]
fn the_voting_window_is_emitted_as_an_instant_the_scheduler_can_read() {
    let workbook = to_workbook(&sound()).expect("a sound plan should compile");
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

    let report = validate_plan(&plan);

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
    let report = validate_plan(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "opened"));
}

#[test]
fn a_contest_electing_more_than_a_voter_may_choose_is_refused() {
    let mut plan = sound();
    plan.elections[0].contests[0].winners = 3;
    let report = validate_plan(&plan);
    assert!(says(&report, "a voter may only choose"));
    assert!(codes(&report).contains(&"ContestArithmetic".to_string()));
}

#[test]
fn a_contest_electing_more_than_it_has_candidates_is_refused() {
    let mut plan = sound();
    plan.elections[0].contests[0].max_votes = 5;
    plan.elections[0].contests[0].winners = 5;
    let report = validate_plan(&plan);
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
        explicit_blank: true,
        explicit_invalid: false,

        image: None,
    });
    contest.max_votes = 2;
    contest.winners = 2;

    let report = validate_plan(&plan);
    assert!(says(&report, "from a field of 1"));
}

#[test]
fn a_contest_with_no_candidates_yet_is_a_warning() {
    let mut plan = sound();
    plan.elections[0].contests[0].candidates.clear();
    let report = validate_plan(&plan);
    assert!(!report.has_errors(), "{report}");
    assert!(says(&report, "no candidates yet"));
}

#[test]
fn a_plan_with_no_elections_is_refused() {
    let mut plan = sound();
    plan.elections.clear();
    assert!(validate_plan(&plan).has_errors());
}

#[test]
fn a_plan_with_no_name_or_identifier_is_refused() {
    let mut plan = sound();
    plan.name = Translated::default();
    plan.external_id = "  ".to_string();
    let report = validate_plan(&plan);
    assert_eq!(report.errors().count(), 2, "{report}");
}

#[test]
fn no_points_of_contact_is_worth_saying() {
    let mut plan = sound();
    plan.contacts.clear();
    assert!(says(&validate_plan(&plan), "who gets called"));
}

#[test]
fn a_plan_from_a_newer_version_is_refused_rather_than_half_read() {
    // Opening it would silently drop whatever that version added, and the author
    // would not know which parts survived.
    let mut plan = sound();
    plan.version = BLUEPRINT_VERSION + 1;
    let report = validate_plan(&plan);
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
        max_votes: 1,
        winners: 1,
        allow_writeins: false,
        write_in_slots: 0,
        candidates: vec![PlannedCandidate {
            external_id: "carol".to_string(),
            name: Translated::new("Carol"),
            description: Translated::default(),
            explicit_blank: false,
            explicit_invalid: false,

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
    let report = validate_plan(&plan);
    assert!(says(&report, "no area has the identifier 'nowhere'"));
    assert!(report.has_errors());
}

#[test]
fn two_areas_may_not_share_a_name() {
    // The voters CSV resolves by name, so voters would land in whichever the
    // importer found first.
    let mut plan = districted();
    plan.areas[1].name = "North Region".to_string();
    let report = validate_plan(&plan);
    assert!(says(&report, "both named 'North Region'"));
}

#[test]
fn an_area_needs_a_name_because_that_is_what_a_voter_is_matched_on() {
    let mut plan = districted();
    plan.areas[0].name = "  ".to_string();
    let report = validate_plan(&plan);
    assert!(says(&report, "identifies a voter's area by name"));
}

#[test]
fn an_area_cannot_be_inside_itself() {
    let mut plan = districted();
    plan.areas[0].parent_external_id = Some("region-north".to_string());
    let report = validate_plan(&plan);
    assert!(says(&report, "cannot be inside itself"));
    assert!(codes(&report).contains(&"AreaCycle".to_string()));
}

#[test]
fn a_parent_that_does_not_exist_is_refused() {
    let mut plan = districted();
    plan.areas[1].parent_external_id = Some("region-south".to_string());
    let report = validate_plan(&plan);
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
            area_name: "North Local 1".into(),
            extra: [("department".to_string(), "engineering".to_string())]
                .into_iter()
                .collect(),
        },
        PlannedVoter {
            username: "grace".into(),
            area_name: "North Local 1".into(),
            ..Default::default()
        },
    ];

    let workbook = to_workbook(&plan).expect("this plan is sound");
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

/// A plan with no census has no sheet, rather than an empty one.
#[test]
fn no_census_is_not_an_empty_census() {
    // `build_tables` reads a present-but-empty Voters sheet as "this election
    // has no voters", which is a different claim from "this plan does not carry
    // the census" — and produces a bundle importing an election nobody can vote
    // in.
    let workbook = to_workbook(&sound()).expect("this plan is sound");

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

    let workbook = to_workbook(&plan).expect("this plan is sound");
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

    let report = validate_plan(&plan);

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
        // retyping a name rather than copying it produces.
        area_name: "N. Local 1".into(),
        ..Default::default()
    }];

    let report = validate_plan(&plan);

    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.path == "voters[0].area_name"),
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

    let report = validate_plan(&plan);

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
    let workbook = to_workbook(&plan).expect("a workbook");
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
    let workbook = to_workbook(&sound()).expect("a workbook");
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

    let workbook = to_workbook(&plan).expect("a workbook");

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

    let workbook = to_workbook(&plan).expect("a workbook");
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

/// The trustees become a key ceremony the importer can act on.
///
/// The plan has collected trustees and a threshold from the beginning and emitted
/// no ceremony, so an event imported with a key nobody had been asked to generate.
#[test]
fn the_trustees_become_a_key_ceremony() {
    let plan = sound();
    let bundle = compiled(&plan);
    let ceremonies = bundle.export["keys_ceremonies"]
        .as_array()
        .expect("an array");
    assert_eq!(ceremonies.len(), 1);

    let ceremony = &ceremonies[0];
    assert_eq!(
        ceremony["threshold"].as_i64(),
        Some(i64::from(plan.trustee_threshold))
    );

    // Names, not identifiers. `import_election_event.rs` builds a
    // `HashMap<name, id>` from `get_all_trustees(tenant_id)` and maps this field
    // through it — the same way a voter's area name is resolved. Emitting
    // identifiers here would resolve to nothing.
    let named: Vec<&str> = ceremony["trustee_ids"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|value| value.as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        named,
        plan.trustees
            .iter()
            .map(|trustee| trustee.name.as_str())
            .collect::<Vec<&str>>()
    );

    // The tenant and event it belongs to, so the importer does not have to guess.
    assert_eq!(
        ceremony["election_event_id"].as_str(),
        Some(bundle.event_id.as_str())
    );
    assert_eq!(
        ceremony["tenant_id"].as_str(),
        Some(bundle.tenant_id.as_str())
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
    assert!(says(&validate_plan(&plan), "no trustees"));
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

    let report = validate_plan(&plan);
    assert!(report.has_errors());
    assert!(says(&report, "resolves the key ceremony's"));
}

/// Shown by default, because verifiability should be argued out of, not into.
#[test]
fn a_voter_can_look_up_their_own_ballot_unless_the_plan_says_otherwise() {
    let plan = sound();
    assert_eq!(plan.show_cast_vote_logs, "show-logs-tab");
    assert_eq!(
        to_workbook(&plan)
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

    let report = validate_plan(&plan);
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

    let report = validate_plan(&plan);
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
    assert!(validate_plan(&plan)
        .problems
        .iter()
        .any(|problem| problem.id.as_deref() == Some("trustees.only-one")));
}

/// The half of the language question that had no control.
#[test]
fn a_plan_can_say_which_language_a_voter_starts_in() {
    let mut plan = sound();
    plan.language_detection_policy = Some("force-default".to_string());

    let workbook = to_workbook(&plan).expect("a workbook");
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

    let report = validate_plan(&plan);
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

    let workbook = to_workbook(&plan).expect("a workbook");
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

    let workbook = to_workbook(&plan).expect("a workbook");
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
    let workbook = to_workbook(&sound()).expect("a workbook");
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

/// Absent, `KeysCeremony::policy()` reads manual — so silence is a decision.
#[test]
fn the_ceremony_says_who_runs_it() {
    // The bug this closes: the builder wrote no `settings`, so every event this
    // tool has ever produced imported as a manual ceremony whether or not that is
    // what anybody chose, and no screen said so.
    let plan = sound();
    assert_eq!(plan.ceremony_policy, CeremoniesPolicy::MANUAL_CEREMONIES);

    let bundle = compiled(&plan);
    assert_eq!(
        bundle.export["keys_ceremonies"][0]["settings"]["policy"],
        "manual-ceremonies"
    );

    let mut automated = sound();
    automated.ceremony_policy = CeremoniesPolicy::AUTOMATED_CEREMONIES;
    assert_eq!(
        compiled(&automated).export["keys_ceremonies"][0]["settings"]["policy"],
        "automated-ceremonies"
    );
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

    let workbook = to_workbook(&plan).expect("a workbook");
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

    let workbook = to_workbook(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    // The sheet, with the bytes deliberately withheld — which is exactly what a
    // workbook arriving without its folder of files looks like.
    let outcome = build(&workbook, &templates, &BuildOptions::default());

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
    let workbook = to_workbook(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    let options = BuildOptions {
        materials: vec![MaterialFile {
            document_id: String::new(),
            file_name: "orphan.pdf".to_string(),
            bytes: b"%PDF".to_vec(),
        }],
        ..BuildOptions::default()
    };

    let bundle = build(&workbook, &templates, &options).expect("a clean build");
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

    let images = plan_images(&plan);
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

    let workbook = to_workbook(&plan).expect("a workbook");
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

    let report = validate_plan(&plan);
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

    let report = validate_plan(&plan);
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

    let workbook = to_workbook(&plan).expect("a workbook");
    let templates = TemplateSet::builtin().unwrap();
    // The bytes deliberately withheld — a workbook arriving without its folder.
    let outcome = build(&workbook, &templates, &BuildOptions::default());
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

/// A schedule nobody honours is worse than no schedule, so it is said out loud.
#[test]
fn a_planned_message_says_that_nothing_will_send_it() {
    let mut plan = sound();
    plan.messages.push(PlannedMessage {
        kind: MessageKind::InvitationToVote,
        subject: Translated::new("Your ballot is ready"),
        body: Translated::new("Vote here."),
        html: Translated::default(),
        schedule: MessageSchedule::default(),
    });

    let report = validate_plan(&plan);
    assert!(!report.has_errors(), "a message is not a defect");
    assert!(says(&report, "nothing sends these"));
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

    // And not inside the import, which is the whole point of it being here.
    let workbook = to_workbook(&plan).expect("a workbook");
    assert!(
        !format!("{workbook:?}").contains("get-out-the-vote"),
        "a tenant's templates must not ride in an election event import"
    );
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
            area_name: String::new(),
            ..Default::default()
        }];
        cases.push(("a voter with no area", blank_area));

        // The same, spelled wrong rather than left out — the likelier mistake,
        // and the one a copy-paste from a spreadsheet makes.
        let mut wrong_area = sound();
        wrong_area.voters = vec![PlannedVoter {
            username: "ada".into(),
            area_name: "Nowhere".into(),
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
        let compiled =
            compile_plan(&plan, &templates, &BuildOptions::default(), None);

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

        let verdict = validate_plan(&plan);
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
    let compiled =
        compile_plan(&sound(), &templates, &BuildOptions::default(), None)
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
        .fold((0u32, 0u32, 0u32), |(a, b, _), next| (if a == 0 { next } else { a }, b, 0));

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

    let compiled = compile_plan(&plan, &templates, &BuildOptions::default(), None)
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
