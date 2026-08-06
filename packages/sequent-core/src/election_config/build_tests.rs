// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`], in their own file because there are more of them than
//! there is builder.

use super::*;
use crate::election_config::paths::Cell;
use crate::election_config::sheet::Sheet;

fn text(value: &str) -> Cell {
    Cell::text(value)
}

/// A document that builds cleanly, for a test to break one thing about.
///
/// Deliberately minimal: one election, one contest with two candidates, two areas
/// and the ballot links between them. Everything a test needs is either here or
/// the point of the test.
fn sound() -> Workbook {
    workbook(vec![
        (
            "ElectionEvent",
            vec![
                vec![text("external_id"), text("presentation.i18n.en.name")],
                vec![text("union-2027"), text("Union Election 2027")],
            ],
        ),
        (
            "Elections",
            vec![
                vec![text("external_id"), text("presentation.i18n.en.name")],
                vec![text("statewide"), text("Statewide Officers")],
            ],
        ),
        (
            "Contests",
            vec![
                vec![
                    text("external_id"),
                    text("election.external_id"),
                    text("presentation.i18n.en.name"),
                    text("max_votes"),
                ],
                vec![
                    text("president"),
                    text("statewide"),
                    text("President"),
                    Cell::Int(1),
                ],
            ],
        ),
        (
            "Candidates",
            vec![
                vec![
                    text("external_id"),
                    text("contest.external_id"),
                    text("presentation.i18n.en.name"),
                ],
                vec![text("alice"), text("president"), text("Alice")],
                vec![text("bob"), text("president"), text("Bob")],
            ],
        ),
        (
            "Areas",
            vec![
                vec![text("external_id"), text("name")],
                vec![text("area-north"), text("North")],
                vec![text("area-south"), text("South")],
            ],
        ),
        (
            "AreaContests",
            vec![
                vec![text("area.external_id"), text("contest.external_id")],
                vec![text("area-north"), text("president")],
                vec![text("area-south"), text("president")],
            ],
        ),
    ])
}

fn workbook(sheets: Vec<(&str, Vec<Vec<Cell>>)>) -> Workbook {
    Workbook::new(
        sheets
            .into_iter()
            .map(|(name, grid)| Sheet::from_grid(name, &grid).unwrap())
            .collect(),
    )
    .unwrap()
}

/// The sound document with one sheet replaced.
fn with_sheet(name: &str, grid: Vec<Vec<Cell>>) -> Workbook {
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != name)
        .cloned()
        .collect();
    sheets.push(Sheet::from_grid(name, &grid).unwrap());
    Workbook::new(sheets).unwrap()
}

fn built(workbook: &Workbook) -> Bundle {
    let templates = TemplateSet::builtin().unwrap();
    match build(workbook, &templates, &BuildOptions::default()) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    }
}

fn refused(workbook: &Workbook) -> Report {
    let templates = TemplateSet::builtin().unwrap();
    match build(workbook, &templates, &BuildOptions::default()) {
        Ok(_) => panic!("expected a refusal"),
        Err(report) => report,
    }
}

// -- the happy path -------------------------------------------------------

#[test]
fn a_sound_document_builds() {
    let bundle = built(&sound());
    assert_eq!(bundle.event_external_id, "union-2027");
    assert_eq!(bundle.export["elections"].as_array().unwrap().len(), 1);
    assert_eq!(bundle.export["contests"].as_array().unwrap().len(), 1);
    assert_eq!(bundle.export["candidates"].as_array().unwrap().len(), 2);
    assert_eq!(bundle.export["areas"].as_array().unwrap().len(), 2);
    assert_eq!(bundle.export["area_contests"].as_array().unwrap().len(), 2);
    assert!(!bundle.warnings.has_errors());
}

#[test]
fn what_it_builds_is_a_bundle_the_platform_accepts() {
    // The property that makes sharing this code worth anything: the document the
    // builder produces deserializes into the importer's own struct and passes the
    // importer's own validation. Two implementations that merely look similar
    // would not.
    let bundle = built(&sound());
    let schema: crate::election_config::ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone())
            .expect("the built export must deserialize into the import schema");

    let report = crate::election_config::validate(&schema);
    assert!(
        !report.has_errors(),
        "a document that builds cleanly must also validate:\n{report}"
    );
}

#[test]
fn the_same_document_builds_the_same_bytes_twice() {
    // Fixed timestamps and derived ids exist for this: a regenerated bundle must
    // diff only where the author changed something.
    let first = serde_json::to_string(&built(&sound()).export).unwrap();
    let second = serde_json::to_string(&built(&sound()).export).unwrap();
    assert_eq!(first, second);
}

