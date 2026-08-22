// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`], in their own file because there are more of them than
//! there is builder.

use super::*;
use crate::election_config::emit::JsonField;
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

#[test]
fn a_numeric_id_matches_in_the_areas_and_area_contests_sheets_too() {
    // The Areas and AreaContests builders read their reference cells through a
    // different accessor from the Elections one, so a numeric id registered in one
    // pass and vanished in the next — and in AreaContests two different numeric pairs
    // both keyed the duplicate check as ("", "").
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "Areas" && sheet.name != "AreaContests")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "Areas",
            &[
                vec![text("external_id"), text("name")],
                vec![Cell::Int(2001), text("North")],
                vec![Cell::Int(2002), text("South")],
            ],
        )
        .unwrap(),
    );
    sheets.push(
        Sheet::from_grid(
            "AreaContests",
            &[
                vec![text("area.external_id"), text("contest.external_id")],
                vec![Cell::Int(2001), text("president")],
                vec![Cell::Int(2002), text("president")],
            ],
        )
        .unwrap(),
    );

    let bundle = built(&Workbook::new(sheets).unwrap());
    assert_eq!(bundle.export["areas"].as_array().unwrap().len(), 2);
    // Two links, not one collapsed pair and not a spurious duplicate refusal.
    assert_eq!(bundle.export["area_contests"].as_array().unwrap().len(), 2);
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
    // Namespaced the way the SEIU1000 bundle already carries them, so a
    // regenerated event does not move its annotations.
    assert_eq!(
        bundle.export["election_event"]["annotations"]
            ["janitor.param.client.helpdesk_phone"],
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
        .and_then(|annotations| annotations
            .get("janitor.param.settings.saml_idp_metadata_url"))
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
    assert!(
        !bundle
            .warnings
            .warnings()
            .any(|problem| problem.message.contains("parameter")),
        "{}",
        bundle.warnings
    );
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

// -- voters ---------------------------------------------------------------

/// The sound document with a Voters sheet added.
fn with_voters(grid: Vec<Vec<Cell>>) -> Workbook {
    with_sheet("Voters", grid)
}

#[test]
fn a_voter_row_becomes_a_csv_row_the_importer_understands() {
    let bundle = built(&with_voters(vec![
        vec![
            text("username"),
            text("email"),
            text("first_name"),
            text("last_name"),
            text("area.external_id"),
        ],
        vec![
            text("m-1001"),
            text("alice@example.org"),
            text("Alice"),
            text("Adams"),
            text("area-north"),
        ],
    ]));

    let voters = &bundle.voters;
    let column = |name: &str| voters.column(name).expect(name);
    let row = &voters.rows[0];

    assert_eq!(row[column("username")], "m-1001");
    assert_eq!(row[column("email")], "alice@example.org");
    // The area travels as a name, because that is what the importer resolves by.
    assert_eq!(row[column("area_name")], "North");
    assert!(!row[column("id")].is_empty());
}

#[test]
fn a_voter_with_an_address_is_treated_as_verified() {
    // An unverified address blocks delivery of the one-time code, and a census
    // address is one the client asserts is correct.
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("email"), text("area.external_id")],
        vec![text("with"), text("a@example.org"), text("area-north")],
        vec![text("without"), Cell::Blank, text("area-north")],
    ]));
    let verified = bundle.voters.column("email_verified").unwrap();
    assert_eq!(bundle.voters.rows[0][verified], "true");
    assert_eq!(bundle.voters.rows[1][verified], "false");
}

#[test]
fn a_voter_is_enabled_unless_the_source_says_otherwise() {
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("enabled"), text("area.external_id")],
        vec![text("default"), Cell::Blank, text("area-north")],
        vec![text("ticked"), text("x"), text("area-north")],
        vec![text("off"), text("no"), text("area-north")],
    ]));
    let enabled = bundle.voters.column("enabled").unwrap();
    assert_eq!(bundle.voters.rows[0][enabled], "true");
    assert_eq!(bundle.voters.rows[1][enabled], "true");
    assert_eq!(bundle.voters.rows[2][enabled], "false");
}

#[test]
fn a_voter_with_no_election_restriction_is_authorized_for_all_of_them() {
    // Writing an empty attribute would deny access to every election instead.
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("area.external_id")],
        vec![text("m-1001"), text("area-north")],
    ]));
    let column = bundle.voters.column("authorized-election-ids").unwrap();
    assert_eq!(
        bundle.voters.rows[0][column],
        bundle.export["elections"][0]["id"].as_str().unwrap()
    );
}

#[test]
fn a_restricted_voter_gets_the_elections_named_resolved_to_ids() {
    let bundle = built(&with_voters(vec![
        vec![
            text("username"),
            text("authorized-election-ids"),
            text("area.external_id"),
        ],
        vec![text("m-1001"), text("statewide"), text("area-north")],
    ]));
    let column = bundle.voters.column("authorized-election-ids").unwrap();
    assert_eq!(
        bundle.voters.rows[0][column],
        bundle.export["elections"][0]["id"].as_str().unwrap()
    );
}

#[test]
fn a_voter_naming_an_election_nobody_configured_is_refused() {
    let report = refused(&with_voters(vec![
        vec![
            text("username"),
            text("authorized-election-ids"),
            text("area.external_id"),
        ],
        vec![text("m-1001"), text("no-such-election"), text("area-north")],
    ]));
    assert!(has_error_saying(
        &report,
        "no election has external_id 'no-such-election'"
    ));
}

#[test]
fn a_voter_needs_a_username_and_an_area() {
    let report = refused(&with_voters(vec![
        vec![text("username"), text("area.external_id")],
        vec![Cell::Blank, text("area-north")],
        vec![text("m-1002"), Cell::Blank],
        vec![text("m-1003"), text("nowhere")],
    ]));
    assert!(has_error_saying(&report, "a voter needs a username"));
    assert!(has_error_saying(&report, "a voter needs an area"));
    assert!(has_error_saying(
        &report,
        "no area has external_id 'nowhere'"
    ));
}

