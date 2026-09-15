// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Run the actual helper binary. Invalid configuration must fail before any
//! connection, regardless of deployment environment inherited by the test host.

use std::process::{Command, Output};

fn helper(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bb_helper"))
        .env_remove("IMMUDB_SERVER_URL")
        .env_remove("IMMUDB_BOARD_DBNAME")
        .env_remove("IMMUDB_USERNAME")
        .env_remove("IMMUDB_PASSWORD")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn missing_settings_fail_in_the_order_the_operator_can_fix_them() {
    let cases = [
        (vec!["--cache-dir", "/unused"], "IMMUDB_SERVER_URL"),
        (
            vec![
                "--cache-dir",
                "/unused",
                "--server-url",
                "http://127.0.0.1:1",
            ],
            "IMMUDB_BOARD_DBNAME",
        ),
        (
            vec![
                "--cache-dir",
                "/unused",
                "--server-url",
                "http://127.0.0.1:1",
                "--board-dbname",
                "test",
            ],
            "IMMUDB_USERNAME",
        ),
        (
            vec![
                "--cache-dir",
                "/unused",
                "--server-url",
                "http://127.0.0.1:1",
                "--board-dbname",
                "test",
                "--username",
                "test",
            ],
            "IMMUDB_PASSWORD",
        ),
    ];
    for (arguments, missing) in cases {
        let output = helper(&arguments);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(missing),
            "{output:?}"
        );
    }
}

#[test]
fn help_and_invalid_actions_do_not_require_a_database() {
    let help = helper(&["--help"]);
    assert!(help.status.success());
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("upsert-board-db"));
    assert!(text.contains("delete-board-db"));
    assert!(!helper(&["--cache-dir", "/unused", "delete-everything"])
        .status
        .success());
}

#[test]
fn every_supported_log_level_keeps_configuration_errors_visible() {
    for level in ["off", "error", "warn", "info", "debug", "trace"] {
        let output = helper(&["--cache-dir", "/unused", "--log-level", level]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("IMMUDB_SERVER_URL"));
    }
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
