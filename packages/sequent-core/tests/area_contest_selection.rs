// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "areas")]

use sequent_core::services::area_tree::{ContestsData, TreeNode, TreeNodeArea};
use sequent_core::types::hasura::core::AreaContest;
use std::collections::HashSet;

fn tree() -> TreeNode<ContestsData> {
    let areas = [
        ("parent", None),
        ("child", Some("parent")),
        ("grandchild", Some("child")),
        ("other-root", None),
    ]
    .into_iter()
    .map(|(id, parent)| TreeNodeArea {
        id: id.into(),
        tenant_id: "test-tenant".into(),
        annotations: None,
        election_event_id: "test-event".into(),
        parent_id: parent.map(str::to_owned),
    })
    .collect();
    let assignments: Vec<AreaContest> = [
        ("parent", "selected"),
        ("child", "local"),
        ("other-root", "unrelated"),
    ]
    .into_iter()
    .map(|(area, contest)| AreaContest {
        id: format!("{area}-{contest}"),
        area_id: area.into(),
        contest_id: contest.into(),
    })
    .collect();
    TreeNode::<()>::from_areas(areas)
        .unwrap()
        .get_contests_data_tree(&assignments)
}

fn matches(
    tree: &TreeNode<ContestsData>,
    selected: &[&str],
) -> HashSet<(String, String)> {
    let selected = selected.iter().map(|id| (*id).to_owned()).collect();
    tree.get_contest_matches(&selected)
        .into_iter()
        .map(|row| (row.area_id, row.contest_id))
        .collect()
}

fn expected(rows: &[(&str, &str)]) -> HashSet<(String, String)> {
    rows.iter()
        .map(|(area, contest)| ((*area).into(), (*contest).into()))
        .collect()
}

#[test]
fn requested_contests_include_direct_and_inherited_assignments_only() {
    let tree = tree();
    assert_eq!(
        matches(&tree, &["selected"]),
        expected(&[
            ("parent", "selected"),
            ("child", "selected"),
            ("grandchild", "selected"),
        ])
    );
    assert_eq!(
        matches(&tree, &["local"]),
        expected(&[("child", "local"), ("grandchild", "local")])
    );
}

#[test]
fn empty_or_unknown_selection_has_no_matches() {
    let tree = tree();
    assert!(matches(&tree, &[]).is_empty());
    assert!(matches(&tree, &["unknown"]).is_empty());
}

#[test]
fn multiple_requested_contests_exclude_other_contests() {
    assert_eq!(
        matches(&tree(), &["selected", "local", "unknown"]),
        expected(&[
            ("parent", "selected"),
            ("child", "selected"),
            ("grandchild", "selected"),
            ("child", "local"),
            ("grandchild", "local"),
        ])
    );
}
