// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use std::process::{Command, Output};

fn helper(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_electoral-log-admin"))
        .env_remove("ELECTORAL_LOG_PG_HOST")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn missing_configuration_fails_before_connecting() {
    let output = helper(&["init"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ELECTORAL_LOG_PG_HOST"));
}

#[test]
fn help_and_invalid_actions_do_not_require_a_database() {
    let help = helper(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("create-board"));
    assert!(text.contains("delete-board"));
    assert!(!helper(&["delete-everything"]).status.success());
}

#[test]
fn board_names_remove_uuid_separators_and_keep_tenant_and_event_boundaries() {
    assert_eq!(
        electoral_log::util::get_event_board("tenant-001", "event-002"),
        "tenanttenant001eventevent002"
    );
    assert_ne!(
        electoral_log::util::get_event_board("a", "bc"),
        electoral_log::util::get_event_board("ab", "c")
    );
}