#[test]
fn two_voters_may_not_share_a_username() {
    let report = refused(&with_voters(vec![
        vec![text("username"), text("area.external_id")],
        vec![text("m-1001"), text("area-north")],
        vec![text("m-1001"), text("area-south")],
    ]));
    assert!(has_error_saying(&report, "already used by row 2"));
}

#[test]
fn an_unknown_column_is_carried_through_as_a_voter_attribute() {
    // How a client adds a reporting breakout column with no code change.
    let bundle = built(&with_voters(vec![
        vec![
            text("username"),
            text("area.external_id"),
            text("local-number"),
        ],
        vec![text("m-1001"), text("area-north"), text("1000")],
    ]));
    let column = bundle.voters.column("local-number").expect("local-number");
    assert_eq!(bundle.voters.rows[0][column], "1000");
    // And it lands after the derived columns, not among them.
    assert!(column >= VOTER_LEADING_COLUMNS.len());
}

#[test]
fn a_passthrough_column_blank_for_every_voter_is_dropped() {
    // Not cosmetic: get_copy_from_query treats the mere presence of a `password`
    // header as "hash a password for each of these voters", so a blank one would
    // give every voter an empty credential.
    let bundle = built(&with_voters(vec![
        vec![
            text("username"),
            text("area.external_id"),
            text("password"),
            text("local-number"),
        ],
        vec![
            text("m-1001"),
            text("area-north"),
            Cell::Blank,
            text("1000"),
        ],
    ]));
    assert!(bundle.voters.column("password").is_none());
    assert!(bundle.voters.column("local-number").is_some());
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("password")));
}

#[test]
fn a_derived_column_is_kept_even_when_every_voter_leaves_it_blank() {
    // The importer expects them, and an absent `email` header is not the same as
    // an empty one.
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("area.external_id")],
        vec![text("m-1001"), text("area-north")],
    ]));
    for column in VOTER_LEADING_COLUMNS {
        assert!(bundle.voters.column(column).is_some(), "{column}");
    }
}

#[test]
fn voters_with_no_way_to_receive_a_code_are_warned_about() {
    // Not an error: credentials are sometimes distributed on paper.
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("email"), text("area.external_id")],
        vec![text("reachable"), text("a@example.org"), text("area-north")],
        vec![text("not"), Cell::Blank, text("area-north")],
    ]));
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("1 of 2 voters have neither an email address")));
}

#[test]
fn a_census_with_no_contact_column_at_all_makes_every_voter_unreachable() {
    // `email` is a derived column, so it is in the table whether or not the
    // source has it — which is why there is no separate "no contact column"
    // case. Every voter simply has an empty one.
    let bundle = built(&with_voters(vec![
        vec![text("username"), text("area.external_id")],
        vec![text("m-1001"), text("area-north")],
        vec![text("m-1002"), text("area-south")],
    ]));
    assert!(bundle.voters.column("email").is_some());
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("2 of 2 voters have neither an email address")));
}

#[test]
fn a_column_name_the_importer_would_reject_is_caught_before_the_upload() {
    // Otherwise it fails mid-import with nothing naming the column.
    let report = refused(&with_voters(vec![
        vec![
            text("username"),
            text("area.external_id"),
            text("home address"),
        ],
        vec![text("m-1001"), text("area-north"), text("1 Main St")],
    ]));
    assert!(has_error_saying(
        &report,
        "the importer rejects this column name"
    ));
}

// -- scheduled events -----------------------------------------------------

fn with_schedule(grid: Vec<Vec<Cell>>) -> Workbook {
    with_sheet("ScheduledEvents", grid)
}

#[test]
fn two_rows_scheduling_one_processor_for_one_election_are_rejected() {
    // Both the uuid5 and the task id derive from the processor and the election
    // alone, so the second row would import as the first and its time would be lost.
    let report = refused(&with_schedule(vec![
        vec![
            text("event_type"),
            text("scheduled_datetime"),
            text("election.external_id"),
        ],
        vec![
            text("START_VOTING_PERIOD"),
            text("2027-03-01T16:00:00Z"),
            text("statewide"),
        ],
        vec![
            text("START_VOTING_PERIOD"),
            text("2027-03-02T16:00:00Z"),
            text("statewide"),
        ],
    ]));

    assert!(
        report
            .errors()
            .any(|problem| problem.code == Code::DuplicateId),
        "expected a duplicate identity to be reported, got:\n{report}"
    );
}

#[test]
fn a_voting_window_becomes_two_rows_of_the_scheduled_events_csv() {
    let bundle = built(&with_schedule(vec![
        vec![
            text("event_type"),
            text("scheduled_datetime"),
            text("election.external_id"),
        ],
        vec![
            text("START_VOTING_PERIOD"),
            text("2027-03-01T16:00:00Z"),
            text("statewide"),
        ],
        vec![
            text("END_VOTING_PERIOD"),
            text("2027-03-15T23:59:00Z"),
            text("statewide"),
        ],
    ]));
    assert_eq!(bundle.scheduled_events.len(), 2);

    let row = &bundle.scheduled_events.rows[0];
    assert_eq!(row.len(), 12);
    assert_eq!(row[8], JsonField::string("START_VOTING_PERIOD"));
    assert_eq!(
        row[9],
        JsonField::Value(json!({
            "cron": Value::Null,
            "scheduled_date": "2027-03-01T16:00:00Z",
        }))
    );
    // A SQL NULL, written bare rather than as a quoted JSON null.
    assert_eq!(row[4], JsonField::Null);
}

#[test]
fn the_payload_names_the_election_and_the_task_id_matches_the_platform() {
    // The platform looks its task up by this name; a different shape means a task
    // that never fires.
    let bundle = built(&with_schedule(vec![
        vec![
            text("event_type"),
            text("scheduled_datetime"),
            text("election.external_id"),
        ],
        vec![
            text("START_VOTING_PERIOD"),
            text("2027-03-01T16:00:00Z"),
            text("statewide"),
        ],
    ]));
    let election_id = bundle.export["elections"][0]["id"].as_str().unwrap();
    let row = &bundle.scheduled_events.rows[0];

    assert_eq!(
        row[10],
        JsonField::Value(json!({"election_id": election_id}))
    );
    assert_eq!(
        row[11],
        JsonField::string(format!(
            "tenant_{}_event_{}_election_{election_id}_START_VOTING_PERIOD",
            bundle.tenant_id, bundle.event_id
        ))
    );
}

