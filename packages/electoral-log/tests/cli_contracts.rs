// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Run the actual helper binary. Invalid configuration must fail before any
//! connection, regardless of deployment environment inherited by the test host.

use std::process::{Command, Output};

fn helper(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bb_helper"))
        .env_remove("WBRAID_AUDIT_DATABASE_URL")
        .args(arguments)
        .output()
        .unwrap()
}
#[test]
fn missing_audit_database_is_an_actionable_configuration_error() {
    let output = helper(&["--board-name", "test", "upsert-board"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("WBRAID_AUDIT_DATABASE_URL"));
}
#[test]
fn native_helper_supports_archival_and_rejects_unknown_actions_without_connecting() {
    let output = helper(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("archive-board"));
    assert!(help.contains("upsert-board"));
    assert!(!helper(&["--board-name", "test", "delete-everything"])
        .status
        .success());
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
