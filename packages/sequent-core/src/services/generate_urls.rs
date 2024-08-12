// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[derive(Debug)]
pub enum AuthAction {
    Login,
    Enroll,
}

pub fn get_auth_url(
    base_url: &str,
    tenant_id: &str,
    event_id: &str,
    auth_action: AuthAction,
) -> String {
    let action_str = match auth_action {
        AuthAction::Login => "login",
        AuthAction::Enroll => "enroll",
    };

    format!("{base_url}/tenant/{tenant_id}/event/{event_id}/{action_str}")
}