#[test]
fn an_event_wide_schedule_leaves_the_election_out_of_both() {
    let bundle = built(&with_schedule(vec![
        vec![text("event_type"), text("scheduled_datetime")],
        vec![text("START_VOTING_PERIOD"), text("2027-03-01T16:00:00Z")],
    ]));
    let row = &bundle.scheduled_events.rows[0];
    assert_eq!(row[10], JsonField::Value(json!({"election_id": null})));
    assert_eq!(
        row[11],
        JsonField::string(format!(
            "tenant_{}_event_{}_START_VOTING_PERIOD",
            bundle.tenant_id, bundle.event_id
        ))
    );
}

#[test]
fn an_author_may_write_an_event_type_the_way_they_speak_it() {
    let bundle = built(&with_schedule(vec![
        vec![text("event_type"), text("scheduled_datetime")],
        vec![text("start voting period"), text("2027-03-01T16:00:00Z")],
        vec![text("end-voting-period"), text("2027-03-15T23:59:00Z")],
    ]));
    assert_eq!(
        bundle.scheduled_events.rows[0][8],
        JsonField::string("START_VOTING_PERIOD")
    );
    assert_eq!(
        bundle.scheduled_events.rows[1][8],
        JsonField::string("END_VOTING_PERIOD")
    );
}

#[test]
fn an_event_type_nothing_processes_is_refused_with_the_list() {
    let report = refused(&with_schedule(vec![
        vec![text("event_type"), text("scheduled_datetime")],
        vec![text("OPEN_THE_POLLS"), text("2027-03-01T16:00:00Z")],
    ]));
    assert!(has_error_saying(&report, "is not an event processor"));
    assert!(has_error_saying(&report, "START_VOTING_PERIOD"));
}

#[test]
fn a_scheduled_event_with_no_time_is_refused() {
    let report = refused(&with_schedule(vec![
        vec![text("event_type"), text("scheduled_datetime")],
        vec![text("START_VOTING_PERIOD"), Cell::Blank],
    ]));
    assert!(has_error_saying(&report, "needs a scheduled_datetime"));
}

#[test]
fn an_event_name_is_kept_as_an_annotation_so_the_csv_reads_like_the_source() {
    let bundle = built(&with_schedule(vec![
        vec![
            text("event_name"),
            text("event_type"),
            text("scheduled_datetime"),
        ],
        vec![
            text("Polls open"),
            text("START_VOTING_PERIOD"),
            text("2027-03-01T16:00:00Z"),
        ],
    ]));
    assert_eq!(
        bundle.scheduled_events.rows[0][7],
        JsonField::Value(json!({"janitor.event_name": "Polls open"}))
    );
}

#[test]
fn an_election_whose_window_never_opens_is_warned_about() {
    // It imports fine and then quietly never opens.
    let bundle = built(&with_schedule(vec![
        vec![
            text("event_type"),
            text("scheduled_datetime"),
            text("election.external_id"),
        ],
        vec![
            text("START_VOTING_PERIOD"),
            text("2027-03-01T16:00:00Z"),
            text("statewide"),
        ],
    ]));
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("no END_VOTING_PERIOD scheduled event")));
}

#[test]
fn an_event_wide_window_covers_every_election() {
    let bundle = built(&with_schedule(vec![
        vec![text("event_type"), text("scheduled_datetime")],
        vec![text("START_VOTING_PERIOD"), text("2027-03-01T16:00:00Z")],
        vec![text("END_VOTING_PERIOD"), text("2027-03-15T23:59:00Z")],
    ]));
    assert!(
        !bundle
            .warnings
            .warnings()
            .any(|problem| problem.message.contains("voting period will not")),
        "{}",
        bundle.warnings
    );
}

#[test]
fn a_document_with_no_schedule_at_all_says_it_will_need_hands() {
    let bundle = built(&sound());
    assert!(bundle.scheduled_events.is_empty());
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("by hand in the Admin Portal")));
}

// -- reports --------------------------------------------------------------

#[test]
fn no_reports_sheet_means_no_reports_member() {
    // Absent and empty are the same thing, and an empty CSV is not a valid one.
    assert!(built(&sound()).reports.is_none());
}

#[test]
fn a_report_row_becomes_a_positional_csv_row() {
    let bundle = built(&with_sheet(
        "Reports",
        vec![
            vec![
                text("report_type"),
                text("election.external_id"),
                text("encryption_policy"),
                text("permission_label"),
            ],
            vec![
                text("tally"),
                text("statewide"),
                text("configured_password"),
                text("statewide-officers || auditors"),
            ],
        ],
    ));
    let reports = bundle.reports.expect("a reports table");
    let row = &reports.rows[0];

    assert_eq!(row.len(), 8);
    assert_eq!(
        row[1],
        bundle.export["elections"][0]["id"].as_str().unwrap()
    );
    assert_eq!(row[2], "tally");
    // Read from the row, not from the template's default: this is the field that
    // silently became `unencrypted` once.
    assert_eq!(row[5], "configured_password");
    // Option<Vec<String>>, split on "|" by process_reports_file.
    assert_eq!(row[7], "statewide-officers|auditors");
}

#[test]
fn a_report_with_no_policy_falls_back_to_unencrypted() {
    let bundle = built(&with_sheet(
        "Reports",
        vec![vec![text("report_type")], vec![text("tally")]],
    ));
    let reports = bundle.reports.expect("a reports table");
    assert_eq!(reports.rows[0][5], "unencrypted");
}

#[test]
fn a_report_needs_a_type() {
    let report = refused(&with_sheet(
        "Reports",
        vec![
            vec![text("report_type"), text("election.external_id")],
            vec![Cell::Blank, text("statewide")],
        ],
    ));
    assert!(has_error_saying(&report, "a report needs a report_type"));
}