#[test]
fn the_references_resolve_to_the_ids_the_entities_carry() {
    let bundle = built(&sound());
    let election_id = bundle.export["elections"][0]["id"].as_str().unwrap();
    let contest = &bundle.export["contests"][0];
    let candidate = &bundle.export["candidates"][0];

    assert_eq!(contest["election_id"].as_str().unwrap(), election_id);
    assert_eq!(
        candidate["contest_id"].as_str().unwrap(),
        contest["id"].as_str().unwrap()
    );
    assert_eq!(
        bundle.export["area_contests"][0]["contest_id"]
            .as_str()
            .unwrap(),
        contest["id"].as_str().unwrap()
    );
}

#[test]
fn every_entity_belongs_to_the_event_and_the_tenant() {
    let bundle = built(&sound());
    for key in ["elections", "contests", "candidates", "areas"] {
        for entity in bundle.export[key].as_array().unwrap() {
            assert_eq!(
                entity["election_event_id"].as_str().unwrap(),
                bundle.event_id,
                "{key}"
            );
            assert_eq!(
                entity["tenant_id"].as_str().unwrap(),
                bundle.tenant_id,
                "{key}"
            );
        }
    }
}

#[test]
fn a_column_becomes_a_nested_field() {
    // The dotted-header mapping, end to end: a new column lands in the output
    // with no code change, which is what keeps the builder client-agnostic.
    let bundle = built(&sound());
    assert_eq!(
        bundle.export["contests"][0]["presentation"]["i18n"]["en"]["name"],
        json!("President")
    );
    assert_eq!(bundle.export["contests"][0]["max_votes"], json!(1));
}

#[test]
fn a_control_column_is_consumed_rather_than_merged() {
    // `election.external_id` names a reference; it must not become a field called
    // `external_id` on an object called `election`.
    let bundle = built(&sound());
    assert!(bundle.export["contests"][0].get("election").is_none());
    assert!(bundle.export["areas"][0].get("parent").is_none());
    assert!(bundle.export["area_contests"][0].get("area").is_none());
}

#[test]
fn the_reports_and_scheduled_events_the_importer_ignores_are_left_alone() {
    // Both travel in their own CSV. A populated array here is silently dropped,
    // which is how a report goes missing without an error.
    let bundle = built(&sound());
    assert_eq!(bundle.export["reports"], json!([]));
    assert_eq!(bundle.export["scheduled_events"], Value::Null);
}

// -- identity -------------------------------------------------------------

#[test]
fn an_event_with_no_external_id_stops_the_build_immediately() {
    // Every generated id is derived from it, so there is nothing to build.
    let report = refused(&with_sheet(
        "ElectionEvent",
        vec![
            vec![text("external_id"), text("presentation.i18n.en.name")],
            vec![Cell::Blank, text("Nameless")],
        ],
    ));
    assert!(has_error_saying(&report, "needs an external_id"));
}

#[test]
fn an_empty_event_sheet_says_what_it_wanted() {
    let report = refused(&with_sheet(
        "ElectionEvent",
        vec![vec![text("external_id")]],
    ));
    assert!(has_error_saying(&report, "exactly one row"));
}

#[test]
fn two_event_rows_are_refused_rather_than_the_first_one_winning() {
    // Picking one silently would import half of what someone meant.
    let report = refused(&with_sheet(
        "ElectionEvent",
        vec![
            vec![text("external_id")],
            vec![text("one")],
            vec![text("two")],
        ],
    ));
    assert!(has_error_saying(&report, "exactly one election event"));
}

#[test]
fn a_duplicated_external_id_names_the_row_that_used_it_first() {
    let report = refused(&with_sheet(
        "Elections",
        vec![
            vec![text("external_id")],
            vec![text("statewide")],
            vec![text("statewide")],
        ],
    ));
    assert!(has_error_saying(&report, "already used by row 2"));
}

#[test]
fn every_problem_is_reported_in_one_run() {
    // An author fixing a spreadsheet wants the whole list, not one round trip per
    // mistake.
    let report = refused(&with_sheet(
        "Candidates",
        vec![
            vec![text("external_id"), text("contest.external_id")],
            vec![Cell::Blank, text("president")],
            vec![text("alice"), text("no-such-contest")],
            vec![text("bob"), Cell::Blank],
        ],
    ));
    assert_eq!(report.errors().count(), 3, "{report}");
}

