// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "reports")]

use sequent_core::services::reports::render_template_text;
use serde_json::{json, Map, Value};

fn render_next(index: u64) -> String {
    let variables: Map<String, Value> = Map::from_iter([
        ("values".into(), json!(["first", "second"])),
        ("index".into(), json!(index)),
    ]);
    render_template_text("{{next values index}}", variables).unwrap()
}

#[test]
fn next_item_uses_the_successor_and_returns_empty_at_the_end() {
    assert_eq!(render_next(0), "second");
    assert_eq!(render_next(1), "");
    assert_eq!(render_next(2), "");
}

#[test]
fn an_unrepresentable_successor_is_absent() {
    assert_eq!(render_next(u64::MAX), "");
    assert_eq!(render_next(u32::MAX.into()), "");
}