#[test]
fn a_report_naming_a_template_nobody_defined_is_refused() {
    let report = refused(&with_sheet(
        "Reports",
        vec![
            vec![text("report_type"), text("template.alias")],
            vec![text("tally"), text("no-such-template")],
        ],
    ));
    assert!(has_error_saying(
        &report,
        "no Templates row has alias 'no-such-template'"
    ));
}

#[test]
fn a_report_password_is_flagged_as_a_secret() {
    let mut sheets: Vec<Sheet> = sound().sheets().to_vec();
    sheets.push(
        Sheet::from_grid(
            "Reports",
            &[
                vec![text("report_type"), text("password")],
                vec![text("tally"), text("s3cret")],
            ],
        )
        .unwrap(),
    );
    let bundle = built(&Workbook::new(sheets).unwrap());
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("clear text")));
}

// -- admin users, permissions, templates ----------------------------------

#[test]
fn admin_users_keep_their_own_columns() {
    // Not an event-import member: the sheet is the shape, whatever it holds.
    let bundle = built(&with_sheet(
        "Admin Users",
        vec![
            vec![text("username"), text("email"), text("permission_labels")],
            vec![
                text("admin1"),
                text("admin@example.org"),
                text("statewide-officers || auditors"),
            ],
        ],
    ));
    let admins = bundle.admin_users.expect("an admin users table");
    assert_eq!(admins.columns, ["username", "email", "permission_labels"]);
    // "||" in, "|" out.
    assert_eq!(admins.rows[0][2], "statewide-officers|auditors");
}

#[test]
fn an_admin_user_needs_a_username() {
    let report = refused(&with_sheet(
        "Admin Users",
        vec![
            vec![text("username"), text("email")],
            vec![Cell::Blank, text("a@b.c")],
        ],
    ));
    assert!(has_error_saying(&report, "an admin user needs a username"));
}

#[test]
fn admin_passwords_are_flagged_once_however_many_rows_carry_them() {
    let bundle = built(&with_sheet(
        "Admin Users",
        vec![
            vec![text("username"), text("password")],
            vec![text("admin1"), text("s3cret")],
            vec![text("admin2"), text("s3cret2")],
        ],
    ));
    assert_eq!(
        bundle
            .warnings
            .warnings()
            .filter(|problem| problem.message.contains("clear-text passwords"))
            .count(),
        1
    );
}

#[test]
fn the_permission_matrix_is_transposed_into_the_platform_shape() {
    // A matrix is what a human can check at a glance; role,permissions is what
    // export_tenant_config.rs writes.
    let bundle = built(&with_sheet(
        "Permissions",
        vec![
            vec![text("permission"), text("admin"), text("auditor")],
            vec![text("election:read"), text("x"), text("x")],
            vec![text("election:write"), text("x"), Cell::Blank],
        ],
    ));
    let permissions = bundle.role_permissions.expect("a permissions table");
    assert_eq!(permissions.columns, ["role", "permissions"]);
    assert_eq!(
        permissions.rows,
        vec![
            vec![
                "admin".to_string(),
                "election:read|election:write".to_string()
            ],
            vec!["auditor".to_string(), "election:read".to_string()],
        ]
    );
}

#[test]
fn a_permission_matrix_with_no_roles_says_what_it_expected() {
    let report = refused(&with_sheet(
        "Permissions",
        vec![vec![text("permission")], vec![text("election:read")]],
    ));
    assert!(has_error_saying(&report, "has no role columns"));
}

#[test]
fn a_template_becomes_a_file_beside_the_bundle() {
    // The event zip has no member for communication templates.
    let bundle = built(&with_sheet(
        "Templates",
        vec![
            vec![
                text("name"),
                text("alias"),
                text("type"),
                text("communication_method"),
                text("template.document"),
            ],
            vec![
                text("Voter Credentials"),
                text("voter_credentials"),
                text("VOTER_CREDENTIALS"),
                text("EMAIL"),
                text(r"Dear {{name}},\n\nYour code is {{code}}."),
            ],
        ],
    ));
    assert_eq!(bundle.templates.len(), 1);
    let template = &bundle.templates[0];
    assert_eq!(template.alias, "voter_credentials");
    assert_eq!(template.name, "Voter Credentials");
    assert_eq!(template.template_type.as_deref(), Some("VOTER_CREDENTIALS"));
    assert_eq!(template.communication_method.as_deref(), Some("EMAIL"));
    // The literal \n a copy-paste out of a JSON export leaves behind.
    assert_eq!(
        template.document,
        "Dear {{name}},\n\nYour code is {{code}}."
    );
    assert_eq!(template.file_name(), "voter_credentials.hbs");
}

#[test]
fn a_template_falls_back_to_its_name_when_it_has_no_alias() {
    let bundle = built(&with_sheet(
        "Templates",
        vec![
            vec![text("name"), text("template.document")],
            vec![text("Reminder"), text("hello")],
        ],
    ));
    assert_eq!(bundle.templates[0].alias, "Reminder");
    assert_eq!(bundle.templates[0].name, "Reminder");
}

#[test]
fn a_template_needs_a_name_and_a_document() {
    let report = refused(&with_sheet(
        "Templates",
        vec![
            vec![text("name"), text("alias"), text("template.document")],
            vec![Cell::Blank, Cell::Blank, text("orphan")],
            vec![text("No document"), text("nodoc"), Cell::Blank],
        ],
    ));
    assert!(has_error_saying(&report, "needs a name or an alias"));
    assert!(has_error_saying(&report, "a template needs a document"));
}

#[test]
fn two_templates_may_not_share_an_alias() {
    let report = refused(&with_sheet(
        "Templates",
        vec![
            vec![text("alias"), text("template.document")],
            vec![text("otp"), text("one")],
            vec![text("otp"), text("two")],
        ],
    ));
    assert!(has_error_saying(&report, "already used by row 2"));
}

// -- everything together --------------------------------------------------