#[test]
fn a_problem_names_the_sheet_and_row_to_look_at() {
    let report = refused(&with_sheet(
        "Contests",
        vec![
            vec![text("external_id"), text("election.external_id")],
            vec![text("president"), text("no-such-election")],
        ],
    ));
    let problem = report.errors().next().unwrap();
    assert_eq!(
        problem.path,
        "sheet 'Contests' row 2 column 'election.external_id'"
    );
    assert_eq!(problem.code, Code::DanglingReference);
}

// -- references -----------------------------------------------------------

#[test]
fn a_contest_pointing_at_no_election_is_refused() {
    let report = refused(&with_sheet(
        "Contests",
        vec![
            vec![text("external_id"), text("election.external_id")],
            vec![text("president"), text("nowhere")],
        ],
    ));
    assert!(has_error_saying(
        &report,
        "no election has external_id 'nowhere'"
    ));
}

#[test]
fn a_missing_reference_column_is_refused_too() {
    let report = refused(&with_sheet(
        "Contests",
        vec![vec![text("external_id")], vec![text("president")]],
    ));
    assert!(has_error_saying(
        &report,
        "'election.external_id' is required: it names the election this row \
         belongs to"
    ));
}

#[test]
fn a_numeric_id_matches_the_same_number_written_as_text() {
    // Whether a cell was formatted as a number is not something an author
    // controls per column, and an id is an id.
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "Elections" && sheet.name != "Contests")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "Elections",
            &[vec![text("external_id")], vec![Cell::Int(1001)]],
        )
        .unwrap(),
    );
    sheets.push(
        Sheet::from_grid(
            "Contests",
            &[
                vec![text("external_id"), text("election.external_id")],
                vec![text("president"), text("1001")],
            ],
        )
        .unwrap(),
    );
    let bundle = built(&Workbook::new(sheets).unwrap());
    assert_eq!(
        bundle.export["contests"][0]["election_id"],
        bundle.export["elections"][0]["id"]
    );
}

// -- areas ----------------------------------------------------------------

#[test]
fn a_parent_may_appear_below_its_own_child() {
    // Authors do not sort their spreadsheets topologically, so ids are collected
    // in a first pass.
    let bundle = built(&with_sheet(
        "Areas",
        vec![
            vec![
                text("external_id"),
                text("name"),
                text("parent.external_id"),
            ],
            vec![text("area-north"), text("North"), text("area-state")],
            vec![text("area-south"), text("South"), text("area-state")],
            vec![text("area-state"), text("Statewide"), Cell::Blank],
        ],
    ));
    let north = &bundle.export["areas"][0];
    let state = &bundle.export["areas"][2];
    assert_eq!(north["parent_id"], state["id"]);
    assert_eq!(state["parent_id"], Value::Null);
}

#[test]
fn an_area_cannot_be_its_own_parent() {
    let report = refused(&with_sheet(
        "Areas",
        vec![
            vec![
                text("external_id"),
                text("name"),
                text("parent.external_id"),
            ],
            vec![text("area-north"), text("North"), text("area-north")],
        ],
    ));
    assert!(has_error_saying(&report, "cannot be its own parent"));
    assert_eq!(report.errors().next().unwrap().code, Code::AreaCycle);
}

#[test]
fn an_area_needs_a_name_because_the_voters_csv_resolves_by_name() {
    let report = refused(&with_sheet(
        "Areas",
        vec![
            vec![text("external_id"), text("name")],
            vec![text("area-north"), Cell::Blank],
        ],
    ));
    assert!(has_error_saying(
        &report,
        "identifies a voter's area by name"
    ));
}

#[test]
fn two_areas_may_not_share_a_name() {
    // The voters CSV resolves an area by name, so a duplicate silently assigns
    // voters to whichever one the importer happens to find.
    let report = refused(&with_sheet(
        "Areas",
        vec![
            vec![text("external_id"), text("name")],
            vec![text("area-north"), text("North")],
            vec![text("area-south"), text("North")],
        ],
    ));
    assert!(has_error_saying(&report, "both named 'North'"));
}

// -- ballot coverage ------------------------------------------------------

#[test]
fn a_document_with_no_ballot_links_is_refused() {
    let report = refused(&with_sheet(
        "AreaContests",
        vec![vec![text("area.external_id"), text("contest.external_id")]],
    ));
    assert!(has_error_saying(&report, "no voter would see a ballot"));
}

