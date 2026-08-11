// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One door, three formats — and the reason the telling-apart is in Rust.

use super::*;
use crate::election_config::architect::{
    compile_plan, to_workbook, BLUEPRINT_VERSION,
};
use crate::election_config::build::BuildOptions;
use crate::election_config::render::TemplateSet;
use crate::election_config::xlsx_write::write_xlsx;

fn sound() -> Blueprint {
    serde_json::from_value(serde_json::json!({
        "version": BLUEPRINT_VERSION,
        "external_id": "union-2027",
        "name": {"en": "Union Election 2027"},
        "languages": ["en"],
        "trustees": [
            {"name": "Ada Lovelace", "email": "ada@example.org"},
            {"name": "Grace Hopper", "email": "grace@example.org"}
        ],
        "elections": [{
            "external_id": "officers",
            "name": {"en": "Officers"},
            "contests": [{
                "external_id": "president",
                "name": {"en": "President"},
                "max_votes": 1,
                "winners": 1,
                "candidates": [
                    {"external_id": "alice", "name": {"en": "Alice Okonjo"}},
                    {"external_id": "bob", "name": {"en": "Bob Iyer"}}
                ]
            }]
        }]
    }))
    .unwrap()
}

fn delivery_of(plan: &Blueprint) -> Vec<u8> {
    let compiled = compile_plan(
        plan,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        None,
    )
    .expect("the sample plan compiles");
    super::super::archive::delivery(&compiled.layout)
        .expect("and packs")
        .bytes
}

#[test]
fn a_delivery_opens_as_the_plan_that_built_it() {
    let plan = sound();
    let opened = open(&delivery_of(&plan)).expect("a delivery is a plan");

    assert_eq!(opened.source, Source::Delivery);
    assert_eq!(opened.plan.external_id, plan.external_id);
    assert_eq!(opened.plan.trustees.len(), 2);
}

#[test]
fn a_bare_plan_file_opens() {
    let plan = sound();
    let text = serde_json::to_vec(&plan).unwrap();

    let opened = open(&text).expect("a plan is a plan");
    assert_eq!(opened.source, Source::Plan);
    assert_eq!(opened.plan.external_id, "union-2027");
}

/// The case that cannot be told apart outside Rust.
///
/// A spreadsheet has the same `PK` magic as a delivery, so a front end sniffing
/// bytes hands it to the delivery reader, which looks for `blueprint.json`, does
/// not find it, and reports a broken delivery about a perfectly good workbook.
#[test]
fn a_workbook_opens_as_a_plan_rather_than_as_a_broken_delivery() {
    let plan = sound();
    let bytes = write_xlsx(&to_workbook(&plan).unwrap()).unwrap();

    assert!(
        is_zip(&bytes),
        "an .xlsx is a zip, which is the whole problem"
    );

    let opened = open(&bytes).expect("and it is a workbook");
    assert_eq!(opened.source, Source::Workbook);
    assert_eq!(opened.plan.external_id, "union-2027");
    assert_eq!(opened.plan.elections[0].contests[0].candidates.len(), 2);
}

/// The workbook inside a delivery opens as one.
///
/// The round trip a client actually does: build, unzip, edit the spreadsheet, open
/// it again.
#[test]
fn the_workbook_inside_a_delivery_opens_on_its_own() {
    let plan = sound();
    let bytes = delivery_of(&plan);

    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let mut workbook = Vec::new();
    {
        use std::io::Read;
        let mut member = archive
            .by_name(super::super::archive::WORKBOOK_MEMBER)
            .expect("a delivery carries the spreadsheet");
        member.read_to_end(&mut workbook).unwrap();
    }

    let opened = open(&workbook).expect("and it opens on its own");
    assert_eq!(opened.source, Source::Workbook);
    assert_eq!(opened.plan.external_id, plan.external_id);
}

#[test]
fn an_empty_zip_says_what_it_contained_rather_than_guessing() {
    // The end-of-central-directory record and nothing else — the signature that
    // used to reach `JSON.parse` and produce "Unexpected token 'P'".
    let empty: Vec<u8> = vec![
        0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    let refused =
        open(&empty).expect_err("an empty zip is not a configuration");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.message.contains("neither a delivery nor a")));
}

#[test]
fn text_that_is_not_a_plan_is_refused_readably() {
    let refused =
        open(b"hello, world").expect_err("that is not a configuration");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.message.contains("not a plan file")));
}

#[test]
fn bytes_that_are_nothing_at_all_are_refused() {
    let refused = open(&[0xff, 0xfe, 0x00, 0x01])
        .expect_err("not a zip and not text either");
    assert!(refused.has_errors());
}

/// A broken workbook comes back as a report, not as an exception.
///
/// The rule the whole module follows: a report renders on a screen and an
/// exception does not.
#[test]
fn a_workbook_full_of_problems_comes_back_as_a_report() {
    let broken = crate::election_config::sheet::Workbook::new(vec![
        crate::election_config::sheet::Sheet::from_grid(
            "ElectionEvent",
            &[
                vec![crate::election_config::paths::Cell::text("external_id")],
                vec![crate::election_config::paths::Cell::text("union-2027")],
            ],
        )
        .unwrap(),
        crate::election_config::sheet::Sheet::from_grid(
            "Contests",
            &[
                vec![
                    crate::election_config::paths::Cell::text("external_id"),
                    crate::election_config::paths::Cell::text(
                        "election.external_id",
                    ),
                ],
                vec![
                    crate::election_config::paths::Cell::text("president"),
                    crate::election_config::paths::Cell::text("nowhere"),
                ],
            ],
        )
        .unwrap(),
    ])
    .unwrap();

    let bytes = write_xlsx(&broken).unwrap();
    let refused = open(&bytes).expect_err("a contest with no election");

    let problem = refused
        .problems
        .iter()
        .find(|problem| problem.code == Code::DanglingReference)
        .expect("it says which reference dangles");
    let at = problem.at.as_ref().expect("and points at the cell");
    assert_eq!(at.sheet, "Contests");
}

/// Warnings load; they do not stop anything.
#[test]
fn a_workbook_with_only_warnings_opens_and_says_so() {
    let mut plan = sound();
    plan.logo = Some(crate::election_config::architect::CandidateImage {
        file_name: "logo.png".to_string(),
        bytes: vec![1, 2, 3],
    });
    let bytes = write_xlsx(&to_workbook(&plan).unwrap()).unwrap();

    let opened = open(&bytes).expect("a lost image does not stop a load");
    assert!(!opened.report.has_errors());
    assert!(
        opened
            .report
            .warnings()
            .any(|problem| problem.message.contains("logo.png")),
        "and it names what could not travel"
    );
}
