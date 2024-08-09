// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub fn get_auth_url(
    base_url: &str,
    tenant_id: &str,
    event_id: &str,
    auth_action: &str
) -> String {
    let action = match auth_action {
        "login" => "login",
        "enroll" => "enroll",
        _ => panic!("Invalid auth_action: must be either 'login' or 'enroll'"),
    };

    format!("{base_url}/tenant/{tenant_id}/event/{event_id}/{action}")
}
