// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::election_config::{
    build, validate, BuildOptions, Bundle, ImportElectionEventSchema,
    TemplateSet,
};

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
        external_id: "union-2027".to_string(),
        name: Translated::new("Union Election 2027"),
        languages: vec!["en".to_string(), "es".to_string()],
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
            milestones: vec![Milestone {
                event: "Candidate nominations close".to_string(),
                date: "2027-01-15".to_string(),
            }],
        },
        elections: vec![PlannedElection {
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
                areas: vec![],
            }],
        }],
        policies: Policies::default(),
        notes: String::new(),
    }
}

fn compiled(plan: &Blueprint) -> Bundle {
    let workbook = to_workbook(plan).expect("the plan compiles to rows");
    let templates = TemplateSet::builtin().unwrap();
    match build(&workbook, &templates, &BuildOptions::default()) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    }
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

#[test]
fn the_policies_become_the_platforms_own_values() {
    let mut plan = sound();
    plan.policies = Policies {
        over_vote: Policy::Restricted,
        blank_vote: Policy::Allowed,
        under_vote: Policy::Warn,
        invalid_vote: Policy::Restricted,
    };
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
    assert_eq!(presentation["under_vote_policy"], serde_json::json!("warn"));
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
        explicit_blank: false,
        explicit_invalid: false,
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
            explicit_blank: true,
            explicit_invalid: false,
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
fn a_threshold_of_one_is_allowed_but_said_out_loud() {
    let mut plan = sound();
    plan.trustee_threshold = 1;
    let report = validate_plan(&plan);
    assert!(!report.has_errors());
    assert!(says(&report, "one trustee alone"));
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
        explicit_blank: true,
        explicit_invalid: false,
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
        },
        PlannedArea {
            external_id: "local-1".to_string(),
            name: "North Local 1".to_string(),
            parent_external_id: Some("region-north".to_string()),
        },
        PlannedArea {
            external_id: "local-2".to_string(),
            name: "North Local 2".to_string(),
            parent_external_id: Some("region-north".to_string()),
        },
    ];
    // The president is everywhere; the local officer is one local's business.
    plan.elections[0].contests.push(PlannedContest {
        external_id: "local-officer".to_string(),
        name: Translated::new("Local Officer"),
        description: String::new(),
        max_votes: 1,
        winners: 1,
        candidates: vec![PlannedCandidate {
            external_id: "carol".to_string(),
            name: Translated::new("Carol"),
            explicit_blank: false,
            explicit_invalid: false,
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
