// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[path = "support/http_cli.rs"]
mod http_cli;
use http_cli::command;
use serde_json::json;

#[test]
fn refreshing_access_preserves_omitted_or_empty_refresh_tokens_and_accepts_rotation() {
    for (response, expected) in [
        (json!({"access_token":"fresh-access"}), "stored-refresh"),
        (
            json!({"access_token":"fresh-access","refresh_token":""}),
            "stored-refresh",
        ),
        (
            json!({"access_token":"fresh-access","refresh_token":"rotated-refresh"}),
            "rotated-refresh",
        ),
    ] {
        let (output, config, requests) = command(&["refresh-token"], Some(response));
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("Configuration refreshed successfully"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(config["auth_token"], "fresh-access");
        assert_eq!(config["refresh_token"], expected);
        assert_eq!(config["username"], "operator");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].1, b"grant_type=refresh_token&client_id=client&client_secret=secret&refresh_token=stored-refresh");
    }
}
