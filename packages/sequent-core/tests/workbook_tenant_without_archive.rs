// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(all(
    feature = "election_config_templates",
    not(feature = "election_config_archive")
))]

use sequent_core::election_config::{
    build, sources::Sources, BuildOptions, Cell, Sheet, TemplateSet, Workbook,
};

#[test]
fn tenant_values_do_not_become_event_annotations_without_archives() {
    let sheets = [
        (
            "ElectionEvent",
            vec![
                vec!["external_id", "presentation.i18n.en.name"],
                vec!["example-event", "Example Event"],
            ],
        ),
        (
            "Elections",
            vec![
                vec!["external_id", "presentation.i18n.en.name"],
                vec!["example-election", "Example Election"],
            ],
        ),
        (
            "Contests",
            vec![
                vec![
                    "external_id",
                    "election.external_id",
                    "presentation.i18n.en.name",
                    "max_votes",
                ],
                vec!["contest", "example-election", "Example contest", "1"],
            ],
        ),
        (
            "Candidates",
            vec![
                vec![
                    "external_id",
                    "contest.external_id",
                    "presentation.i18n.en.name",
                ],
                vec!["candidate", "contest", "Example candidate"],
            ],
        ),
        (
            "Areas",
            vec![vec!["external_id", "name"], vec!["area", "Area"]],
        ),
        (
            "AreaContests",
            vec![
                vec!["area.external_id", "contest.external_id"],
                vec!["area", "contest"],
            ],
        ),
        (
            "Parameters",
            vec![
                vec!["key", "value"],
                vec![
                    "tenant_package",
                    r#"{"privateSentinel":"should never export"}"#,
                ],
            ],
        ),
    ]
    .into_iter()
    .map(|(name, rows)| {
        let cells: Vec<Vec<Cell>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(Cell::text).collect())
            .collect();
        Sheet::from_grid(name, &cells).unwrap()
    })
    .collect();
    let workbook = Workbook::new(sheets).unwrap();
    let built = build(
        &workbook,
        &TemplateSet::builtin().unwrap(),
        &BuildOptions::default(),
        &Sources::default(),
    )
    .unwrap();
    assert!(built
        .warnings
        .problems
        .iter()
        .any(|p| p.path == "tenant_package"));
    assert!(!built.export.to_string().contains("privateSentinel"));
}