#[test]
fn a_full_document_builds_every_member_and_still_validates() {
    // The whole surface at once, because the members share resolved ids and a
    // mistake in one shows up in another.
    let mut sheets: Vec<Sheet> = sound().sheets().to_vec();
    for (name, grid) in [
        (
            "Voters",
            vec![
                vec![
                    text("username"),
                    text("email"),
                    text("area.external_id"),
                    text("local-number"),
                ],
                vec![
                    text("m-1001"),
                    text("alice@example.org"),
                    text("area-north"),
                    text("1000"),
                ],
                vec![
                    text("m-1002"),
                    text("bob@example.org"),
                    text("area-south"),
                    text("2000"),
                ],
            ],
        ),
        (
            "ScheduledEvents",
            vec![
                vec![text("event_type"), text("scheduled_datetime")],
                vec![text("START_VOTING_PERIOD"), text("2027-03-01T16:00:00Z")],
                vec![text("END_VOTING_PERIOD"), text("2027-03-15T23:59:00Z")],
            ],
        ),
        (
            "Templates",
            vec![
                vec![text("alias"), text("template.document")],
                vec![text("tally_report"), text("Results for {{election}}")],
            ],
        ),
        (
            "Reports",
            vec![
                vec![text("report_type"), text("template.alias")],
                vec![text("tally"), text("tally_report")],
            ],
        ),
        (
            "Admin Users",
            vec![
                vec![text("username"), text("permission_labels")],
                vec![text("admin1"), text("statewide-officers")],
            ],
        ),
        (
            "Permissions",
            vec![
                vec![text("permission"), text("admin")],
                vec![text("election:read"), text("x")],
            ],
        ),
    ] {
        sheets.push(Sheet::from_grid(name, &grid).unwrap());
    }

    let bundle = built(&Workbook::new(sheets).unwrap());

    assert_eq!(bundle.voters.len(), 2);
    assert_eq!(bundle.scheduled_events.len(), 2);
    assert_eq!(bundle.reports.as_ref().map(PlainTable::len), Some(1));
    assert_eq!(bundle.admin_users.as_ref().map(PlainTable::len), Some(1));
    assert_eq!(
        bundle.role_permissions.as_ref().map(PlainTable::len),
        Some(1)
    );
    assert_eq!(bundle.templates.len(), 1);

    // And the JSON document is still one the platform accepts.
    let schema: crate::election_config::ImportElectionEventSchema =
        serde_json::from_value(bundle.export.clone()).unwrap();
    let report = crate::election_config::validate(&schema);
    assert!(!report.has_errors(), "{report}");
}

#[test]
fn the_csv_members_render_through_the_shared_writers() {
    // The tables exist to be written by emit, so the join has to hold.
    use crate::election_config::emit::{json_csv, plain_csv};

    let bundle = built(&with_voters(vec![
        vec![text("username"), text("email"), text("area.external_id")],
        vec![
            text("m-1001"),
            text("alice@example.org"),
            text("area-north"),
        ],
    ]));

    let columns: Vec<&str> =
        bundle.voters.columns.iter().map(String::as_str).collect();
    let rendered = plain_csv(&columns, &bundle.voters.rows);
    assert!(rendered.starts_with("id,email,email_verified"));
    assert!(rendered.contains("alice@example.org"));
    assert!(rendered.ends_with('\n'));

    let schedule_columns: Vec<&str> = bundle
        .scheduled_events
        .columns
        .iter()
        .map(String::as_str)
        .collect();
    let schedule = json_csv(&schedule_columns, &bundle.scheduled_events.rows);
    assert!(schedule.starts_with("id,tenant_id,election_event_id"));
}

// -- the realm ------------------------------------------------------------

/// A realm with the pieces the presets expect, small enough to read.
fn base_realm() -> Value {
    json!({
        "realm": "some-other-realm",
        "id": "99999999-9999-4999-8999-999999999999",
        "identityProviders": [{"alias": "environment-idp", "enabled": true}],
        "authenticationFlows": [{
            "alias": "browser",
            "authenticationExecutions": [
                {"authenticator": "message-otp-authenticator"},
            ],
        }, {
            "alias": "saml-first-broker-flow",
            "authenticationExecutions": [],
        }],
        "authenticatorConfig": [{"alias": "deferred", "config": {}}],
        "components": {
            "org.keycloak.userprofile.UserProfileProvider": [{
                "config": {"kc.user.profile.config": [
                    r#"{"attributes":[{"name":"username"},{"name":"dateOfBirth"}]}"#
                ]},
            }],
        },
        "clients": [{
            "clientId": "voting-portal",
            "rootUrl": "https://vote.example.org/99999999-9999-4999-8999-999999999999",
        }],
    })
}

fn with_options(workbook: &Workbook, options: BuildOptions) -> Bundle {
    let templates = TemplateSet::builtin().unwrap();
    match build(workbook, &templates, &options) {
        Ok(bundle) => bundle,
        Err(report) => panic!("expected a clean build, got:\n{report}"),
    }
}

#[test]
fn no_base_export_means_no_realm_at_all() {
    // The importer takes keycloak_event_realm wholesale, so a realm invented here
    // would replace the environment's provisioned default rather than merge into
    // it. Emitting none is the safe answer.
    let bundle = built(&sound());
    assert_eq!(bundle.export["keycloak_event_realm"], Value::Null);
}

#[test]
fn what_the_document_asked_of_the_realm_is_kept_even_with_no_realm_to_apply_it_to(
) {
    // Otherwise an auth_type or a login stylesheet is silently lost.
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("otp_email_or_sms"),
            ],
        ],
    ));
    assert_eq!(bundle.auth_preset, Some("otp_email_or_sms"));
    assert!(!bundle.realm_patch.patch.is_empty());
    assert!(bundle.realm_patch.bind_authenticator_config.is_some());
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains(
            "no base export, so nothing here configures the login page"
        )));
}

#[test]
fn the_realms_name_is_derived_from_the_tenant_and_the_event() {
    // Structural: the voting portal and the smart-link URLs derive it the same
    // way, so it is not a free choice.
    let bundle = with_options(
        &sound(),
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );
    let realm = &bundle.export["keycloak_event_realm"];
    assert_eq!(
        realm["realm"],
        json!(format!(
            "tenant-{}-event-{}",
            bundle.tenant_id, bundle.event_id
        ))
    );
    assert_eq!(realm["id"], json!(bundle.event_id));
}