#[test]
fn the_same_area_and_contest_may_not_be_linked_twice() {
    // Both rows would mint the same id, and one would silently overwrite the
    // other.
    let report = refused(&with_sheet(
        "AreaContests",
        vec![
            vec![text("area.external_id"), text("contest.external_id")],
            vec![text("area-north"), text("president")],
            vec![text("area-north"), text("president")],
        ],
    ));
    assert!(has_error_saying(&report, "already linked"));
}

#[test]
fn an_event_with_no_elections_contests_or_areas_says_so_about_each() {
    let mut sheets: Vec<Sheet> = vec![Sheet::from_grid(
        "ElectionEvent",
        &[vec![text("external_id")], vec![text("empty-event")]],
    )
    .unwrap()];
    for name in ["Elections", "Contests", "Areas", "AreaContests"] {
        sheets.push(
            Sheet::from_grid(name, &[vec![text("external_id")]]).unwrap(),
        );
    }
    let report = refused(&Workbook::new(sheets).unwrap());
    assert!(has_error_saying(&report, "at least one election"));
    assert!(has_error_saying(&report, "at least one contest"));
    assert!(has_error_saying(&report, "at least one area"));
}

// -- options --------------------------------------------------------------

#[test]
fn a_tenant_id_may_be_supplied() {
    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            tenant_id: Some("11111111-1111-4111-8111-111111111111".to_string()),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(bundle.tenant_id, "11111111-1111-4111-8111-111111111111");
    assert_eq!(
        bundle.export["tenant_id"],
        json!("11111111-1111-4111-8111-111111111111")
    );
}

#[test]
fn a_parameter_supplies_the_tenant_id_when_no_option_does() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("tenant_id"),
                text("22222222-2222-4222-8222-222222222222"),
            ],
        ],
    ));
    assert_eq!(bundle.tenant_id, "22222222-2222-4222-8222-222222222222");
}

#[test]
fn without_one_the_tenant_id_is_derived_and_stable() {
    // Only a fallback, but it must not change between runs or every regeneration
    // is a diff.
    assert_eq!(built(&sound()).tenant_id, built(&sound()).tenant_id);
}

#[test]
fn the_slug_comes_from_the_event_or_the_caller() {
    assert_eq!(built(&sound()).slug, "union-2027");

    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            slug: Some("chosen".to_string()),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(bundle.slug, "chosen");
}

#[test]
fn a_slug_is_filesystem_safe_whatever_the_external_id_looks_like() {
    assert_eq!(
        slugify("SEIU 1000 / Leadership 2027"),
        "seiu-1000-leadership-2027"
    );
    assert_eq!(slugify("  --already--  "), "already");
    assert_eq!(slugify("!!!"), "election-event");
    assert_eq!(slugify(""), "election-event");
}

#[test]
fn a_created_at_may_be_supplied_and_reaches_every_entity() {
    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            created_at: Some("2030-06-01T00:00:00.000000Z".to_string()),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        bundle.export["contests"][0]["created_at"],
        json!("2030-06-01T00:00:00.000000Z")
    );
}

// -- parameters -----------------------------------------------------------

#[test]
fn a_dotted_parameter_patches_the_event() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("event"),
                text("election_event.presentation.theme"),
                text("dark"),
            ],
        ],
    ));
    assert_eq!(
        bundle.export["election_event"]["presentation"]["theme"],
        json!("dark")
    );
}

#[test]
fn a_parameter_nothing_interprets_is_recorded_and_said_out_loud() {
    // Dropping it silently is how a setting goes missing on election day.
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![text("client"), text("helpdesk_phone"), text("555-0100")],
        ],
    ));
    assert_eq!(
        bundle.export["election_event"]["annotations"]["client.helpdesk_phone"],
        json!("555-0100")
    );
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("helpdesk_phone")));
}

#[test]
fn a_parameter_with_no_value_is_ignored_with_a_warning() {
    // A placeholder an author left blank, pending something from the client.
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![text("settings"), text("saml_idp_metadata_url"), Cell::Blank],
        ],
    ));
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("has no value and is ignored")));
    assert!(bundle.export["election_event"]
        .get("annotations")
        .and_then(
            |annotations| annotations.get("settings.saml_idp_metadata_url")
        )
        .is_none());
}

#[test]
fn a_row_with_no_key_is_a_note_to_the_author_and_is_skipped() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value"), text("comment")],
            vec![
                Cell::Blank,
                Cell::Blank,
                Cell::Blank,
                text("ask the client"),
            ],
        ],
    ));
    assert!(bundle.warnings.is_empty(), "{}", bundle.warnings);
}

// -- base export ----------------------------------------------------------

