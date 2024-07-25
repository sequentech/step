// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub fn get_login_url(
    base_url: &str,
    tenant_id: &str,
    event_id: &str,
) -> String {
    format!("{base_url}/tenant/{tenant_id}/event/{event_id}/login",)
}