#[test]
fn a_stale_event_id_in_the_realms_urls_is_swapped_for_this_events() {
    // Hosts belong to the environment and stay, but the base export embeds its own
    // event id in those URLs, and import remaps every UUID it finds — so a stale
    // one would be remapped to something unrelated.
    let bundle = with_options(
        &sound(),
        BuildOptions {
            base_export: Some(json!({
                "keycloak_event_realm": base_realm(),
                "election_event": {"id": "99999999-9999-4999-8999-999999999999"},
            })),
            ..BuildOptions::default()
        },
    );
    let root = bundle.export["keycloak_event_realm"]["clients"][0]["rootUrl"]
        .as_str()
        .unwrap();
    assert!(root.starts_with("https://vote.example.org/"), "{root}");
    assert!(root.ends_with(&bundle.event_id), "{root}");
    assert!(!root.contains("99999999-9999-4999-8999-999999999999"));
}

#[test]
fn a_preset_adds_its_provider_without_removing_the_environments() {
    // identityProviders is referenced by alias from elsewhere in the realm;
    // replacing the list would strip what the environment configured on purpose.
    let workbook = with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("saml_sso_idp_initiated"),
            ],
            vec![
                text("settings"),
                text("saml_idp_metadata_url"),
                text("https://idp.example.org/metadata"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );

    let providers = bundle.export["keycloak_event_realm"]["identityProviders"]
        .as_array()
        .unwrap();
    let aliases: Vec<&str> = providers
        .iter()
        .map(|provider| provider["alias"].as_str().unwrap())
        .collect();
    assert_eq!(aliases, ["environment-idp", "client-saml-idp"]);
}

#[test]
fn the_otp_preset_binds_its_config_to_the_authenticator_in_the_realm() {
    // Registering the config without binding it leaves the step unconfigured, and
    // nothing about the realm would say so.
    let workbook = with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("otp_email_or_sms"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );

    let realm = &bundle.export["keycloak_event_realm"];
    assert_eq!(
        realm["authenticationFlows"][0]["authenticationExecutions"][0]
            ["authenticatorConfig"],
        json!("janitor-otp-by-availability")
    );
    // And the config it points at is present alongside the realm's own.
    let aliases: Vec<&str> = realm["authenticatorConfig"]
        .as_array()
        .unwrap()
        .iter()
        .map(|config| config["alias"].as_str().unwrap())
        .collect();
    assert!(aliases.contains(&"deferred"));
    assert!(aliases.contains(&"janitor-otp-by-availability"));
}

#[test]
fn the_link_preset_patches_the_user_profile_inside_its_stringified_blob() {
    // It lives inside a Keycloak component as a single JSON string, so it has to
    // be parsed, patched and re-serialised rather than merged.
    let workbook = with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("voter_link_plus_dob"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );

    let raw = bundle.export["keycloak_event_realm"]["components"]
        ["org.keycloak.userprofile.UserProfileProvider"][0]["config"]
        ["kc.user.profile.config"][0]
        .as_str()
        .unwrap();
    let profile: Value = serde_json::from_str(raw).unwrap();
    let attributes = profile["attributes"].as_array().unwrap();

    let date_of_birth = attributes
        .iter()
        .find(|attribute| attribute["name"] == json!("dateOfBirth"))
        .unwrap();
    assert_eq!(
        date_of_birth["annotations"]["loginHintPrefillPolicy"],
        json!("IGNORE")
    );
    let username = attributes
        .iter()
        .find(|attribute| attribute["name"] == json!("username"))
        .unwrap();
    assert_eq!(
        username["annotations"]["loginHintPrefillPolicy"],
        json!("READ_ONLY")
    );
}

#[test]
fn a_preset_whose_flow_the_realm_lacks_is_warned_about_rather_than_applied_blindly(
) {
    let workbook = with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("digital_certificates"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("has no flow 'certificate-first-login-flow'")));
}

#[test]
fn a_preset_missing_a_required_parameter_refuses_the_build() {
    // The SEIU document's own case: it declares SAML and leaves the IdP metadata
    // URL blank pending the client's identity provider.
    let report = refused(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("saml_sso_idp_initiated"),
            ],
        ],
    ));
    assert!(has_error_saying(
        &report,
        "needs a 'saml_idp_metadata_url' parameter"
    ));
}

#[test]
fn selecting_the_none_preset_ignores_what_the_document_declares() {
    // Which is how a document declaring SAML still builds while the client has not
    // supplied their metadata URL.
    let workbook = with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("saml_sso_idp_initiated"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            auth_preset: Some("none".to_string()),
            ..BuildOptions::default()
        },
    );
    assert_eq!(bundle.auth_preset, None);
    assert!(bundle.realm_patch.bind_authenticator_config.is_none());
}

#[test]
fn a_preset_nobody_wrote_is_refused_with_the_list() {
    let report = refused(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![text("settings"), text("auth_type"), text("magic_link")],
        ],
    ));
    assert!(has_error_saying(&report, "is not an authentication preset"));
    assert!(has_error_saying(&report, "otp_email_or_sms"));
}

#[test]
fn a_preset_that_authenticates_the_voter_elsewhere_stops_asking_for_contacts() {
    // Under SAML the client's IdP authenticates the voter, so "no voter can be
    // sent a code" is noise rather than a finding.
    let mut sheets: Vec<Sheet> = sound().sheets().to_vec();
    sheets.push(
        Sheet::from_grid(
            "Voters",
            &[
                vec![text("username"), text("area.external_id")],
                vec![text("m-1001"), text("area-north")],
            ],
        )
        .unwrap(),
    );
    sheets.push(
        Sheet::from_grid(
            "Parameters",
            &[
                vec![text("type"), text("key"), text("value")],
                vec![
                    text("settings"),
                    text("auth_type"),
                    text("saml_sso_idp_initiated"),
                ],
                vec![
                    text("settings"),
                    text("saml_idp_metadata_url"),
                    text("https://idp.example.org/metadata"),
                ],
            ],
        )
        .unwrap(),
    );
    let bundle = built(&Workbook::new(sheets).unwrap());
    assert!(
        !bundle
            .warnings
            .warnings()
            .any(|problem| problem.message.contains("one-time code")),
        "{}",
        bundle.warnings
    );
}

