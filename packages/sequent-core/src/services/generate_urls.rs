// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[derive(Debug)]
pub enum AuthAction {
    Login,
    Enroll,
    EnrollKiosk
}

impl std::fmt::Display for AuthAction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AuthAction::Login => write!(f, "login"),
            AuthAction::Enroll | AuthAction::EnrollKiosk => write!(f, "enroll")
        }
    }
}

pub fn get_auth_url(
    base_url: &str,
    tenant_id: &str,
    event_id: &str,
    auth_action: AuthAction,
) -> String {
    let base = format!("{base_url}/tenant/{tenant_id}/event/{event_id}/{auth_action}");
    
    match auth_action {
        AuthAction::EnrollKiosk => format!("{}?channel=kiosk", base),
        _ => base
    }
}