#[test]
fn a_base_export_contributes_fields_the_templates_do_not_know() {
    // What a base export is for: a newer platform version's additions arrive
    // without a template change.
    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            base_export: Some(json!({
                "election_event": {"a_new_field": "from the base"},
                "version": "v11.0.0",
            })),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        bundle.export["election_event"]["a_new_field"],
        json!("from the base")
    );
    assert_eq!(bundle.export["version"], json!("v11.0.0"));
}

#[test]
fn a_base_export_never_supplies_identity() {
    // The base names its own ids, its own board and its own keys. Carrying any of
    // them over produces an event that looks configured and is not.
    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            base_export: Some(json!({
                "election_event": {
                    "id": "99999999-9999-4999-8999-999999999999",
                    "tenant_id": "88888888-8888-4888-8888-888888888888",
                    "external_id": "someone-elses-event",
                    "bulletin_board_reference": "board-of-the-other-event",
                    "public_key": "not our key",
                    "statistics": {"votes": 4213},
                    "status": "finished",
                },
            })),
            ..BuildOptions::default()
        },
    )
    .unwrap();

    let event = &bundle.export["election_event"];
    assert_eq!(event["id"].as_str().unwrap(), bundle.event_id);
    assert_eq!(event["tenant_id"].as_str().unwrap(), bundle.tenant_id);
    assert_eq!(event["external_id"], json!("union-2027"));

    // The templates declare all four with platform defaults, so what scrubbing
    // guarantees is that the base's own values are gone — a fresh board, no key,
    // and a status describing an event that has not run.
    assert_eq!(event["bulletin_board_reference"], Value::Null);
    assert_eq!(event["public_key"], Value::Null);
    assert_ne!(event["statistics"], json!({"votes": 4213}));
    assert_ne!(event["status"], json!("finished"));
    assert_eq!(event["status"]["voting_status"], json!("NOT_STARTED"));
}

#[test]
fn a_base_export_does_not_override_what_the_author_wrote() {
    // Merged under the templates, not over them.
    let templates = TemplateSet::builtin().unwrap();
    let bundle = build(
        &sound(),
        &templates,
        &BuildOptions {
            base_export: Some(json!({
                "elections": [{"presentation": {"i18n": {"en": {"name": "Base name"}}}}],
            })),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        bundle.export["elections"][0]["presentation"]["i18n"]["en"]["name"],
        json!("Statewide Officers")
    );
}

#[test]
fn a_base_export_with_nothing_useful_in_it_changes_nothing() {
    let templates = TemplateSet::builtin().unwrap();
    let with_base = build(
        &sound(),
        &templates,
        &BuildOptions {
            base_export: Some(json!({"elections": [], "election_event": {}})),
            ..BuildOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_string(&with_base.export).unwrap(),
        serde_json::to_string(&built(&sound()).export).unwrap()
    );
}

// -- templates ------------------------------------------------------------

#[test]
fn an_overridden_template_is_what_gets_built() {
    // The split that keeps client configuration out of the code.
    let templates = TemplateSet::with_overrides(&[(
        "area",
        r#"{"id": "{{id}}", "name": "", "description": "", "presentation": {"allow_early_voting": "early_voting_allowed"}}"#,
    )])
    .unwrap();
    let bundle = build(&sound(), &templates, &BuildOptions::default()).unwrap();
    assert_eq!(
        bundle.export["areas"][0]["presentation"]["allow_early_voting"],
        json!("early_voting_allowed")
    );
    // And the row's own columns still win over the override.
    assert_eq!(bundle.export["areas"][0]["name"], json!("North"));
}

#[test]
fn a_template_that_renders_broken_json_is_reported_not_panicked() {
    let templates =
        TemplateSet::with_overrides(&[("area", "{\"oops\": }")]).unwrap();
    let report = build(&sound(), &templates, &BuildOptions::default())
        .expect_err("a broken template must refuse the build");
    assert!(has_error_saying(&report, "did not render valid JSON"));
}

// -- shape conflicts ------------------------------------------------------

#[test]
fn columns_that_disagree_about_a_shape_are_reported_against_the_cell() {
    let report = refused(&with_sheet(
        "Elections",
        vec![
            vec![
                text("external_id"),
                text("presentation"),
                text("presentation.i18n"),
            ],
            vec![text("statewide"), text("plain"), text("{}")],
        ],
    ));
    let problem = report.errors().next().unwrap();
    assert_eq!(problem.code, Code::ConflictingColumns);
    assert!(problem.path.contains("sheet 'Elections' row 2"));
}