#[test]
fn the_event_languages_become_the_login_pages() {
    // The platform never syncs supportedLocales, so this is the only thing that
    // puts a language in Keycloak's picker.
    let bundle = built(&with_sheet(
        "ElectionEvent",
        vec![
            vec![
                text("external_id"),
                text("presentation.i18n.en.name"),
                text("presentation.language_conf.enabled_language_codes"),
                text("presentation.language_conf.default_language_code"),
            ],
            vec![
                text("union-2027"),
                text("Union Election 2027"),
                text(r#"["eng", "spa"]"#),
                text("spa"),
            ],
        ],
    ));
    assert_eq!(
        bundle.realm_patch.patch["supportedLocales"],
        json!(["en", "es"])
    );
    assert_eq!(bundle.realm_patch.patch["defaultLocale"], json!("es"));
    assert_eq!(
        bundle.realm_patch.patch["internationalizationEnabled"],
        json!(true)
    );
}

#[test]
fn the_event_title_becomes_the_realms_display_name() {
    // Otherwise every client's voters see "Election Event" above the login form.
    let bundle = built(&sound());
    assert_eq!(
        bundle.realm_patch.patch["displayName"],
        json!("Union Election 2027")
    );
}

#[test]
fn login_css_reaches_every_enabled_language_escaped_for_message_format() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("login_custom_css"),
                text(".logo { display: none; }"),
            ],
        ],
    ));
    let texts = &bundle.realm_patch.patch["localizationTexts"];
    assert_eq!(
        texts["en"]["loginCustomCss"],
        json!(".logo '{' display: none; '}'")
    );
}

#[test]
fn an_explicit_realm_parameter_wins_over_anything_derived() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("keycloak_event_realm.displayName"),
                text("Chosen By Hand"),
            ],
        ],
    ));
    assert_eq!(
        bundle.realm_patch.patch["displayName"],
        json!("Chosen By Hand")
    );
}

#[test]
fn admin_realm_parameters_travel_separately_because_they_are_tenant_scoped() {
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("keycloak_admin_realm.smtpServer.host"),
                text("smtp.example.org"),
            ],
        ],
    ));
    assert_eq!(
        bundle.admin_realm_patch["smtpServer"]["host"],
        json!("smtp.example.org")
    );
    // And not into the event's realm patch.
    assert!(bundle.realm_patch.patch.get("smtpServer").is_none());
}

#[test]
fn a_parameter_a_preset_consumes_is_not_also_reported_as_uninterpreted() {
    // Reporting it as ignored while a preset acts on it contradicts itself.
    let bundle = built(&with_sheet(
        "Parameters",
        vec![
            vec![text("type"), text("key"), text("value")],
            vec![
                text("settings"),
                text("auth_type"),
                text("otp_email_or_sms"),
            ],
            vec![text("settings"), text("otp_length"), Cell::Int(8)],
        ],
    ));
    // Nothing was carried, so annotations stays whatever the template said —
    // null, not an object with the preset's own parameters in it.
    let annotations = &bundle.export["election_event"]["annotations"];
    let carried: Vec<&String> = annotations
        .as_object()
        .map(|annotations| annotations.keys().collect())
        .unwrap_or_default();
    assert!(
        carried.is_empty(),
        "a preset's own parameters were carried as uninterpreted: {carried:?}"
    );
}

// -- permission labels ----------------------------------------------------

#[test]
fn a_permission_label_no_administrator_holds_is_warned_about() {
    // The failure this guards against is quiet and expensive: the event imports
    // cleanly and the Elections list is empty. It happened on the first real
    // import, where a document labelled an election 'dlc-officers-dburs' while its
    // own administrators carried 'dlc-officers'.
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "Elections")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "Elections",
            &[
                vec![text("external_id"), text("permission_label")],
                vec![text("statewide"), text("dlc-officers-dburs")],
            ],
        )
        .unwrap(),
    );
    sheets.push(
        Sheet::from_grid(
            "Admin Users",
            &[
                vec![text("username"), text("permission_labels")],
                vec![text("admin1"), text("dlc-officers")],
            ],
        )
        .unwrap(),
    );

    let bundle = built(&Workbook::new(sheets).unwrap());
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("no administrator in the Admin Users sheet carries it")));
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("permission labels in use")));
}

#[test]
fn a_label_an_administrator_does_hold_is_only_noted_not_flagged() {
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "Elections")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "Elections",
            &[
                vec![text("external_id"), text("permission_label")],
                vec![text("statewide"), text("statewide-officers")],
            ],
        )
        .unwrap(),
    );
    sheets.push(
        Sheet::from_grid(
            "Admin Users",
            &[
                vec![text("username"), text("permission_labels")],
                vec![text("admin1"), text("statewide-officers")],
            ],
        )
        .unwrap(),
    );

    let bundle = built(&Workbook::new(sheets).unwrap());
    assert!(!bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("carries it")));
    // Whoever imports still needs the label on their own attribute.
    assert!(bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("permission labels in use")));
}

#[test]
fn a_document_that_grants_no_labels_at_all_says_so_differently() {
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "Elections")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "Elections",
            &[
                vec![text("external_id"), text("permission_label")],
                vec![text("statewide"), text("statewide-officers")],
            ],
        )
        .unwrap(),
    );
    let bundle = built(&Workbook::new(sheets).unwrap());
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("grants no permission labels to anyone")));
}

#[test]
fn a_document_with_no_labels_says_nothing_about_them() {
    let bundle = built(&sound());
    assert!(!bundle
        .warnings
        .warnings()
        .any(|problem| problem.message.contains("permission label")));
}

// -- inherited branding ---------------------------------------------------

#[test]
fn voter_facing_copy_inherited_from_a_base_export_is_named() {
    // Useful when the base is a reference event and wrong when it is another
    // client's: their login title and instruction copy would come along silently.
    let bundle = with_options(
        &sound(),
        BuildOptions {
            base_export: Some(json!({
                "election_event": {
                    "presentation": {
                        "i18n": {"en": {
                            "login_instructions": "Ring the other client's helpdesk",
                        }},
                        "theme": "other-client",
                    },
                },
            })),
            ..BuildOptions::default()
        },
    );
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("presentation.i18n.en.login_instructions")));
    assert!(bundle.warnings.warnings().any(|problem| problem
        .message
        .contains("presentation settings inherited from the base export")));
}

#[test]
fn copy_the_document_sets_itself_is_not_reported_as_inherited() {
    let mut sheets: Vec<Sheet> = sound()
        .sheets()
        .iter()
        .filter(|sheet| sheet.name != "ElectionEvent")
        .cloned()
        .collect();
    sheets.push(
        Sheet::from_grid(
            "ElectionEvent",
            &[
                vec![
                    text("external_id"),
                    text("presentation.i18n.en.name"),
                    text("presentation.i18n.en.login_instructions"),
                ],
                vec![
                    text("union-2027"),
                    text("Union Election 2027"),
                    text("Ring our helpdesk"),
                ],
            ],
        )
        .unwrap(),
    );

    let bundle = with_options(
        &Workbook::new(sheets).unwrap(),
        BuildOptions {
            base_export: Some(json!({
                "election_event": {"presentation": {"i18n": {"en": {
                    "login_instructions": "Ring the other client's helpdesk",
                }}}},
            })),
            ..BuildOptions::default()
        },
    );
    assert!(
        !bundle
            .warnings
            .warnings()
            .any(|problem| problem.message.contains("login_instructions")),
        "{}",
        bundle.warnings
    );
    assert_eq!(
        bundle.export["election_event"]["presentation"]["i18n"]["en"]
            ["login_instructions"],
        json!("Ring our helpdesk")
    );
}

/// The gap `EA-81` closed: a census column Keycloak never hears about.
///
/// A column the wizard has no field for becomes a **Keycloak user attribute** — that
/// passthrough is why a client can carry a reporting breakout without a code change.
/// But Keycloak only stores an attribute its user profile declares; an undeclared one
/// is dropped or refused depending on the realm's unmanaged-attribute policy. Either
/// way the column is in the file, in the import, and not on the voter, and nothing
/// said so.
#[test]
fn a_census_column_of_its_own_is_declared_in_the_realms_user_profile() {
    let workbook = with_sheet(
        "Voters",
        vec![
            vec![
                text("username"),
                text("email"),
                text("area.external_id"),
                text("branch_code"),
                text("seniority"),
            ],
            vec![
                text("ada"),
                text("ada@example.org"),
                text("area-north"),
                text("B-14"),
                text("1998"),
            ],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );

    let raw = bundle.export["keycloak_event_realm"]["components"]
        ["org.keycloak.userprofile.UserProfileProvider"][0]["config"]
        ["kc.user.profile.config"][0]
        .as_str()
        .unwrap();
    let profile: Value = serde_json::from_str(raw).unwrap();
    let names: Vec<&str> = profile["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|attribute| attribute["name"].as_str().unwrap())
        .collect();

    assert!(names.contains(&"branch_code"), "got {names:?}");
    assert!(names.contains(&"seniority"), "got {names:?}");

    // Readable and writable by an administrator, and nothing more. This knows the
    // column exists and nothing about what belongs in it — a guessed validator or a
    // `required` flag would refuse data the client's own file contains.
    let branch = profile["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|attribute| attribute["name"] == json!("branch_code"))
        .unwrap();
    assert_eq!(branch["permissions"]["edit"], json!(["admin"]));
    assert_eq!(branch["required"], Value::Null);
    assert_eq!(branch["validations"], Value::Null);
}

#[test]
fn the_platforms_own_census_columns_are_not_redeclared() {
    // `username`, `email` and the rest are already in every realm. Adding them again
    // would produce two attributes of one name, and Keycloak reads the first.
    let workbook = with_sheet(
        "Voters",
        vec![
            vec![text("username"), text("email"), text("area.external_id")],
            vec![text("ada"), text("ada@example.org"), text("area-north")],
        ],
    );
    let bundle = with_options(
        &workbook,
        BuildOptions {
            base_export: Some(json!({"keycloak_event_realm": base_realm()})),
            ..BuildOptions::default()
        },
    );

    let raw = bundle.export["keycloak_event_realm"]["components"]
        ["org.keycloak.userprofile.UserProfileProvider"][0]["config"]
        ["kc.user.profile.config"][0]
        .as_str()
        .unwrap();
    let profile: Value = serde_json::from_str(raw).unwrap();
    let usernames = profile["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|attribute| attribute["name"] == json!("username"))
        .count();
    assert_eq!(usernames, 1);
}

/// The point of the locator, asserted where it is actually produced.
///
/// `Problem::path` has said this all along, as the sentence `sheet 'Voters' row 3
/// column 'username'`. A screen that wants to group four hundred complaints by tab
/// and point at a cell would have to parse that back apart, which is the thing the
/// structured field exists to prevent.
#[test]
fn a_workbook_problem_names_the_cell_it_came_from() {
    // A voter row missing the one column the builder requires.
    let workbook = with_sheet(
        "Voters",
        vec![
            vec![Cell::text("username"), Cell::text("email")],
            vec![Cell::Blank, Cell::text("nobody@example.org")],
        ],
    );

    let report = refused(&workbook);
    let located = report
        .problems
        .iter()
        .find(|problem| problem.at.is_some())
        .expect("at least one problem points at a cell");
    let at = located.at.as_ref().unwrap();

    assert_eq!(at.sheet, "Voters");
    assert!(at.row.is_some(), "a row problem has a row");
    // And the sentence still says the same thing, so nothing reading `path` has
    // to change.
    assert!(located.path.contains("sheet 'Voters'"));
}
